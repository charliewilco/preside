# Repository Guidelines

## Project Structure & Module Organization

Preside is a Rust CLI package named `preside` with binary `pp`.

- `src/main.rs`: CLI entrypoint.
- `src/lib.rs`: command dispatch and presentation logic.
- `src/cli.rs`: clap command definitions.
- `src/config.rs`: TOML config loading, saving, and mutation.
- `src/service.rs`: service model, validation, and readiness types.
- `src/process.rs`: foreground/detached process handling, logs, status, and runtime state.
- `src/paths.rs`: config and state path resolution.
- `tests/cli.rs`: integration tests using `assert_cmd`.
- `docs/`: user and maintainer documentation.
- `.github/workflows/`: CI and release workflows.
- `Formula/` and `contrib/`: install and shell completion helpers.

## Build, Test, and Development Commands

Use `just` for routine workflows:

```sh
just format        # run cargo fmt
just format-check  # verify formatting
just build         # cargo build
just build-release # cargo build --release
just test          # run all tests
just lint          # clippy with warnings denied
just check         # format-check, lint, test
```

Run locally with `cargo run -- <args>`, for example:

```sh
cargo run -- add personal --dir ~/Code/personal --command "npm run dev" --port 8712
```

## Coding Style & Naming Conventions

Use idiomatic Rust 2021. Let `rustfmt` decide formatting. Prefer clear module boundaries over large catch-all helpers. Keep command names and config fields lowercase and stable, e.g. `health_path`, `readiness`, `env.KEY`. Use `anyhow` at CLI boundaries and typed errors in domain modules when useful.

## Testing Guidelines

Add integration tests in `tests/cli.rs` for user-visible CLI behavior. Use isolated temporary `HOME` directories so tests never touch real Preside config. Prefer focused tests for commands, config paths, process edge cases, JSON output, and diagnostics. Run `just check` before handing off changes.

## Commit & Pull Request Guidelines

History uses short imperative commit subjects, for example `Add just task runner` and `Pin workflow actions`. Keep commits focused by behavior or documentation slice. Pull requests should include a concise summary, test results, linked issues when applicable, and screenshots only for UI-facing docs or rendered output changes.

## Security & Configuration Tips

Do not edit `/etc/hosts`, install daemons, or introduce proxy/TLS behavior in v1 work. Be careful with PID handling: never signal unknown live processes. Keep workflow actions pinned to SHAs when updating CI or release automation.
