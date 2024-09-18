<h1 align="center">watr</h1>
<div align="center">
  <strong>Wa</strong>sm <strong>T</strong>ask <strong>R</strong>unner for Cargo
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/cargo-task">
    <img src="https://img.shields.io/crates/v/cargo-task.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/cargo-task">
    <img src="https://img.shields.io/crates/d/cargo-task.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/cargo-task">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://docs.rs/cargo-task">
      API Docs
    </a>
    <span> | </span>
    <a href="https://github.com/yoshuawuyts/cargo-task/releases">
      Releases
    </a>
    <span> | </span>
    <a href="https://github.com/yoshuawuyts/cargo-task/blob/master.github/CONTRIBUTING.md">
      Contributing
    </a>
  </h3>
</div>

## About

`watr` provides a new `cargo task` subcommand that can be used to run
project-local tasks inside a secure WebAssembly sandbox. It looks for files in a
`tasks/` subdirectory of your project's root, and compiles those to [Wasm
Components](https://component-model.bytecodealliance.org). This is an attempt at
formalizing [cargo-xtask](https://github.com/matklad/cargo-xtask) pattern into a
secure, first-class workflow.

## Installation

The `cargo task` subcommand compiles Rust to Wasm Components targeting [WASI
0.2](https://wasi.dev). In order to do that a working WASI 0.2 toolchain needs
to be present on the host system.

```sh
$ rustup +beta target add wasip2  # Install the WASI 0.2 target
$ cargo install watr              # Install the `cargo task` subcommand
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

Tasks in `cargo task` follow the [principle of least
privilege](https://en.wikipedia.org/wiki/Principle_of_least_privilege). By
default they only get access to the working directory, and can access any
additional command line arguments passed to it. Additional permissions can be
configured via a `[tasks]` section in `Cargo.toml`.

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2021"

[task-dependencies]
wstd = "0.4.0"

[tasks]
print = { path = "tasks/print.rs" }                 # define a custom path for the task
env = { permissions = { inherit-env = ["FOO"] } }   # inherit specific env vars
```

## See Also

- [Custom tasks in Cargo (Aaron Turon, 2016)](http://aturon.github.io/tech/2018/04/05/workflows/)
- [`matklad/cargo-xtask` (Alex Kladov, 2019)](https://github.com/matklad/cargo-xtask)

## Safety
This crate uses ``#![deny(unsafe_code)]`` to ensure everything is implemented in
100% Safe Rust.

## Contributing
Want to join us? Check out our ["Contributing" guide][contributing] and take a
look at some of these issues:

- [Issues labeled "good first issue"][good-first-issue]
- [Issues labeled "help wanted"][help-wanted]

[contributing]: https://github.com/yoshuawuyts/cargo-task/blob/master.github/CONTRIBUTING.md
[good-first-issue]: https://github.com/yoshuawuyts/cargo-task/labels/good%20first%20issue
[help-wanted]: https://github.com/yoshuawuyts/cargo-task/labels/help%20wanted

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
