//! App's methods for saving, opening and applying palettes and font presets.
//!
//! This is a second `impl App { ... }` block. Rust allows an `impl` of a type
//! in another file of the same crate. `app/app_state.rs` has the struct. The other files of the folder have the other `App`
//! methods.
//!
//! The palette methods ([`App::save`](crate::app::app_state::App::save), [`App::save_as`](crate::app::app_state::App::save_as), [`App::apply`](crate::app::app_state::App::apply)) and
//! the font preset methods ([`App::fonts_save`](crate::app::app_state::App::fonts_save), [`App::fonts_save_as`](crate::app::app_state::App::fonts_save_as)) are
//! side by side on purpose: they follow the same pattern (ask for a name if
//! the item is new, save, then optionally apply). [`App::guard`](crate::app::app_state::App::guard) runs an
//! action at once, or asks first when there are unsaved changes.

use super::app_state::App;
use super::state::nav::{Focus, For, Overlay, Pending};
use crate::model::color::Rgb;

impl App {
    pub(super) fn save(&mut self, then_apply: bool) {
        self.overlay_for = For::Palette;
        if self.is_new {
            self.overlay = Overlay::SaveAs {
                input: String::new(),
                overwrite: false,
                then_apply,
            };
            return;
        }
        let name = self.name.clone();
        self.save_as(&name, then_apply);
    }

    pub(crate) fn save_as(&mut self, name: &str, then_apply: bool) {
        match self.store.save(name, &self.editor.palette) {
            Ok(()) => {
                self.name = name.to_string();
                self.is_new = false;
                self.editor.mark_saved();
                if then_apply {
                    self.apply_saved();
                } else {
                    self.notify_saved(format!("Saved \"{name}\""));
                }
            }
            Err(e) => self.messages.error(format!("Save failed: {e:#}")),
        }
    }

    /// Saves if needed, then makes this palette the system theme.
    pub(super) fn apply(&mut self) {
        if self.dirty() {
            self.save(true);
        } else {
            self.apply_saved();
        }
    }

    pub(super) fn apply_saved(&mut self) {
        let name = self.name.clone();
        match crate::system::switch::apply_theme(&self.store, &name) {
            Ok(()) => {
                self.active = Some(name.clone());
                self.system_palette = Some(self.editor.palette.clone());
                self.notify_saved(format!("Saved and applied \"{name}\""));
            }
            Err(e) => self.messages.error(format!("Apply failed: {e:#}")),
        }
    }

    pub(super) fn open(&mut self, name: &str) {
        match self.store.load(name) {
            Ok(p) => {
                self.name = name.to_string();
                self.is_new = false;
                self.editor.load(p);
                self.reset_history();
                self.messages.info(format!("Opened \"{name}\""));
            }
            Err(e) => self.messages.error(format!("{e:#}")),
        }
    }

    pub(super) fn new_palette(&mut self) {
        self.name = "untitled".to_string();
        self.is_new = true;
        self.editor.start_blank();
        self.reset_history();
        self.messages
            .info("New palette: edit the 8 base colours first, the rest follow");
    }

    pub(crate) fn reset_history(&mut self) {
        self.editor.clear_history();
        self.sel = 0;
        self.list_scroll = 0;
        self.focus = Focus::List;
        self.load_picker();
    }

    /// Runs `p` now, or asks first when there are unsaved changes.
    pub(super) fn guard(&mut self, p: Pending) {
        let dirty = match p {
            Pending::NewTheme | Pending::OpenTheme(_) => self.theme_dirty(),
            Pending::Quit => self.theme_dirty() || self.settings_page.draft.is_some(),
            _ if p.is_fonts() => self.fonts.dirty(),
            _ => self.dirty(),
        };
        if dirty {
            self.overlay = Overlay::Confirm(p);
        } else {
            self.run_pending(p);
        }
    }

    pub(super) fn run_pending(&mut self, p: Pending) {
        self.overlay = Overlay::None;
        match p {
            Pending::Quit => self.quit = true,
            Pending::New => self.new_palette(),
            Pending::Open(n) => self.open(&n),
            Pending::NewTheme => self.new_theme(),
            Pending::OpenTheme(n) => self.open_theme(&n),
            Pending::NewFonts => {
                self.fonts.new_preset();
                self.messages
                    .info("New font preset (a copy of the current fonts); s asks for a name");
            }
            Pending::OpenFonts(n) => match self.fonts.open(&n) {
                Ok(()) => self.messages.info(format!("Opened font preset \"{n}\"")),
                Err(e) => self.messages.error(format!("{e:#}")),
            },
        }
    }

    /// "Save first" from the unsaved-changes prompt: saves whatever `p`
    /// would throw away (both for quit), if it already has a name.
    pub(super) fn save_before(&mut self, p: &Pending) {
        // themes (and quitting): the whole theme, parts included
        if matches!(p, Pending::Quit | Pending::NewTheme | Pending::OpenTheme(_)) && !self.theme_new
        {
            if self.theme_dirty() {
                self.theme_save(false);
            }
            if matches!(p, Pending::Quit) && self.settings_page.draft.is_some() {
                self.settings_save();
            }
            return;
        }
        let palette = matches!(p, Pending::Quit) || !p.is_fonts();
        let fonts = matches!(p, Pending::Quit) || p.is_fonts();
        if palette && self.dirty() && !self.is_new {
            let name = self.name.clone();
            self.save_as(&name, false);
        }
        if fonts && self.fonts.dirty() && !self.fonts.is_new {
            let name = self.fonts.name.clone();
            self.fonts_save_as(&name, false);
        }
        if matches!(p, Pending::Quit) && self.settings_page.draft.is_some() {
            self.settings_save();
        }
    }

    /// The palette list shows the highlighted palette everywhere (the
    /// colour list, the preview windows, the sample); closing it any way
    /// but choosing puts back the palette from before.
    pub(super) fn sync_palette_preview(&mut self) {
        let Some(original) = &self.open_original else {
            return;
        };
        match &self.overlay {
            Overlay::Open {
                filter,
                sel,
                entries,
            } if self.overlay_for == For::Palette => {
                let shown = open_matches(entries, filter)
                    .get(*sel)
                    .and_then(|&i| self.open_palettes.get(i).cloned().flatten());
                self.editor.palette = shown.unwrap_or_else(|| original.clone());
            }
            _ => self.end_palette_preview(),
        }
    }

    /// Back to the palette from before the palette list opened.
    pub(super) fn end_palette_preview(&mut self) {
        if let Some(p) = self.open_original.take() {
            self.editor.palette = p;
        }
        self.open_palettes.clear();
    }

    pub(super) fn fonts_save(&mut self, then_apply: bool) {
        if self.fonts.is_new {
            self.overlay_for = For::Fonts;
            self.overlay = Overlay::SaveAs {
                input: String::new(),
                overwrite: false,
                then_apply,
            };
            return;
        }
        let name = self.fonts.name.clone();
        self.fonts_save_as(&name, then_apply);
    }

    pub(super) fn fonts_save_as(&mut self, name: &str, then_apply: bool) {
        match self.fonts.save_as(name) {
            Ok(()) if then_apply => match self.fonts.store.apply(name, self.fonts.preset.sizes()) {
                Ok(()) => {
                    self.fonts.active = Some(name.to_string());
                    self.notify_saved(format!("Saved and applied \"{name}\""));
                }
                Err(e) => self.messages.error(format!("Apply failed: {e:#}")),
            },
            Ok(()) => self.notify_saved(format!("Saved \"{name}\"")),
            Err(e) => self.messages.error(format!("Save failed: {e:#}")),
        }
    }

    pub(super) fn fonts_open_overlay(&mut self) {
        match self.fonts.store.load_all() {
            Ok(all) => {
                let entries: Vec<(String, Vec<Rgb>)> =
                    all.keys().map(|n| (n.clone(), Vec::new())).collect();
                let sel = entries
                    .iter()
                    .position(|(n, _)| *n == self.fonts.name)
                    .unwrap_or(0);
                self.overlay_for = For::Fonts;
                self.overlay = Overlay::Open {
                    filter: String::new(),
                    sel,
                    entries,
                };
            }
            Err(e) => self.messages.error(format!("{e:#}")),
        }
    }

    /// A save confirmation: in the status line like any message, or in a
    /// box (settings.messages.saved).
    pub fn notify_saved(&mut self, s: impl Into<String>) {
        if self.settings.messages.saved_in_popup() {
            self.messages.popup(s);
        } else {
            self.messages.info(s);
        }
    }
}

/// Indices of entries whose name contains `filter` (case-insensitive).
pub fn open_matches(entries: &[(String, Vec<Rgb>)], filter: &str) -> Vec<usize> {
    let f = filter.to_lowercase();
    entries
        .iter()
        .enumerate()
        .filter(|(_, (n, _))| n.to_lowercase().contains(&f))
        .map(|(i, _)| i)
        .collect()
}
