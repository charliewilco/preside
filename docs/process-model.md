# Process Model

Preside manages local development services directly from the CLI. It does not run a daemon.

## Foreground Runs

`pp run <service>` starts:

```txt
sh -c <service.command>
```

from the configured service directory.

The child process inherits terminal stdin/stdout/stderr. Preside exits with the child status.

## Environment Injection

Preside injects environment variables through `Command::env`.

Always injected:

```txt
PORT
PRESIDE_NAME
```

Injected when configured:

```txt
PRESIDE_DOMAIN
```

Service-specific env values from `[services.<name>.env]` are also injected.

The command string is not rewritten.

## Detached Runs

`pp run <service> --detach` starts the service in a new session with `setsid`.

Detached output goes to:

```txt
~/.local/state/preside/logs/<service>.log
```

The child PID is written to:

```txt
~/.local/state/preside/pids/<service>.pid
```

Runtime identity is written to:

```txt
~/.local/state/preside/services/<service>.toml
```

The runtime state stores the service name, PID, command, directory, port, domain, env map, start time, process start marker, and log path.

## Startup Checks

Before detached startup, Preside refuses to start when:

- a PID file points at a live process that does not match Preside state
- the configured localhost port is already accepting connections

After spawn, Preside waits briefly. If the process exits immediately, startup fails and recent logs are shown. If the process remains alive but the configured port is not listening yet, startup succeeds with a warning.

## Stopping Services

`pp stop <service>` reads runtime state and checks whether the recorded PID still appears to be the process Preside started.

Stop sequence:

1. verify the PID matches Preside runtime identity
2. send `SIGTERM`
3. wait briefly
4. send `SIGKILL` if still alive
5. remove PID and service state files once the process is gone

Preside attempts to signal the process group first, then falls back to the recorded PID.

## Status

`pp status <service>` reports:

- config completeness
- configured dir, command, port, domain, and URL
- PID
- whether the PID appears alive
- whether the PID matches Preside runtime identity
- whether runtime state matches current config
- whether localhost accepts TCP connections on the configured port
- PID path
- state path
- log path
- log modified time
- recent log lines

The listening check is a TCP connection attempt to `127.0.0.1:<port>`.

## Doctor

`pp doctor` is stricter than `status`. It checks all services, or a selected service, and exits nonzero if it finds issues.

It checks config completeness, directory existence, stale PID files, mismatched live PIDs, state/config drift, and occupied ports.

## Logs

`pp logs <service>` prints recent log lines.

```sh
pp logs personal --lines 100
pp logs personal --follow
pp logs personal --clear
```

Before opening a detached log for append, Preside rotates logs larger than 10 MiB:

```txt
<service>.log -> <service>.log.1
```

Only one rotated file is retained.

## Known Limits

Preside now stores runtime metadata to reduce PID reuse risk, but it is still not a full supervisor. A future daemon or launchd integration could provide stronger lifecycle management.

Preside does not currently manage reverse proxies, local DNS, TLS, or `/etc/hosts`.
