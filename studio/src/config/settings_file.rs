//! The settings.json file: how it is read, topped up, and saved.
//!
//! The data types are in `settings.rs`.

use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use super::files::{read_json, write_json};
use super::settings::Settings;

/// Writes settings.json.
///
/// # Errors
/// Fails when `s` cannot be turned into JSON, or settings.json cannot be written.
pub fn save_settings(dir: &Path, s: &Settings) -> Result<()> {
    write_json(&dir.join("settings.json"), &serde_json::to_value(s)?)
}

/// Settings from the content of settings.json (`None`: no file). Broken
/// content gives a warning and the defaults.
fn parse_settings(found: Option<&Value>, warnings: &mut Vec<String>) -> Settings {
    match found.cloned().map(serde_json::from_value::<Settings>) {
        Some(Ok(mut s)) => {
            s.normalise();
            s
        }
        Some(Err(e)) => {
            warnings.push(format!("settings.json: {e} (using defaults)"));
            Settings::default()
        }
        None => Settings::default(),
    }
}

/// Reads settings.json (the defaults when it is missing or broken) and, when
/// `write` is set, creates or tops up the file.
pub(super) fn load_settings(
    settings_path: &Path,
    write: bool,
    warnings: &mut Vec<String>,
) -> Settings {
    match read_json(settings_path) {
        Ok(found) => {
            let s = parse_settings(found.as_ref(), warnings);
            let full = serde_json::to_value(&s).expect("settings serialise");
            // only top up a file that parsed (or create a missing one)
            let parsed = found
                .as_ref()
                .is_none_or(|f| serde_json::from_value::<Settings>(f.clone()).is_ok());
            if write
                && parsed
                && found.as_ref() != Some(&full)
                && let Err(e) = write_json(settings_path, &full)
            {
                warnings.push(format!("{e:#}"));
            }
            s
        }
        Err(e) => {
            warnings.push(format!("{e:#} (using defaults)"));
            Settings::default()
        }
    }
}
