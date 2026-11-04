# Preside

Preside is a Rust CLI for managing named local development services.

The package is named `preside`; the binary is named `pp`.

Instead of remembering project directories, ports, and commands:

```sh
cd ~/Code/personal
PORT=8712 npm run dev
```

define the service once and run it by name:

```sh
pp add personal --dir ~/Code/personal --command "npm run dev" --port 8712 --domain personal.local

pp r personal
```

## macOS App

Build the native service browser and menu bar app:

```sh
just macos-build
open dist/Preside.app
```

Requires macOS 14+, Rust, and Xcode with Swift 6+. The app bundles `pp` and shares your existing services. Use **Add Site…** in its menu bar to serve a static folder or run a server command. See [docs/macos.md](docs/macos.md) for setup, PATH configuration, and current limits.

## Install

From this repository:

```sh
cargo install --path .
```

Homebrew users can install from the included formula as a head build:

```sh
brew install --HEAD ./Formula/preside.rb
```

Or run during development:

```sh
cargo run -- set personal dir ~/Code/personal
cargo run -- r personal
```

## Quick Start

```sh
pp set personal dir ~/Code/personal
pp set personal command "npm run dev"
pp set personal port 8712
pp set personal domain personal.local
pp set personal readiness port

pp list
pp list --json
pp info personal
pp status personal --json
pp doctor --json
pp doctor --fix
pp r personal
```

Run in the background:

```sh
pp r personal --detach
pp status personal
pp logs personal
pp stop personal
```

Open the service URL:

```sh
pp open personal
```

`pp open` uses `http://<domain>` when `domain` is configured, otherwise `http://localhost:<port>`.

## Static Sites

Serve a folder without installing another server:

```sh
pp add docs --dir ~/Code/docs/dist --port 8080 --static
pp run docs --detach
pp open docs
pp stop docs
```

Static sites use Preside's built-in loopback-only HTTP server. Existing command-based services continue to work unchanged.

## Commands

```txt
pp list
pp list --json
pp add <service> --dir <dir> --command <command> --port <port>
pp run <service>
pp r <service>
pp run <service> --detach
pp r <service> -d
pp info <service>
pp i <service>
pp info <service> --json
pp set <service> <field> <value>
pp unset <service> <field>
pp remove <service>
pp rename <old> <new>
pp doctor [service] [--fix] [--json]
pp edit
pp path [--state|--log <service>|--pid <service>|--service-state <service>]
pp completions <shell>
pp __complete services
pp open <service>
pp stop <service>
pp logs <service> [--lines <n>|--follow|--clear]
pp status <service>
pp status <service> --json
```

See [docs/commands.md](docs/commands.md) for command details.

## Configuration

Preside reads and writes TOML config. The canonical path is:

```txt
~/.config/preside/config.toml
```

It also supports existing configs at:

```txt
~/.config/preside.toml
~/.preside.toml
```

Each service has:

```toml
[services.personal]
dir = "~/Code/personal"
command = "npm run dev"
port = 8712
domain = "personal.local"
readiness = "port"
health_path = "/healthz"

[services.personal.env]
NODE_ENV = "development"
```

See [docs/configuration.md](docs/configuration.md) for config rules and path precedence.

## State

Detached service state is stored under:

```txt
~/.local/state/preside/
```

PID files live in `pids/`, logs live in `logs/`, and runtime identity files live in `services/`. See [docs/process-model.md](docs/process-model.md).

## Daily Driver Tools

Use `pp doctor` when something feels off. It checks config completeness, directory existence, process identity, stale PID/state, and port conflicts.

Use `pp path` to find files quickly:

```sh
pp path
pp path --state
pp path --log personal
pp path --pid personal
pp path --service-state personal
```

Generate shell completions with:

```sh
pp completions zsh > _pp
```

For dynamic zsh service-name completions, source [contrib/completions/pp.zsh](contrib/completions/pp.zsh). It reads configured services through `pp __complete services`.

## Development

Required checks:

```sh
just check
```

See [docs/development.md](docs/development.md) for local and CI notes.

## Current Scope

This is v1: named local services, config, foreground and detached process management, PID tracking, logs, status, and open/stop helpers.

A native macOS app now exposes service browsing, start/stop, logs, diagnostics, and service creation. Preside does not currently implement a daemon, launchd integration, reverse proxying, TLS, or `/etc/hosts` edits.
