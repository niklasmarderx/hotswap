<div align="center">

<pre>
 ██   ██  ██████  ████████ ███████ ██     ██  █████  ██████  
 ██   ██ ██    ██    ██    ██      ██     ██ ██   ██ ██   ██ 
 ███████ ██    ██    ██    ███████ ██  █  ██ ███████ ██████  
 ██   ██ ██    ██    ██         ██ ██ ███ ██ ██   ██ ██      
 ██   ██  ██████     ██    ███████  ███ ███  ██   ██ ██      
</pre>

**Your config changed. Your app didn't restart.**

[![CI](https://github.com/niklasmarderx/hotswap/actions/workflows/ci.yml/badge.svg)](https://github.com/niklasmarderx/hotswap/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

[Features](#features) · [Install](#installation) · [Quick Start](#quick-start) · [Config File](#config-file) · [Why hotswap?](#the-problem)

</div>

---

<p align="center"><img src="assets/demo.gif" width="700" /></p>

## The Problem

You change `DATABASE_URL` in your `.env`. Now what?

- **Restart the process?** Downtime. Dropped connections. Angry users.
- **Use direnv?** Only works in your shell. Running processes don't care.
- **Use watchexec?** It re-runs commands. It doesn't reload configs.
- **Write a custom watcher?** For every. single. project.

**There was no tool that watches config changes AND gracefully signals running processes.** Until now.

## The Solution

```bash
hotswap --watch .env --signal myapp
```

That's it. When `.env` changes:
1. **Validates** the new config (catches syntax errors before your app does)
2. **Shows a diff** of what changed (colored, readable, instant)
3. **Sends SIGHUP** to your process (zero downtime, no restart)

Your app picks up the new config. No restart. No downtime. No drama.

## Features

**Watch anything** — `.env`, `.toml`, `.json`, `.yaml`, globs, directories

**Validate first** — Built-in TOML/JSON/YAML validators + custom commands (`nginx -t`, `cargo check`, anything). Broken config? Signal never sent.

**Env diffing** — See exactly what changed, color-coded:
```
  diff env diff:
    + NEW_VAR=hello
    ~ DATABASE_URL  postgres://old → postgres://new
    - REMOVED_VAR=goodbye
```

**Multi-process coordination** — Reload backend → wait for health check → reload frontend → reload workers. Sequential or parallel. With dependency ordering.

**Health checks** — `curl -sf http://localhost:3000/health` between reloads. If it fails, stop. Don't cascade failures.

**Dry run** — See exactly what would happen without doing it.

**Single binary** — No runtime, no dependencies. Built in Rust. Fast.

## Installation

```bash
# From source
cargo install --path .

# From crates.io (coming soon)
cargo install hotswap
```

Pre-built binaries for Linux and macOS are available on the [Releases](https://github.com/niklasmarderx/hotswap/releases) page.

## Quick Start

```bash
# The basics: watch a file, signal a process
hotswap --watch .env --signal myapp

# Validate before reload — don't signal on broken configs
hotswap --watch config.toml --validate toml --signal myapp

# Just show me what changed, don't signal anything
hotswap --watch .env --diff-only

# What would happen? (without actually doing it)
hotswap --watch .env --signal myapp --dry-run
```

## Real-World Examples

### Web App: Reload nginx after config change

```bash
hotswap --watch /etc/nginx/nginx.conf \
  --validate "nginx -t" \
  --signal nginx \
  --signal-type SIGHUP
```

nginx validates → nginx reloads config → zero dropped requests.

### Microservices: Coordinated multi-service reload

```bash
hotswap --config hotswap.toml
```

```toml
# hotswap.toml
strategy = "sequential"

[watch]
files = [".env", "config/*.toml"]

[[targets]]
name = "api"
signal = "SIGHUP"
pid_file = "/tmp/api.pid"
health_check = "curl -sf http://localhost:3000/health"
health_timeout_ms = 5000

[[targets]]
name = "worker"
signal = "SIGUSR1"
process_name = "worker"
depends_on = ["api"]
```

API reloads first → health check passes → worker reloads. If API health check fails, worker is never touched.

### Development: Watch everything, see all diffs

```bash
hotswap --watch .env --watch config/ --diff-only
```

No signals, just a live feed of every config change with colored diffs.

### CI/Production: Machine-readable output

```bash
hotswap --watch .env --signal myapp --json
```

```json
{"event":"file_changed","data":{"path":".env"},"timestamp":"2026-04-04T12:00:00Z"}
{"event":"env_diff","data":{"added":{"NEW_KEY":"value"},"changed":{},"removed":{}},"timestamp":"2026-04-04T12:00:00Z"}
{"event":"signal_sent","data":{"signal":"SIGHUP","target":"myapp","pid":42},"timestamp":"2026-04-04T12:00:00Z"}
```

## Config File

For complex setups, use `hotswap.toml`:

```toml
# Reload strategy: "sequential" (default) or "parallel"
strategy = "sequential"

[watch]
files = [".env", "config/*.toml"]
ignore = ["*.tmp", "*.bak"]
debounce_ms = 500

[[targets]]
name = "backend"
signal = "SIGHUP"              # Signal to send (SIGHUP, SIGUSR1, SIGUSR2, SIGTERM)
pid_file = "/tmp/backend.pid"  # Find process by PID file
validate = "toml"              # Validate before signaling
health_check = "curl -sf http://localhost:3000/health"
health_timeout_ms = 5000       # Health check timeout (default: 5000ms)

[[targets]]
name = "frontend"
signal = "SIGUSR1"
process_name = "node"          # Find process by name
depends_on = ["backend"]       # Reload after backend is healthy
```

See [examples/hotswap.toml](examples/hotswap.toml) for a full annotated example.

## CLI Reference

```
Usage: hotswap [OPTIONS]

Options:
  -w, --watch <WATCH_PATHS>        Files or directories to watch (supports globs)
  -s, --signal <SIGNAL_TARGETS>    Process names to signal on change
      --signal-type <SIGNAL_TYPE>  Signal type to send [default: SIGHUP]
      --pid-file <PID_FILE>        PID file to read process ID from
      --validate <VALIDATORS>      Validators: toml, json, yaml, or shell command
  -d, --debounce <DEBOUNCE>        Debounce interval in milliseconds [default: 500]
      --dry-run                    Dry run — show what would happen without sending signals
      --diff-only                  Diff-only mode — show env diffs without signaling
  -c, --config <CONFIG>            Path to a hotswap.toml config file
  -i, --ignore <IGNORE>            Patterns to ignore (globs)
  -q, --quiet                      Suppress banner output
      --strategy <STRATEGY>        Reload strategy: sequential or parallel [default: sequential]
      --json                       Output as JSON (structured logging)
      --completions <COMPLETIONS>  Generate shell completions (bash, zsh, fish, elvish, powershell)
  -h, --help                       Print help
  -V, --version                    Print version
```

## How It Works

```
  .env changed
       │
       ▼
  ┌─────────┐     ┌──────────┐     ┌──────────┐     ┌─────────┐
  │  Watch  │────▶│ Validate │────▶│   Diff   │────▶│  Signal │
  │ (notify)│     │ (syntax) │     │  (.env)  │     │ (kill)  │
  └─────────┘     └──────────┘     └──────────┘     └─────────┘
                       │                                  │
                  Invalid?                           Health check
                  Don't signal.                      before next.
```

1. **Watch** — OS-native file watching (FSEvents/inotify) with debounce
2. **Validate** — Syntax check (TOML/JSON/YAML) or custom commands. Invalid = abort.
3. **Diff** — Parse `.env` files, compute and display changes
4. **Signal** — Send Unix signal to process. Sequential or parallel with dependency ordering.
5. **Health Check** — Verify process is healthy before continuing to next target.

## Comparison

| | hotswap | watchexec | entr | nodemon | direnv |
|---|:---:|:---:|:---:|:---:|:---:|
| Watch files | **Yes** | Yes | Yes | Yes | No |
| Signal running processes | **Yes** | No (re-runs) | No (re-runs) | No (restarts) | No |
| Config validation | **Yes** | No | No | No | No |
| Env diffing | **Yes** | No | No | No | No |
| Multi-process coordination | **Yes** | No | No | No | No |
| Health checks | **Yes** | No | No | No | No |
| Zero downtime | **Yes** | No | No | No | N/A |
| Single binary | **Yes** | Yes | Yes | No (Node) | Yes |

## Shell Completions

```bash
# Bash
hotswap --completions bash > /etc/bash_completion.d/hotswap

# Zsh
hotswap --completions zsh > ~/.zfunc/_hotswap

# Fish
hotswap --completions fish > ~/.config/fish/completions/hotswap.fish
```

## Supported Platforms

- **macOS** — FSEvents
- **Linux** — inotify

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
