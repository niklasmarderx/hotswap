use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::config::{HooksConfig, ReloadStrategy, SignalType, TargetConfig};
use crate::diff;
use crate::env::{self, is_env_file, EnvMap};
use crate::hooks;
use crate::output::Output;
use crate::process::{self, ProcessInfo};
use crate::signal;
use crate::validate::{self, Validator};

/// Wrapper to send *mut Target across thread boundary.
/// SAFETY: caller must ensure each thread gets a unique target (no aliasing).
struct SendTarget(*mut Target);
unsafe impl Send for SendTarget {}

impl SendTarget {
    unsafe fn as_mut(&mut self) -> &mut Target {
        unsafe { &mut *self.0 }
    }
}

/// A resolved target — a process to signal when files change.
#[derive(Debug)]
pub struct Target {
    pub name: String,
    pub signal_type: SignalType,
    pub process: Option<ProcessInfo>,
    pub validators: Vec<(String, Validator)>,
    pub health_check: Option<String>,
    pub health_timeout: Duration,
    pub depends_on: Vec<String>,
    pub pre_signal: Option<String>,
    pub post_signal: Option<String>,
    pub enable_rollback: bool,
}

/// Options for a single reload cycle.
pub struct ReloadOptions<'a> {
    pub targets: &'a mut [Target],
    pub validators: &'a [(String, Validator)],
    pub dry_run: bool,
    pub diff_only: bool,
    pub strategy: &'a ReloadStrategy,
    pub hooks: Option<&'a HooksConfig>,
    pub rollback: bool,
}

/// Diff state tracked across reload cycles — env files + structured config files.
pub struct DiffState {
    pub env_snapshots: HashMap<String, EnvMap>,
    pub config_snapshots: HashMap<String, std::collections::BTreeMap<String, String>>,
}

impl DiffState {
    pub fn new(watch_paths: &[String]) -> Self {
        let mut env_snapshots = HashMap::new();
        let mut config_snapshots = HashMap::new();
        for file in watch_paths {
            if is_env_file(file) {
                env_snapshots.insert(file.clone(), env::snapshot_env(file));
            } else if diff::is_config_file(file) {
                config_snapshots.insert(file.clone(), diff::snapshot_config(file));
            }
        }
        Self {
            env_snapshots,
            config_snapshots,
        }
    }

    pub fn update_and_diff_env(&mut self, file: &str) -> Option<env::EnvDiff> {
        let old = self.env_snapshots.get(file).cloned().unwrap_or_default();
        let new = env::snapshot_env(file);
        self.env_snapshots.insert(file.to_string(), new.clone());
        let d = env::diff_env(&old, &new);
        if d.is_empty() {
            None
        } else {
            Some(d)
        }
    }

    pub fn update_and_diff_config(&mut self, file: &str) -> Option<diff::ConfigDiff> {
        let old = self.config_snapshots.get(file).cloned().unwrap_or_default();
        let new = diff::snapshot_config(file);
        self.config_snapshots.insert(file.to_string(), new.clone());
        let d = diff::diff_config(&old, &new);
        if d.is_empty() {
            None
        } else {
            Some(d)
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

    if let Some(pf) = pid_file {
        match process::read_pid_file(pf) {
            Ok(info) => {
                Output::process_found(&info.name, info.pid);
                targets.push(Target {
                    name: info.name.clone(),
                    signal_type: signal_type.clone(),
                    process: Some(info),
                    validators: Vec::new(),
                    health_check: None,
                    health_timeout: Duration::from_secs(5),
                    depends_on: Vec::new(),
                    pre_signal: None,
                    post_signal: None,
                    enable_rollback: false,
                });
            }
            Err(e) => {
                Output::error(&format!("pid file: {e}"));
            }
        }
        return targets;
    }

    for name in signal_names {
        if SignalType::from_str_loose(name).is_some() {
            continue;
        }

        let procs = process::find_by_name(name);
        if procs.is_empty() {
            Output::process_not_found(name);
            targets.push(Target {
                name: name.clone(),
                signal_type: signal_type.clone(),
                process: None,
                validators: Vec::new(),
                health_check: None,
                health_timeout: Duration::from_secs(5),
                depends_on: Vec::new(),
                pre_signal: None,
                post_signal: None,
                enable_rollback: false,
            });
        } else {
            for p in procs {
                Output::process_found(&p.name, p.pid);
                targets.push(Target {
                    name: name.clone(),
                    signal_type: signal_type.clone(),
                    process: Some(p),
                    validators: Vec::new(),
                    health_check: None,
                    health_timeout: Duration::from_secs(5),
                    depends_on: Vec::new(),
                    pre_signal: None,
                    post_signal: None,
                    enable_rollback: false,
                });
            }
        }
    }

    targets
}

/// Resolve targets from a hotswap.toml config file.
pub fn resolve_targets_from_config(targets: &[TargetConfig]) -> Result<Vec<Target>> {
    let mut seen = HashSet::new();
    for tc in targets {
        if !seen.insert(&tc.name) {
            anyhow::bail!("duplicate target name: '{}'", tc.name);
        }
    }

    let mut resolved = Vec::new();

    for tc in targets {
        let signal_type = tc
            .signal
            .as_deref()
            .and_then(SignalType::from_str_loose)
            .unwrap_or(SignalType::Sighup);

        // Resolve the process
        let process = if let Some(ref pf) = tc.pid_file {
            match process::read_pid_file(pf) {
                Ok(info) => {
                    Output::process_found(&info.name, info.pid);
                    Some(info)
                }
                Err(e) => {
                    Output::error(&format!("pid file for {}: {e}", tc.name));
                    None
                }
            }
        } else {
            let proc_name = tc.process_name.as_deref().unwrap_or(&tc.name);
            let procs = process::find_by_name(proc_name);
            if procs.is_empty() {
                Output::process_not_found(proc_name);
                None
            } else {
                if procs.len() > 1 {
                    Output::warn(&format!(
                        "found {} processes named '{}', using pid {}",
                        procs.len(),
                        proc_name,
                        procs[0].pid
                    ));
                }
                let p = procs.into_iter().next().unwrap();
                Output::process_found(&p.name, p.pid);
                Some(p)
            }
        };

        // Parse per-target validator
        let validators = tc
            .validate
            .as_ref()
            .map(|v| vec![(v.clone(), Validator::parse(v))])
            .unwrap_or_default();

        let health_timeout = Duration::from_millis(tc.health_timeout_ms.unwrap_or(5000));

        resolved.push(Target {
            name: tc.name.clone(),
            signal_type,
            process,
            validators,
            health_check: tc.health_check.clone(),
            health_timeout,
            depends_on: tc.depends_on.clone().unwrap_or_default(),
            pre_signal: tc.pre_signal.clone(),
            post_signal: tc.post_signal.clone(),
            enable_rollback: tc.enable_rollback.unwrap_or(false),
        });
    }

    Ok(resolved)
}

/// Sort targets by dependency order using Kahn's algorithm (topological sort).
/// Returns indices into the targets slice in dependency-first order.
pub fn sort_by_dependencies(targets: &[Target]) -> Result<Vec<usize>> {
    let name_to_idx: HashMap<&str, usize> = targets
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.as_str(), i))
        .collect();

    let n = targets.len();
    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, target) in targets.iter().enumerate() {
        for dep in &target.depends_on {
            if let Some(&dep_idx) = name_to_idx.get(dep.as_str()) {
                adj[dep_idx].push(i);
                in_degree[i] += 1;
            } else {
                anyhow::bail!(
                    "target '{}' depends on '{}' which is not defined",
                    target.name,
                    dep
                );
            }
        }
    }

    let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut sorted = Vec::with_capacity(n);

    while let Some(idx) = queue.pop() {
        sorted.push(idx);
        for &next in &adj[idx] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push(next);
            }
        }
    }

    if sorted.len() != n {
        let cycle_names: Vec<&str> = (0..n)
            .filter(|&i| in_degree[i] > 0)
            .map(|i| targets[i].name.as_str())
            .collect();
        anyhow::bail!(
            "dependency cycle detected among: {}",
            cycle_names.join(", ")
        );
    }

    Ok(sorted)
}

/// Run a health check command with a timeout.
fn run_health_check(command: &str, timeout: Duration) -> Result<()> {
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to run health check: {e}"))?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                } else {
                    let stderr = child
                        .stderr
                        .take()
                        .map(|mut s| {
                            let mut buf = String::new();
                            std::io::Read::read_to_string(&mut s, &mut buf).ok();
                            buf
                        })
                        .unwrap_or_default();
                    anyhow::bail!("health check failed (exit {}): {}", status, stderr.trim());
                }
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    anyhow::bail!("health check timed out after {}ms", timeout.as_millis());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                anyhow::bail!("health check error: {e}");
            }
        }
    }
}

/// Send signal to a single target and optionally run its health check.
fn signal_target(target: &mut Target, dry_run: bool) -> Result<()> {
    // Re-resolve process if not found at startup
    if target.process.is_none() {
        let procs = process::find_by_name(&target.name);
        if let Some(p) = procs.into_iter().next() {
            Output::process_found(&p.name, p.pid);
            target.process = Some(p);
        }
    }

    if let Some(ref proc) = target.process {
        // Run pre_signal hook
        if let Some(ref cmd) = target.pre_signal {
            if !dry_run {
                hooks::run_hook(&format!("pre_signal[{}]", target.name), cmd, None)?;
            } else {
                Output::info(&format!(
                    "would run pre_signal hook for {}: {}",
                    target.name, cmd
                ));
            }
        }

        if dry_run {
            Output::info(&format!(
                "would send {} to {} (pid {})",
                target.signal_type, target.name, proc.pid
            ));
            return Ok(());
        }

        signal::send_signal(proc.pid, &target.signal_type).map_err(|e| {
            anyhow::anyhow!("failed to signal {} (pid {}): {}", target.name, proc.pid, e)
        })?;
        Output::signal_sent(target.signal_type.name(), &target.name, proc.pid);

        // Run health check if configured
        if let Some(ref hc) = target.health_check {
            Output::health_check_start(&target.name, hc);
            match run_health_check(hc, target.health_timeout) {
                Ok(()) => Output::health_check_ok(&target.name),
                Err(e) => {
                    Output::health_check_failed(&target.name, &e.to_string());
                    return Err(e);
                }
            }
        }

        // Run post_signal hook
        if let Some(ref cmd) = target.post_signal {
            hooks::run_hook(&format!("post_signal[{}]", target.name), cmd, None)?;
        }
    } else {
        Output::warn(&format!(
            "target '{}' — no running process, skipping signal",
            target.name
        ));
    }
    Ok(())
}

/// Backup of config files for rollback.
struct ConfigBackup {
    backups: HashMap<String, Vec<u8>>,
}

impl ConfigBackup {
    fn capture(paths: &[String]) -> Result<Self> {
        let mut backups = HashMap::new();
        for path in paths {
            if let Ok(content) = std::fs::read(path) {
                backups.insert(path.clone(), content);
            }
        }
        Ok(Self { backups })
    }

    fn restore(&self) -> Result<()> {
        let files: Vec<String> = self.backups.keys().cloned().collect();
        Output::rollback_restoring(&files);
        for (path, content) in &self.backups {
            std::fs::write(path, content)
                .map_err(|e| anyhow::anyhow!("failed to restore {}: {}", path, e))?;
        }
        Ok(())
    }
}

/// Run one reload cycle: validate, diff env/config, run hooks, send signals.
pub fn reload(
    opts: &mut ReloadOptions,
    diff_state: &mut DiffState,
    changed_paths: &[String],
) -> Result<()> {
    // Step 1: Validate changed files (global validators from CLI)
    for path in changed_paths {
        for (pattern, validator) in opts.validators {
            if path_matches(path, pattern) {
                match validator.validate(path) {
                    Ok(()) => Output::validation_ok(path, validator.name()),
                    Err(e) => {
                        Output::validation_failed(path, &e.to_string());
                        Output::error("reload aborted — fix validation errors first");
                        run_on_error_hook(opts.hooks);
                        return Ok(());
                    }
                }
            }
        }

        // Per-target validators
        for target in opts.targets.iter() {
            for (pattern, validator) in &target.validators {
                if path_matches(path, pattern) {
                    match validator.validate(path) {
                        Ok(()) => Output::validation_ok(path, validator.name()),
                        Err(e) => {
                            Output::validation_failed(path, &e.to_string());
                            Output::error(&format!(
                                "reload aborted for {} — fix validation errors first",
                                target.name
                            ));
                            run_on_error_hook(opts.hooks);
                            return Ok(());
                        }
                    }
                }
            }
        }

        // Auto-detect validators for config files (only if no explicit validators)
        if opts.validators.is_empty() && opts.targets.iter().all(|t| t.validators.is_empty()) {
            if let Some(validator) = validate::detect_validator(path) {
                match validator.validate(path) {
                    Ok(()) => Output::validation_ok(path, validator.name()),
                    Err(e) => {
                        Output::validation_failed(path, &e.to_string());
                        Output::error("reload aborted — fix validation errors first");
                        run_on_error_hook(opts.hooks);
                        return Ok(());
                    }
                }
            }
        }
    }

    // Step 2: Show diffs for env files and structured config files
    for path in changed_paths {
        if is_env_file(path) {
            if let Some(d) = diff_state.update_and_diff_env(path) {
                Output::env_diff(&d.added, &d.changed, &d.removed);
            }
        } else if diff::is_config_file(path) {
            if let Some(d) = diff_state.update_and_diff_config(path) {
                Output::config_diff(path, &d);
            }
        }
    }

    // Step 3: If diff-only mode, stop here
    if opts.diff_only {
        return Ok(());
    }

    if opts.dry_run {
        Output::dry_run_header();
    }

    // Step 3.5: Capture backup for rollback if enabled
    let rollback_enabled = opts.rollback || opts.targets.iter().any(|t| t.enable_rollback);
    let backup = if rollback_enabled && !opts.dry_run {
        Some(ConfigBackup::capture(changed_paths)?)
    } else {
        None
    };

    // Step 4: Run pre_reload hook
    if let Some(hooks) = opts.hooks {
        if let Some(ref cmd) = hooks.pre_reload {
            if !opts.dry_run {
                if let Err(e) = hooks::run_hook("pre_reload", cmd, None) {
                    Output::error(&format!("pre_reload hook failed: {}", e));
                    run_on_error_hook(opts.hooks);
                    return Ok(());
                }
            } else {
                Output::info(&format!("would run pre_reload hook: {}", cmd));
            }
        }
    }

    // Step 5: Send signals based on strategy
    let signal_result = match opts.strategy {
        ReloadStrategy::Sequential => reload_sequential(opts.targets, opts.dry_run),
        ReloadStrategy::Parallel => reload_parallel(opts.targets, opts.dry_run),
    };

    // Step 5.5: Rollback on failure
    if let Err(ref e) = signal_result {
        if let Some(ref backup) = backup {
            Output::rollback_start(&e.to_string());
            match backup.restore() {
                Ok(()) => {
                    Output::rollback_ok();
                    // Re-signal targets with restored config
                    Output::info("re-signaling with restored config");
                    match match opts.strategy {
                        ReloadStrategy::Sequential => reload_sequential(opts.targets, opts.dry_run),
                        ReloadStrategy::Parallel => reload_parallel(opts.targets, opts.dry_run),
                    } {
                        Ok(()) => Output::info("recovery successful"),
                        Err(re) => Output::error(&format!("re-signaling failed: {}", re)),
                    }
                }
                Err(re) => {
                    Output::rollback_failed(&re.to_string());
                }
            }
            run_on_error_hook(opts.hooks);
            return Ok(());
        }
        run_on_error_hook(opts.hooks);
        return signal_result;
    }

    // Step 6: Run post_reload hook
    if let Some(hooks) = opts.hooks {
        if let Some(ref cmd) = hooks.post_reload {
            if !opts.dry_run {
                if let Err(e) = hooks::run_hook("post_reload", cmd, None) {
                    Output::error(&format!("post_reload hook failed: {}", e));
                }
            } else {
                Output::info(&format!("would run post_reload hook: {}", cmd));
            }
        }
    }

    Ok(())
}

/// Run the on_error hook if configured.
fn run_on_error_hook(hooks: Option<&HooksConfig>) {
    if let Some(hooks) = hooks {
        if let Some(ref cmd) = hooks.on_error {
            if let Err(e) = hooks::run_hook("on_error", cmd, None) {
                Output::error(&format!("on_error hook failed: {}", e));
            }
        }
    }
}

/// Sequential reload: signal targets one by one in dependency order.
fn reload_sequential(targets: &mut [Target], dry_run: bool) -> Result<()> {
    let order = sort_by_dependencies(targets)?;

    for &idx in &order {
        let target = &mut targets[idx];
        if let Err(e) = signal_target(target, dry_run) {
            Output::error(&format!(
                "stopping sequential reload at '{}': {}",
                target.name, e
            ));
            return Err(e);
        }
    }

    Ok(())
}

/// Parallel reload: signal targets in dependency layers concurrently.
fn reload_parallel(targets: &mut [Target], dry_run: bool) -> Result<()> {
    let order = sort_by_dependencies(targets)?;

    // Group into dependency layers
    let name_to_idx: HashMap<&str, usize> = targets
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.as_str(), i))
        .collect();

    let mut depth = vec![0usize; targets.len()];
    for &idx in &order {
        let max_dep_depth = targets[idx]
            .depends_on
            .iter()
            .filter_map(|d| name_to_idx.get(d.as_str()).map(|&i| depth[i]))
            .max()
            .unwrap_or(0);
        depth[idx] = if targets[idx].depends_on.is_empty() {
            0
        } else {
            max_dep_depth + 1
        };
    }

    let max_depth = depth.iter().max().copied().unwrap_or(0);
    for d in 0..=max_depth {
        let layer: Vec<usize> = order.iter().copied().filter(|&i| depth[i] == d).collect();

        if layer.len() == 1 {
            if let Err(e) = signal_target(&mut targets[layer[0]], dry_run) {
                Output::error(&format!(
                    "stopping parallel reload at '{}': {}",
                    targets[layer[0]].name, e
                ));
                return Err(e);
            }
        } else {
            // Multiple targets in layer — signal concurrently
            let send_targets: Vec<SendTarget> = layer
                .iter()
                .map(|&idx| SendTarget(&mut targets[idx] as *mut Target))
                .collect();

            let errors: Vec<String> = std::thread::scope(|s| {
                let handles: Vec<_> = send_targets
                    .into_iter()
                    .map(|mut st| {
                        s.spawn(move || {
                            let target = unsafe { st.as_mut() };
                            signal_target(target, dry_run)
                        })
                    })
                    .collect();

                handles
                    .into_iter()
                    .filter_map(|h| h.join().ok().and_then(|r| r.err()))
                    .map(|e| e.to_string())
                    .collect()
            });

            if !errors.is_empty() {
                for err in &errors {
                    Output::error(err);
                }
                anyhow::bail!("parallel reload failed: {}", errors.join("; "));
            }
        }
    }

    Ok(())
}

fn path_matches(path: &str, pattern: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target(name: &str, depends_on: Vec<&str>) -> Target {
        Target {
            name: name.to_string(),
            signal_type: SignalType::Sighup,
            process: None,
            validators: Vec::new(),
            health_check: None,
            health_timeout: Duration::from_secs(5),
            depends_on: depends_on.into_iter().map(String::from).collect(),
            pre_signal: None,
            post_signal: None,
            enable_rollback: false,
        }
    }

    #[test]
    fn diff_state_new_and_diff() {
        let dir = tempfile::tempdir().unwrap();
        let env_path = dir.path().join(".env");
        std::fs::write(&env_path, "KEY=value\n").unwrap();

        let path_str = env_path.to_string_lossy().to_string();
        let mut state = DiffState::new(std::slice::from_ref(&path_str));

        // No change yet
        assert!(state.update_and_diff_env(&path_str).is_none());

        // Modify the file
        std::fs::write(&env_path, "KEY=new_value\nADDED=yes\n").unwrap();
        let diff = state.update_and_diff_env(&path_str).unwrap();
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.added.len(), 1);
    }

    #[test]
    fn sort_no_dependencies() {
        let targets = vec![
            make_target("a", vec![]),
            make_target("b", vec![]),
            make_target("c", vec![]),
        ];
        let order = sort_by_dependencies(&targets).unwrap();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn sort_linear_chain() {
        let targets = vec![
            make_target("a", vec![]),
            make_target("b", vec!["a"]),
            make_target("c", vec!["b"]),
        ];
        let order = sort_by_dependencies(&targets).unwrap();

        // a must come before b, b before c
        let pos_a = order.iter().position(|&i| i == 0).unwrap();
        let pos_b = order.iter().position(|&i| i == 1).unwrap();
        let pos_c = order.iter().position(|&i| i == 2).unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn sort_diamond_dependency() {
        let targets = vec![
            make_target("a", vec![]),
            make_target("b", vec!["a"]),
            make_target("c", vec!["a"]),
            make_target("d", vec!["b", "c"]),
        ];
        let order = sort_by_dependencies(&targets).unwrap();
        let pos_a = order.iter().position(|&i| i == 0).unwrap();
        let pos_b = order.iter().position(|&i| i == 1).unwrap();
        let pos_c = order.iter().position(|&i| i == 2).unwrap();
        let pos_d = order.iter().position(|&i| i == 3).unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
    }

    #[test]
    fn sort_cycle_detection() {
        let targets = vec![make_target("a", vec!["b"]), make_target("b", vec!["a"])];
        let result = sort_by_dependencies(&targets);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cycle"));
    }

    #[test]
    fn sort_unknown_dependency() {
        let targets = vec![make_target("a", vec!["nonexistent"])];
        let result = sort_by_dependencies(&targets);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[test]
    fn path_matches_glob() {
        assert!(path_matches("config.toml", "*.toml"));
        assert!(path_matches("/path/to/config.toml", "*.toml"));
        assert!(!path_matches("config.json", "*.toml"));
    }

    #[test]
    fn path_matches_literal() {
        assert!(path_matches("/path/to/config.toml", "config.toml"));
    }

    #[test]
    fn reload_diff_only_does_not_signal() {
        let mut targets = vec![make_target("test", vec![])];
        let validators: Vec<(String, Validator)> = Vec::new();
        let mut opts = ReloadOptions {
            targets: &mut targets,
            validators: &validators,
            dry_run: false,
            diff_only: true,
            strategy: &ReloadStrategy::Sequential,
            hooks: None,
            rollback: false,
        };
        let mut diff_state = DiffState::new(&[]);
        let result = reload(&mut opts, &mut diff_state, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn reload_dry_run_does_not_signal() {
        let mut targets = vec![make_target("test", vec![])];
        let validators: Vec<(String, Validator)> = Vec::new();
        let mut opts = ReloadOptions {
            targets: &mut targets,
            validators: &validators,
            dry_run: true,
            diff_only: false,
            strategy: &ReloadStrategy::Sequential,
            hooks: None,
            rollback: false,
        };
        let mut diff_state = DiffState::new(&[]);
        let result = reload(&mut opts, &mut diff_state, &[]);
        assert!(result.is_ok());
    }
}
