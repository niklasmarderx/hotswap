use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// Parsed env file contents as ordered key-value pairs.
pub type EnvMap = BTreeMap<String, String>;

/// Represents the diff between two env files.
#[derive(Debug, Default)]
pub struct EnvDiff {
    /// New variables that were added.
    pub added: BTreeMap<String, String>,
    /// Variables whose values changed: key -> (old, new).
    pub changed: BTreeMap<String, (String, String)>,
    /// Variables that were removed.
    pub removed: BTreeMap<String, String>,
}

impl EnvDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }

    pub fn total_changes(&self) -> usize {
        self.added.len() + self.changed.len() + self.removed.len()
    }
}

/// Parse a .env file into key-value pairs.
pub fn parse_env_file(path: &str) -> Result<EnvMap> {
    let path = Path::new(path);
    if !path.exists() {
        bail!(".env file not found: {}", path.display());
    }

    let mut map = BTreeMap::new();

    let iter = dotenvy::from_path_iter(path)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", path.display(), e))?;

    for item in iter {
        let (key, value) =
            item.map_err(|e| anyhow::anyhow!("parse error in {}: {}", path.display(), e))?;
        map.insert(key, value);
    }

    Ok(map)
}

/// Compute the diff between two env maps.
pub fn diff_env(old: &EnvMap, new: &EnvMap) -> EnvDiff {
    let mut diff = EnvDiff::default();

    // Find added and changed
    for (key, new_value) in new {
        match old.get(key) {
            Some(old_value) if old_value != new_value => {
                diff.changed
                    .insert(key.clone(), (old_value.clone(), new_value.clone()));
            }
            None => {
                diff.added.insert(key.clone(), new_value.clone());
            }
            _ => {} // unchanged
        }
    }

    // Find removed
    for (key, old_value) in old {
        if !new.contains_key(key) {
            diff.removed.insert(key.clone(), old_value.clone());
        }
    }

    diff
}

/// Read the current snapshot of a .env file. Returns empty map if file doesn't exist.
pub fn snapshot_env(path: &str) -> EnvMap {
    parse_env_file(path).unwrap_or_default()
}
