# Preside SPEC

Preside is a Rust CLI for managing named local development services.

Binary name: `pp`

Package name: `preside`

## Product idea

Preside lets me define local projects once, then run them by name.

Instead of remembering directories, ports, and commands:

```sh
cd ~/Code/personal
PORT=8712 npm run dev
```

I can run:

```sh
pp r personal
```

Preside should make local dev services feel like named places on my machine.

## Roadmap

The first macOS app is now implemented ahead of proxy/TLS work. See [docs/macos.md](docs/macos.md) for its scope and build instructions. The v1 non-goals below describe the original CLI milestone.

### v1 — Named services

Preside runs named services.

Scope:

- global TOML config
- named service registry
- directory + command + port + domain/alias
- optional service-specific env vars
- foreground and detached process launching
- env injection
- logs
- PID and runtime identity tracking
- status/info/list/open/stop/doctor/path/edit commands

### v2 — Alias / proxy / TLS

Preside makes services feel like real local apps.

Possible approaches:

- integrate with Caddy
- generate reverse proxy config
- optionally interoperate with tools like `localias`
- support local domains such as `personal.local`
- support TLS later

Preside should remain the source of truth. Proxy/DNS tools are implementation details.

### v3 — macOS menu bar app

A small SwiftUI menu bar app controls Preside services.

The menu bar app should shell out to `pp` rather than reimplementing service management.

## v1 goals

The v1 CLI should support this workflow:

```sh
pp set personal dir ~/Code/personal
pp set personal command "npm run dev"
pp set personal port 8712
pp set personal domain personal.local
pp set personal env.NODE_ENV development

pp r personal
pp status personal
pp doctor personal
pp open personal
pp stop personal
```

## Config

Config path:

```txt
~/.config/preside/config.toml
```

Alternate existing config paths:

```txt
~/.config/preside.toml
~/.preside.toml
```

The first existing path is used for reads and writes. If none exist, Preside creates `~/.config/preside/config.toml`.

Example config:

```toml
[services.personal]
dir = "~/Code/personal"
command = "npm run dev"
port = 8712
domain = "personal.local"
readiness = "http"
health_path = "/healthz"

[services.personal.env]
NODE_ENV = "development"

[services.burton-docs]
dir = "~/Code/burton/docs"
command = "pnpm dev"
port = 4321
domain = "burton-docs.local"
```

## Service model

Each service has:

```txt
name: string
dir: path
command: string
port: u16
domain: optional string
readiness: optional enum, one of process/port/http
health_path: optional string
env: optional string map
```

Use `domain`, not `alias`, in the config. It points toward the v2 proxy/TLS direction more clearly.

## CLI commands

Required commands:

```sh
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
pp doctor [service]
pp doctor [service] --fix
pp doctor [service] --json
pp edit
pp path
pp path --state
pp path --log <service>
pp path --pid <service>
pp path --service-state <service>
pp completions <shell>
pp __complete services
pp open <service>
pp stop <service>
pp logs <service>
pp logs <service> --lines <n>
pp logs <service> --follow
pp logs <service> --clear
pp status <service>
pp status <service> --json
```

## Command behavior

### `pp list`

Lists all configured services.

Output should be human-readable.

Example:

```txt
personal       8712   personal.local       ~/Code/personal
burton-docs    4321   burton-docs.local    ~/Code/burton/docs
```

`--json` prints configured services as JSON.

### `pp add <service>`

Creates a complete service in one command.

Examples:

```sh
pp add personal --dir ~/Code/personal --command "npm run dev" --port 8712 --domain personal.local
pp add worker --dir ~/Code/worker --command "cargo run" --port 9000 --readiness process
```

### `pp info <service>`

Shows the configured fields for a service.

Alias:

```sh
pp i personal
```

Should include:

- name
- dir
- command
- port
- domain
- config path

### `pp set <service> <field> <value>`

Creates or updates a service.

Valid fields:

```txt
dir
command
port
domain
readiness
health_path
env.<KEY>
```

Examples:

```sh
pp set personal dir ~/Code/personal
pp set personal command "npm run dev"
pp set personal port 8712
pp set personal domain personal.local
pp set personal env.NODE_ENV development
```

If the service does not exist, create it.

If the field is invalid, return a useful error.

If the port is not a valid `u16`, return a useful error.

Environment keys must use letters, numbers, and underscores and start with a letter or underscore.

`readiness` must be `process`, `port`, or `http`.

`health_path` must start with `/` and contain no whitespace.

### `pp unset <service> <field>`

Removes a configured field, including `env.<KEY>`.

### `pp remove <service>`

Removes a service from config. It does not stop a running service.

### `pp rename <old> <new>`

Renames a configured service. It does not rename runtime state.

### `pp edit`

Opens the active config path using `$VISUAL`, `$EDITOR`, or the platform opener. Before opening, write a backup. After the editor exits, parse the config and report errors.

### `pp path`

Prints the active config path by default. Flags print state, PID, log, or runtime state paths.

### `pp doctor [service]`

Checks config completeness, directory existence, stale PID files, mismatched live PIDs, state/config drift, and occupied ports. Exits nonzero when issues are found.

`--fix` removes stale PID and runtime state files when the process is gone. It must not signal unknown live processes or edit service config.

`--json` prints a machine-readable report.

### `pp run <service>`

Runs the service in the foreground.

Alias:

```sh
pp r personal
```

Behavior:

- load config
- find service
- expand `~` in `dir`
- expand environment variables in `dir`
- run `command` from `dir`
- inject environment variables
- stream stdout/stderr to the current terminal
- exit with the child process status

Preside should run commands through the shell so normal commands work:

```rust
Command::new("sh")
    .arg("-c")
    .arg(&service.command)
```

Do not prefix env vars into the command string. Use `.env(...)`.

### Injected environment variables

Always inject:

```txt
PORT=<service.port>
PRESIDE_NAME=<service-name>
```

If domain is configured, also inject:

```txt
PRESIDE_DOMAIN=<service.domain>
```

Configured service env values are also injected:

```toml
[services.personal.env]
NODE_ENV = "development"
```

Example effective command:

```sh
PORT=8712 PRESIDE_NAME=personal PRESIDE_DOMAIN=personal.local npm run dev
```

### `pp run <service> --detach`

Runs the service detached from the current terminal.

Alias:

```sh
pp r personal -d
```

Behavior:

- run service from configured directory
- inject env vars
- detach from terminal as much as practical on macOS/Unix
- redirect stdout and stderr to a log file
- write a PID file
- write a runtime identity state file
- fail if the process exits shortly after launch
- warn if the process remains alive but readiness is not passing yet

State paths:

```txt
~/.local/state/preside/pids/<service>.pid
~/.local/state/preside/logs/<service>.log
~/.local/state/preside/services/<service>.toml
```

If a PID file already exists and the process matches Preside runtime state, show that the service appears to already be running.

If a live PID does not match runtime state, refuse to start over it.

If readiness is `port` or `http` and the configured port already appears occupied by another process, refuse to start detached.

Readiness modes:

```txt
process  process staying alive is sufficient
port     localhost:<port> must accept TCP connections
http     health_path on localhost:<port> must return an HTTP 2xx response
```

### `pp stop <service>`

Stops a detached service.

Behavior:

- read PID file
- read runtime state file
- check whether process exists and matches runtime state
- send termination signal
- escalate to SIGKILL if SIGTERM does not stop it
- remove PID and runtime state files if process is gone

Use Unix signals through `nix`.

If the service is not running, show a useful message.

### `pp status <service>`

Reports service status.

Should include:

- configured port
- configured domain
- URL
- PID path
- runtime state path
- log path
- whether PID file exists
- whether process appears alive
- whether process identity matches runtime state
- whether runtime state matches current config
- whether the configured port is listening
- recent log lines

`--json` prints a machine-readable report.

Example:

```txt
personal
  port: 8712
  domain: personal.local
  url: http://personal.local
  status: running
  listening: yes
  identity: matches
  pid: 12345
  log: ~/.local/state/preside/logs/personal.log
```

### `pp logs <service>`

Prints the log path and tails or displays recent log output.

Supports:

```sh
pp logs personal --lines 100
pp logs personal --follow
pp logs personal --clear
```

Logs larger than 10 MiB rotate to `<service>.log.1` before a detached run opens the log.

Preside writes lifecycle events to detached logs with a `[preside]` prefix.

### `pp completions <shell>`

Prints static shell completions.

### `pp __complete services`

Hidden helper that prints configured service names for dynamic shell completion scripts.

### `pp open <service>`

Opens the service URL in the default browser.

If `domain` exists:

```txt
http://<domain>
```

Otherwise:

```txt
http://localhost:<port>
```

For v1, do not edit `/etc/hosts`, run a proxy, or configure TLS.

## Filesystem behavior

Use appropriate user directories.

Config:

```txt
~/.config/preside/config.toml
```

State:

```txt
~/.local/state/preside/
```

Create directories as needed.

## Dependencies

Use these crates unless there is a strong reason not to:

```toml
clap = { version = "4", features = ["derive"] }
clap_complete = "4"
serde_json = "1"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
anyhow = "1"
thiserror = "1"
directories = "5"
shellexpand = "3"
open = "5"
nix = { version = "0.29", features = ["process", "signal"] }
```

For tests, use:

```toml
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

## Suggested project structure

```txt
src/
  main.rs
  cli.rs
  config.rs
  service.rs
  process.rs
  paths.rs
```

### `cli.rs`

Clap command definitions.

### `config.rs`

Load, save, and mutate TOML config.

### `service.rs`

Service data model and validation.

### `process.rs`

Run, detach, stop, status, and log helpers.

### `paths.rs`

Config and state path helpers.

## Error handling

Use `anyhow` at the CLI boundary.

Use helpful messages for:

- missing config
- missing service
- invalid field
- invalid port
- missing directory
- command failed to spawn
- PID exists but process is dead

## Non-goals for v1

Do not implement:

- daemon process
- launchd
- reverse proxy
- Caddy integration
- TLS
- `/etc/hosts` editing
- wildcard local DNS
- macOS menu bar app
- GUI
- background auto-start on login

## Acceptance criteria

A fresh clone should support:

```sh
cargo build
cargo test
cargo run -- set personal dir ~/Code/personal
cargo run -- set personal command "npm run dev"
cargo run -- set personal port 8712
cargo run -- set personal domain personal.local
cargo run -- info personal
cargo run -- list
```

The CLI should compile cleanly.

The implementation should include tests for:

- config creation
- setting service fields
- listing services
- info output
- invalid field error
- invalid port error
- URL selection for `open`
- env injection behavior where practical
- PID/status behavior where practical
- doctor behavior
- detached startup failure behavior
- log line/clear behavior
- completion generation

## Future notes

v2 should explore alias/proxy/TLS support. Preside should likely remain the source of truth and either:

- generate Caddy config
- orchestrate Caddy
- integrate with a local aliasing tool
- or expose a config format that another proxy tool can consume

v3 should be a tiny macOS menu bar app using SwiftUI `MenuBarExtra`.

The menu bar app should call `pp` commands rather than duplicate the service manager.
