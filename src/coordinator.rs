use anyhow::Result;

use crate::config::SignalType;
use crate::env::{self, EnvMap};
use crate::output::Output;
use crate::process::{self, ProcessInfo};
use crate::signal;
use crate::validate::{self, Validator};

/// A resolved target — a process to signal when files change.
#[derive(Debug)]
pub struct Target {
    pub name: String,
    pub signal_type: SignalType,
    pub process: Option<ProcessInfo>,
}

/// Options for a single reload cycle.
pub struct ReloadOptions {
    pub targets: Vec<Target>,
    pub validators: Vec<(String, Validator)>,
    pub dry_run: bool,
    pub diff_only: bool,
    pub env_files: Vec<String>,
}

/// Env state tracked across reload cycles.
pub struct EnvState {
    pub snapshots: std::collections::HashMap<String, EnvMap>,
}

impl EnvState {
    pub fn new(env_files: &[String]) -> Self {
        let mut snapshots = std::collections::HashMap::new();
        for file in env_files {
            snapshots.insert(file.clone(), env::snapshot_env(file));
        }
        Self { snapshots }
    }

    pub fn update_and_diff(&mut self, file: &str) -> Option<env::EnvDiff> {
        let old = self.snapshots.get(file).cloned().unwrap_or_default();
        let new = env::snapshot_env(file);
        self.snapshots.insert(file.to_string(), new.clone());
        let diff = env::diff_env(&old, &new);
        if diff.is_empty() {
            None
        } else {
            Some(diff)
        }
    }
}

/// Resolve targets from CLI arguments.
pub fn resolve_targets(
    signal_names: &[String],
    signal_type: &SignalType,
    pid_file: Option<&str>,
) -> Vec<Target> {
    let mut targets = Vec::new();

    // If a PID file is specified and there's at most one signal name
    if let Some(pf) = pid_file {
        match process::read_pid_file(pf) {
            Ok(info) => {
                Output::process_found(&info.name, info.pid);
                targets.push(Target {
                    name: info.name.clone(),
                    signal_type: signal_type.clone(),
                    process: Some(info),
                });
            }
            Err(e) => {
                Output::error(&format!("pid file: {e}"));
            }
        }
        return targets;
    }

    for name in signal_names {
        // Check if the name is actually a signal type (like SIGUSR1)
        if SignalType::from_str_loose(name).is_some() {
            // This is a signal specifier used as target — skip, handled by --signal-type
            continue;
        }

        let procs = process::find_by_name(name);
        if procs.is_empty() {
            Output::process_not_found(name);
            targets.push(Target {
                name: name.clone(),
                signal_type: signal_type.clone(),
                process: None,
            });
        } else {
            for p in procs {
                Output::process_found(&p.name, p.pid);
                targets.push(Target {
                    name: name.clone(),
                    signal_type: signal_type.clone(),
                    process: Some(p),
                });
            }
        }
    }

    targets
}

/// Run one reload cycle: validate, diff env, send signals.
pub fn reload(
    opts: &ReloadOptions,
    env_state: &mut EnvState,
    changed_paths: &[String],
) -> Result<()> {
    // Step 1: Validate changed files
    for path in changed_paths {
        // Check explicit validators
        for (pattern, validator) in &opts.validators {
            if path_matches(path, pattern) {
                match validator.validate(path) {
                    Ok(()) => Output::validation_ok(path, validator.name()),
                    Err(e) => {
                        Output::validation_failed(path, &e.to_string());
                        Output::error("reload aborted — fix validation errors first");
                        return Ok(());
                    }
                }
            }
        }

        // Auto-detect validators for config files
        if opts.validators.is_empty() {
            if let Some(validator) = validate::detect_validator(path) {
                match validator.validate(path) {
                    Ok(()) => Output::validation_ok(path, validator.name()),
                    Err(e) => {
                        Output::validation_failed(path, &e.to_string());
                        Output::error("reload aborted — fix validation errors first");
                        return Ok(());
                    }
                }
            }
        }
    }

    // Step 2: Show env diffs for .env files
    for path in changed_paths {
        if is_env_file(path) {
            if let Some(diff) = env_state.update_and_diff(path) {
                Output::env_diff(&diff.added, &diff.changed, &diff.removed);
            }
        }
    }

    // Step 3: If diff-only mode, stop here
    if opts.diff_only {
        return Ok(());
    }

    // Step 4: Send signals
    if opts.dry_run {
        Output::dry_run_header();
    }

    for target in &opts.targets {
        if let Some(ref proc) = target.process {
            if opts.dry_run {
                Output::info(&format!(
                    "would send {} to {} (pid {})",
                    target.signal_type, target.name, proc.pid
                ));
            } else {
                match signal::send_signal(proc.pid, &target.signal_type) {
                    Ok(()) => {
                        Output::signal_sent(target.signal_type.name(), &target.name, proc.pid);
                    }
                    Err(e) => {
                        Output::error(&format!(
                            "failed to signal {} (pid {}): {}",
                            target.name, proc.pid, e
                        ));
                    }
                }
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

fn path_matches(path: &str, pattern: &str) -> bool {
    // Simple matching: check if the pattern matches the filename or is a glob
    if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
        glob_pattern.matches(path)
            || std::path::Path::new(path)
                .file_name()
                .and_then(|f| f.to_str())
                .map(|f| glob_pattern.matches(f))
                .unwrap_or(false)
    } else {
        path.ends_with(pattern)
    }
}
