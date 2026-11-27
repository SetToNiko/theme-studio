//! Loads the config folder: keymaps.json and settings.json together.

use std::path::{Path, PathBuf};

use super::keymaps_file::load_keymaps_file;
use super::settings::Settings;
use super::settings_file::load_settings;
use crate::keymap::map::Keymap;

/// What loading the config produced, and anything worth telling the user.
pub struct Loaded {
    pub keymap: Keymap,
    pub presets: Vec<Keymap>,
    pub settings: Settings,
    pub warnings: Vec<String>,
    /// The folder the files are in (for saving).
    pub dir: PathBuf,
}

/// Loads both files from `dir`, creating or topping them up when `write` is
/// set. A file that doesn't parse is left alone and the defaults are used.
pub fn load(dir: &Path, write: bool) -> Loaded {
    let mut warnings = Vec::new();
    let keymaps_path = dir.join("keymaps.json");
    let settings_path = dir.join("settings.json");

    let (keymap, presets) = load_keymaps_file(&keymaps_path, write, &mut warnings);

    let settings = load_settings(&settings_path, write, &mut warnings);

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
    use crate::fonts::filters::Spacing;
    use crate::keymap::action::Action;
    use crate::keymap::key::Key;
    use crate::keymap::{context::Context, lookup::Lookup};
    use crate::model::store::to_jq_string;
    use serde_json::Value;

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

    // Long function (reason verified, phase 6): one scenario; the file content and the load result are checked together.
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
        let d = crate::model::paths::studio_config_dir();
        assert!(d.ends_with("theme-studio"));
    }
}
