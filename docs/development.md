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

## Homebrew

The repository includes a head-installable formula:

```sh
brew install --HEAD ./Formula/preside.rb
```

A stable tap formula needs a release URL and SHA generated from the first tagged release.

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

Workflow actions are pinned to commit SHAs. When bumping an action, update the SHA and keep the trailing comment that identifies the source tag or branch.
