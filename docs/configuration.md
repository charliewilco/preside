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
```

## Validation

Service names may contain letters, numbers, dots, dashes, and underscores. They must start with a letter or number.

`port` must be a valid `u16`.

`domain` must be a hostname-like value without:

- URL scheme
- path
- spaces
- port

Use `personal.local`, not `http://personal.local` or `personal.local:8712`.

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

Commands that require a runnable service, such as `pp run`, `pp status`, and `pp open`, require `dir`, `command`, and `port` to be present.
