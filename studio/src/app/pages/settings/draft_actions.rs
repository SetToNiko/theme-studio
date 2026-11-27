//! App's methods for the Settings page's unsaved changes (the draft).
//!
//! This is a second `impl App { ... }` block (see `persist.rs` for the same
//! style). The draft data (`Draft`, `keymaps_value`) stays in `settings_page/draft.rs`. The
//! tests of these methods are in the test module of `settings_page/draft.rs` too (it builds
//! an `App`); this file only holds the methods that need the whole `App`:
//! running an edit on the draft, showing it, counting its changes, save and
//! discard.

use crate::app::app_state::App;
use crate::settings_page::draft::{Draft, keymaps_value};

impl App {
    /// The live settings, keymap and presets as a [`Draft`] (a copy).
    fn live_as_draft(&self) -> Draft {
        Draft {
            settings: self.settings.clone(),
            keymap: self.keymap.clone(),
            presets: self.presets.clone(),
        }
    }

    /// Swaps the draft and the live settings, keymap and presets.
    fn swap_draft(&mut self, d: &mut Draft) {
        std::mem::swap(&mut self.settings, &mut d.settings);
        std::mem::swap(&mut self.keymap, &mut d.keymap);
        std::mem::swap(&mut self.presets, &mut d.presets);
    }

    /// Runs a Settings page edit on the draft (made from what's live when
    /// there's none yet). A draft that ends up like what's saved goes away.
    pub fn with_draft<R>(&mut self, f: impl FnOnce(&mut App) -> R) -> R {
        self.settings_page.reloaded = false;
        let mut d = self
            .settings_page
            .draft
            .take()
            .unwrap_or_else(|| self.live_as_draft());
        self.swap_draft(&mut d);
        // `d` is now what's live
        self.settings_page.draft = Some(d);
        let r = f(self);
        if std::mem::take(&mut self.settings_page.reloaded) {
            // a file was restored or undone: what's loaded now is live
            self.settings_page.draft = None;
            return r;
        }
        let Some(mut live) = self.settings_page.draft.take() else {
            return r;
        };
        self.swap_draft(&mut live);
        // `live` is now the edited draft
        if live.files() != self.live_as_draft().files() {
            self.settings_page.draft = Some(live);
        }
        r
    }

    /// Draws (or reads) the Settings page as edited.
    pub fn show_draft<R>(&mut self, f: impl FnOnce(&mut App) -> R) -> R {
        let Some(mut d) = self.settings_page.draft.take() else {
            return f(self);
        };
        self.swap_draft(&mut d);
        let r = f(self);
        self.swap_draft(&mut d);
        self.settings_page.draft = Some(d);
        r
    }

    /// How many values the draft changes (0: nothing unsaved).
    pub fn draft_changes(&self) -> usize {
        let Some(d) = &self.settings_page.draft else {
            return 0;
        };
        let live = self.live_as_draft();
        let mut n = crate::settings_page::diff::diff_lines(
            &live.files()["settings"],
            &d.files()["settings"],
            false,
        )
        .iter()
        .filter(|l| !matches!(l, crate::settings_page::diff::DiffLine::Head { .. }))
        .count();
        n += crate::settings_page::diff::diff_lines(
            &live.files()["keymaps"],
            &d.files()["keymaps"],
            false,
        )
        .iter()
        .filter(|l| !matches!(l, crate::settings_page::diff::DiffLine::Head { .. }))
        .count();
        n.max(1)
    }

    /// Writes the draft to keymaps.json and settings.json, and uses it.
    pub fn settings_save(&mut self) {
        let Some(mut d) = self.settings_page.draft.take() else {
            return self.messages.info("No unsaved settings");
        };
        let old_sizes = self.settings.layout.sizes.clone();
        let before = self.live_as_draft().files();
        self.swap_draft(&mut d);
        let after = self.live_as_draft().files();
        let errors = self.write_changed_files(&before, &after);
        if errors.is_empty() {
            // new default pane sizes apply now too
            if self.settings.layout.sizes != old_sizes {
                self.layout.sizes = self.settings.layout.sizes.clone();
            }
            self.notify_saved("Saved");
        } else {
            // nothing (or not everything) was written: put the old live values
            // back and keep the draft, so the user can retry or discard it
            self.swap_draft(&mut d);
            self.settings_page.draft = Some(d);
            self.messages
                .error(format!("Saving: {}", errors.join("; ")));
        }
    }

    /// Writes the files whose contents differ between `before` and `after`
    /// (so `<leader>u` undoes the right one). Returns what failed, one text
    /// per file.
    fn write_changed_files(
        &self,
        before: &serde_json::Value,
        after: &serde_json::Value,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        if before["keymaps"] != after["keymaps"] {
            let keymaps = keymaps_value(&self.presets, &self.keymap.name);
            if let Err(e) = crate::model::store::write_json_file(
                &self.config_dir.join("keymaps.json"),
                &keymaps,
            ) {
                errors.push(format!("keymaps.json: {e:#}"));
            }
        }
        if before["settings"] != after["settings"]
            && let Err(e) =
                crate::config::settings_file::save_settings(&self.config_dir, &self.settings)
        {
            errors.push(format!("settings.json: {e:#}"));
        }
        errors
    }

    /// Drops the unsaved changes.
    pub fn settings_discard(&mut self) {
        if self.settings_page.draft.take().is_some() {
            self.settings_page.browse = None;
            self.messages.info("Discarded the unsaved settings");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::app_state::App;
    use crate::app::startup::Start;
    use crate::app::test_support::*;

    #[test]
    fn a_failed_settings_save_keeps_the_draft_and_the_old_live_values() {
        let store = temp_store("save-fails");
        // a file where the config folder should be: every write fails
        let config_dir = store.dir.join("config-is-a-file");
        std::fs::write(&config_dir, "not a folder").unwrap();
        let config = crate::config::load::Loaded {
            keymap: crate::keymap::map::Keymap::vim(),
            presets: crate::keymap::map::Keymap::builtin(),
            settings: crate::config::settings::Settings::default(),
            warnings: Vec::new(),
            dir: config_dir,
        };
        let mut app = App::with_config(store, Start::Named("One".into()), config).unwrap();
        app.preview.set_headless();
        let live = app.settings.font_list.hide_bitmap;
        app.with_draft(|a| a.settings.font_list.hide_bitmap = !live);
        app.settings_save();
        assert!(app.settings_page.draft.is_some(), "the draft is kept");
        assert_eq!(
            app.settings.font_list.hide_bitmap, live,
            "live is unchanged"
        );
        assert_eq!(app.draft_changes(), 1);
        let (m, is_error) = app.messages.message.clone().unwrap();
        assert!(is_error && m.starts_with("Saving:"), "{m}");
    }
}
