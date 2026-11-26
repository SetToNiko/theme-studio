//! App's methods for the Keymaps tab's preset browser.
//!
//! This is another `impl App { ... }` block (see `persist.rs` for the same
//! style). The preset types and the drawing (`KmDiff`, `keymap_diff`,
//! `draw_browse`) stay in `settings_page/keymap_presets.rs`; this file only holds the methods that
//! need the whole `App`: opening the list, its keys, clicks, and making,
//! deleting and resetting presets.

use crate::app::App;
use crate::app::state::nav::{Deferred, Naming, Overlay};
use crate::keymap::Keymap;
use crate::keymap::action::Action;
use crate::settings_page::keymap_presets::is_shipped;

impl App {
    /// Opens the preset list, on the active preset.
    pub fn open_browse(&mut self) {
        let i = self
            .presets
            .iter()
            .position(|p| p.name == self.keymap.name)
            .unwrap_or(0);
        self.settings_page.browse = Some(i);
        self.settings_page.browse_scroll = 0;
    }

    fn highlighted(&self) -> Option<Keymap> {
        self.settings_page
            .browse
            .and_then(|i| self.presets.get(i).cloned())
    }

    /// Makes the highlighted preset the active keymap (unsaved).
    fn load_highlighted(&mut self) {
        let Some(p) = self.highlighted() else { return };
        // the current keymap's edits stay in its preset
        self.sync_preset();
        self.messages
            .info(format!("Loaded the preset \"{}\" (unsaved)", p.name));
        self.keymap = p;
        self.settings_page.browse = None;
    }

    /// Keys while the preset list is open (inside the draft).
    pub fn run_browse(&mut self, a: Action) {
        let Some(i) = self.settings_page.browse else {
            return;
        };
        match a {
            Action::Confirm | Action::Right => return self.load_highlighted(),
            Action::SaveNamed | Action::Delete | Action::ResetPreset => self.browse_edit(a, i),
            _ => self.browse_move(a, i),
        }
        if self.settings_page.browse != Some(i) {
            self.settings_page.browse_scroll = 0;
        }
    }

    /// Moving in the preset list (`i` is the highlighted one): cursor,
    /// scrolling, "show all", closing it.
    fn browse_move(&mut self, a: Action, i: usize) {
        let n = self.presets.len();
        let p = &mut self.settings_page;
        match a {
            Action::Up | Action::Left => p.browse = Some(i.saturating_sub(1)),
            Action::Down => p.browse = Some((i + 1).min(n.saturating_sub(1))),
            Action::Top => p.browse = Some(0),
            Action::Bottom => p.browse = Some(n.saturating_sub(1)),
            Action::PageUp => p.browse_scroll = p.browse_scroll.saturating_sub(10),
            Action::PageDown => p.browse_scroll += 10,
            Action::ToggleAll => {
                p.browse_all = !p.browse_all;
                p.browse_scroll = 0;
            }
            Action::Back => p.browse = None,
            _ => {}
        }
    }

    /// Making, deleting and resetting the highlighted preset `i` (each asks
    /// for a name or a yes first, or says why it can't).
    fn browse_edit(&mut self, a: Action, i: usize) {
        let name = self.presets[i].name.clone();
        match a {
            Action::SaveNamed => {
                self.overlay = Overlay::Prompt {
                    title: format!("Name for a new preset made from \"{name}\""),
                    input: String::new(),
                    purpose: Naming::PresetFrom(name),
                };
            }
            Action::Delete if is_shipped(&name) => self
                .messages
                .info("Shipped presets can't be deleted (R resets them)"),
            Action::Delete => {
                self.overlay = Overlay::Ask {
                    question: format!("Delete the preset \"{name}\"? (Save makes it final)"),
                    then: Deferred::DeletePreset(name),
                };
            }
            Action::ResetPreset if is_shipped(&name) => {
                self.overlay = Overlay::Ask {
                    question: format!("Reset \"{name}\" to the shipped keys?"),
                    then: Deferred::ResetPreset(name),
                };
            }
            Action::ResetPreset => self
                .messages
                .info("Only shipped presets can be reset; this one is yours"),
            _ => {}
        }
    }

    /// A click on a preset: highlights it; on the highlighted one, loads it.
    pub fn preset_click(&mut self, i: usize) {
        if self.settings_page.browse == Some(i) {
            self.load_highlighted();
        } else {
            self.settings_page.browse = Some(i);
            self.settings_page.browse_scroll = 0;
        }
    }

    /// A new preset, a copy of `from`, highlighted in the list.
    pub fn new_preset_from(&mut self, from: &str, name: &str) {
        if self.presets.iter().any(|p| p.name == name) {
            return self
                .messages
                .error(format!("There's already a preset \"{name}\""));
        }
        let Some(mut km) = self.presets.iter().find(|p| p.name == from).cloned() else {
            return;
        };
        km.name = name.to_string();
        self.presets.push(km);
        self.settings_page.browse = Some(self.presets.len() - 1);
        self.messages.info(format!(
            "New preset \"{name}\" from \"{from}\" (unsaved); Enter loads it"
        ));
    }

    /// Deletes a preset (the active one: Vim becomes active).
    pub fn delete_preset(&mut self, name: &str) {
        self.presets.retain(|p| p.name != name);
        if self.keymap.name == name {
            self.keymap = self
                .presets
                .iter()
                .find(|p| p.name == "Vim")
                .cloned()
                .unwrap_or_else(Keymap::vim);
        }
        if let Some(i) = self.settings_page.browse {
            self.settings_page.browse = Some(i.min(self.presets.len().saturating_sub(1)));
        }
        self.messages
            .info(format!("Deleted the preset \"{name}\" (unsaved)"));
    }

    /// Puts a shipped preset back to its shipped keys.
    pub fn reset_preset(&mut self, name: &str) {
        let Some(b) = Keymap::builtin().into_iter().find(|b| b.name == name) else {
            return;
        };
        if let Some(p) = self.presets.iter_mut().find(|p| p.name == name) {
            *p = b.clone();
        }
        if self.keymap.name == name {
            self.keymap = b;
        }
        self.messages
            .info(format!("\"{name}\" is back to the shipped keys (unsaved)"));
    }
}
