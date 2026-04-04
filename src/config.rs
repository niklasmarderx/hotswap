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
#[allow(dead_code)]
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
}

/// Watch section of hotswap.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct WatchConfig {
    pub files: Vec<String>,
    pub ignore: Option<Vec<String>>,
    pub debounce_ms: Option<u64>,
}

/// Root hotswap.toml config.
#[derive(Debug, Clone, Deserialize)]
pub struct HotswapConfig {
    pub watch: WatchConfig,
    pub targets: Vec<TargetConfig>,
}

impl HotswapConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
