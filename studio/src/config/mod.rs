//! The studio's own settings: `$XDG_CONFIG_HOME/theme-studio/` (default
//! `~/.config/theme-studio/`) with keymaps.json and settings.json. Missing
//! files are created and missing entries filled in from the defaults, so
//! things added in newer versions show up while your changes are kept.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde_json::{Map, Value};

use crate::keymap::Keymap;

pub mod settings;
use settings::Settings;

/// `$XDG_CONFIG_HOME/theme-studio`, or `~/.config/theme-studio`.
pub fn dir() -> PathBuf {
    crate::core::paths::studio_config_dir()
}

/// What loading the config produced, and anything worth telling the user.
pub struct Loaded {
    pub keymap: Keymap,
    pub presets: Vec<Keymap>,
    pub settings: Settings,
    pub warnings: Vec<String>,
    /// The folder the files are in (for saving).
    pub dir: PathBuf,
}

fn write_json(path: &Path, v: &Value) -> Result<()> {
    crate::core::store::write_json_file(path, v)
}

fn read_json(path: &Path) -> Result<Option<Value>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?,
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// keymaps.json as the defaults would write it.
fn default_keymaps() -> Value {
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

/// keymaps.json.s format version. 2: panes are `<leader>1`…`<leader>7`.
/// 3: `use-for-mono` / `-ui` / `-bar` are `use-for-monospace` /
/// `-proportional` / `-icons`, after the font roles.
pub const KEYMAPS_VERSION: u64 = 3;

/// Updates an older keymaps.json. Version 1 (no "version") had panes on the
/// plain digits; bindings still exactly like that move to the leader, so
/// your own changes are left alone.
fn migrate(mut v: Value) -> Value {
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(1);
    if version < 2
        && let Some(presets) = v.get_mut("presets").and_then(Value::as_object_mut)
    {
        for preset in presets.values_mut() {
            let Some(global) = preset
                .pointer_mut("/bindings/global")
                .and_then(Value::as_object_mut)
            else {
                continue;
            };
            for n in 1..=9 {
                let id = format!("panel-{n}");
                if global.get(&id) == Some(&serde_json::json!([n.to_string()])) {
                    global.insert(id, serde_json::json!([format!("<leader>{n}")]));
                }
            }
        }
    }
    if version < 3
        && let Some(presets) = v.get_mut("presets").and_then(Value::as_object_mut)
    {
        for preset in presets.values_mut() {
            let Some(contexts) = preset.get_mut("bindings").and_then(Value::as_object_mut) else {
                continue;
            };
            for ctx in contexts.values_mut().filter_map(Value::as_object_mut) {
                let old = std::mem::take(ctx);
                *ctx = old
                    .into_iter()
                    .map(|(k, keys)| {
                        let role = k.strip_prefix("use-for-").and_then(|r| {
                            crate::fonts::presets::OLD_ROLES
                                .iter()
                                .position(|o| *o == r)
                        });
                        match role {
                            Some(r) => (format!("use-for-{}", crate::fonts::types::ROLES[r]), keys),
                            None => (k, keys),
                        }
                    })
                    .collect();
            }
        }
    }
    v
}

/// Reads keymaps.json: every preset (shipped ones topped up with actions the
/// file doesn't mention), and the active one.
fn load_keymaps(v: &Value, warnings: &mut Vec<String>) -> (Keymap, Vec<Keymap>, Value) {
    let v = &migrate(v.clone());
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
    let active_name = v
        .get("active")
        .and_then(Value::as_str)
        .unwrap_or("Vim")
        .to_string();
    let active = match presets.iter().find(|p| p.name == active_name) {
        Some(p) => p.clone(),
        None => {
            warnings.push(format!(
                "keymaps.json: no preset \"{active_name}\"; using Vim"
            ));
            presets
                .iter()
                .find(|p| p.name == "Vim")
                .cloned()
                .unwrap_or_else(Keymap::vim)
        }
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

/// Writes settings.json.
pub fn save_settings(dir: &Path, s: &Settings) -> Result<()> {
    write_json(&dir.join("settings.json"), &serde_json::to_value(s)?)
}

/// Loads both files from `dir`, creating or topping them up when `write` is
/// set. A file that doesn't parse is left alone and the defaults are used.
pub fn load(dir: &Path, write: bool) -> Loaded {
    let mut warnings = Vec::new();
    let keymaps_path = dir.join("keymaps.json");
    let settings_path = dir.join("settings.json");

    let (keymap, presets) = match read_json(&keymaps_path) {
        Ok(found) => {
            let v = found.clone().unwrap_or_else(default_keymaps);
            let (active, presets, full) = load_keymaps(&v, &mut warnings);
            if write
                && found.as_ref() != Some(&full)
                && let Err(e) = write_json(&keymaps_path, &full)
            {
                warnings.push(format!("{e:#}"));
            }
            (active, presets)
        }
        Err(e) => {
            warnings.push(format!("{e:#} (using the built-in Vim keys)"));
            (Keymap::vim(), Keymap::builtin())
        }
    };

    let settings = match read_json(&settings_path) {
        Ok(found) => {
            let s: Settings = match found.clone().map(serde_json::from_value::<Settings>) {
                Some(Ok(mut s)) => {
                    s.normalise();
                    s
                }
                Some(Err(e)) => {
                    warnings.push(format!("settings.json: {e} (using defaults)"));
                    Settings::default()
                }
                None => Settings::default(),
            };
            let full = serde_json::to_value(&s).expect("settings serialise");
            // only top up a file that parsed (or create a missing one)
            let parsed = found
                .as_ref()
                .is_none_or(|f| serde_json::from_value::<Settings>(f.clone()).is_ok());
            if write
                && parsed
                && found.as_ref() != Some(&full)
                && let Err(e) = write_json(&settings_path, &full)
            {
                warnings.push(format!("{e:#}"));
            }
            s
        }
        Err(e) => {
            warnings.push(format!("{e:#} (using defaults)"));
            Settings::default()
        }
    };

    Loaded {
        dir: dir.to_path_buf(),
        keymap,
        presets,
        settings,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::store::to_jq_string;
    use crate::fonts::filters::Spacing;
    use crate::keymap::action::Action;
    use crate::keymap::key::Key;
    use crate::keymap::{Context, Lookup};

    fn temp(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("theme-studio-config-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn creates_files_with_defaults_in_jq_format() {
        let d = temp("create");
        let l = load(&d, true);
        assert!(l.warnings.is_empty(), "{:?}", l.warnings);
        assert_eq!(l.keymap.name, "Vim");
        for f in ["keymaps.json", "settings.json"] {
            let text = std::fs::read_to_string(d.join(f)).unwrap();
            let v: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(text, to_jq_string(&v), "{f} is jq-formatted");
        }
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(d.join("keymaps.json")).unwrap())
                .unwrap();
        assert_eq!(v["presets"]["Vim"]["leader"], "<Space>");
        assert_eq!(
            v["presets"]["Vim"]["bindings"]["global"]["save-palette"][0],
            "<leader>sp"
        );
        // loading again changes nothing
        let before = std::fs::read_to_string(d.join("keymaps.json")).unwrap();
        load(&d, true);
        assert_eq!(
            std::fs::read_to_string(d.join("keymaps.json")).unwrap(),
            before
        );
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn edits_are_kept_and_missing_actions_filled_in() {
        let d = temp("edit");
        std::fs::create_dir_all(&d).unwrap();
        // rebinds quit, unbinds undo, sets the leader; mentions nothing else
        std::fs::write(
            d.join("keymaps.json"),
            r#"{"active": "Vim", "presets": {"Vim": {"leader": "<lt>", "bindings": {
                "global": {"quit": ["Q"], "nope": ["x"]},
                "colours": {"undo": []},
                "colour-list": {"down": ["<bogus>"]}
            }}}}"#,
        )
        .unwrap();
        let l = load(&d, true);
        assert_eq!(l.warnings.len(), 2, "{:?}", l.warnings); // unknown action, bad key
        let km = &l.keymap;
        let ctx = [Context::ColourList, Context::Colours, Context::Global];
        let seq = |s: &str| Key::parse_seq(s, km.leader).unwrap();
        assert_eq!(km.lookup(&ctx, &seq("Q")), Lookup::Run(Action::Quit));
        assert_eq!(km.lookup(&ctx, &seq("q")), Lookup::None);
        assert_eq!(
            km.lookup(&ctx, &seq("u")),
            Lookup::None,
            "unbound stays unbound"
        );
        // not mentioned: shipped keys, with the new leader
        assert_eq!(
            km.lookup(&ctx, &seq("<lt>sp")),
            Lookup::Run(Action::SavePalette)
        );
        assert_eq!(km.lookup(&ctx, &seq("y")), Lookup::Run(Action::CopyHex));
        // the file now lists everything, with the edits kept
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(d.join("keymaps.json")).unwrap())
                .unwrap();
        assert_eq!(
            v["presets"]["Vim"]["bindings"]["global"]["quit"],
            serde_json::json!(["Q"])
        );
        assert_eq!(
            v["presets"]["Vim"]["bindings"]["colours"]["undo"],
            serde_json::json!([])
        );
        assert_eq!(
            v["presets"]["Vim"]["bindings"]["colours"]["copy-hex"],
            serde_json::json!(["y"])
        );
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn broken_files_are_left_alone() {
        let d = temp("broken");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("keymaps.json"), "{ not json").unwrap();
        std::fs::write(
            d.join("settings.json"),
            r#"{"font-list": {"hide-bitmap": "yes"}}"#,
        )
        .unwrap();
        let l = load(&d, true);
        assert_eq!(l.keymap.name, "Vim");
        assert_eq!(l.warnings.len(), 2, "{:?}", l.warnings);
        assert_eq!(
            std::fs::read_to_string(d.join("keymaps.json")).unwrap(),
            "{ not json"
        );
        assert!(
            std::fs::read_to_string(d.join("settings.json"))
                .unwrap()
                .contains("\"yes\"")
        );
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn settings_fill_in_and_map_to_filters() {
        let d = temp("settings");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("settings.json"),
            r#"{"font-list": {"hide-bitmap": false, "spacing": "mono"}}"#,
        )
        .unwrap();
        let l = load(&d, true);
        let f = l.settings.font_list.filters();
        assert!(f.show_bitmap && !f.show_special);
        assert_eq!(f.spacing, Spacing::Mono);
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(d.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(
            v["font-list"]["hide-cjk-emoji-math"], true,
            "missing entries filled in"
        );
        assert_eq!(v["font-list"]["hide-bitmap"], false, "edits kept");
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn xdg_config_home_is_respected() {
        // only checks the path rule; doesn't change the environment
        let d = dir();
        assert!(d.ends_with("theme-studio"));
    }
}

#[cfg(test)]
mod migrate_tests {
    use super::*;

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
                &[crate::keymap::Context::Downloads],
                crate::keymap::action::Action::UseFor(0)
            )
            .as_deref(),
            Some("7")
        );
    }
}
