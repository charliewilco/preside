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

### v1 — Named services

Preside runs named services.

Scope:

- global TOML config
- named service registry
- directory + command + port + domain/alias
- foreground and detached process launching
- env injection
- logs
- PID tracking
- status/info/list/open/stop commands

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

pp r personal
pp status personal
pp open personal
pp stop personal
```

## Config

Config path:

```txt
~/.config/preside/config.toml
```

Example config:

```toml
[services.personal]
dir = "~/Code/personal"
command = "npm run dev"
port = 8712
domain = "personal.local"

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
```

Use `domain`, not `alias`, in the config. It points toward the v2 proxy/TLS direction more clearly.

## CLI commands

Required commands:

```sh
pp list
pp run <service>
pp r <service>
pp run <service> --detach
pp r <service> -d
pp info <service>
pp i <service>
pp set <service> <field> <value>
pp open <service>
pp stop <service>
pp logs <service>
pp status <service>
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
```

Examples:

```sh
pp set personal dir ~/Code/personal
pp set personal command "npm run dev"
pp set personal port 8712
pp set personal domain personal.local
```

If the service does not exist, create it.

If the field is invalid, return a useful error.

If the port is not a valid `u16`, return a useful error.

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
- return immediately

State paths:

```txt
~/.local/state/preside/pids/<service>.pid
~/.local/state/preside/logs/<service>.log
```

If a PID file already exists and the process is alive, show that the service appears to already be running.

### `pp stop <service>`

Stops a detached service.

Behavior:

- read PID file
- check whether process exists
- send termination signal
- remove PID file if process is gone

Use Unix signals through `nix`.

If the service is not running, show a useful message.

### `pp status <service>`

Reports service status.

Should include:

- configured port
- configured domain
- PID path
- log path
- whether PID file exists
- whether process appears alive

Example:

```txt
personal
  port: 8712
  domain: personal.local
  status: running
  pid: 12345
  log: ~/.local/state/preside/logs/personal.log
```

### `pp logs <service>`

Prints the log path and tails or displays recent log output.

For v1, it is acceptable to print the last chunk of the log file.

Do not implement a complex live tail unless simple.

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

## Future notes

v2 should explore alias/proxy/TLS support. Preside should likely remain the source of truth and either:

- generate Caddy config
- orchestrate Caddy
- integrate with a local aliasing tool
- or expose a config format that another proxy tool can consume

v3 should be a tiny macOS menu bar app using SwiftUI `MenuBarExtra`.

The menu bar app should call `pp` commands rather than duplicate the service manager.
