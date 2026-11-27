//! Open, new, and save of the presets of the Fonts tab.

use anyhow::Result;

use crate::fonts::tab::state::FontsTab;
use crate::fonts::types::FontFocus;

impl FontsTab {
    /// Opens a preset: its fonts, at the sizes in use (they're the theme's).
    ///
    /// # Errors
    /// Fails when fonts.json cannot be read, has no preset with that name, or the preset is not
    /// valid. The open preset stays as it was.
    pub fn open(&mut self, name: &str) -> Result<()> {
        let mut p = self.store.load(name)?;
        p.set_sizes(self.preset.sizes());
        self.name = name.to_string();
        self.is_new = false;
        self.saved = p.clone();
        self.preset = p;
        self.focus = FontFocus::Slots;
        Ok(())
    }

    pub fn new_preset(&mut self) {
        self.name = "untitled".to_string();
        self.is_new = true;
        self.saved = self.preset.clone();
        self.focus = FontFocus::Slots;
    }

    /// Saves the preset under `name` and makes `name` the open preset.
    ///
    /// # Errors
    /// Fails when fonts.json cannot be written. The open name stays as it was.
    pub fn save_as(&mut self, name: &str) -> Result<()> {
        self.store.save(name, &self.preset)?;
        self.name = name.to_string();
        self.is_new = false;
        self.saved = self.preset.clone();
        Ok(())
    }
}
