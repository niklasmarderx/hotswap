use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// The kind of validator to run.
#[derive(Debug, Clone)]
pub enum Validator {
    /// Validate TOML syntax.
    Toml,
    /// Validate JSON syntax.
    Json,
    /// Validate YAML syntax.
    Yaml,
    /// Run a custom shell command. Exit code 0 = valid.
    Command(String),
}

impl Validator {
    /// Parse a validator specifier.
    /// Built-in: "toml", "json", "yaml"
    /// Anything else is treated as a custom command.
    pub fn parse(spec: &str) -> Self {
        match spec.to_lowercase().as_str() {
            "toml" => Self::Toml,
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            _ => Self::Command(spec.to_string()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Toml => "TOML",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Command(cmd) => cmd,
        }
    }

    /// Validate a file. Returns Ok(()) if valid, Err with description if not.
    pub fn validate(&self, path: &str) -> Result<()> {
        match self {
            Self::Toml => validate_toml(path),
            Self::Json => validate_json(path),
            Self::Yaml => validate_yaml(path),
            Self::Command(cmd) => validate_command(cmd, path),
        }
    }
}

/// Auto-detect the appropriate validator based on file extension.
pub fn detect_validator(path: &str) -> Option<Validator> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("toml") => Some(Validator::Toml),
        Some("json") => Some(Validator::Json),
        Some("yaml" | "yml") => Some(Validator::Yaml),
        _ => None,
    }
}

fn validate_toml(path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    content
        .parse::<toml::Table>()
        .map_err(|e| anyhow::anyhow!("TOML syntax error: {e}"))?;
    Ok(())
}

fn validate_json(path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|e| anyhow::anyhow!("JSON syntax error: {e}"))?;
    Ok(())
}

fn validate_yaml(path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    serde_yaml::from_str::<serde_yaml::Value>(&content)
        .map_err(|e| anyhow::anyhow!("YAML syntax error: {e}"))?;
    Ok(())
}

fn validate_command(cmd: &str, path: &str) -> Result<()> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("HOTSWAP_FILE", path)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run validator command: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.is_empty() {
            stderr.trim().to_string()
        } else if !stdout.is_empty() {
            stdout.trim().to_string()
        } else {
            format!("exited with code {}", output.status.code().unwrap_or(-1))
        };
        anyhow::bail!("validator `{cmd}` failed: {detail}");
    }
}
