use colored::Colorize;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

static JSON_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_json_mode(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::Relaxed);
}

fn is_json() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

fn json_event(event_type: &str, data: serde_json::Value) {
    let event = serde_json::json!({
        "event": event_type,
        "data": data,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    println!("{}", serde_json::to_string(&event).unwrap());
}

/// Styles for terminal output.
pub struct Output;

impl Output {
    pub fn banner() {
        if is_json() {
            return;
        }
        eprintln!(
            "{}",
            r#"
  ██╗  ██╗ ██████╗ ████████╗███████╗██╗    ██╗ █████╗ ██████╗
  ██║  ██║██╔═══██╗╚══██╔══╝██╔════╝██║    ██║██╔══██╗██╔══██╗
  ███████║██║   ██║   ██║   ███████╗██║ █╗ ██║███████║██████╔╝
  ██╔══██║██║   ██║   ██║   ╚════██║██║███╗██║██╔══██║██╔═══╝
  ██║  ██║╚██████╔╝   ██║   ███████║╚███╔███╔╝██║  ██║██║
  ╚═╝  ╚═╝ ╚═════╝    ╚═╝   ╚══════╝ ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝"#
                .cyan()
                .bold()
        );
        eprintln!(
            "  {} {}\n",
            "zero-downtime config reloader".dimmed(),
            format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
        );
    }

    pub fn watching(paths: &[String]) {
        if is_json() {
            json_event("watching", serde_json::json!({ "paths": paths }));
            return;
        }
        for path in paths {
            eprintln!("  {} {}", "watching".green().bold(), path);
        }
    }

    pub fn file_changed(path: &str) {
        if is_json() {
            json_event("file_changed", serde_json::json!({ "path": path }));
            return;
        }
        eprintln!("\n  {} {}", "changed".yellow().bold(), path.white().bold());
    }

    pub fn signal_sent(signal: &str, target: &str, pid: u32) {
        if is_json() {
            json_event(
                "signal_sent",
                serde_json::json!({ "signal": signal, "target": target, "pid": pid }),
            );
            return;
        }
        eprintln!(
            "  {} {} {} {} (pid {})",
            "signal".magenta().bold(),
            signal.cyan(),
            "→".dimmed(),
            target.white().bold(),
            pid
        );
    }

    pub fn process_found(name: &str, pid: u32) {
        if is_json() {
            json_event(
                "process_found",
                serde_json::json!({ "name": name, "pid": pid }),
            );
            return;
        }
        eprintln!(
            "  {} {} (pid {})",
            "found".green().bold(),
            name.white(),
            pid
        );
    }

    pub fn process_not_found(name: &str) {
        if is_json() {
            json_event("process_not_found", serde_json::json!({ "name": name }));
            return;
        }
        eprintln!(
            "  {} process not found: {}",
            "error".red().bold(),
            name.white()
        );
    }

    pub fn validation_ok(path: &str, kind: &str) {
        if is_json() {
            json_event(
                "validation_ok",
                serde_json::json!({ "path": path, "validator": kind }),
            );
            return;
        }
        eprintln!("  {} {} ({})", "valid".green().bold(), path, kind.dimmed());
    }

    pub fn validation_failed(path: &str, error: &str) {
        if is_json() {
            json_event(
                "validation_failed",
                serde_json::json!({ "path": path, "error": error }),
            );
            return;
        }
        eprintln!(
            "  {} {} — {}",
            "invalid".red().bold(),
            path.white(),
            error.red()
        );
    }

    pub fn dry_run_header() {
        if is_json() {
            json_event("dry_run", serde_json::json!({}));
            return;
        }
        eprintln!(
            "\n  {}",
            "DRY RUN — no signals will be sent".yellow().bold()
        );
    }

    pub fn env_diff(
        added: &BTreeMap<String, String>,
        changed: &BTreeMap<String, (String, String)>,
        removed: &BTreeMap<String, String>,
    ) {
        if added.is_empty() && changed.is_empty() && removed.is_empty() {
            if is_json() {
                return;
            }
            eprintln!("  {} no env changes detected", "info".dimmed());
            return;
        }

        if is_json() {
            let changed_json: serde_json::Map<String, serde_json::Value> = changed
                .iter()
                .map(|(k, (o, n))| (k.clone(), serde_json::json!({"old": o, "new": n})))
                .collect();
            json_event(
                "env_diff",
                serde_json::json!({
                    "added": added,
                    "changed": changed_json,
                    "removed": removed
                }),
            );
            return;
        }

        eprintln!("  {} env diff:", "diff".cyan().bold());

        for (key, value) in added {
            eprintln!(
                "    {} {}={}",
                "+".green().bold(),
                key.green(),
                mask_env_value(key, value).green()
            );
        }

        for (key, (old, new)) in changed {
            eprintln!(
                "    {} {}  {} {} {}",
                "~".yellow().bold(),
                key.yellow(),
                mask_env_value(key, old).dimmed().strikethrough(),
                "→".dimmed(),
                mask_env_value(key, new).yellow()
            );
        }

        for (key, value) in removed {
            eprintln!(
                "    {} {}={}",
                "-".red().bold(),
                key.red(),
                mask_env_value(key, value).red()
            );
        }
    }

    pub fn error(msg: &str) {
        if is_json() {
            json_event("error", serde_json::json!({ "message": msg }));
            return;
        }
        eprintln!("  {} {}", "error".red().bold(), msg);
    }

    pub fn info(msg: &str) {
        if is_json() {
            json_event("info", serde_json::json!({ "message": msg }));
            return;
        }
        eprintln!("  {} {}", "info".blue().bold(), msg);
    }

    pub fn warn(msg: &str) {
        if is_json() {
            json_event("warning", serde_json::json!({ "message": msg }));
            return;
        }
        eprintln!("  {} {}", "warn".yellow().bold(), msg);
    }

    pub fn health_check_start(target: &str, command: &str) {
        if is_json() {
            json_event(
                "health_check_start",
                serde_json::json!({ "target": target, "command": command }),
            );
            return;
        }
        eprintln!(
            "  {} {} ({})",
            "health".blue().bold(),
            target.white(),
            command.dimmed()
        );
    }

    pub fn health_check_ok(target: &str) {
        if is_json() {
            json_event("health_check_ok", serde_json::json!({ "target": target }));
            return;
        }
        eprintln!(
            "  {} {} is healthy",
            "health".green().bold(),
            target.white()
        );
    }

    pub fn health_check_failed(target: &str, error: &str) {
        if is_json() {
            json_event(
                "health_check_failed",
                serde_json::json!({ "target": target, "error": error }),
            );
            return;
        }
        eprintln!(
            "  {} {} — {}",
            "health".red().bold(),
            target.white(),
            error.red()
        );
    }

    pub fn waiting() {
        if is_json() {
            json_event("ready", serde_json::json!({}));
            return;
        }
        eprintln!("\n  {} waiting for changes...\n", "ready".green().bold());
    }
}

/// Patterns in variable names that indicate sensitive values.
const SENSITIVE_PATTERNS: &[&str] = &[
    "PASSWORD",
    "SECRET",
    "TOKEN",
    "API_KEY",
    "APIKEY",
    "PRIVATE_KEY",
    "AUTH",
    "CREDENTIAL",
];

/// Mask potentially sensitive values (passwords, tokens, etc.)
fn mask_env_value(key: &str, value: &str) -> String {
    let upper = key.to_uppercase();
    let is_sensitive = SENSITIVE_PATTERNS.iter().any(|p| upper.contains(p));
    if is_sensitive && value.len() > 4 {
        format!("{}****", &value[..4])
    } else if is_sensitive {
        "****".to_string()
    } else if value.len() > 60 {
        format!("{}...", &value[..57])
    } else {
        value.to_string()
    }
}
