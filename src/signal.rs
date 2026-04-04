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
    let raw_pid =
        i32::try_from(pid).map_err(|_| anyhow::anyhow!("PID {} exceeds maximum value", pid))?;
    let nix_pid = Pid::from_raw(raw_pid);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_signal_to_nonexistent_process() {
        let result = send_signal(4_294_967, &SignalType::Sighup);
        assert!(result.is_err());
    }
}
