use anyhow::Result;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::config::SignalType;

impl SignalType {
    /// Convert to a nix Signal.
    fn to_nix_signal(&self) -> Signal {
        match self {
            Self::Sighup => Signal::SIGHUP,
            Self::Sigusr1 => Signal::SIGUSR1,
            Self::Sigusr2 => Signal::SIGUSR2,
            Self::Sigterm => Signal::SIGTERM,
        }
    }
}

/// Send a signal to a process by PID.
pub fn send_signal(pid: u32, signal_type: &SignalType) -> Result<()> {
    let nix_pid = Pid::from_raw(pid as i32);
    let nix_signal = signal_type.to_nix_signal();

    signal::kill(nix_pid, nix_signal).map_err(|e| {
        anyhow::anyhow!(
            "failed to send {} to pid {}: {}",
            signal_type.name(),
            pid,
            e
        )
    })?;

    Ok(())
}

/// Send a signal to a process group (negative PID).
pub fn send_signal_to_group(pgid: u32, signal_type: &SignalType) -> Result<()> {
    let nix_pid = Pid::from_raw(-(pgid as i32));
    let nix_signal = signal_type.to_nix_signal();

    signal::kill(nix_pid, nix_signal).map_err(|e| {
        anyhow::anyhow!(
            "failed to send {} to process group {}: {}",
            signal_type.name(),
            pgid,
            e
        )
    })?;

    Ok(())
}

/// Verify we can signal a process (send signal 0).
pub fn can_signal(pid: u32) -> bool {
    let nix_pid = Pid::from_raw(pid as i32);
    signal::kill(nix_pid, None).is_ok()
}
