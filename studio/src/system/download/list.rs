//! The installed-fonts list file, read by packaging/nix/fonts.nix.

use std::path::Path;

use anyhow::Result;
use serde_json::{Map, Value, json};

/// fonts-installed.json: `{"version": 1, "fonts": {name: {source, id, …}}}`,
/// read by packaging/nix/fonts.nix.
pub fn read_list(path: &Path) -> Map<String, Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("fonts").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

pub fn write_list(path: &Path, fonts: Map<String, Value>) -> Result<()> {
    crate::model::store::write_json_file(path, &json!({"version": 1, "fonts": fonts}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_round_trips() {
        let d = std::env::temp_dir().join(format!("theme-studio-dl-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join("fonts-installed.json");
        assert!(read_list(&path).is_empty());
        let mut m = Map::new();
        m.insert("Inter".into(), json!({"source": "google", "id": "inter"}));
        write_list(&path, m.clone()).unwrap();
        assert_eq!(read_list(&path), m);
        std::fs::remove_dir_all(d).unwrap();
    }
}
