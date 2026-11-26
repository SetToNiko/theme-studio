//! Unsaved Settings page changes. Keymap and Behaviour edits go to a draft;
//! nothing is written or takes effect until Save (the preset's Save key, or
//! the button). Discard drops the draft; quitting with one asks first.
//!
//! The test module below also tests the `App` methods of
//! `app/draft_actions.rs` (it builds an `App`).

use serde_json::{Map, Value};

use crate::config::settings::Settings;
use crate::keymap::Keymap;

/// The Settings page's edited copy of what keymaps.json and settings.json
/// hold.
#[derive(Clone, Debug)]
pub struct Draft {
    pub settings: Settings,
    pub keymap: Keymap,
    pub presets: Vec<Keymap>,
}

/// keymaps.json's contents for a set of presets and the active one.
pub fn keymaps_value(presets: &[Keymap], active: &str) -> Value {
    let mut obj = Map::new();
    for p in presets {
        obj.insert(p.name.clone(), p.to_json());
    }
    let mut m = Map::new();
    m.insert(
        "version".into(),
        Value::from(crate::config::KEYMAPS_VERSION),
    );
    m.insert("active".into(), Value::String(active.to_string()));
    m.insert("presets".into(), Value::Object(obj));
    Value::Object(m)
}

impl Draft {
    /// Both files' contents, for comparing and counting changes.
    pub(crate) fn files(&self) -> Value {
        serde_json::json!({
            "keymaps": keymaps_value(&self.presets, &self.keymap.name),
            "keymap": self.keymap.to_json(),
            "settings": serde_json::to_value(&self.settings).unwrap_or(Value::Null),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Start};
    use crate::palette::Palette;
    use crate::store::{Store, data_file, to_jq_string};

    /// An app on a temp store with a temp config folder (never ~/.config).
    /// Returns the app and the config folder.
    fn test_app(tag: &str) -> (App, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("theme-studio-draft-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut m = Map::new();
        m.insert("One".into(), Value::Object(Palette::blank().to_json()));
        std::fs::write(
            dir.join("palettes.json"),
            to_jq_string(&data_file("palettes", m)),
        )
        .unwrap();
        // saving records history: keep it in the temp folder
        crate::history::init_for_test(dir.join("history"));
        let config_dir = dir.join("studio-config");
        let config = crate::config::Loaded {
            keymap: Keymap::vim(),
            presets: Keymap::builtin(),
            settings: Settings::default(),
            warnings: Vec::new(),
            dir: config_dir.clone(),
        };
        let mut app = App::with_config(Store::at(dir), Start::Named("One".into()), config).unwrap();
        app.preview.set_headless();
        (app, config_dir)
    }

    /// An edit as the Settings page makes it.
    fn flip_hide_bitmap(app: &mut App) {
        app.with_draft(|a| a.settings.font_list.hide_bitmap = !a.settings.font_list.hide_bitmap);
    }

    /// A leader change as the Behaviour tab makes it: the keymap, then its preset.
    fn change_leader(app: &mut App) {
        app.with_draft(|a| {
            let comma = crate::keymap::key::Key::parse_seq(",", a.keymap.leader).unwrap()[0];
            a.keymap = a.keymap.with_leader(comma);
            a.sync_preset();
        });
    }

    #[test]
    fn keymaps_value_lists_version_active_name_and_each_preset() {
        let presets = Keymap::builtin();
        let v = keymaps_value(&presets, "vim");
        assert_eq!(v["version"], Value::from(crate::config::KEYMAPS_VERSION));
        assert_eq!(v["active"], "vim");
        let obj = v["presets"].as_object().unwrap();
        assert_eq!(obj.len(), presets.len());
        for p in &presets {
            assert_eq!(obj[&p.name], p.to_json());
        }
    }

    #[test]
    fn no_draft_means_no_changes() {
        let (app, _) = test_app("none");
        assert!(app.settings_page.draft.is_none());
        assert_eq!(app.draft_changes(), 0);
    }

    #[test]
    fn with_draft_edits_the_draft_and_leaves_live_settings_alone() {
        let (mut app, _) = test_app("edit");
        let live = app.settings.font_list.hide_bitmap;
        flip_hide_bitmap(&mut app);
        assert_eq!(app.settings.font_list.hide_bitmap, live);
        assert!(app.settings_page.draft.is_some());
        assert_eq!(app.draft_changes(), 1);
        assert_eq!(app.show_draft(|a| a.settings.font_list.hide_bitmap), !live);
    }

    #[test]
    fn with_draft_returns_the_closure_result() {
        let (mut app, _) = test_app("result");
        let n = app.with_draft(|_| 42);
        assert_eq!(n, 42);
        // an edit that changes nothing leaves no draft
        assert!(app.settings_page.draft.is_none());
    }

    #[test]
    fn an_edit_undone_by_a_second_edit_drops_the_draft() {
        let (mut app, _) = test_app("undone");
        flip_hide_bitmap(&mut app);
        assert!(app.settings_page.draft.is_some());
        flip_hide_bitmap(&mut app);
        assert!(app.settings_page.draft.is_none());
        assert_eq!(app.draft_changes(), 0);
    }

    #[test]
    fn show_draft_without_a_draft_reads_the_live_settings() {
        let (mut app, _) = test_app("show-live");
        let live = app.settings.font_list.hide_bitmap;
        assert_eq!(app.show_draft(|a| a.settings.font_list.hide_bitmap), live);
        assert!(app.settings_page.draft.is_none());
    }

    #[test]
    fn show_draft_puts_everything_back_after_the_closure() {
        let (mut app, _) = test_app("show-back");
        flip_hide_bitmap(&mut app);
        let live = app.settings.font_list.hide_bitmap;
        app.show_draft(|_| ());
        assert_eq!(app.settings.font_list.hide_bitmap, live);
        assert_eq!(app.draft_changes(), 1);
    }

    #[test]
    fn a_reload_during_the_edit_drops_the_draft() {
        let (mut app, _) = test_app("reloaded");
        flip_hide_bitmap(&mut app);
        // a restored file: the closure says what is loaded is now live
        app.with_draft(|a| {
            a.settings.font_list.hide_bitmap = true;
            a.settings_page.reloaded = true;
        });
        assert!(app.settings_page.draft.is_none());
        // what the closure loaded stays live
        assert!(app.settings.font_list.hide_bitmap);
    }

    #[test]
    fn draft_changes_counts_values_in_settings_and_keymaps() {
        let (mut app, _) = test_app("count");
        app.with_draft(|a| {
            a.settings.font_list.hide_bitmap = !a.settings.font_list.hide_bitmap;
            a.settings.font_list.hide_cjk_emoji_math = !a.settings.font_list.hide_cjk_emoji_math;
        });
        assert_eq!(app.draft_changes(), 2);
        change_leader(&mut app);
        // the leader is one value in the active preset (keymaps.json)
        assert_eq!(app.draft_changes(), 3);
    }

    #[test]
    fn save_without_a_draft_only_says_so() {
        let (mut app, dir) = test_app("save-none");
        app.settings_save();
        assert_eq!(
            app.messages.message,
            Some(("No unsaved settings".to_string(), false))
        );
        assert!(!dir.join("settings.json").exists());
    }

    #[test]
    fn save_writes_only_the_changed_file_and_makes_the_draft_live() {
        let (mut app, dir) = test_app("save");
        flip_hide_bitmap(&mut app);
        let want = app.show_draft(|a| a.settings.font_list.hide_bitmap);
        app.settings_save();
        assert!(app.settings_page.draft.is_none());
        assert_eq!(app.settings.font_list.hide_bitmap, want);
        assert!(dir.join("settings.json").exists());
        // the keymaps did not change: keymaps.json is not written
        assert!(!dir.join("keymaps.json").exists());
        let loaded = crate::config::load(&dir, false);
        assert_eq!(loaded.settings.font_list.hide_bitmap, want);
        assert_eq!(
            app.messages.message,
            Some(("Saved".to_string(), false)),
            "the default is the status line"
        );
        assert_eq!(app.messages.notice, None);
    }

    #[test]
    fn save_writes_keymaps_json_when_the_keymap_changed() {
        let (mut app, dir) = test_app("save-keymap");
        let comma = crate::keymap::key::Key::parse_seq(",", app.keymap.leader).unwrap()[0];
        change_leader(&mut app);
        app.settings_save();
        assert!(dir.join("keymaps.json").exists());
        assert!(!dir.join("settings.json").exists());
        assert_eq!(app.keymap.leader, comma);
        assert_eq!(crate::config::load(&dir, false).keymap.leader, comma);
    }

    #[test]
    fn save_shows_a_box_when_settings_say_popup() {
        let (mut app, _) = test_app("save-popup");
        app.settings.messages.saved = "popup".into();
        flip_hide_bitmap(&mut app);
        app.settings_save();
        assert_eq!(app.messages.notice.as_deref(), Some("Saved"));
        assert_eq!(app.messages.message, None);
    }

    #[test]
    fn save_applies_new_default_pane_sizes_to_the_running_layout() {
        let (mut app, _) = test_app("save-sizes");
        app.with_draft(|a| a.settings.layout.sizes.fonts_width = 0.123);
        assert_ne!(app.layout.sizes.fonts_width, 0.123);
        app.settings_save();
        assert_eq!(app.layout.sizes.fonts_width, 0.123);
    }

    #[test]
    fn save_keeps_running_sizes_when_the_default_sizes_did_not_change() {
        let (mut app, _) = test_app("save-keep-sizes");
        app.layout.sizes.fonts_width = 0.77; // the user dragged a border
        flip_hide_bitmap(&mut app);
        app.settings_save();
        assert_eq!(app.layout.sizes.fonts_width, 0.77);
    }

    #[test]
    fn save_reports_a_write_error_and_still_uses_the_draft() {
        let (mut app, dir) = test_app("save-error");
        // a file where the config folder should be: the write fails
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::fs::write(&dir, "not a folder").unwrap();
        flip_hide_bitmap(&mut app);
        let want = app.show_draft(|a| a.settings.font_list.hide_bitmap);
        app.settings_save();
        let (m, is_error) = app.messages.message.clone().unwrap();
        assert!(is_error && m.starts_with("Saving: settings.json:"), "{m}");
        // behaviour as it is: the draft is gone and its values are live
        // although nothing was written
        assert!(app.settings_page.draft.is_none());
        assert_eq!(app.settings.font_list.hide_bitmap, want);
    }

    #[test]
    fn discard_drops_the_draft_and_closes_the_preset_browser() {
        let (mut app, _) = test_app("discard");
        let live = app.settings.font_list.hide_bitmap;
        flip_hide_bitmap(&mut app);
        app.settings_page.browse = Some(0);
        app.settings_discard();
        assert!(app.settings_page.draft.is_none());
        assert_eq!(app.settings_page.browse, None);
        assert_eq!(app.settings.font_list.hide_bitmap, live);
        assert_eq!(
            app.messages.message,
            Some(("Discarded the unsaved settings".to_string(), false))
        );
    }

    #[test]
    fn discard_without_a_draft_does_nothing() {
        let (mut app, _) = test_app("discard-none");
        app.settings_page.browse = Some(1);
        app.settings_discard();
        assert_eq!(app.settings_page.browse, Some(1));
        assert_eq!(app.messages.message, None);
    }
}
