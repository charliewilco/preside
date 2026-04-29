# Development

## Local Checks

Run the same checks as CI before handing over changes:

```sh
just check
```

The underlying commands are:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

For routine local work:

```sh
just format
just build
just test
just lint
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

## Releases

Pushing a tag that starts with `v` runs the release workflow:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The workflow builds release binaries on Linux and macOS, packages `pp` as `.tar.gz` artifacts, and attaches them to a GitHub Release.

The current release workflow uses common third-party actions for Rust setup, Cargo caching, and GitHub Release creation. Pin action SHAs before using it in a stricter supply-chain environment.
