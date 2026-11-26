//! App's methods for themes: open, save, apply, the open list, and sizes.
//!
//! This is another `impl App { ... }` block (see `persist.rs` for the same
//! style). The theme data (`Theme`, `ThemeStore`, reading and writing
//! themes.json and the current theme file) stays in `model/theme.rs`, and so do
//! its tests; this file only holds the methods that need the whole `App`.

use crate::app::App;
use crate::app::state::nav::{For, Overlay};
use crate::model::color::Rgb;
use crate::model::theme::{Theme, ThemeStore};

impl App {
    /// The theme's parts as they are now.
    fn theme_now(&self) -> Theme {
        Theme {
            palette: self.name.clone(),
            fonts: self.fonts.name.clone(),
            sizes: Some(self.fonts.preset.sizes()),
        }
    }

    /// Unsaved: new, a part edited, or another palette / preset swapped in.
    pub fn theme_dirty(&self) -> bool {
        self.theme_new
            || self.dirty()
            || self.fonts.dirty()
            || self.theme_saved.as_ref() != Some(&self.theme_now())
    }

    /// At start: the theme to show. `named` (the command line) or the active
    /// theme is opened if it exists; else the one whose parts are open; the
    /// first start makes a theme from the active palette and font preset.
    pub fn init_theme(&mut self, named: Option<&str>) {
        let all = self.themes.load_all();
        let wanted = match named {
            Some(n) => Some(n.to_string()),
            None => ThemeStore::active_name(),
        };
        if let Some(w) = wanted
            && let Some((n, t)) = all.iter().find(|(n, _)| *n == w)
        {
            let (n, t) = (n.clone(), t.clone());
            self.load_theme_parts(&t);
            self.set_saved_theme(n, t);
            return;
        }
        let now = self.theme_now();
        if let Some((n, t)) = all.iter().find(|(_, t)| t.same_parts(&now)) {
            let (n, t) = (n.clone(), t.clone());
            if let Some(s) = t.sizes {
                self.fonts.preset.set_sizes(s);
            }
            self.set_saved_theme(n, t);
            return;
        }
        self.theme = self.name.clone();
        self.theme_new = true;
        self.theme_saved = None;
        // the first start: the active palette and fonts become a theme
        if !self.themes.exists() && !self.is_new && !self.fonts.is_new {
            self.save_first_theme(now);
        }
    }

    /// `t` (saved as `name`) is now the open theme, with nothing unsaved.
    fn set_saved_theme(&mut self, name: String, t: Theme) {
        self.theme = name;
        self.theme_new = false;
        self.theme_saved = Some(t);
    }

    /// The first start: saves `now` (the active palette and fonts) as a
    /// theme named like the palette, and makes it the active theme.
    fn save_first_theme(&mut self, now: Theme) {
        match self.themes.save(&self.name, &now) {
            Ok(()) => {
                self.theme_new = false;
                self.theme_saved = Some(now);
                let _ = ThemeStore::set_active(&self.theme);
                self.messages.info(format!(
                    "Made the theme \"{}\" from the active palette and fonts",
                    self.theme
                ));
            }
            Err(e) => self.messages.error(format!("themes.json: {e:#}")),
        }
    }

    /// Opens a theme's palette and font preset (messages say what's missing).
    fn load_theme_parts(&mut self, t: &Theme) {
        match self.store.load(&t.palette) {
            Ok(p) => {
                self.name.clone_from(&t.palette);
                self.is_new = false;
                self.editor.load(p);
                self.reset_history();
            }
            Err(e) => self
                .messages
                .error(format!("Palette \"{}\": {e:#}", t.palette)),
        }
        if let Err(e) = self.fonts.open(&t.fonts) {
            self.messages
                .error(format!("Font preset \"{}\": {e:#}", t.fonts));
        }
        if let Some(s) = t.sizes {
            self.fonts.preset.set_sizes(s);
            self.fonts.saved.set_sizes(s);
        }
    }

    /// A role's size changed (h / l) and the keys have been still a moment:
    /// the theme keeps the new sizes and they're applied, like a picked font.
    /// An unsaved theme gets them when it's saved; they're applied now if
    /// its font preset is saved.
    pub fn apply_sizes(&mut self) {
        self.fonts.sized_at = None;
        let sizes = self.fonts.preset.sizes();
        let shown = sizes.map(|s| s.to_string()).join(" · ");
        let mut saved = String::new();
        if !self.theme_new
            && let Some(mut t) = self.themes.get(&self.theme)
        {
            t.sizes = Some(sizes);
            if let Err(e) = self.themes.save(&self.theme, &t) {
                return self.messages.error(format!("Saving the sizes: {e:#}"));
            }
            if let Some(s) = self.theme_saved.as_mut() {
                s.sizes = Some(sizes);
            }
            saved = format!(", saved to \"{}\"", self.theme);
        }
        if self.fonts.is_new {
            return self.messages.info(format!(
                "Sizes {shown}: applied when the font preset is saved"
            ));
        }
        let name = self.fonts.name.clone();
        match self.fonts.store.apply(&name, sizes) {
            Ok(()) => {
                self.fonts.active = Some(name);
                self.messages
                    .info(format!("Sizes {shown}{saved} and applied"));
            }
            Err(e) => self.messages.error(format!("Applying the sizes: {e:#}")),
        }
    }

    pub fn open_theme(&mut self, name: &str) {
        let Some(t) = self.themes.get(name) else {
            return self.messages.error(format!("No theme \"{name}\""));
        };
        self.load_theme_parts(&t);
        self.theme = name.to_string();
        self.theme_new = false;
        self.theme_saved = Some(t);
        self.messages.info(format!("Opened the theme \"{name}\""));
    }

    /// A new theme: a copy of this one, named when saved.
    pub fn new_theme(&mut self) {
        self.theme = "untitled".into();
        self.theme_new = true;
        self.theme_saved = None;
        self.messages
            .info("New theme (a copy of the current one); s asks for a name");
    }

    /// Save (s): asks for a name the first time.
    pub fn theme_save(&mut self, then_apply: bool) {
        if self.theme_new {
            self.overlay_for = For::Theme;
            self.overlay = Overlay::SaveAs {
                input: String::new(),
                overwrite: false,
                then_apply,
            };
            return;
        }
        let name = self.theme.clone();
        self.theme_save_as(&name, then_apply);
    }

    /// Saves the parts that changed, then the theme; with `then_apply`,
    /// applies it. New parts get the theme's name. Saving a saved theme
    /// under another name forks: edited parts are saved under the new name
    /// (other themes using them keep theirs); unedited parts stay shared.
    pub fn theme_save_as(&mut self, name: &str, then_apply: bool) {
        let fork = !self.theme_new && name != self.theme;
        if !self.save_theme_parts(name, fork) {
            return;
        }
        let t = self.theme_now();
        if let Err(e) = self.themes.save(name, &t) {
            return self.messages.error(format!("Saving the theme: {e:#}"));
        }
        self.theme = name.to_string();
        self.theme_new = false;
        self.theme_saved = Some(t);
        if then_apply {
            self.theme_apply_saved();
        } else {
            self.notify_saved(format!("Saved \"{name}\""));
        }
    }

    /// Saves the palette and the font preset when they changed (under
    /// `name` when new or forked, see [`Self::theme_save_as`]). False when a
    /// part could not be saved; its message says why.
    fn save_theme_parts(&mut self, name: &str, fork: bool) -> bool {
        if !self.save_palette_part(name, fork) || !self.save_fonts_part(name, fork) {
            return false;
        }
        // a part's save can have failed (its message says why)
        !(self.dirty() || self.fonts.dirty())
    }

    /// The palette half of [`Self::save_theme_parts`]. False when the name is taken.
    fn save_palette_part(&mut self, name: &str, fork: bool) -> bool {
        if self.is_new || (fork && self.dirty()) {
            let palettes = self.store.load_all().unwrap_or_default();
            if palettes.contains_key(name) && name != self.name {
                self.messages.error(format!(
                    "There's a palette \"{name}\" already: pick another theme name"
                ));
                return false;
            }
            self.save_as(name, false);
        } else if self.dirty() {
            let current = self.name.clone();
            self.save_as(&current, false);
        }
        true
    }

    /// The font preset half of [`Self::save_theme_parts`]. False when the name
    /// is taken or the save failed.
    fn save_fonts_part(&mut self, name: &str, fork: bool) -> bool {
        if !(self.fonts.is_new || self.fonts.dirty()) {
            return true;
        }
        let preset = if self.fonts.is_new || fork {
            name.to_string()
        } else {
            self.fonts.name.clone()
        };
        let presets = self.fonts.store.load_all().unwrap_or_default();
        if preset != self.fonts.name && presets.contains_key(&preset) {
            self.messages.error(format!(
                "There's a font preset \"{preset}\" already: pick another theme name"
            ));
            return false;
        }
        if let Err(e) = self.fonts.save_as(&preset) {
            self.messages.error(format!("Saving the fonts: {e:#}"));
            return false;
        }
        true
    }

    /// Apply (a): saves if needed, then applies fonts and palette (one i3
    /// restart: the palette's) and records the theme as active.
    pub fn theme_apply(&mut self) {
        if self.theme_dirty() {
            self.theme_save(true);
        } else {
            self.theme_apply_saved();
        }
    }

    fn theme_apply_saved(&mut self) {
        let t = self.theme_now();
        let sizes = t.sizes.unwrap_or(crate::fonts::presets::DEFAULT_SIZES);
        if let Err(e) = self.fonts.store.apply_quietly(&t.fonts, sizes) {
            return self.messages.error(format!("Applying the fonts: {e:#}"));
        }
        self.fonts.active = Some(t.fonts.clone());
        if let Err(e) = self.store.apply(&t.palette) {
            return self.messages.error(format!("Applying the palette: {e:#}"));
        }
        self.active = Some(t.palette.clone());
        self.system_palette = Some(self.editor.palette.clone());
        let _ = ThemeStore::set_active(&self.theme);
        self.notify_saved(format!("Saved and applied \"{}\"", self.theme));
    }

    /// Open (o): the list of themes, with their palettes' colours.
    pub fn theme_open_overlay(&mut self) {
        let palettes = self.store.load_all().unwrap_or_default();
        let entries: Vec<(String, Vec<Rgb>)> = self
            .themes
            .load_all()
            .into_iter()
            .map(|(n, t)| {
                let sw = palettes
                    .get(&t.palette)
                    .map(crate::app::input::overlay::swatches)
                    .unwrap_or_default();
                (n, sw)
            })
            .collect();
        if entries.is_empty() {
            return self.messages.info("No themes saved yet (s saves this one)");
        }
        let sel = entries
            .iter()
            .position(|(n, _)| *n == self.theme)
            .unwrap_or(0);
        self.overlay_for = For::Theme;
        self.overlay = Overlay::Open {
            filter: String::new(),
            sel,
            entries,
        };
    }
}
