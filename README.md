<h1 align="center">cargo-task-wasm</h1>
<div align="center">
  <strong>
    A sandboxed local task runner for Rust
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

## About

This project provides a new `cargo task` subcommand that can be used to run
project-local tasks inside a secure WebAssembly sandbox. It looks for files in a
`tasks/` subdirectory of your project's root, and compiles those to [Wasm
Components](https://component-model.bytecodealliance.org). This is an attempt at
formalizing [cargo-xtask](https://github.com/matklad/cargo-xtask) pattern into a
first-class, secure workflow.

## Roadmap

- [x] Sketch out a repository layout or whatever workflow example
- [x] Create a new `cargo` subcommand
- [x] Hook up wasmtime to the subcommand
- [x] Add support for manual paths in a `[tasks]` section in `Cargo.toml`
- [x] Figure out how to configure capabilities for the tasks
- [x] Add support for compiling cargo deps as part of subcommands
- [x] Store config in Cargo metadata section
- [x] Add support for using submodules
- [ ] Add the remainder of the capabilities
- [ ] Add support for installing tasks from crates.io
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

Examples:
  cargo task codegen     # run a task called `codegen`
```

## Configuration

### Capabilities

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

[package.metadata.tasks]
env-filter = { inherit-env = ["FOO"] }  # inherit specific env vars
env-all = { inherit-env = true }        # inherit all env vars
```

The reason why this is stored in `[package.metadata.tasks]` rather than a
top-level `[tasks]` section is because that is [the canonical extension
point](https://doc.rust-lang.org/cargo/reference/manifest.html#the-metadata-table)
Cargo recommends using for third-party extensions. Should a `cargo tasks`
command ever become a first-class extension to Cargo, the `package.metadata`
prefix can be dropped.

### Dependencies

Tasks must specify their own dependencies via
`[package.metadata.task-dependencies]` in `Cargo.toml`. These dependencies are
separate from Cargo's existing `[dev-dependencies]` and `[build-dependencies]`
because these dependencies must be able to be compiled to Rust's `wasm32-wasip2`
target. Not all dev or build deps may fit these requirements, which is why task
dependencies are listed separately.

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2021"

[package.metadata.task-dependencies]
wstd = "0.4.0"
```

### Paths

Tasks are discovered in the local `tasks/` directory of your project. This is a
treated as standalone workspace where each file is treated as an individual task
to be compiled and executed. This behaves not unlike the `tests/` directory in
Cargo projects. It is possible to use both submodules and dependencies with
tasks like you would expect. A typical project structure will look like this:

```text
example/
├── Cargo.toml
├── src
│   └── lib.rs
└── tasks
    ├── codegen.rs
    └── test.rs
```

This structure will give you access to the `cargo task codegen` and `cargo task
test` subcommands.

## Limitations

By default tasks only get access to the local project directory and any
additional arguments passed via the CLI. Additional capabilities such as network
or filesystem access can be configured via `Cargo.toml`. Sandboxing is provided
by the Wasmtime runtime, and the available APIs are part of the
`wasi:cli/command` world. Some limitations however still exist, and are good to
be aware of:

- **Limited ecosystem support**: At the time of writing WASI 0.2 is a fairly new
compile target, and so ecoystem support is still in its infancy. Not all crates
are expected to work, and may need to be updated first.
- **Limited stdlib support**: For similar reasons: not all functionality in the
stdlib will work yet. In particular network support for WASI 0.2 is still being
implemented. This is expected to land in Rust 1.84 in the second half of 2024.
If you want to access the network before then, you can try and use the
[wasi](https://docs.rs/wasi) or [wstd](https://docs.rs/wstd) crates.
- **No threading support**: At the time of writing support for threading in WASI
0.2 has not yet been implemented. Work on this is still ongoing upstream in the
WASI subgroup. Consensus on a design seems to have formed, and implementation
work has started - but this is unlikely to stabilize before the start of 2025.
- **No support for exec/fork**: WASI 0.2 does not allow you to spawn or fork new
processes. Providing access to this would be a sandbox escape, and so we don't
provide access to it. This means it's not possible to shell out to call global
tools, which may at times be impractical but is also a necessary limitation to
guarantee security.

In the future we hope to provide a way to instrument Cargo or Rustc directly
from inside the sandbox. However this will need to be carefully evaluated and
designed to ensure the sandbox cannot be escaped.

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

## Acknowledgements

This project was built as a collaboration between [Michael
Woerister](https://github.com/michaelwoerister) and [Yosh
Wuyts](https://github.com/yoshuawuyts) as part of the 2024 Microsoft Hackathon,
targeting the [Microsoft Secure Future
Initiative](https://www.microsoft.com/en-us/microsoft-cloud/resources/secure-future-initiative).
Special thanks to [Pat Hickey](https://github.com/pchickey) for showing us how
to configure Wasmtime as a Rust library.

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
