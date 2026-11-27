//! Reads and writes the JSON files of the config folder.
//!
//! Both helpers keep the jq format of the files that the shell scripts also read.

use std::path::Path;

use anyhow::{Context as _, Result};
use serde_json::Value;

/// Writes a JSON file in jq's format (see `write_json_file`).
///
/// # Errors
/// Fails when the file or its folder cannot be written.
pub(super) fn write_json(path: &Path, v: &Value) -> Result<()> {
    crate::model::store::write_json_file(path, v)
}

/// Reads a JSON file. A file that does not exist gives `None`.
///
/// # Errors
/// Fails when the file exists but cannot be read, or is not valid JSON.
pub(super) fn read_json(path: &Path) -> Result<Option<Value>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?,
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}
