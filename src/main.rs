#![allow(dead_code)]

mod config;
mod coordinator;
mod env;
mod output;
mod process;
mod signal;
mod validate;
mod watcher;

use anyhow::Result;
use clap::Parser;

use config::SignalType;
use coordinator::{EnvState, ReloadOptions};
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
    #[arg(short, long = "watch", required_unless_present = "config")]
    watch_paths: Vec<String>,

    /// Process names to signal on change
    #[arg(short, long = "signal")]
    signal_targets: Vec<String>,

    /// Signal type to send [default: SIGHUP]
    #[arg(long = "signal-type", default_value = "SIGHUP")]
    signal_type: String,

    /// PID file to read process ID from
    #[arg(long)]
    pid_file: Option<String>,

    /// Validators to run before signaling (toml, json, yaml, or shell command)
    #[arg(short = 'V', long = "validate")]
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("hotswap=info".parse().unwrap()),
        )
        .with_target(false)
        .without_time()
        .init();

    if !cli.quiet {
        Output::banner();
    }

    // Parse signal type
    let signal_type = SignalType::from_str_loose(&cli.signal_type).unwrap_or_else(|| {
        Output::error(&format!(
            "unknown signal type: {} (using SIGHUP)",
            cli.signal_type
        ));
        SignalType::Sighup
    });

    // Determine watch paths
    let watch_paths = if let Some(ref config_path) = cli.config {
        let cfg = config::HotswapConfig::from_file(config_path)?;
        cfg.watch.files
    } else {
        cli.watch_paths.clone()
    };

    if watch_paths.is_empty() {
        anyhow::bail!("no files to watch — specify --watch or --config");
    }

    // Build ignore patterns
    let mut ignore = watcher::default_ignore_patterns();
    ignore.extend(cli.ignore.iter().cloned());

    // Display what we're watching
    Output::watching(&watch_paths);

    // Identify env files for tracking diffs
    let env_files: Vec<String> = watch_paths
        .iter()
        .filter(|p| is_env_file(p))
        .cloned()
        .collect();

    // Build validators
    let validators: Vec<(String, Validator)> = cli
        .validators
        .iter()
        .map(|v| {
            let validator = Validator::parse(v);
            (v.clone(), validator)
        })
        .collect();

    // Resolve targets
    if cli.diff_only {
        // No targets needed
    } else {
        coordinator::resolve_targets(&cli.signal_targets, &signal_type, cli.pid_file.as_deref());
    };

    // Initialize env state
    let mut env_state = EnvState::new(&env_files);

    // Start watching
    let rx = watcher::start_watching(&watch_paths, cli.debounce, &ignore)?;
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

                let opts = ReloadOptions {
                    targets: coordinator::resolve_targets(
                        &cli.signal_targets,
                        &signal_type,
                        cli.pid_file.as_deref(),
                    ),
                    validators: validators.clone(),
                    dry_run: cli.dry_run,
                    diff_only: cli.diff_only,
                    env_files: env_files.clone(),
                };

                if let Err(e) = coordinator::reload(&opts, &mut env_state, &changed) {
                    Output::error(&e.to_string());
                }

                Output::waiting();
            }
            Ok(WatchEvent::Error(e)) => {
                Output::error(&e);
            }
            Err(_) => {
                Output::error("watcher channel closed");
                break;
            }
        }
    }

    Ok(())
}

fn is_env_file(path: &str) -> bool {
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    filename.starts_with(".env") || filename.ends_with(".env")
}
