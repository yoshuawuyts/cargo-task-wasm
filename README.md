<h1 align="center">cargo-task-wasm</h1>
<div align="center">
  <strong>
    A secure `cargo task` subcommand for Rust
  </strong>
</div>

 <br />

<div align="center">
  <a href="https://crates.io/crates/cargo-task-wasm">
    <img src="https://img.shields.io/crates/v/cargo-task-wasm.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <a href="https://crates.io/crates/cargo-task-wasm">
    <img src="https://img.shields.io/crates/d/cargo-task-wasm.svg?style=flat-square"
      alt="Download" />
  </a>
  <a href="https://docs.rs/cargo-task-wasm">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://docs.rs/cargo-task-wasm">
      API Docs
    </a>
    <span> | </span>
    <a href="https://github.com/yoshuawuyts/cargo-task-wasm/releases">
      Releases
    </a>
    <span> | </span>
    <a href="https://github.com/yoshuawuyts/cargo-task-wasm/blob/master.github/CONTRIBUTING.md">
      Contributing
    </a>
  </h3>
</div>

## About

This project provides a new `cargo task` subcommand that can be used to run
project-local tasks inside a secure WebAssembly sandbox. It looks for files in a
`tasks/` subdirectory of your project's root, and compiles those to [Wasm
Components](https://component-model.bytecodealliance.org). This is an attempt at
formalizing [cargo-xtask](https://github.com/matklad/cargo-xtask) pattern into a
secure, first-class workflow.

## Roadmap

- [x] Sketch out a repository layout or whatever workflow example
- [x] Create a new `cargo` subcommand
- [x] Hook up wasmtime to the subcommand
- [x] Add support for manual paths in a `[tasks]` section in `Cargo.toml`
- [x] Figure out how to configure capabilities for the tasks
- [x] Add support for compiling cargo deps as part of subcommands
- [x] Store config in Cargo metadata section
- [ ] Add support for installing tasks from crates.io
- [ ] Add the remainder of the permissions
- [ ] Support workspaces and [`[workspace.metadata]`](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-metadata-table)

## Installation

The `cargo task` subcommand compiles Rust to Wasm Components targeting [WASI
0.2](https://wasi.dev). In order to do that a working WASI 0.2 toolchain needs
to be present on the host system.

```sh
$ rustup +beta target add wasip2  # Install the WASI 0.2 target
$ cargo install cargo-task-wasm   # Install the `cargo task` subcommand
```

## Usage

```text
Usage: cargo task <TASK_NAME> [ARGS]...

Arguments:
  <TASK_NAME>  The name of the task to run
  [ARGS]...    Optional arguments to pass to the task

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Configuration

### Permissions

Tasks in `cargo task` follow the [principle of least
privilege](https://en.wikipedia.org/wiki/Principle_of_least_privilege). By
default they only get access to the working directory, and can access any
additional command line arguments passed to it. Additional permissions can be
configured via a `[package.metadata.tasks]` section in `Cargo.toml`.

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2021"

[package.metadata.task-dependencies]
wstd = "0.4.0"

[package.metadata.tasks]
env = { inherit-env = ["FOO"] }   # inherit specific env vars
```

The reason why this is stored in `[package.metadata.tasks]` rather than a
top-level `[tasks]` section is because that is [the canonical extension
point](https://doc.rust-lang.org/cargo/reference/manifest.html#the-metadata-table)
Cargo recommends using for third-party extensions. Should a `cargo tasks`
command ever become a first-class extension to Cargo, the `package.metadata`
prefix can be dropped.

### Paths

By default the `cargo task` subcommand will look for commands in the `tasks/`
directory of the workspace. However, it is also able to find tasks located in
other locations by specifying custom paths.

```toml
[tasks]
print = { path = "tasks/print.rs" }   # define a custom path for the task
```

## See Also

- [Custom tasks in Cargo (Aaron Turon, 2018)](http://aturon.github.io/tech/2018/04/05/workflows/) - First proposed a `cargo task` subcommand for custom tasks.
- [`matklad/cargo-xtask` (Alex Kladov, 2019)](https://github.com/matklad/cargo-xtask) - A convention-based implementation of `cargo task`.
- [`dtolnay/watt` (David Tolnay 2019)](https://github.com/dtolnay/watt) - Executing Rust procedural macros compiled as WebAssembly.

## Safety
This crate uses ``#![deny(unsafe_code)]`` to ensure everything is implemented in
100% Safe Rust.

## Contributing
Want to join us? Check out our ["Contributing" guide][contributing] and take a
look at some of these issues:

- [Issues labeled "good first issue"][good-first-issue]
- [Issues labeled "help wanted"][help-wanted]

[contributing]: https://github.com/yoshuawuyts/cargo-task-wasm/blob/master.github/CONTRIBUTING.md
[good-first-issue]: https://github.com/yoshuawuyts/cargo-task-wasm/labels/good%20first%20issue
[help-wanted]: https://github.com/yoshuawuyts/cargo-task-wasm/labels/help%20wanted

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
