# hotswap

```
  ██╗  ██╗ ██████╗ ████████╗███████╗██╗    ██╗ █████╗ ██████╗
  ██║  ██║██╔═══██╗╚══██╔══╝██╔════╝██║    ██║██╔══██╗██╔══██╗
  ███████║██║   ██║   ██║   ███████╗██║ █╗ ██║███████║██████╔╝
  ██╔══██║██║   ██║   ██║   ╚════██║██║███╗██║██╔══██║██╔═══╝
  ██║  ██║╚██████╔╝   ██║   ███████║╚███╔███╔╝██║  ██║██║
  ╚═╝  ╚═╝ ╚═════╝    ╚═╝   ╚══════╝ ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝
```

[![CI](https://github.com/niklasmarderx/hotswap/actions/workflows/ci.yml/badge.svg)](https://github.com/niklasmarderx/hotswap/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

**Zero-downtime config & env reloader.** Watch config files, validate changes, and signal your processes to reload — without restarts.

Like `watchexec` but specifically built for config/env reloading with validation, env diffing, and multi-process coordination.

## Why hotswap?

| Tool | Watches files | Reloads env | Validates config | Signals processes |
|------|:---:|:---:|:---:|:---:|
| `entr` / `watchexec` | Yes | No | No | Re-runs command |
| `nodemon` | Yes | No | No | Restarts (not zero-downtime) |
| `direnv` | Shell entry only | Yes | No | No |
| **`hotswap`** | **Yes** | **Yes** | **Yes** | **Yes (SIGHUP, etc.)** |

## Installation

### From source

```bash
cargo install --path .
```

### From crates.io (coming soon)

```bash
cargo install hotswap
```

## Quick Start

```bash
# Watch .env and signal your app on changes
hotswap --watch .env --signal myapp

# Watch config with validation — won't signal if config is broken
hotswap --watch config.toml --validate toml --signal myapp

# See env diffs without signaling anything
hotswap --watch .env --diff-only

# Dry run — see what would happen
hotswap --watch .env --signal myapp --dry-run
```

## Usage

```
hotswap — zero-downtime config & env reloader

Usage: hotswap [OPTIONS] --watch <WATCH_PATHS>

Options:
  -w, --watch <WATCH_PATHS>       Files or directories to watch (supports globs)
  -s, --signal <SIGNAL_TARGETS>   Process names to signal on change
      --signal-type <SIGNAL_TYPE> Signal to send [default: SIGHUP]
      --pid-file <PID_FILE>       PID file to read process ID from
  -V, --validate <VALIDATORS>     Validators: toml, json, yaml, or shell command
  -d, --debounce <MS>             Debounce interval in ms [default: 500]
      --dry-run                   Show what would happen without doing it
      --diff-only                 Show env diffs without signaling
  -c, --config <PATH>             Path to hotswap.toml config file
  -i, --ignore <PATTERNS>         Glob patterns to ignore
  -q, --quiet                     Suppress banner
  -h, --help                      Print help
      --version                   Print version
```

## Examples

### Watch .env and reload your server

```bash
hotswap --watch .env --signal myserver
```

When `.env` changes, hotswap will:
1. Parse the new `.env` and show a colored diff of what changed
2. Send `SIGHUP` to `myserver`

### Validate before reload

```bash
# Built-in validators
hotswap --watch config.toml --validate toml --signal myapp
hotswap --watch config.json --validate json --signal myapp
hotswap --watch config.yaml --validate yaml --signal myapp

# Custom validator command
hotswap --watch nginx.conf --validate "nginx -t" --signal nginx
```

If validation fails, hotswap **won't signal** your process — preventing broken configs from being loaded.

### Multiple files

```bash
hotswap --watch .env --watch config.toml --validate toml --signal myapp
```

### Use a PID file

```bash
hotswap --watch .env --pid-file /var/run/myapp.pid --signal-type SIGUSR1
```

### Send different signals

```bash
hotswap --watch .env --signal myapp --signal-type SIGUSR1
hotswap --watch .env --signal myapp --signal-type SIGUSR2
hotswap --watch .env --signal myapp --signal-type SIGTERM
```

### Multi-process reload

```bash
hotswap --watch .env \
  --signal frontend \
  --signal backend \
  --signal worker
```

### Env diff only (no signaling)

```bash
hotswap --watch .env --diff-only
```

Output:
```
  changed .env

  diff env diff:
    + NEW_VAR=hello
    ~ DATABASE_URL  postgres://old → postgres://new
    - REMOVED_VAR=goodbye
```

## Config File

For complex setups, use a `hotswap.toml`:

```toml
[watch]
files = [".env", "config.toml"]
ignore = ["*.tmp", ".git/**"]
debounce_ms = 500

[[targets]]
name = "backend"
signal = "SIGHUP"
pid_file = "/tmp/backend.pid"
validate = "toml"

[[targets]]
name = "frontend"
signal = "SIGUSR1"
process_name = "node"
```

Then run:

```bash
hotswap --config hotswap.toml
```

## How It Works

1. **Watch** — Uses OS-native file watching (`FSEvents` on macOS, `inotify` on Linux) with configurable debounce
2. **Validate** — Checks config syntax (TOML/JSON/YAML) or runs custom commands before signaling
3. **Diff** — Parses `.env` files and computes what changed (added/modified/removed vars)
4. **Signal** — Sends Unix signals (`SIGHUP`, `SIGUSR1`, `SIGUSR2`, `SIGTERM`) to your process
5. **Discover** — Finds processes by name (via `sysinfo`) or PID file

## Supported Platforms

- macOS (FSEvents)
- Linux (inotify)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
