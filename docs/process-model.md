# Process Model

Preside v1 manages local development services directly from the CLI. It does not run a daemon.

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

## Stopping Services

`pp stop <service>` reads the PID file and checks whether the process appears alive.

Stop sequence:

1. send `SIGTERM`
2. wait briefly
3. send `SIGKILL` if still alive
4. remove the PID file once the process is gone

Preside attempts to signal the process group first, then falls back to the recorded PID.

## Status

`pp status <service>` reports:

- configured port
- configured domain
- PID
- whether the PID appears alive
- whether localhost accepts TCP connections on the configured port
- PID path
- log path

The listening check is a TCP connection attempt to `127.0.0.1:<port>`.

## Logs

`pp logs <service>` prints the configured log path and the most recent 8 KiB of log content.

Before opening a detached log for append, Preside rotates logs larger than 10 MiB:

```txt
<service>.log -> <service>.log.1
```

Only one rotated file is retained.

## Known Limits

PID files alone cannot fully prove process identity. A future daemon or launchd integration could store richer metadata and provide stronger supervision.

Preside does not currently manage reverse proxies, local DNS, TLS, or `/etc/hosts`.
