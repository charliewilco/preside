# Commands

Preside's binary is `pp`.

## `pp list`

Lists configured services in a human-readable table.

```sh
pp list
pp list --json
```

Services are sorted by name because config is stored in a `BTreeMap`.

## `pp add <service>`

Creates a complete service in one command.

```sh
pp add personal --dir ~/Code/personal --command "npm run dev" --port 8712 --domain personal.local
pp add worker --dir ~/Code/worker --command "cargo run" --port 9000 --readiness process
```

Optional flags:

- `--domain <domain>`
- `--readiness process|port|http`
- `--health-path <path>`
- `--env KEY=value`

## `pp set <service> <field> <value>`

Creates or updates a service.

```sh
pp set personal dir ~/Code/personal
pp set personal command "npm run dev"
pp set personal port 8712
pp set personal domain personal.local
pp set personal env.NODE_ENV development
```

Valid scalar fields:

```txt
dir
command
port
domain
readiness
health_path
```

Environment fields use `env.<KEY>`.

```sh
pp set personal env.RUST_LOG debug
```

`dir`, `command`, and `port` are required before a service can run. `domain`, `readiness`, `health_path`, and `env` are optional.

## `pp unset <service> <field>`

Removes one configured field from a service.

```sh
pp unset personal domain
pp unset personal env.NODE_ENV
```

This can make a service incomplete. Commands that need a runnable service will report missing required fields.

## `pp remove <service>`

Removes a service from config.

```sh
pp remove personal
```

This only edits config. It does not stop a running detached process; run `pp stop <service>` first when needed.

## `pp rename <old> <new>`

Renames a configured service.

```sh
pp rename personal work
```

This edits config only. Runtime state for the old name is not renamed.

## `pp info <service>`

Shows configured fields and the config path in use.

```sh
pp info personal
pp i personal
pp info personal --json
```

## `pp edit`

Opens the active config file.

```sh
pp edit
```

Preside uses `$VISUAL`, then `$EDITOR`, then falls back to the platform opener.

## `pp path`

Prints important Preside paths.

```sh
pp path
pp path --state
pp path --log personal
pp path --pid personal
pp path --service-state personal
```

With no flags, `pp path` prints the active config path.

## `pp doctor [service]`

Checks all services, or one service when a name is provided.

```sh
pp doctor
pp doctor personal
pp doctor --json
pp doctor --fix
```

Doctor checks:

- config completeness
- directory existence
- stale PID files
- live PIDs that do not match Preside state
- runtime state that no longer matches config
- occupied ports

It exits successfully when no issues are found and nonzero when issues are found.

`--fix` removes stale PID and runtime state files when the process is gone. It does not signal unknown live processes or edit service config.

## `pp run <service>`

Runs the configured command in the foreground.

```sh
pp run personal
pp r personal
```

Behavior:

- expands `~` and environment variables in `dir`
- runs from the configured directory
- invokes the command through `sh -c`
- injects Preside and configured service environment variables
- streams stdout and stderr to the current terminal
- exits with the child process status

Preside injects environment variables through the child process environment:

```txt
PORT=<service.port>
PRESIDE_NAME=<service>
PRESIDE_DOMAIN=<service.domain>
```

`PRESIDE_DOMAIN` is only set when `domain` is configured.

## `pp run <service> --detach`

Runs the configured command detached from the current terminal.

```sh
pp run personal --detach
pp r personal -d
```

Detached services write:

```txt
~/.local/state/preside/pids/<service>.pid
~/.local/state/preside/logs/<service>.log
~/.local/state/preside/services/<service>.toml
```

Before starting, Preside refuses to start over a live mismatched PID. For `port` and `http` readiness, it also refuses to start when the configured port is already occupied.

After starting, Preside waits briefly. If the process exits immediately, the command fails and prints recent logs. If the process remains alive but readiness is not passing yet, Preside reports that and leaves the service running.

## `pp status <service>`

Shows configured and runtime status.

```sh
pp status personal
pp status personal --json
```

Status includes:

- config completeness
- dir, command, port, domain, and URL
- running/stopped based on PID checks
- process identity match
- runtime state match
- whether localhost is accepting TCP connections on the configured port
- PID file path
- state file path
- log path and recent log lines

## `pp logs <service>`

Prints the log path and recent log lines.

```sh
pp logs personal
pp logs personal --lines 200
pp logs personal --follow
pp logs personal --clear
```

`--follow` is a simple live tail. `--clear` truncates the log.

## `pp stop <service>`

Stops a detached service.

```sh
pp stop personal
```

Preside only signals a live PID when it matches Preside runtime state. It sends `SIGTERM`, waits briefly, then sends `SIGKILL` if the process still appears alive.

## `pp open <service>`

Opens the service URL in the default browser.

```sh
pp open personal
```

URL selection:

- `http://<domain>` when `domain` is configured
- `http://localhost:<port>` otherwise

## `pp completions <shell>`

Prints a shell completion script.

```sh
pp completions bash
pp completions zsh
pp completions fish
```

For dynamic zsh service-name completions, source `contrib/completions/pp.zsh`. The helper uses:

```sh
pp __complete services
```
