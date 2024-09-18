//! A task runner for Cargo
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::{fs, io};

use clap::Parser;
use wasmtime::{component::Component, Result, *};
use wasmtime_wasi::bindings::Command;
use wasmtime_wasi::{DirPerms, FilePerms, ResourceTable, WasiView};

mod cargo;

use cargo::{CargoToml, InheritEnv, Permissions};

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
        "cargo-task should be invoked as a `cargo` subcommand"
    );

    // Find task in tasks directory
    let workspace_dir = findup_workspace(&std::env::current_dir()?)?;

    let cargo_toml_path = workspace_dir.join("Cargo.toml");
    let cargo_toml: CargoToml = toml::from_str(&fs::read_to_string(&cargo_toml_path)?)?;
    dbg!(&cargo_toml);

    let task_definition = resolve_task(&args.task_name, &cargo_toml, &workspace_dir)?;
    dbg!(&task_definition);

    // let task_rs = workspace_dir
    //     .join("tasks")
    //     .join(format!("{}.rs", args.task_name));
    // if !task_rs.exists() {
    //     panic!("could not find {:#?}", task_rs);
    // }

    // Create the output dir for the compiled task
    let target_tasks_dir = workspace_dir.join("target/tasks");
    fs::create_dir_all(&target_tasks_dir)?;

    // Compile the task
    let task_wasm_path = target_tasks_dir.join(format!("{}.wasm", task_definition.name));
    let _rustc_status = std::process::Command::new("rustc")
        .arg("+beta")
        .arg("--target")
        .arg("wasm32-wasip2")
        .arg("-Ccodegen-units=1")
        .arg(&task_definition.path)
        .arg("-o")
        .arg(&task_wasm_path)
        .status()?;

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

    match task_definition.env {
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
    wasi.preopened_dir(&workspace_dir, "/", DirPerms::all(), FilePerms::all())?;
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

/// Try and find a task by name on disk. This can either originate from the
/// `tasks/` subdirectory, or a custom path defined in `Cargo.toml` as part of
/// `[tasks]`.
fn resolve_task(
    task_name_to_look_up: &str,
    cargo_toml: &CargoToml,
    workspace_dir: &Path,
) -> io::Result<TaskDefinition> {
    let default_path = || PathBuf::from(format!("tasks/{task_name_to_look_up}.rs"));

    if let Some(task_details) = cargo_toml.tasks.get(task_name_to_look_up) {
        let task_path = match &task_details.path {
            Some(task_path) => PathBuf::from(task_path),
            None => default_path(),
        };
        return Ok(TaskDefinition {
            name: task_name_to_look_up.to_string(),
            path: task_path,
            env: dbg!(build_sandbox_env(&task_details.permissions)),
        });
    }

    let task_path = default_path();
    if workspace_dir.join(&task_path).exists() {
        return Ok(TaskDefinition {
            name: task_name_to_look_up.to_string(),
            path: task_path,
            env: EnvVars::None,
        });
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Task `{}` not found", task_name_to_look_up),
    ))
}

/// Which environment variables should we map?
#[derive(Debug, PartialEq, Clone)]
enum EnvVars {
    All,
    None,
    AllowList(Vec<(String, String)>),
}

/// Construct the environment variables from the list of permissions.
fn build_sandbox_env(permissions: &Option<Permissions>) -> EnvVars {
    let Some(permissions) = &permissions else {
        eprintln!("build_sandbox_env: permissions are None");
        return EnvVars::None;
    };

    let Some(inherit_env) = &permissions.inherit_env else {
        eprintln!("build_sandbox_env: inherit_env is None");
        return EnvVars::None;
    };

    eprintln!("build_sandbox_env: inherit_env = {:?}", inherit_env);

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
