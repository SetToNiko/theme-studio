//! App's methods for the Settings page as a whole (F3): the shared helpers,
//! clicks, scrolling, the keys of the page and the readme tab, and the
//! prompt and yes/no questions that the tabs open.
//!
//! This is another `impl App { ... }` block (see `fonts_preview_actions.rs`
//! for the same style). The tab-specific methods live in
//! `settings_keymaps_actions.rs` and `settings_history_actions.rs`. The page
//! data (`SettingsPage`, `Tab`, `KmRow`, the diff types) stays in
//! `settings_page/mod.rs`; this file only holds the methods that need the whole `App`.

use std::path::PathBuf;

use crate::app::App;
use crate::app::state::nav::{Deferred, Naming, Overlay};
use crate::app::state::steps::{offset, to_i32};
use crate::keymap::action::Action;
use crate::keymap::key::Key;
use crate::model::history::History;
use crate::settings_page::{FILES, HistFocus, KmRow, SettingsPage, TABS, Tab, ends, wrap};

impl App {
    // ----- shared ----------------------------------------------------------

    /// The cursor can rest on everything but the blank lines.
    pub fn keymap_selectable(rows: &[KmRow]) -> Vec<bool> {
        rows.iter().map(|r| !matches!(r, KmRow::Gap)).collect()
    }

    /// Moves a list cursor `sel` over `n` rows for a movement action (up,
    /// down, page, top, bottom). False when `a` is not a movement.
    pub(crate) fn move_sel(sel: &mut usize, n: usize, action: Action) -> bool {
        let last = to_i32(n.saturating_sub(1));
        let delta: i32 = match action {
            Action::Up => -1,
            Action::Down => 1,
            Action::PageUp => -10,
            Action::PageDown => 10,
            Action::Top => -to_i32(*sel),
            Action::Bottom => last - to_i32(*sel),
            _ => return false,
        };
        *sel = offset(*sel, delta).min(n.saturating_sub(1));
        true
    }

    /// Where a file with a history lives.
    pub fn file_path(&self, file: &str) -> PathBuf {
        match file {
            "themes.json" | "palettes.json" | "fonts.json" => self.store.dir.join(file),
            _ => self.config_dir.join(file),
        }
    }

    /// Copies the active keymap into its preset (in the draft: Save writes).
    pub(crate) fn sync_preset(&mut self) {
        let active = self.keymap.name.clone();
        if let Some(p) = self.presets.iter_mut().find(|p| p.name == active) {
            *p = self.keymap.clone();
        } else {
            self.presets.push(self.keymap.clone());
        }
    }

    /// After a config file changed on disk (restore, undo): use it.
    pub fn reload_file(&mut self, file: &str) {
        match file {
            "palettes.json" if !self.dirty() => {
                if let Ok(p) = self.store.load(&self.name) {
                    self.editor.load(p);
                }
            }
            "fonts.json" if !self.fonts.dirty() => {
                let name = self.fonts.name.clone();
                let _ = self.fonts.open(&name);
            }
            "keymaps.json" | "settings.json" => {
                let l = crate::config::load(&self.config_dir, false);
                self.keymap = l.keymap;
                self.presets = l.presets;
                self.settings = l.settings;
                // unsaved Settings page changes were made against the old file
                if self.settings_page.draft.take().is_some() && !self.settings_page.reloaded {
                    self.messages.info("The unsaved settings were dropped");
                }
                self.settings_page.reloaded = true;
            }
            _ => {}
        }
    }

    /// `<leader>u`: puts back the newest snapshot of the file the studio last
    /// wrote (which itself is kept, so doing it again redoes).
    pub fn undo_config(&mut self) {
        let Some((file, path)) = crate::model::history::last_write() else {
            return self.messages.info("No config change to undo yet");
        };
        let Some(h) = History::current() else { return };
        let newest = h
            .list_info(&file)
            .into_iter()
            .find(|(_, s)| s.name.is_none());
        let Some(snap) = newest.and_then(|(p, _)| History::load(&p)) else {
            return self.messages.info(format!("No earlier version of {file}"));
        };
        match crate::model::store::write_json_file(&path, &snap.content) {
            Ok(()) => {
                self.reload_file(&file);
                self.messages.info(format!(
                    "Undid the last change to {file} ({}); again to redo",
                    snap.summary
                ));
            }
            Err(e) => self.messages.error(format!("{e:#}")),
        }
    }

    // ----- mouse -----------------------------------------------------------

    /// A click on the Settings page: selects; on the selected row, acts
    /// like Enter.
    // Long function: one match over the click targets of the Settings page.
    pub fn settings_click(&mut self, target: crate::app::state::hit::Target) {
        use crate::app::state::hit::Target as T;
        if let T::PresetRow(index) = target {
            return self.with_draft(|app| app.preset_click(index));
        }
        let p = &mut self.settings_page;
        if !matches!(target, T::SettingsTab(_)) {
            p.tabs_focused = false;
        }
        match target {
            T::SettingsTab(index) => p.tab = TABS[index as usize].0,
            T::HistFile(index) => {
                p.hist_focus = HistFocus::Files;
                if p.hist_file != index {
                    p.hist_file = index;
                    p.hist_sel = 0;
                    p.diff_scroll = 0;
                }
            }
            T::SettingsRow(index) => {
                let sel = match p.tab {
                    Tab::Keymaps => &mut p.km_sel,
                    Tab::Behaviour => &mut p.bh_sel,
                    Tab::History => {
                        p.hist_focus = HistFocus::Versions;
                        if p.hist_sel != index {
                            p.diff_scroll = 0;
                        }
                        &mut p.hist_sel
                    }
                    Tab::Readme => return,
                };
                let again = *sel == index;
                *sel = index;
                if again {
                    self.run_settings(Action::Confirm);
                }
            }
            _ => {}
        }
    }

    /// The wheel over a list moves its selection; over the preview, scrolls.
    pub fn settings_scroll(&mut self, t: crate::app::state::hit::Target, d: i32) {
        use crate::app::state::hit::Target as T;
        // the README: three lines a notch (and never up onto the tabs)
        if self.settings_page.tab == Tab::Readme {
            for _ in 0..3 {
                self.run_readme_tab(if d > 0 { Action::Down } else { Action::Up });
            }
            return;
        }
        if t == T::SettingsDiff && self.settings_page.browse.is_some() {
            let p = &mut self.settings_page;
            p.browse_scroll = offset(p.browse_scroll, d * 3);
            return;
        }
        if let T::PresetRow(_) = t {
            return self
                .with_draft(|app| app.run_browse(if d > 0 { Action::Down } else { Action::Up }));
        }
        if t == T::SettingsDiff {
            let p = &mut self.settings_page;
            p.diff_scroll = offset(p.diff_scroll, d * 3);
            return;
        }
        if self.settings_page.tab == Tab::History {
            self.settings_page.hist_focus = HistFocus::Versions;
        }
        self.run_settings(if d > 0 { Action::Down } else { Action::Up });
    }

    // ----- keys ------------------------------------------------------------

    // Long function: one match over the page-wide keys, then the hand-over to the current tab.
    pub fn run_settings(&mut self, a: Action) {
        let top = self.settings.layout.inner_tabs_on_top();
        if self.settings_page.tabs_focused {
            return self.run_settings_tab_strip(a, top);
        }
        match a {
            Action::TabNext | Action::TabPrev => {
                switch_tab(
                    &mut self.settings_page,
                    if a == Action::TabNext { 1 } else { -1 },
                );
                return;
            }
            // the preset's Save key saves the page; Discard drops the changes
            Action::Save => return self.settings_save(),
            Action::Discard => return self.settings_discard(),
            // Esc closes the preset list first, then leaves the page
            Action::Back if self.settings_page.browse.is_some() => {
                self.settings_page.browse = None;
                return;
            }
            Action::Back => {
                self.page = crate::app::state::nav::Page::Main;
                return;
            }
            // past the edge of the list next to the tabs: onto the tabs
            Action::Up if top && self.settings_at_edge(true) => {
                self.settings_page.tabs_focused = true;
                return;
            }
            Action::Down if !top && self.settings_at_edge(false) => {
                self.settings_page.tabs_focused = true;
                return;
            }
            // the other edge: on to the page tabs, if they're there
            Action::Up if self.settings_at_edge(true) && self.focus_page_tabs(true) => return,
            Action::Down if self.settings_at_edge(false) && self.focus_page_tabs(false) => return,
            _ => {}
        }
        // Keymaps and Behaviour edit the draft; History acts at once
        match self.settings_page.tab {
            Tab::Keymaps => self.with_draft(|app| app.run_keymaps_tab(a)),
            Tab::Behaviour => self.with_draft(|app| app.run_behaviour_tab(a)),
            Tab::History => self.run_history_tab(a),
            Tab::Readme => self.run_readme_tab(a),
        }
    }

    /// Keys while the keyboard is on the tab strip: left/right change tabs;
    /// moving back towards the page, Enter or Esc return to it. `top` is
    /// whether the strip is above the page.
    fn run_settings_tab_strip(&mut self, a: Action, top: bool) {
        match a {
            Action::Left | Action::TabPrev => switch_tab(&mut self.settings_page, -1),
            Action::Right | Action::TabNext => switch_tab(&mut self.settings_page, 1),
            Action::Down if top => self.settings_page.tabs_focused = false,
            Action::Up if !top => self.settings_page.tabs_focused = false,
            // on past the page's tabs to the page tabs (F1 / F2 / F3)
            Action::Up if top => {
                self.focus_page_tabs(true);
            }
            Action::Down if !top => {
                self.focus_page_tabs(false);
            }
            Action::Confirm | Action::Back => self.settings_page.tabs_focused = false,
            _ => {}
        }
    }

    /// README tab keys: scrolling, paging, and jumping between headings.
    fn run_readme_tab(&mut self, a: Action) {
        let p = &mut self.settings_page;
        let Some((_, doc)) = &p.readme_doc else {
            return;
        };
        let last = doc.lines.len().saturating_sub(1);
        let s = &mut p.readme_scroll;
        match a {
            Action::Down => *s = (*s + 1).min(last),
            Action::Up => *s = s.saturating_sub(1),
            Action::PageDown => *s = (*s + 20).min(last),
            Action::PageUp => *s = s.saturating_sub(20),
            Action::Top => *s = 0,
            Action::Bottom => *s = last,
            Action::NextHeading => {
                if let Some(h) = doc.headings.iter().find(|&&h| h > *s) {
                    *s = *h;
                }
            }
            Action::PrevHeading => {
                if let Some(h) = doc.headings.iter().rev().find(|&&h| h < *s) {
                    *s = *h;
                }
            }
            _ => {}
        }
    }

    /// The cursor is on the first (`up`) or last row of the tab's list.
    fn settings_at_edge(&mut self, up: bool) -> bool {
        let at = |sel: usize, n: usize| if up { sel == 0 } else { sel + 1 >= n };
        match self.settings_page.tab {
            Tab::Keymaps if self.settings_page.browse.is_some() => false,
            Tab::Keymaps => {
                let ok = self.show_draft(|app| Self::keymap_selectable(&app.keymap_rows()));
                let (first, last) = ends(&ok);
                self.settings_page.km_sel == if up { first } else { last }
            }
            Tab::Behaviour => self.behaviour_at_edge(up),
            // scrolled to the top: on to the tabs (never at the bottom, so
            // the last lines can be scrolled to)
            Tab::Readme => up && self.settings_page.readme_scroll == 0,
            Tab::History => match self.settings_page.hist_focus {
                HistFocus::Files => at(self.settings_page.hist_file, FILES.len()),
                HistFocus::Versions => {
                    let n = self.versions().len();
                    at(self.settings_page.hist_sel, n)
                }
            },
        }
    }

    pub(crate) fn prompt(&mut self, title: &str, purpose: Naming, input: &str) {
        self.overlay = Overlay::Prompt {
            title: title.to_string(),
            input: input.to_string(),
            purpose,
        };
    }

    pub(crate) fn ask(&mut self, question: String, then: Deferred) {
        self.overlay = Overlay::Ask { question, then };
    }

    /// The prompt's text, confirmed.
    // Long function: one match over the kinds of name or text prompt.
    pub fn finish_prompt(&mut self, purpose: Naming, input: &str) {
        let text = input.trim();
        if text.is_empty() {
            return;
        }
        match purpose {
            Naming::PresetFrom(from) => self.new_preset_from(&from, text),
            Naming::Preset => {
                if self.presets.iter().any(|p| p.name == text) {
                    return self
                        .messages
                        .error(format!("There's already a preset \"{text}\""));
                }
                self.keymap.name = text.to_string();
                self.sync_preset();
                self.messages
                    .info(format!("New preset \"{text}\", now active (unsaved)"));
            }
            Naming::Version(file) => {
                let now = std::fs::read_to_string(self.file_path(file))
                    .ok()
                    .and_then(|t| serde_json::from_str(&t).ok());
                match (History::current(), now) {
                    (Some(h), Some(content)) => match h.save_named(file, text, &content) {
                        Ok(_) => self.notify_saved(format!("Saved {file} as \"{text}\"")),
                        Err(e) => self.messages.error(format!("{e:#}")),
                    },
                    _ => self.messages.error(format!("Can't read {file}")),
                }
            }
            Naming::BindingText { ctx, action, add } => {
                let mut seqs = Vec::new();
                for part in text.split_whitespace() {
                    match Key::parse_seq(part, self.keymap.leader) {
                        Ok(k) => seqs.push(k),
                        Err(e) => return self.messages.error(format!("{part}: {e}")),
                    }
                }
                self.bind(ctx, action, seqs, add);
            }
        }
    }

    /// The question was answered yes.
    // Long function: one match over every yes/no question the page asks.
    pub fn run_deferred(&mut self, d: Deferred) {
        match d {
            Deferred::Restore { file, version } => {
                let Some(snap) = History::load(&version) else {
                    return self.messages.error("That version can't be read");
                };
                match crate::model::store::write_json_file(&self.file_path(file), &snap.content) {
                    Ok(()) => {
                        self.reload_file(file);
                        self.settings_page.hist_sel = 0;
                        self.messages.info(format!(
                            "Restored {file} (the version before is in the history)"
                        ));
                    }
                    Err(e) => self.messages.error(format!("{e:#}")),
                }
            }
            Deferred::DeleteVersion(path) => {
                if let Some(h) = History::current() {
                    match h.delete(&path) {
                        Ok(()) => self.messages.info("Deleted that version"),
                        Err(e) => self.messages.error(format!("{e:#}")),
                    }
                }
            }
            Deferred::DeletePreset(name) => self.delete_preset(&name),
            Deferred::ResetPreset(name) => self.reset_preset(&name),
            Deferred::StartTournament(pool) => self.tour_start(pool),
            Deferred::NerdForRoles { id, group, roles } => {
                self.install_nerd_for_roles(id, group, roles);
            }
            Deferred::ClearPreviews => match crate::system::download::preview::clear_previews() {
                Ok(()) => {
                    let pv = self.pv();
                    pv.families.clear();
                    pv.image = None;
                    self.prefetch.reset();
                    self.messages.info("Deleted the preview fonts");
                }
                Err(e) => self.messages.error(format!("{e:#}")),
            },
        }
    }
}

/// Moves the Settings page to the next (`d` = 1) or previous (-1) tab.
fn switch_tab(p: &mut SettingsPage, d: i32) {
    let i = TABS.iter().position(|(t, _)| *t == p.tab).unwrap_or(0);
    p.tab = TABS[wrap(i, d, TABS.len())].0;
}
