//! The keymaps.json file: its defaults, its presets, and how it is loaded.
//!
//! A preset that the file lacks comes back from the shipped ones. An older file
//! is upgraded first (see `migrate.rs`).

use std::path::Path;

use serde_json::{Map, Value};

use super::files::{read_json, write_json};
use super::migrate::migrate;
use crate::keymap::map::Keymap;

/// keymaps.json.s format version. 2: panes are `<leader>1`…`<leader>7`.
/// 3: `use-for-mono` / `-ui` / `-bar` are `use-for-monospace` /
/// `-proportional` / `-icons`, after the font roles.
pub const KEYMAPS_VERSION: u64 = 3;

/// keymaps.json as the defaults would write it.
pub(super) fn default_keymaps() -> Value {
    let mut presets = Map::new();
    for km in Keymap::builtin() {
        presets.insert(km.name.clone(), km.to_json());
    }
    let mut m = Map::new();
    m.insert("version".into(), Value::from(KEYMAPS_VERSION));
    m.insert("active".into(), Value::String("Vim".into()));
    m.insert("presets".into(), Value::Object(presets));
    Value::Object(m)
}

/// Reads keymaps.json: every preset (shipped ones topped up with actions the
/// file doesn't mention), and the active one.
pub(super) fn load_keymaps(v: &Value, warnings: &mut Vec<String>) -> (Keymap, Vec<Keymap>, Value) {
    let v = &migrate(v.clone());
    let presets = read_presets(v, warnings);
    let active_name = v
        .get("active")
        .and_then(Value::as_str)
        .unwrap_or("Vim")
        .to_string();
    let active = if let Some(p) = presets.iter().find(|p| p.name == active_name) {
        p.clone()
    } else {
        warnings.push(format!(
            "keymaps.json: no preset \"{active_name}\"; using Vim"
        ));
        presets
            .iter()
            .find(|p| p.name == "Vim")
            .cloned()
            .unwrap_or_else(Keymap::vim)
    };
    // what the file should contain now (adds anything missing)
    let mut out = Map::new();
    out.insert("version".into(), Value::from(KEYMAPS_VERSION));
    out.insert("active".into(), Value::String(active.name.clone()));
    let mut obj = Map::new();
    for p in &presets {
        obj.insert(p.name.clone(), p.to_json());
    }
    out.insert("presets".into(), Value::Object(obj));
    (active, presets, Value::Object(out))
}

/// The presets of a keymaps.json: those in the file (topped up with missing
/// actions) and then the shipped ones that the file lacks.
fn read_presets(v: &Value, warnings: &mut Vec<String>) -> Vec<Keymap> {
    let builtin = Keymap::builtin();
    let mut presets: Vec<Keymap> = Vec::new();
    if let Some(obj) = v.get("presets").and_then(Value::as_object) {
        for (name, p) in obj {
            let base = builtin.iter().find(|b| b.name == *name);
            let (km, w) = Keymap::from_json(name, p, base);
            warnings.extend(w);
            presets.push(km);
        }
    }
    // shipped presets missing from the file come back
    for b in builtin {
        if !presets.iter().any(|p| p.name == b.name) {
            presets.push(b);
        }
    }
    presets
}

/// Reads keymaps.json (the built-in presets when it is broken) and, when
/// `write` is set, creates or tops up the file.
pub(super) fn load_keymaps_file(
    keymaps_path: &Path,
    write: bool,
    warnings: &mut Vec<String>,
) -> (Keymap, Vec<Keymap>) {
    match read_json(keymaps_path) {
        Ok(found) => {
            let v = found.clone().unwrap_or_else(default_keymaps);
            let (active, presets, full) = load_keymaps(&v, warnings);
            if write
                && found.as_ref() != Some(&full)
                && let Err(e) = write_json(keymaps_path, &full)
            {
                warnings.push(format!("{e:#}"));
            }
            (active, presets)
        }
        Err(e) => {
            warnings.push(format!("{e:#} (using the built-in Vim keys)"));
            (Keymap::vim(), Keymap::builtin())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Long function (reason verified, phase 6): one scenario; one old file shows which bindings move and which stay.
    #[test]
    fn version_1_panes_move_to_the_leader_but_edits_stay() {
        let old = serde_json::json!({"active": "Vim", "presets": {"Vim": {"leader": "<Space>", "bindings": {"global": {
            "panel-1": ["1"], "panel-2": ["9"], "panel-4": ["4", "x"]
        }}}}});
        let mut w = Vec::new();
        let (km, _, full) = load_keymaps(&old, &mut w);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(full["version"], KEYMAPS_VERSION);
        let g = &full["presets"]["Vim"]["bindings"]["global"];
        assert_eq!(
            g["panel-1"],
            serde_json::json!(["<leader>1"]),
            "old default moves"
        );
        assert_eq!(
            g["panel-2"],
            serde_json::json!(["9"]),
            "your own binding stays"
        );
        assert_eq!(
            g["panel-4"],
            serde_json::json!(["4", "x"]),
            "changed lists stay"
        );
        assert_eq!(
            g["panel-7"],
            serde_json::json!(["<leader>7"]),
            "missing ones come from the preset"
        );
        // version 2 files aren't touched again
        let (_, _, again) = load_keymaps(&full, &mut w);
        assert_eq!(again, full);
        let _ = km;
    }

    #[test]
    fn version_2_use_for_keys_follow_the_role_names() {
        let old = serde_json::json!({"version": 2, "active": "Vim", "presets": {"Vim": {"leader": "<Space>", "bindings": {"downloads": {
            "use-for-mono": ["7"], "use-for-ui": ["2"], "use-for-bar": ["3"]
        }}}}});
        let mut w = Vec::new();
        let (km, _, full) = load_keymaps(&old, &mut w);
        assert!(w.is_empty(), "{w:?}");
        let d = &full["presets"]["Vim"]["bindings"]["downloads"];
        assert_eq!(
            d["use-for-monospace"],
            serde_json::json!(["7"]),
            "rebinds kept"
        );
        assert_eq!(d["use-for-icons"], serde_json::json!(["3"]));
        assert!(d.get("use-for-mono").is_none());
        assert_eq!(
            km.key_for(
                &[crate::keymap::context::Context::Downloads],
                crate::keymap::action::Action::UseFor(0)
            )
            .as_deref(),
            Some("7")
        );
    }
}
