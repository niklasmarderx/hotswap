use anyhow::Result;
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebouncedEvent};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use crate::output::Output;

/// Events produced by the file watcher.
#[derive(Debug)]
pub enum WatchEvent {
    /// One or more files changed.
    FilesChanged(Vec<PathBuf>),
    /// An error occurred in the watcher.
    Error(String),
    /// Shutdown requested (e.g. Ctrl+C).
    Shutdown,
}

/// Handle that keeps the file watcher alive. Drop to stop watching.
pub struct WatchHandle {
    _debouncer: notify_debouncer_full::Debouncer<
        notify::RecommendedWatcher,
        notify_debouncer_full::FileIdMap,
    >,
}

/// Starts watching the given paths with debounce.
/// Returns (receiver, sender, handle). The sender can be used to inject events (e.g. Shutdown).
pub fn start_watching(
    paths: &[String],
    debounce_ms: u64,
    ignore_patterns: &[String],
) -> Result<(
    mpsc::Receiver<WatchEvent>,
    mpsc::Sender<WatchEvent>,
    WatchHandle,
)> {
    let (tx, rx) = mpsc::channel();
    let ignore_globs = compile_ignore_patterns(ignore_patterns);

    let event_tx = tx.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(debounce_ms),
        None,
        move |result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| match result {
            Ok(events) => {
                let changed: Vec<PathBuf> = events
                    .into_iter()
                    .flat_map(|e| e.event.paths)
                    .filter(|p| !is_ignored(p, &ignore_globs))
                    .collect();

                if !changed.is_empty() {
                    let _ = event_tx.send(WatchEvent::FilesChanged(changed));
                }
            }
            Err(errors) => {
                for e in errors {
                    let _ = event_tx.send(WatchEvent::Error(e.to_string()));
                }
            }
        },
    )?;

    for path_str in paths {
        // Expand glob patterns
        let matched: Vec<PathBuf> = glob::glob(path_str)
            .map(|entries| entries.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();

        if matched.is_empty() {
            if is_glob_pattern(path_str) {
                Output::warn(&format!("glob '{}' matched no files", path_str));
                continue;
            }
            // Not a glob — watch as a literal path
            let path = Path::new(path_str);
            let mode = if path.is_dir() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            debouncer.watch(path, mode)?;

            // Also watch the parent dir for non-recursive, so we catch file creation
            if !path.is_dir() {
                if let Some(parent) = path.parent() {
                    if parent.exists() {
                        let _ = debouncer.watch(parent, RecursiveMode::NonRecursive);
                    }
                }
            }
        } else {
            for path in &matched {
                let mode = if path.is_dir() {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };
                debouncer.watch(path, mode)?;
            }
        }
    }

    let handle = WatchHandle {
        _debouncer: debouncer,
    };

    Ok((rx, tx, handle))
}

/// Default ignore patterns.
pub fn default_ignore_patterns() -> Vec<String> {
    vec![
        ".git/**".to_string(),
        "node_modules/**".to_string(),
        "target/**".to_string(),
        "*.swp".to_string(),
        "*.swo".to_string(),
        "*~".to_string(),
    ]
}

fn is_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn compile_ignore_patterns(patterns: &[String]) -> Vec<glob::Pattern> {
    patterns
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect()
}

fn is_ignored(path: &Path, patterns: &[glob::Pattern]) -> bool {
    let path_str = path.to_string_lossy();
    patterns.iter().any(|p| p.matches(&path_str))
}
