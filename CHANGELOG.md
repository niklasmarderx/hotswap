# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-04

### Added
- Core file watching with OS-native events (FSEvents/inotify) and debounce
- `.env` file diffing with color-coded output (added/changed/removed)
- Structured config diffing for TOML, JSON, and YAML with dot-notation keys
- Built-in validators for TOML, JSON, YAML syntax checking
- Custom validator commands (e.g. `nginx -t`, `cargo check`)
- Auto-detection of validators by file extension
- Unix signal sending (SIGHUP, SIGUSR1, SIGUSR2, SIGTERM) to named processes or PID files
- Multi-process coordination with dependency ordering (topological sort)
- Sequential and parallel reload strategies
- Health checks with configurable timeout between target reloads
- Pre/post hooks: global (`pre_reload`, `post_reload`, `on_error`) and per-target (`pre_signal`, `post_signal`)
- Automatic rollback on health check failure (backup config, restore, re-signal)
- `hotswap init` subcommand: scans for config files and running processes, generates `hotswap.toml`
- `hotswap.toml` config file support for complex multi-target setups
- JSON output mode (`--json`) for structured logging and CI integration
- Dry run mode (`--dry-run`) to preview actions without sending signals
- Diff-only mode (`--diff-only`) to watch changes without signaling
- Glob pattern support for watch paths and ignore patterns
- Shell completion generation (bash, zsh, fish, elvish, powershell)
- Sensitive value masking in diffs (passwords, tokens, API keys)
- Graceful shutdown on Ctrl+C
- Colored terminal output with ASCII banner

### Platforms
- macOS (FSEvents)
- Linux (inotify)

[0.1.0]: https://github.com/niklasmarderx/hotswap/releases/tag/v0.1.0
