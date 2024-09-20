//! A task runner for Cargo
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

use clap::Parser;
use fs_extra::dir::CopyOptions;
use wasmtime::{component::Component, Result, *};
use wasmtime_wasi::bindings::Command;
use wasmtime_wasi::{DirPerms, FilePerms, ResourceTable, WasiView};

mod cargo;

use cargo::{CargoToml, InheritEnv, TaskDetail};

/// A sandboxed task runner for cargo
#[derive(clap::Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The `task` subcommand
    #[arg(hide = true)]
    argv0: String,
    /// The name of the task to run
    task_name: String,
    /// Optional arguments to pass to the task
    args: Vec<String>,
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// The shared context for our component instantiation.
///
/// Each store owns one of these structs. In the linker this maps: names in the
/// component -> functions on the host side.
struct Ctx {
    // Anything that WASI can access is mediated though this. This contains
    // capabilities, preopens, etc.
    wasi: wasmtime_wasi::WasiCtx,
    // NOTE: this might go away eventually
    // We need something which owns the host representation of the resources; we
    // store them in here. Think of it as a `HashMap<i32, Box<dyn Any>>`
    table: wasmtime::component::ResourceTable,
}
impl WasiView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.wasi
    }
}

/// A representation of a task on-disk.
///
/// Tasks are assumed to either exist in the `tasks/` subdirectory,
/// or can be defined in `Cargo.toml` as part of the `[tasks]` section.
#[derive(Debug, PartialEq, Clone)]
struct TaskDefinition {
    name: String,
    path: PathBuf,
    env: EnvVars,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Parse command line arguments
    let args = Args::parse();
    assert_eq!(
        args.argv0, "task",
        "cargo-task-wasm should be invoked as a `cargo` subcommand"
    );

    // Find task in tasks directory
    let workspace_root_dir = findup_workspace(&std::env::current_dir()?)?;

    let cargo_toml_path = workspace_root_dir.join("Cargo.toml");
    let cargo_toml: CargoToml = toml::from_str(&fs::read_to_string(&cargo_toml_path)?)?;

    let target_workspace_dir = build_task_workspace(
        &cargo_toml,
        &workspace_root_dir.join("target/tasks"),
        &workspace_root_dir.join("tasks"),
    )?;

    // Compile the task
    // TODO: change this to use `cargo` instead!
    // TODO: tell cargo to use a different target directory inside of our
    //       target directory, use the manifest-path

    let tasks_target_dir = target_workspace_dir.join("../target");

    let _cargo_status = std::process::Command::new("cargo")
        .arg("+beta")
        .arg("build")
        .arg("--target")
        .arg("wasm32-wasip2")
        .current_dir(&target_workspace_dir)
        .arg("--target-dir")
        .arg(&tasks_target_dir)
        .status()?;

    let tasks = resolve_tasks(&cargo_toml, &workspace_root_dir.join("tasks"))?;
    let task_definition = tasks.get(&args.task_name).expect(&format!(
        "could not find a task with the name {:?}",
        &args.task_name
    ));

    let task_wasm_path =
        tasks_target_dir.join(format!("wasm32-wasip2/debug/{}.wasm", task_definition.name));

    // Ok, it's time to setup Wasmtime and load our component. This goes through two phases:
    // 1. Load the program and link it - this is done once and can be reused multiple times
    //    between various instances. In our program here though we'll just create a single instance.
    // 2. Create an instance of the program and give it its own memory, etc. This is a working copy
    //    of the program and we need to give it some of its own state to work with.

    // Setup the engine.
    // These pieces can be reused for multiple component instantiations.
    let mut config = Config::default();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, task_wasm_path)?;

    // Setup the linker and add the `wasi:cli/command` world's imports to this
    // linker.
    let mut linker: component::Linker<Ctx> = component::Linker::new(&engine);
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    // Instantiate the component!
    let mut wasi = wasmtime_wasi::WasiCtxBuilder::new();
    wasi.inherit_stderr();
    wasi.inherit_stdout();

    match &task_definition.env {
        EnvVars::None => {}
        EnvVars::All => {
            wasi.inherit_env();
        }
        EnvVars::AllowList(vars) => {
            for (key, value) in vars {
                eprintln!("setting environment variable: {key} = {value}");
                wasi.env(key, value);
            }
        }
    }

    wasi.args(&args.args);
    wasi.preopened_dir(
        &target_workspace_dir,
        "/",
        DirPerms::all(),
        FilePerms::all(),
    )?;
    let host = Ctx {
        wasi: wasi.build(),
        table: wasmtime::component::ResourceTable::new(),
    };
    let mut store: Store<Ctx> = Store::new(&engine, host);

    // Instantiate the component and we're off to the races.
    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    let program_result = command.wasi_cli_run().call_run(&mut store).await?;
    match program_result {
        Ok(()) => Ok(()),
        Err(()) => std::process::exit(1),
    }
}

/// Recurse upwards in the directory structure until we find a
/// directory containing a `Cargo.toml` file.
///
/// This does not yet find the root of the workspace, but instead will
/// abort on the first `Cargo.toml` it finds.
fn findup_workspace(entry: &Path) -> io::Result<PathBuf> {
    if entry.join("Cargo.toml").exists() {
        return Ok(entry.to_path_buf());
    }

    let parent = entry
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No Cargo.toml found"))?;

    findup_workspace(parent)
}

/// Which environment variables should we map?
#[derive(Debug, PartialEq, Clone)]
enum EnvVars {
    All,
    None,
    AllowList(Vec<(String, String)>),
}

/// Construct the environment variables from the task details.
fn build_sandbox_env(task_details: &TaskDetail) -> EnvVars {
    let Some(inherit_env) = &task_details.inherit_env else {
        return EnvVars::None;
    };

    match inherit_env {
        InheritEnv::Bool(true) => EnvVars::All,
        InheritEnv::Bool(false) => EnvVars::None,
        InheritEnv::AllowList(vars) => {
            let mut map = Vec::new();
            for var in vars {
                if let Ok(value) = std::env::var(&var) {
                    map.push((var.clone(), value));
                }
            }
            EnvVars::AllowList(map)
        }
    }
}

fn build_task_workspace(
    cargo_toml: &CargoToml,
    root_dir: &Path,
    input_tasks_dir: &Path,
) -> Result<PathBuf, Error> {
    use std::fmt::Write;

    let workspace_dir = root_dir.join("ws");
    if workspace_dir.exists() {
        fs::remove_dir_all(&workspace_dir)?;
    }
    fs::create_dir_all(&workspace_dir)?;
    let mut virtual_cargo_toml_contents = String::new();

    writeln!(
        virtual_cargo_toml_contents,
        r#"
    [package]
    name = "_tasks"
    version = "0.1.0"
    edition = "2021"
    "#
    )?;

    // Copy the tasks' rs files into the workspace
    let src_dir = workspace_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    fs_extra::dir::copy(
        input_tasks_dir,
        src_dir,
        &CopyOptions::new().content_only(true),
    )?;

    let tasks = resolve_tasks(cargo_toml, input_tasks_dir)?;

    for task_definition in tasks.values() {
        let task_name = &task_definition.name;

        writeln!(
            virtual_cargo_toml_contents,
            r#"
            [[bin]]
            name = "{task_name}"
            path = "src/{task_name}.rs"
            "#
        )?;
    }

    if let Some(metadata) = &cargo_toml.package.metadata {
        if let Some(task_deps) = &metadata.task_dependencies {
            // Add task dependencies as normal dependencies for this workspace
            writeln!(virtual_cargo_toml_contents, "[dependencies]")?;
            for (dep_name, dep_version) in task_deps.iter() {
                writeln!(
                    virtual_cargo_toml_contents,
                    r#"{dep_name} = "{dep_version}""#
                )?;
            }
        }
    }

    std::fs::write(
        workspace_dir.join("Cargo.toml"),
        virtual_cargo_toml_contents,
    )?;

    Ok(workspace_dir)
}

/// Try and find a task by name on disk. This can either originate from the
/// `tasks/` subdirectory, or a custom path defined in `Cargo.toml` as part of
/// `[tasks]`.
fn resolve_tasks(
    cargo_toml: &CargoToml,
    tasks_dir: &Path,
) -> io::Result<BTreeMap<String, TaskDefinition>> {
    let mut tasks = BTreeMap::new();

    let default_path = |task_name: &str| PathBuf::from(format!("tasks/{task_name}.rs"));

    if let Some(metadata) = &cargo_toml.package.metadata {
        if let Some(toml_tasks) = &metadata.tasks {
            for (task_name, task_details) in toml_tasks {
                let task_path = default_path(&task_name[..]);
                let task_env = build_sandbox_env(task_details);

                tasks.insert(
                    task_name.to_string(),
                    TaskDefinition {
                        name: task_name.to_string(),
                        path: task_path,
                        env: task_env,
                    },
                );
            }
        }
    }

    for entry in fs::read_dir(tasks_dir)? {
        if let Ok(entry) = entry {
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();
            if let Some(task_name) = filename.strip_suffix(".rs") {
                if tasks.contains_key(task_name) {
                    continue;
                }

                tasks.insert(
                    task_name.to_string(),
                    TaskDefinition {
                        name: task_name.to_string(),
                        path: default_path(&task_name[..]),
                        env: EnvVars::None,
                    },
                );
            }
        }
    }

    Ok(tasks)
}
