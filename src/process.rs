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
    let name = sys
        .process(sysinfo_pid)
        .map(|p| p.name().to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("pid:{pid}"));

    Ok(ProcessInfo { pid, name })
}

/// Check if a process with the given PID is alive.
pub fn is_alive(pid: u32) -> bool {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.process(sysinfo::Pid::from_u32(pid)).is_some()
}
