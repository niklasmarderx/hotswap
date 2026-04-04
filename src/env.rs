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

    #[allow(dead_code)] // Used by output/logging
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

/// Check if a path is an .env file (starts or ends with ".env").
pub fn is_env_file(path: &str) -> bool {
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    (filename == ".env" || filename.starts_with(".env.")) || filename.ends_with(".env")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_env(dir: &std::path::Path, name: &str, content: &str) -> String {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn parse_valid_env_file() {
        let path = "tests/fixtures/valid.env";
        let map = parse_env_file(path).unwrap();
        assert_eq!(
            map.get("DATABASE_URL").unwrap(),
            "postgres://localhost:5432/myapp"
        );
        assert_eq!(map.get("PORT").unwrap(), "3000");
        assert_eq!(map.get("DEBUG").unwrap(), "true");
        assert_eq!(map.len(), 5);
    }

    #[test]
    fn parse_missing_file_returns_error() {
        let result = parse_env_file("/nonexistent/.env");
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_env_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_env(dir.path(), ".env", "");
        let map = parse_env_file(&path).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn diff_added_keys() {
        let old = BTreeMap::new();
        let mut new = BTreeMap::new();
        new.insert("KEY".into(), "value".into());

        let diff = diff_env(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert!(diff.changed.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn diff_changed_values() {
        let mut old = BTreeMap::new();
        old.insert("KEY".into(), "old".into());
        let mut new = BTreeMap::new();
        new.insert("KEY".into(), "new".into());

        let diff = diff_env(&old, &new);
        assert!(diff.added.is_empty());
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed["KEY"], ("old".into(), "new".into()));
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn diff_removed_keys() {
        let mut old = BTreeMap::new();
        old.insert("KEY".into(), "value".into());
        let new = BTreeMap::new();

        let diff = diff_env(&old, &new);
        assert!(diff.added.is_empty());
        assert!(diff.changed.is_empty());
        assert_eq!(diff.removed.len(), 1);
    }

    #[test]
    fn diff_no_changes() {
        let mut map = BTreeMap::new();
        map.insert("KEY".into(), "value".into());

        let diff = diff_env(&map, &map);
        assert!(diff.is_empty());
        assert_eq!(diff.total_changes(), 0);
    }

    #[test]
    fn diff_mixed_changes() {
        let mut old = BTreeMap::new();
        old.insert("KEEP".into(), "same".into());
        old.insert("CHANGE".into(), "old".into());
        old.insert("REMOVE".into(), "gone".into());

        let mut new = BTreeMap::new();
        new.insert("KEEP".into(), "same".into());
        new.insert("CHANGE".into(), "new".into());
        new.insert("ADD".into(), "fresh".into());

        let diff = diff_env(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.total_changes(), 3);
    }

    #[test]
    fn snapshot_missing_file_returns_empty() {
        let map = snapshot_env("/nonexistent/.env");
        assert!(map.is_empty());
    }

    #[test]
    fn is_env_file_detection() {
        assert!(is_env_file(".env"));
        assert!(is_env_file(".env.local"));
        assert!(is_env_file(".env.production"));
        assert!(is_env_file("app.env"));
        assert!(is_env_file("/path/to/.env"));
        assert!(!is_env_file("config.toml"));
        assert!(!is_env_file("envfile"));
        assert!(!is_env_file("README.md"));
        assert!(!is_env_file(".envelope"));
        assert!(!is_env_file(".env2backup"));
        assert!(!is_env_file(".environment"));
    }
}
