# Development

## Local Checks

Run the same checks as CI before handing over changes:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

For routine local formatting:

```sh
cargo fmt
```

## Project Structure

```txt
src/
  main.rs
  lib.rs
  cli.rs
  config.rs
  paths.rs
  process.rs
  service.rs
tests/
  cli.rs
```

Module responsibilities:

- `cli`: clap command definitions
- `config`: TOML load/save/mutation
- `paths`: config and state path resolution
- `service`: service model and validation
- `process`: foreground/detached process behavior, status, logs, URL selection

## Testing Notes

Integration tests use isolated temporary `HOME` directories so they do not read or write a developer's real Preside config.

Detached process tests start a simple shell loop, assert status/log behavior, and stop it before finishing.

## CI

GitHub Actions runs on Linux and macOS:

- format check
- Clippy with warnings as errors
- unit and integration tests

Linux gives a fast baseline. macOS matters because Preside is primarily a local-development tool for Unix/macOS workflows.
