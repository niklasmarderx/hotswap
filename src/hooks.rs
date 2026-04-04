use anyhow::Result;
use std::time::Duration;

use crate::output::Output;

/// Default timeout for hook commands.
const DEFAULT_HOOK_TIMEOUT: Duration = Duration::from_secs(30);

/// Run a hook command (shell command via `sh -c`) with a timeout.
/// Returns Ok(()) on success, Err on failure or timeout.
pub fn run_hook(name: &str, command: &str, timeout: Option<Duration>) -> Result<()> {
    let timeout = timeout.unwrap_or(DEFAULT_HOOK_TIMEOUT);

    Output::hook_start(name, command);

    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to run hook '{}': {}", name, e))?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    Output::hook_ok(name);
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
                    let msg = format!(
                        "hook '{}' failed (exit {}): {}",
                        name,
                        status,
                        stderr.trim()
                    );
                    Output::hook_failed(name, &msg);
                    anyhow::bail!("{}", msg);
                }
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let msg = format!("hook '{}' timed out after {}s", name, timeout.as_secs());
                    Output::hook_failed(name, &msg);
                    anyhow::bail!("{}", msg);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                anyhow::bail!("hook '{}' error: {}", name, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_hook_success() {
        let result = run_hook("test", "true", Some(Duration::from_secs(5)));
        assert!(result.is_ok());
    }

    #[test]
    fn run_hook_failure() {
        let result = run_hook("test", "false", Some(Duration::from_secs(5)));
        assert!(result.is_err());
    }

    #[test]
    fn run_hook_timeout() {
        let result = run_hook("test", "sleep 60", Some(Duration::from_millis(200)));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[test]
    fn run_hook_with_output() {
        let result = run_hook("test", "echo hello", Some(Duration::from_secs(5)));
        assert!(result.is_ok());
    }
}
