//! A task runner for Cargo
#![forbid(unsafe_code)]

use clap::Parser;
use std::path::{Path, PathBuf};
use std::{fs, io};

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

fn main() -> Result<(), Error> {
    // 1. Parse command line arguments
    let args = Args::parse();
    assert_eq!(
        args.argv0, "task",
        "cargo-task should be invoked as a `cargo` subcommand"
    );

    // 2. Find task in tasks directory
    let dir = findup_workspace(&std::env::current_dir()?)?;

    let task_rs = dir.join("tasks").join(format!("{}.rs", args.task_name));
    assert!(&task_rs.exists());

    let target_tasks_dir = dir.join("target/tasks");
    fs::create_dir_all(&target_tasks_dir)?;

    // 3. Find rustc
    std::process::Command::new("rustc")
        .arg("+beta")
        .arg("--target")
        .arg("wasm32-wasip2")
        .arg("-Ccodegen-units=1")
        .arg(task_rs)
        .arg("-o")
        .arg(target_tasks_dir.join(format!("{}.wasm", args.task_name)))
        .spawn()?;

    // 4. Compile the task

    // 5. Run the task

    Ok(())
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
