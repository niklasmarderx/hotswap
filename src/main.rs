mod config;
mod coordinator;
mod env;
mod output;
mod process;
mod signal;
mod validate;
mod watcher;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;

use config::{ReloadStrategy, SignalType};
use coordinator::{EnvState, ReloadOptions};
use env::is_env_file;
use output::Output;
use validate::Validator;
use watcher::WatchEvent;

/// hotswap — zero-downtime config & env reloader
///
/// Watch config files, validate changes, and signal processes to reload
/// without downtime.
#[derive(Parser, Debug)]
#[command(name = "hotswap", version, about, long_about = None)]
struct Cli {
    /// Files or directories to watch (supports globs)
    #[arg(short, long = "watch", required_unless_present_any = ["config", "completions"])]
    watch_paths: Vec<String>,

    /// Process names to signal on change
    #[arg(short, long = "signal")]
    signal_targets: Vec<String>,

    /// Signal type to send [default: SIGHUP]
    #[arg(
        long = "signal-type",
        default_value = "SIGHUP",
        hide_default_value = true
    )]
    signal_type: String,

    /// PID file to read process ID from
    #[arg(long)]
    pid_file: Option<String>,

    /// Validators to run before signaling (toml, json, yaml, or shell command)
    #[arg(long = "validate")]
    validators: Vec<String>,

    /// Debounce interval in milliseconds
    #[arg(short, long, default_value = "500")]
    debounce: u64,

    /// Dry run — show what would happen without sending signals
    #[arg(long)]
    dry_run: bool,

    /// Diff-only mode — show env diffs without signaling
    #[arg(long)]
    diff_only: bool,

    /// Path to a hotswap.toml config file
    #[arg(short, long)]
    config: Option<String>,

    /// Patterns to ignore (globs)
    #[arg(short, long)]
    ignore: Vec<String>,

    /// Suppress banner output
    #[arg(short, long)]
    quiet: bool,

    /// Reload strategy: sequential or parallel
    #[arg(long, default_value = "sequential")]
    strategy: String,

    /// Output as JSON (structured logging)
    #[arg(long)]
    json: bool,

    /// Generate shell completions and exit
    #[arg(long, value_enum)]
    completions: Option<Shell>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle completions generation
    if let Some(shell) = cli.completions {
        let mut cmd = Cli::command();
        clap_complete::generate(shell, &mut cmd, "hotswap", &mut std::io::stdout());
        return Ok(());
    }

    // Set up tracing
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("hotswap=info".parse().unwrap()),
        )
        .with_target(false)
        .without_time();

    if cli.json {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    // Set JSON output mode globally
    output::set_json_mode(cli.json);

    if !cli.quiet && !cli.json {
        Output::banner();
    }

    // Parse signal type (used as default for CLI-based targets)
    let signal_type = SignalType::from_str_loose(&cli.signal_type).unwrap_or_else(|| {
        Output::error(&format!(
            "unknown signal type: {} (using SIGHUP)",
            cli.signal_type
        ));
        SignalType::Sighup
    });

    // Parse strategy
    let strategy = ReloadStrategy::from_str_loose(&cli.strategy).unwrap_or_else(|| {
        Output::error(&format!(
            "unknown strategy: {} (using sequential)",
            cli.strategy
        ));
        ReloadStrategy::Sequential
    });

    // Load config file or use CLI args
    let (watch_paths, ignore_extra, debounce, mut targets, validators, resolved_strategy) =
        if let Some(ref config_path) = cli.config {
            let cfg = config::HotswapConfig::from_file(config_path)?;

            let watch = cfg.watch.files;
            let ignore = cfg.watch.ignore.unwrap_or_default();
            let debounce = cfg.watch.debounce_ms.unwrap_or(cli.debounce);
            let targets = coordinator::resolve_targets_from_config(&cfg.targets)?;
            let strat = cfg.strategy.unwrap_or(strategy);

            (watch, ignore, debounce, targets, Vec::new(), strat)
        } else {
            let targets = if cli.diff_only {
                Vec::new()
            } else {
                coordinator::resolve_targets(
                    &cli.signal_targets,
                    &signal_type,
                    cli.pid_file.as_deref(),
                )
            };

            let validators: Vec<(String, Validator)> = cli
                .validators
                .iter()
                .map(|v| (v.clone(), Validator::parse(v)))
                .collect();

            (
                cli.watch_paths.clone(),
                cli.ignore.clone(),
                cli.debounce,
                targets,
                validators,
                strategy,
            )
        };

    if watch_paths.is_empty() {
        anyhow::bail!("no files to watch — specify --watch or --config");
    }

    // Build ignore patterns
    let mut ignore = watcher::default_ignore_patterns();
    ignore.extend(ignore_extra);

    // Display what we're watching
    Output::watching(&watch_paths);

    // Identify env files for tracking diffs
    let env_files: Vec<String> = watch_paths
        .iter()
        .filter(|p| is_env_file(p))
        .cloned()
        .collect();

    // Initialize env state
    let mut env_state = EnvState::new(&env_files);

    // Start watching
    let (rx, shutdown_tx, _watcher) = watcher::start_watching(&watch_paths, debounce, &ignore)?;

    // Set up Ctrl+C handler
    let ctrlc_tx = shutdown_tx.clone();
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.send(WatchEvent::Shutdown);
    })?;

    Output::waiting();

    // Main event loop
    loop {
        match rx.recv() {
            Ok(WatchEvent::FilesChanged(paths)) => {
                let changed: Vec<String> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();

                for path in &changed {
                    Output::file_changed(path);
                }

                let mut opts = ReloadOptions {
                    targets: &mut targets,
                    validators: &validators,
                    dry_run: cli.dry_run,
                    diff_only: cli.diff_only,
                    strategy: &resolved_strategy,
                };

                if let Err(e) = coordinator::reload(&mut opts, &mut env_state, &changed) {
                    Output::error(&e.to_string());
                }

                Output::waiting();
            }
            Ok(WatchEvent::Error(e)) => {
                Output::error(&e);
            }
            Ok(WatchEvent::Shutdown) => {
                Output::info("shutting down");
                break;
            }
            Err(_) => {
                Output::error("watcher channel closed");
                break;
            }
        }
    }

    Ok(())
}
