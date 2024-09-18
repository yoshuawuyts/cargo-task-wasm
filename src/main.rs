//! A task runner for Cargo
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::{fs, io};

use clap::Parser;
use wasmtime::{component::Component, Result, *};
use wasmtime_wasi::bindings::Command;
use wasmtime_wasi::{ResourceTable, WasiView};

/// A sandboxed task runner for cargo
#[derive(clap::Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The `task` subcommand
    #[arg(hide = true)]
    argv0: String,
    /// The name of the task to run
    task_name: String,
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Parse command line arguments
    let args = Args::parse();
    assert_eq!(
        args.argv0, "task",
        "cargo-task should be invoked as a `cargo` subcommand"
    );

    // Find task in tasks directory
    let dir = findup_workspace(&std::env::current_dir()?)?;
    let task_rs = dir.join("tasks").join(format!("{}.rs", args.task_name));
    assert!(&task_rs.exists());

    // Create the output dir for the compiled task
    let target_tasks_dir = dir.join("target/tasks");
    fs::create_dir_all(&target_tasks_dir)?;

    // Compile the task
    let task_wasm_path = target_tasks_dir.join(format!("{}.wasm", args.task_name));

    let _rustc_status = std::process::Command::new("rustc")
        .arg("+beta")
        .arg("--target")
        .arg("wasm32-wasip2")
        .arg("-Ccodegen-units=1")
        .arg(task_rs)
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
    let pre = linker.instantiate_pre(&component)?;

    // Instantiate the component!
    let host = Ctx {
        wasi: wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stderr()
            .inherit_stdout()
            .inherit_network()
            .build(),
        table: wasmtime::component::ResourceTable::new(),
    };
    let mut store: Store<Ctx> = Store::new(&engine, host);

    // Instantiate the component and we're off to the races.
    let (command, _instance) = Command::instantiate_pre(&mut store, &pre).await?;
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
