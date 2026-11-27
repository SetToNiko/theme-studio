//! App's methods for the Settings page's History tab: a file's versions,
//! the difference view, and the tab's keys.
//!
//! This is another `impl App { ... }` block (see `settings_actions.rs`). The
//! history types (`HistFocus`, `DiffView`, `DiffLine`, `diff_lines`) stay in
//! `settings_page/mod.rs`; this file only holds the methods that need the whole `App`.

use std::path::PathBuf;

use serde_json::Value;

use crate::app::App;
use crate::app::state::nav::{Deferred, Naming};
use crate::app::state::steps::offset;
use crate::keymap::action::Action;
use crate::model::color::Rgb;
use crate::model::history::{History, SnapshotInfo};
use crate::settings_page::diff::{Colours, DiffView, diff_lines};
use crate::settings_page::state::{FILES, HistFocus};

impl App {
    /// The selected file's versions (named first, then newest first);
    /// read again only after something changed.
    pub fn versions(&mut self) -> Vec<(PathBuf, SnapshotInfo)> {
        let generation = crate::model::history::generation();
        let file = self.settings_page.hist_file;
        if let Some(v) = self.settings_page.cached_versions(generation, file) {
            return v;
        }
        let v = History::current()
            .map(|h| h.list_info(FILES[file]))
            .unwrap_or_default();
        self.settings_page
            .store_versions(generation, file, v.clone());
        v
    }

    /// The preview of the selected version against the file now.
    pub fn diff_view(&mut self) -> DiffView {
        let versions = self.versions();
        let p = &self.settings_page;
        let sel = versions.get(p.hist_sel).map(|(path, _)| path.clone());
        let generation = crate::model::history::generation();
        let hist_file = p.hist_file;
        let show_all = p.show_all;
        if let Some(v) = self.settings_page.cached_view(
            generation,
            hist_file,
            sel.as_deref(),
            show_all,
            &self.name,
        ) {
            return v;
        }
        let view = self.build_diff_view(sel.as_deref());
        self.settings_page.store_view(
            generation,
            hist_file,
            sel,
            show_all,
            self.name.clone(),
            view.clone(),
        );
        view
    }

    /// Reads the file now and the chosen version, and compares them. An empty
    /// view when no version is chosen or it cannot be read.
    fn build_diff_view(&self, version: Option<&std::path::Path>) -> DiffView {
        let file = FILES[self.settings_page.hist_file];
        let now: Value = std::fs::read_to_string(self.file_path(file))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null);
        let Some(then) = version.and_then(History::load) else {
            return DiffView::default();
        };
        let palette = (file == "palettes.json").then(|| {
            (
                palette_colours(&now, &self.name),
                palette_colours(&then.content, &self.name),
            )
        });
        DiffView {
            lines: diff_lines(&now, &then.content, self.settings_page.show_all),
            palette,
        }
    }

    pub(crate) fn run_history_tab(&mut self, a: Action) {
        if self.history_nav_key(a) {
            return;
        }
        if self.settings_page.hist_focus == HistFocus::Files {
            return self.run_history_files(a);
        }
        self.run_history_versions(a);
    }

    /// Keys that work on both lists: show all, change the focused list, scroll
    /// the diff. True when `a` was one of them.
    fn history_nav_key(&mut self, a: Action) -> bool {
        let p = &mut self.settings_page;
        if a == Action::ToggleAll {
            p.show_all = !p.show_all;
            p.diff_scroll = 0;
            return true;
        }
        if matches!(
            a,
            Action::FocusNext | Action::FocusPrev | Action::Left | Action::Right
        ) {
            p.hist_focus = match (p.hist_focus, a) {
                (HistFocus::Files, Action::Right | Action::FocusNext | Action::FocusPrev) => {
                    HistFocus::Versions
                }
                (HistFocus::Versions, Action::Left | Action::FocusNext | Action::FocusPrev) => {
                    HistFocus::Files
                }
                (f, _) => f,
            };
            return true;
        }
        if matches!(a, Action::PageUp | Action::PageDown) {
            let d: i32 = if a == Action::PageUp { -10 } else { 10 };
            p.diff_scroll = offset(p.diff_scroll, d);
            return true;
        }
        false
    }

    /// Keys while the version list has the focus: move over the versions,
    /// restore, save a named version, or delete one.
    fn run_history_versions(&mut self, a: Action) {
        let versions = self.versions();
        let p = &mut self.settings_page;
        let before = p.hist_sel;
        if Self::move_sel(&mut p.hist_sel, versions.len(), a) {
            if p.hist_sel != before {
                p.diff_scroll = 0;
            }
            return;
        }
        let file = FILES[p.hist_file];
        let selected = versions.get(p.hist_sel).cloned();
        match a {
            Action::Restore | Action::Confirm => self.ask_restore(file, selected),
            Action::SaveNamed => self.prompt(
                &format!("Name for the current {file}"),
                Naming::Version(file),
                "",
            ),
            Action::Delete => self.ask_delete(selected),
            _ => {}
        }
    }

    /// Asks before `file` goes back to the `selected` version. Says so when
    /// the file has no version.
    fn ask_restore(&mut self, file: &'static str, selected: Option<(PathBuf, SnapshotInfo)>) {
        match selected {
            Some((path, s)) => self.ask(
                format!(
                    "Restore {file} to the version from {} ({})? What you have now goes to the history.",
                    s.when,
                    s.name.unwrap_or(s.summary)
                ),
                Deferred::Restore { file, version: path },
            ),
            None => self.messages.info(format!("{file} has no saved versions yet")),
        }
    }

    /// Asks before the `selected` version is deleted. Only named versions can
    /// be deleted.
    fn ask_delete(&mut self, selected: Option<(PathBuf, SnapshotInfo)>) {
        match selected {
            Some((path, s)) if s.name.is_some() => {
                self.ask(
                    format!("Delete the version \"{}\"?", s.name.unwrap()),
                    Deferred::DeleteVersion(path),
                );
            }
            Some(_) => self
                .messages
                .info("Only named versions can be deleted (snapshots are pruned on their own)"),
            None => {}
        }
    }

    /// Keys while the file list has the focus: move over the files, or
    /// Enter to go on to the file's versions.
    fn run_history_files(&mut self, a: Action) {
        let p = &mut self.settings_page;
        let mut i = p.hist_file;
        if Self::move_sel(&mut i, FILES.len(), a) {
            p.hist_file = i;
            p.hist_sel = 0;
            p.diff_scroll = 0;
        } else if a == Action::Confirm {
            p.hist_focus = HistFocus::Versions;
        }
    }
}

/// The colours of palette `name` in a palettes.json value (or a snapshot of
/// one); empty when it is not there.
fn palette_colours(v: &Value, name: &str) -> Colours {
    let data = v.get("palettes").unwrap_or(v);
    data.get(name)
        .and_then(|p| p.as_object())
        .map(|o| {
            o.iter()
                .filter_map(|(k, c)| Some((k.clone(), Rgb::parse(c.as_str()?)?)))
                .collect()
        })
        .unwrap_or_default()
}
