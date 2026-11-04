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

Required fields (`command` is not required when `kind = "static"`):

```txt
dir
command
port
```

Optional fields:

```txt
kind
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

## Static Site Configuration

```toml
[services.static-docs]
kind = "static"
dir = "~/Code/docs/dist"
port = 8080
readiness = "port"
```

`kind` is `command` or `static`; existing definitions without `kind` remain command services. `command` is required only for command services and is ignored for static sites. Static serving uses Preside's built-in server, requires a port from 1 through 65535, and does not require a system runtime. Choose the public output directory, rather than a repository root containing private files.

A configured domain still requires external DNS/proxy setup. The static server also accepts that domain as a Host header; Preside does not create DNS records or proxies.

## Transactional Changes

Preside serializes CLI/app mutations with an operating-system file lock at `~/.local/state/preside/mutation.lock`. The lock is acquired before configuration discovery and loading so concurrent changes cannot overwrite a stale snapshot. It is released automatically on process exit, including crashes. An operation waiting more than 15 seconds reports a busy error.

Configuration saves write a private temporary file, sync it, replace the destination atomically, and sync the parent directory. Config and runtime files are written with owner-only permissions. Existing config symlinks retain their link and update the resolved target. Direct edits by unrelated text editors do not participate in Preside's lock; use `update --expected-config` for conflict-aware clients.
