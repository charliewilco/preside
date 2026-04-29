# Commands

Preside's binary is `pp`.

## `pp list`

Lists configured services in a human-readable table.

```sh
pp list
```

Services are sorted by name because config is stored in a `BTreeMap`.

## `pp set <service> <field> <value>`

Creates or updates a service.

```sh
pp set personal dir ~/Code/personal
pp set personal command "npm run dev"
pp set personal port 8712
pp set personal domain personal.local
```

Valid fields:

```txt
dir
command
port
domain
```

`dir`, `command`, and `port` are required before a service can run. `domain` is optional.

## `pp unset <service> <field>`

Removes one configured field from a service.

```sh
pp unset personal domain
```

This can make a service incomplete. Commands that need a runnable service will report missing required fields.

## `pp remove <service>`

Removes a service from config.

```sh
pp remove personal
```

This only edits config. It does not stop a running detached process; run `pp stop <service>` first when needed.

## `pp info <service>`

Shows configured fields and the config path in use.

```sh
pp info personal
pp i personal
```

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
```

If the PID file exists and the process appears alive, Preside leaves it running and reports the existing PID.

## `pp status <service>`

Shows configured and runtime status.

```sh
pp status personal
```

Status includes:

- port
- domain
- running/stopped based on PID checks
- whether localhost is accepting TCP connections on the configured port
- PID file path
- log path

## `pp logs <service>`

Prints the log path and the most recent log chunk.

```sh
pp logs personal
```

For v1, this is not a live tail.

## `pp stop <service>`

Stops a detached service.

```sh
pp stop personal
```

Preside sends `SIGTERM`, waits briefly, then sends `SIGKILL` if the process still appears alive.

## `pp open <service>`

Opens the service URL in the default browser.

```sh
pp open personal
```

URL selection:

- `http://<domain>` when `domain` is configured
- `http://localhost:<port>` otherwise
