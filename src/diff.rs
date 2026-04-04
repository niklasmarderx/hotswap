use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

/// Diff result for structured config files (TOML/JSON/YAML).
/// Uses dot-notation keys for nested values (e.g. "server.port").
#[derive(Debug, Default)]
pub struct ConfigDiff {
    pub added: BTreeMap<String, String>,
    pub changed: BTreeMap<String, (String, String)>,
    pub removed: BTreeMap<String, String>,
}

impl ConfigDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}

/// Parse a structured config file into a flat key-value map using dot-notation.
pub fn parse_config(path: &str) -> Result<BTreeMap<String, String>> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    let content = std::fs::read_to_string(path)?;

    let value: serde_json::Value = match ext.as_deref() {
        Some("toml") => {
            let table: toml::Table = content
                .parse()
                .map_err(|e| anyhow::anyhow!("TOML parse error: {e}"))?;
            serde_json::to_value(table)?
        }
        Some("json") => serde_json::from_str(&content)?,
        Some("yaml" | "yml") => {
            let yaml: serde_yml::Value = serde_yml::from_str(&content)?;
            serde_json::to_value(yaml)?
        }
        _ => return Ok(BTreeMap::new()),
    };

    Ok(flatten_value("", &value))
}

/// Snapshot a config file, returning empty map if file is missing or unparseable.
pub fn snapshot_config(path: &str) -> BTreeMap<String, String> {
    parse_config(path).unwrap_or_default()
}

/// Compute diff between two flat config snapshots.
pub fn diff_config(old: &BTreeMap<String, String>, new: &BTreeMap<String, String>) -> ConfigDiff {
    let mut diff = ConfigDiff::default();

    for (key, new_value) in new {
        match old.get(key) {
            Some(old_value) if old_value != new_value => {
                diff.changed
                    .insert(key.clone(), (old_value.clone(), new_value.clone()));
            }
            None => {
                diff.added.insert(key.clone(), new_value.clone());
            }
            _ => {}
        }
    }

    for (key, old_value) in old {
        if !new.contains_key(key) {
            diff.removed.insert(key.clone(), old_value.clone());
        }
    }

    diff
}

/// Check if a file is a supported structured config format.
pub fn is_config_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(ext.as_deref(), Some("toml" | "json" | "yaml" | "yml"))
}

/// Flatten a serde_json::Value into dot-notation key-value pairs.
fn flatten_value(prefix: &str, value: &serde_json::Value) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    match value {
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                map.extend(flatten_value(&full_key, val));
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let full_key = format!("{prefix}[{i}]");
                map.extend(flatten_value(&full_key, val));
            }
        }
        serde_json::Value::String(s) => {
            map.insert(prefix.to_string(), s.clone());
        }
        serde_json::Value::Number(n) => {
            map.insert(prefix.to_string(), n.to_string());
        }
        serde_json::Value::Bool(b) => {
            map.insert(prefix.to_string(), b.to_string());
        }
        serde_json::Value::Null => {
            map.insert(prefix.to_string(), "null".to_string());
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_simple_object() {
        let val = serde_json::json!({
            "host": "localhost",
            "port": 3000
        });
        let flat = flatten_value("", &val);
        assert_eq!(flat.get("host").unwrap(), "localhost");
        assert_eq!(flat.get("port").unwrap(), "3000");
    }

    #[test]
    fn flatten_nested_object() {
        let val = serde_json::json!({
            "server": {
                "host": "localhost",
                "port": 3000
            }
        });
        let flat = flatten_value("", &val);
        assert_eq!(flat.get("server.host").unwrap(), "localhost");
        assert_eq!(flat.get("server.port").unwrap(), "3000");
    }

    #[test]
    fn flatten_array() {
        let val = serde_json::json!({
            "tags": ["a", "b"]
        });
        let flat = flatten_value("", &val);
        assert_eq!(flat.get("tags[0]").unwrap(), "a");
        assert_eq!(flat.get("tags[1]").unwrap(), "b");
    }

    #[test]
    fn diff_added_keys() {
        let old = BTreeMap::new();
        let mut new = BTreeMap::new();
        new.insert("server.port".into(), "3000".into());

        let diff = diff_config(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert!(diff.changed.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn diff_changed_keys() {
        let mut old = BTreeMap::new();
        old.insert("server.port".into(), "3000".into());
        let mut new = BTreeMap::new();
        new.insert("server.port".into(), "8080".into());

        let diff = diff_config(&old, &new);
        assert!(diff.added.is_empty());
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed["server.port"], ("3000".into(), "8080".into()));
    }

    #[test]
    fn diff_removed_keys() {
        let mut old = BTreeMap::new();
        old.insert("server.port".into(), "3000".into());
        let new = BTreeMap::new();

        let diff = diff_config(&old, &new);
        assert!(diff.added.is_empty());
        assert!(diff.changed.is_empty());
        assert_eq!(diff.removed.len(), 1);
    }

    #[test]
    fn diff_no_changes() {
        let mut map = BTreeMap::new();
        map.insert("key".into(), "val".into());
        let diff = diff_config(&map, &map);
        assert!(diff.is_empty());
    }

    #[test]
    fn parse_toml_config() {
        let map = parse_config("tests/fixtures/valid.toml").unwrap();
        assert!(map.contains_key("server.host"));
        assert!(map.contains_key("server.port"));
    }

    #[test]
    fn parse_json_config() {
        let map = parse_config("tests/fixtures/valid.json").unwrap();
        assert!(!map.is_empty());
    }

    #[test]
    fn parse_yaml_config() {
        let map = parse_config("tests/fixtures/valid.yaml").unwrap();
        assert!(!map.is_empty());
    }

    #[test]
    fn is_config_file_detection() {
        assert!(is_config_file("config.toml"));
        assert!(is_config_file("data.json"));
        assert!(is_config_file("config.yaml"));
        assert!(is_config_file("config.yml"));
        assert!(!is_config_file(".env"));
        assert!(!is_config_file("README.md"));
    }
}
