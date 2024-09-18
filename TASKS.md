# Plan of Action

- [x] sketch out a repository layout or whatever workflow example
- [x] create a new `cargo` subcommand
- [x] hook up wasmtime to the subcommand
- [x] add support for manual paths in a `[tasks]` section in `Cargo.toml`
- [ ] figure out how to configure capabilities for the tasks
- [ ] add support for cargo deps to the subcommands
- [ ] add Crates.io support (lol)

## Questions

- What happens when `cargo task xyz` is invoked?
  1. Build a list of tasks that actually exist (just look at tasks dir in workspace for now)
  2. Compile task .rs to wasm  (`wasm32-wasip2` target triple on Beta)
  3. Write the output to `target/tasks`, a new top-level in the `target` subdir
  4. Pass the newly compiled output to `wasmtime` and run it
- Which capabilities should we give tasks out of the box?
  1. Make them useful but restricted, start with just the local repo first

## References

- http://aturon.github.io/tech/2018/04/05/workflows/
- https://github.com/matklad/cargo-xtask


```bash
PATH=$PATH:$(pwd)/target/debug cargo task
```
