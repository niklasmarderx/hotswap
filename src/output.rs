use colored::Colorize;
use std::collections::BTreeMap;

/// Styles for terminal output.
pub struct Output;

impl Output {
    pub fn banner() {
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
        for path in paths {
            eprintln!("  {} {}", "watching".green().bold(), path);
        }
    }

    pub fn file_changed(path: &str) {
        eprintln!("\n  {} {}", "changed".yellow().bold(), path.white().bold());
    }

    pub fn signal_sent(signal: &str, target: &str, pid: u32) {
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
        eprintln!(
            "  {} {} (pid {})",
            "found".green().bold(),
            name.white(),
            pid
        );
    }

    pub fn process_not_found(name: &str) {
        eprintln!(
            "  {} process not found: {}",
            "error".red().bold(),
            name.white()
        );
    }

    pub fn validation_ok(path: &str, kind: &str) {
        eprintln!("  {} {} ({})", "valid".green().bold(), path, kind.dimmed());
    }

    pub fn validation_failed(path: &str, error: &str) {
        eprintln!(
            "  {} {} — {}",
            "invalid".red().bold(),
            path.white(),
            error.red()
        );
    }

    pub fn dry_run_header() {
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
            eprintln!("  {} no env changes detected", "info".dimmed());
            return;
        }

        eprintln!("  {} env diff:", "diff".cyan().bold());

        for (key, value) in added {
            eprintln!(
                "    {} {}={}",
                "+".green().bold(),
                key.green(),
                mask_value(value).green()
            );
        }

        for (key, (old, new)) in changed {
            eprintln!(
                "    {} {}  {} {} {}",
                "~".yellow().bold(),
                key.yellow(),
                mask_value(old).dimmed().strikethrough(),
                "→".dimmed(),
                mask_value(new).yellow()
            );
        }

        for (key, value) in removed {
            eprintln!(
                "    {} {}={}",
                "-".red().bold(),
                key.red(),
                mask_value(value).red()
            );
        }
    }

    pub fn error(msg: &str) {
        eprintln!("  {} {}", "error".red().bold(), msg);
    }

    pub fn info(msg: &str) {
        eprintln!("  {} {}", "info".blue().bold(), msg);
    }

    pub fn waiting() {
        eprintln!("\n  {} waiting for changes...\n", "ready".green().bold());
    }
}

/// Mask potentially sensitive values (passwords, tokens, etc.)
fn mask_value(value: &str) -> String {
    if value.len() > 60 {
        format!("{}...", &value[..57])
    } else {
        value.to_string()
    }
}
