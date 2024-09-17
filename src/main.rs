//! A task runner for Cargo

#![forbid(unsafe_code)]

use clap::Parser;
use std::io;
use std::path::{Path, PathBuf};

#[derive(clap::Parser, Debug)]
struct Args {
    sub_command: String,
    task_name: String,
}

fn main() {
    // 1. Parse command line arguments
    let args = Args::try_parse().unwrap();
    assert_eq!(
        args.sub_command, "task",
        "cargo-task should be invoked as a `cargo` subcommand"
    );

    dbg!(&args);

    // 2. Find task in tasks directory
    let dir = findup_workspace(&std::env::current_dir().unwrap()).unwrap();
    dbg!(&dir);

    let task_rs = dir.join("tasks").join(format!("{}.rs", args.task_name));
    assert!(dbg!(&task_rs).exists());

    let target_tasks_dir = dir.join("target/tasks");
    std::fs::create_dir_all(&target_tasks_dir).unwrap();

    // 3. Find rustc

    let mut rustc = std::process::Command::new("rustc");
    rustc
        .arg("+beta")
        .arg("--target")
        .arg("wasm32-wasip2")
        .arg("-Ccodegen-units=1")
        .arg(task_rs)
        .arg("-o")
        .arg(target_tasks_dir.join(format!("{}.wasm", args.task_name)));
    dbg!(rustc.output());

    // 4. Compile the task

    // 5. Run the task
}

fn findup_workspace(entry: &Path) -> io::Result<PathBuf> {
    if entry.join("Cargo.toml").exists() {
        return Ok(entry.to_path_buf());
    }

    let parent = entry
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No Cargo.toml found"))?;

    findup_workspace(parent)
}
