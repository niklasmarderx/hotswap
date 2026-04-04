# hotswap — Zero-Downtime Config & Env Reloader

## Vision
A single Rust binary that watches config files, .env files, and secrets — then intelligently reloads your running processes without downtime. Like `watchexec` but specifically built for config/env reloading with validation, multi-process coordination, and secret rotation.

## Why This Doesn't Exist Yet
- `entr` / `watchexec` — watch files and re-run commands, but don't reload env vars in running processes
- `nodemon` — Node-only, restarts entire process (not zero-downtime)
- `systemfd` + `listenfd` — Rust-specific, only for socket passing
- `direnv` — loads .env on shell entry, doesn't watch or reload running processes

**The gap:** No tool watches .env/config changes AND gracefully signals running processes with the new values.

## Core Features (v1.0)

### 1. File Watching
- Watch `.env`, `.env.local`, `config.toml`, `config.yaml`, `config.json`, etc.
- Recursive directory watching with glob patterns
- Debounce (default 500ms, configurable)
- Ignore patterns (.git, node_modules, target)

### 2. Process Signaling
- Send configurable signals on change: SIGHUP (default), SIGUSR1, SIGUSR2, SIGTERM+restart
- Named process targeting: `hotswap --watch .env --signal myapp`
- PID file support: `hotswap --watch .env --pid-file /tmp/myapp.pid`
- Process group support: signal all children

### 3. Env Injection
- Parse .env files and compute diff (what changed)
- Show human-readable diff: `DATABASE_URL: old_value → new_value`
- Optional: Write changed vars to a temp file the app can source
- Support for .env expansion ($VAR references)

### 4. Config Validation
- Validate before reload (don't signal if config is broken)
- Built-in validators: JSON, TOML, YAML syntax check
- Custom validators: `hotswap --validate "cargo check" --watch Cargo.toml`
- Show validation errors clearly

### 5. Multi-Process Coordination
- Watch one config, reload multiple processes in order
- `hotswap --watch .env --signal frontend --signal backend --signal worker`
- Sequential or parallel reload strategies
- Health check between reloads: wait for process to be healthy before next

### 6. Dry Run & Logging
- `hotswap --dry-run` — show what would happen without doing it
- Structured JSON logging for production use
- Colored terminal output for dev use
- History of reloads with timestamps

## Architecture

```
hotswap
├── src/
│   ├── main.rs          — CLI entry, clap args
│   ├── watcher.rs       — File system watcher (notify crate)
│   ├── signal.rs        — Process signaling (nix crate)  
│   ├── env.rs           — .env parsing and diffing
│   ├── validate.rs      — Config validation pipeline
│   ├── config.rs        — hotswap's own config (hotswap.toml)
│   ├── process.rs       — Process discovery (by name, PID, pid-file)
│   ├── coordinator.rs   — Multi-process reload orchestration
│   └── output.rs        — Terminal output formatting
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── .github/
│   └── workflows/
│       └── ci.yml
└── tests/
    ├── integration/
    │   ├── env_reload.rs
    │   └── signal_test.rs
    └── fixtures/
        ├── valid.env
        ├── invalid.toml
        └── test-config.yaml
```

## CLI Interface

```bash
# Basic: watch .env file, send SIGHUP to process
hotswap --watch .env --signal myapp

# Multiple files
hotswap --watch .env --watch config.toml --signal myapp

# With validation
hotswap --watch config.toml --validate "toml" --signal myapp

# Custom validation command
hotswap --watch Cargo.toml --validate "cargo check" --signal myapp

# Multi-process coordinated reload
hotswap --watch .env \
  --signal frontend \
  --signal backend \
  --signal worker \
  --strategy sequential

# Dry run
hotswap --watch .env --signal myapp --dry-run

# Show env diff only (no signal)
hotswap --watch .env --diff-only

# With PID file
hotswap --watch .env --pid-file /tmp/myapp.pid --signal SIGUSR1

# Config file mode
hotswap --config hotswap.toml
```

## hotswap.toml (Optional Config File)

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
health_check = "curl -sf http://localhost:3000/health"
health_timeout_ms = 5000

[[targets]]
name = "frontend"  
signal = "SIGUSR1"
process_name = "node"
depends_on = ["backend"]
```

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive", "env"] }
notify = "8"                    # File system watcher
nix = { version = "0.29", features = ["signal", "process"] }  # Unix signals
dotenvy = "0.15"                # .env parsing
toml = "0.8"                    # TOML parsing/validation
serde = { version = "1", features = ["derive"] }
serde_json = "1"                # JSON validation
serde_yaml = "0.9"              # YAML validation  
colored = "2"                   # Terminal colors
tracing = "0.1"                 # Structured logging
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tokio = { version = "1", features = ["full"] }
glob = "0.3"                    # Glob pattern matching
sysinfo = "0.33"                # Process discovery by name
```

## Implementation Order

1. **Phase 1: Core Watch + Signal** (MVP)
   - CLI args (clap)
   - File watcher with debounce
   - Process discovery (by name + PID file)
   - Basic signaling (SIGHUP)
   - Terminal output

2. **Phase 2: Env Diffing + Validation**
   - .env parsing and diff
   - Built-in validators (TOML, JSON, YAML)
   - Custom validator commands
   - Colored diff output

3. **Phase 3: Multi-Process + Config File**
   - hotswap.toml support
   - Multi-target coordination
   - Health checks
   - Sequential/parallel strategies

4. **Phase 4: Polish**
   - README with examples and GIFs
   - CI/CD pipeline
   - Homebrew formula
   - cargo install support
   - Integration tests
