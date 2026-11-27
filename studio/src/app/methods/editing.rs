//! Colour editing: the picker, the selected key, undo and redo, and pins.

use crate::app::app_state::App;
use crate::app::state::nav::Focus;
use crate::model::color::Rgb;
use crate::model::palette;

impl App {
    /// Points the picker at the selected key's colour.
    pub(in crate::app) fn load_picker(&mut self) {
        self.editor.load_picker(self.key());
    }

    /// Sets the selected key from the picker. Consecutive changes to one key
    /// are a single undo step.
    pub(in crate::app) fn edit(&mut self, c: Rgb) {
        let in_picker = self.focus != Focus::List;
        self.editor.edit(self.key(), c, in_picker);
    }

    /// Sets `key` to `c` as its own undo step (paste, copy from another key).
    pub(in crate::app) fn set_key(&mut self, key: &'static str, c: Rgb) {
        self.editor.set_key(key, c);
        if key == self.key() {
            self.load_picker();
        }
    }

    pub(in crate::app) fn set_hsv(&mut self, h: f64, s: f64, v: f64) {
        let in_picker = self.focus != Focus::List;
        self.editor.set_hsv(self.key(), h, s, v, in_picker);
    }

    pub(in crate::app) fn set_hsl(&mut self, s: f64, l: f64) {
        let in_picker = self.focus != Focus::List;
        self.editor.set_hsl(self.key(), s, l, in_picker);
    }

    pub(in crate::app) fn toggle_pin(&mut self) {
        let key = self.key();
        if palette::is_base(key) {
            self.messages
                .info(format!("{key} is a base colour; it's always set by hand"));
            return;
        }
        let message = self.editor.toggle_pin(key, palette::sources(key));
        self.messages.info(message);
    }

    pub(in crate::app) fn undo(&mut self) {
        if !self.editor.undo(self.key()) {
            self.messages.info("Nothing to undo");
        }
    }

    pub(in crate::app) fn redo(&mut self) {
        if !self.editor.redo(self.key()) {
            self.messages.info("Nothing to redo");
        }
    }
}
