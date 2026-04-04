//! Integration tests for hotswap binary.
//!
//! These test the compiled binary end-to-end via `cargo run`.

use std::process::Command;

fn hotswap_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hotswap"))
}

#[test]
fn help_flag_exits_zero() {
    let output = hotswap_cmd().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hotswap"));
    assert!(stdout.contains("--watch"));
    assert!(stdout.contains("init"));
}

#[test]
fn version_flag_exits_zero() {
    let output = hotswap_cmd().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hotswap"));
}

#[test]
fn no_args_shows_error() {
    let output = hotswap_cmd().output().unwrap();
    // Should fail because no --watch or --config provided
    assert!(!output.status.success());
}

#[test]
fn init_generates_config() {
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("hotswap.toml");

    let output = hotswap_cmd()
        .arg("init")
        .arg("--output")
        .arg(output_path.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success(), "init failed: {:?}", output);
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("[watch]"));
    assert!(content.contains("strategy"));
}

#[test]
fn init_refuses_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("hotswap.toml");
    std::fs::write(&output_path, "existing").unwrap();

    let output = hotswap_cmd()
        .arg("init")
        .arg("--output")
        .arg(output_path.to_str().unwrap())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

#[test]
fn completions_bash() {
    let output = hotswap_cmd()
        .arg("--completions")
        .arg("bash")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hotswap"));
}

#[test]
fn completions_zsh() {
    let output = hotswap_cmd()
        .arg("--completions")
        .arg("zsh")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn completions_fish() {
    let output = hotswap_cmd()
        .arg("--completions")
        .arg("fish")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn diff_only_mode_exits_on_missing_file() {
    // Watch a nonexistent file in diff-only mode — should fail to start watcher
    let output = hotswap_cmd()
        .arg("--watch")
        .arg("/nonexistent/path/file.env")
        .arg("--diff-only")
        .output()
        .unwrap();

    // Will fail because watch path doesn't exist
    assert!(!output.status.success());
}

#[test]
fn dry_run_with_nonexistent_target() {
    let dir = tempfile::tempdir().unwrap();
    let env_path = dir.path().join(".env");
    std::fs::write(&env_path, "KEY=value\n").unwrap();

    // Start with dry-run, nonexistent signal target — should start but won't find process
    // We can't let it run forever, so just verify it starts without panic
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_hotswap"))
        .arg("--watch")
        .arg(env_path.to_str().unwrap())
        .arg("--signal")
        .arg("nonexistent_process_12345")
        .arg("--dry-run")
        .arg("--quiet")
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Give it a moment to start, then kill it
    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();

    // Should have started successfully (stderr contains "waiting" or process output)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("waiting") || stderr.contains("not found"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn json_mode_outputs_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let env_path = dir.path().join(".env");
    std::fs::write(&env_path, "KEY=value\n").unwrap();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_hotswap"))
        .arg("--watch")
        .arg(env_path.to_str().unwrap())
        .arg("--diff-only")
        .arg("--json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Each non-empty line should be valid JSON
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        assert!(
            serde_json::from_str::<serde_json::Value>(line).is_ok(),
            "invalid JSON line: {}",
            line
        );
    }
}

#[test]
fn init_help_flag() {
    let output = hotswap_cmd().arg("init").arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--output"));
}
