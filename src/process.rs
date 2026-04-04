use anyhow::{bail, Context, Result};
use std::path::Path;
use sysinfo::System;

/// Represents a discovered process.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

/// Find processes by name using sysinfo.
pub fn find_by_name(name: &str) -> Vec<ProcessInfo> {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    sys.processes()
        .values()
        .filter(|p| {
            let proc_name = p.name().to_string_lossy();
            proc_name == name || proc_name.starts_with(&format!("{name} "))
        })
        .map(|p| ProcessInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string_lossy().into_owned(),
        })
        .collect()
}

/// Read a PID from a PID file.
pub fn read_pid_file(path: &str) -> Result<ProcessInfo> {
    let path = Path::new(path);
    if !path.exists() {
        bail!("PID file not found: {}", path.display());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read PID file: {}", path.display()))?;

    let pid: u32 = content
        .trim()
        .parse()
        .with_context(|| format!("invalid PID in file: {}", path.display()))?;

    // Verify the process actually exists
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let sysinfo_pid = sysinfo::Pid::from_u32(pid);
    match sys.process(sysinfo_pid) {
        Some(p) => Ok(ProcessInfo {
            pid,
            name: p.name().to_string_lossy().into_owned(),
        }),
        None => bail!(
            "process with PID {} (from {}) is not running",
            pid,
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn read_valid_pid_file() {
        let dir = tempfile::tempdir().unwrap();
        let pid_path = dir.path().join("test.pid");
        let current_pid = std::process::id();
        std::fs::write(&pid_path, current_pid.to_string()).unwrap();

        let info = read_pid_file(pid_path.to_str().unwrap()).unwrap();
        assert_eq!(info.pid, current_pid);
    }

    #[test]
    fn read_missing_pid_file() {
        assert!(read_pid_file("/nonexistent/test.pid").is_err());
    }

    #[test]
    fn read_non_numeric_pid_file() {
        let dir = tempfile::tempdir().unwrap();
        let pid_path = dir.path().join("bad.pid");
        let mut f = std::fs::File::create(&pid_path).unwrap();
        f.write_all(b"not-a-number").unwrap();

        assert!(read_pid_file(pid_path.to_str().unwrap()).is_err());
    }

    #[test]
    fn read_pid_file_dead_process() {
        let dir = tempfile::tempdir().unwrap();
        let pid_path = dir.path().join("dead.pid");
        std::fs::write(&pid_path, "4294967").unwrap();
        let result = read_pid_file(pid_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not running"));
    }
}
