use serde::Deserialize;

/// Signal types that can be sent to processes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalType {
    Sighup,
    Sigusr1,
    Sigusr2,
    Sigterm,
}

impl SignalType {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "SIGHUP" | "HUP" => Some(Self::Sighup),
            "SIGUSR1" | "USR1" => Some(Self::Sigusr1),
            "SIGUSR2" | "USR2" => Some(Self::Sigusr2),
            "SIGTERM" | "TERM" => Some(Self::Sigterm),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Sighup => "SIGHUP",
            Self::Sigusr1 => "SIGUSR1",
            Self::Sigusr2 => "SIGUSR2",
            Self::Sigterm => "SIGTERM",
        }
    }
}

impl std::fmt::Display for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Reload strategy for multi-process coordination.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReloadStrategy {
    #[default]
    Sequential,
    Parallel,
}

impl ReloadStrategy {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sequential" | "seq" => Some(Self::Sequential),
            "parallel" | "par" => Some(Self::Parallel),
            _ => None,
        }
    }
}

/// A target process definition from hotswap.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct TargetConfig {
    pub name: String,
    pub signal: Option<String>,
    pub pid_file: Option<String>,
    pub process_name: Option<String>,
    pub validate: Option<String>,
    pub health_check: Option<String>,
    pub health_timeout_ms: Option<u64>,
    pub depends_on: Option<Vec<String>>,
    pub pre_signal: Option<String>,
    pub post_signal: Option<String>,
    pub enable_rollback: Option<bool>,
}

/// Watch section of hotswap.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct WatchConfig {
    pub files: Vec<String>,
    pub ignore: Option<Vec<String>>,
    pub debounce_ms: Option<u64>,
}

/// Global hooks configuration from hotswap.toml.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HooksConfig {
    pub pre_reload: Option<String>,
    pub post_reload: Option<String>,
    pub on_error: Option<String>,
}

/// Root hotswap.toml config.
#[derive(Debug, Clone, Deserialize)]
pub struct HotswapConfig {
    pub watch: WatchConfig,
    pub targets: Vec<TargetConfig>,
    pub strategy: Option<ReloadStrategy>,
    pub hooks: Option<HooksConfig>,
}

impl HotswapConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_type_from_str_full_names() {
        assert_eq!(
            SignalType::from_str_loose("SIGHUP"),
            Some(SignalType::Sighup)
        );
        assert_eq!(
            SignalType::from_str_loose("SIGUSR1"),
            Some(SignalType::Sigusr1)
        );
        assert_eq!(
            SignalType::from_str_loose("SIGUSR2"),
            Some(SignalType::Sigusr2)
        );
        assert_eq!(
            SignalType::from_str_loose("SIGTERM"),
            Some(SignalType::Sigterm)
        );
    }

    #[test]
    fn signal_type_from_str_short_names() {
        assert_eq!(SignalType::from_str_loose("HUP"), Some(SignalType::Sighup));
        assert_eq!(
            SignalType::from_str_loose("USR1"),
            Some(SignalType::Sigusr1)
        );
        assert_eq!(
            SignalType::from_str_loose("USR2"),
            Some(SignalType::Sigusr2)
        );
        assert_eq!(
            SignalType::from_str_loose("TERM"),
            Some(SignalType::Sigterm)
        );
    }

    #[test]
    fn signal_type_case_insensitive() {
        assert_eq!(
            SignalType::from_str_loose("sighup"),
            Some(SignalType::Sighup)
        );
        assert_eq!(
            SignalType::from_str_loose("Sigusr1"),
            Some(SignalType::Sigusr1)
        );
    }

    #[test]
    fn signal_type_invalid() {
        assert!(SignalType::from_str_loose("SIGKILL").is_none());
        assert!(SignalType::from_str_loose("invalid").is_none());
        assert!(SignalType::from_str_loose("").is_none());
    }

    #[test]
    fn signal_type_display() {
        assert_eq!(SignalType::Sighup.to_string(), "SIGHUP");
        assert_eq!(SignalType::Sigterm.name(), "SIGTERM");
    }

    #[test]
    fn reload_strategy_from_str() {
        assert!(matches!(
            ReloadStrategy::from_str_loose("sequential"),
            Some(ReloadStrategy::Sequential)
        ));
        assert!(matches!(
            ReloadStrategy::from_str_loose("seq"),
            Some(ReloadStrategy::Sequential)
        ));
        assert!(matches!(
            ReloadStrategy::from_str_loose("parallel"),
            Some(ReloadStrategy::Parallel)
        ));
        assert!(matches!(
            ReloadStrategy::from_str_loose("par"),
            Some(ReloadStrategy::Parallel)
        ));
        assert!(ReloadStrategy::from_str_loose("invalid").is_none());
    }

    #[test]
    fn parse_hotswap_config() {
        let cfg = HotswapConfig::from_file("tests/fixtures/hotswap.toml").unwrap();
        assert_eq!(cfg.watch.files, vec![".env", "config.toml"]);
        assert_eq!(cfg.watch.debounce_ms, Some(300));
        assert_eq!(cfg.targets.len(), 2);

        let backend = &cfg.targets[0];
        assert_eq!(backend.name, "backend");
        assert_eq!(backend.signal.as_deref(), Some("SIGHUP"));
        assert_eq!(backend.pid_file.as_deref(), Some("/tmp/backend.pid"));
        assert_eq!(backend.validate.as_deref(), Some("toml"));
        assert!(backend.health_check.is_some());
        assert_eq!(backend.health_timeout_ms, Some(5000));

        // Per-target hooks and rollback
        assert_eq!(backend.pre_signal.as_deref(), Some("echo draining"));
        assert_eq!(backend.post_signal.as_deref(), Some("echo warmed"));
        assert_eq!(backend.enable_rollback, Some(true));

        let frontend = &cfg.targets[1];
        assert_eq!(frontend.name, "frontend");
        assert_eq!(
            frontend.depends_on.as_deref(),
            Some(&["backend".to_string()][..])
        );
        assert_eq!(frontend.pre_signal, None);
        assert_eq!(frontend.post_signal, None);
        assert_eq!(frontend.enable_rollback, None);

        // Global hooks
        let hooks = cfg.hooks.as_ref().expect("hooks section should exist");
        assert_eq!(hooks.pre_reload.as_deref(), Some("echo pre"));
        assert_eq!(hooks.post_reload.as_deref(), Some("echo post"));
        assert_eq!(hooks.on_error.as_deref(), Some("echo error"));
    }

    #[test]
    fn parse_config_with_strategy() {
        let cfg = HotswapConfig::from_file("tests/fixtures/hotswap.toml").unwrap();
        assert!(matches!(cfg.strategy, Some(ReloadStrategy::Sequential)));
    }

    #[test]
    fn parse_missing_config_returns_error() {
        assert!(HotswapConfig::from_file("/nonexistent/hotswap.toml").is_err());
    }
}
