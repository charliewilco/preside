# Configuration

Preside stores service definitions in TOML.

## Paths

Preside resolves config in this order:

```txt
~/.config/preside/config.toml
~/.config/preside.toml
~/.preside.toml
```

The first existing path is used for reads and writes.

When no config exists, Preside creates:

```txt
~/.config/preside/config.toml
```

## Format

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
RUST_LOG = "debug"

[services.docs]
dir = "$HOME/Code/docs"
command = "pnpm dev"
port = 4321
```

## Service Fields

Required fields:

```txt
dir
command
port
```

Optional fields:

```txt
domain
env
readiness
health_path
```

Environment variables are stored under `[services.<name>.env]` and are injected into foreground and detached runs.

Readiness controls detached startup checks:

- `process`: only require the process to remain alive
- `port`: wait for `127.0.0.1:<port>` to accept TCP connections
- `http`: wait for a successful HTTP response from `health_path` on `127.0.0.1:<port>`

The default readiness mode is `port`.

## Validation

Service names may contain letters, numbers, dots, dashes, and underscores. They must start with a letter or number.

`port` must be a valid `u16`.

`domain` must be a hostname-like value without:

- URL scheme
- path
- spaces
- port

Use `personal.local`, not `http://personal.local` or `personal.local:8712`.

Environment keys must use letters, numbers, and underscores, and must start with a letter or underscore.

`health_path` must start with `/` and cannot contain whitespace.

## Directory Expansion

Preside expands `~` and environment variables in `dir` before running a service.

Examples:

```toml
dir = "~/Code/personal"
dir = "$HOME/Code/personal"
```

## Partial Services

`pp set` creates services even when only one field is known.

This is valid:

```sh
pp set personal dir ~/Code/personal
```

Commands that require a runnable service, such as `pp run` and `pp open`, require `dir`, `command`, and `port` to be present. `pp status` and `pp doctor` can inspect incomplete services.
