//! The palette being edited: colours, undo history, and the colour picker's numbers.
//!
//! [`PaletteEditor`] knows nothing about the rest of the studio (pages,
//! messages, the preview). It works on one palette and gets the selected key
//! as a parameter. [`App`](super::App) calls it and then does the parts that
//! need the rest of the studio, such as showing a message.

use crate::color::{self, Rgb};
use crate::palette::Palette;

/// The most undo steps we keep.
const MAX_UNDO: usize = 500;

/// The palette that is being edited, with its undo history and picker values.
pub struct PaletteEditor {
    /// The palette as it is now, including unsaved changes.
    pub palette: Palette,
    /// The palette as last loaded or saved, for "unsaved" and the old swatch.
    pub saved: Palette,
    undo: Vec<Palette>,
    redo: Vec<Palette>,
    /// Key being edited by consecutive picker changes (one undo step for all).
    last_edit: Option<&'static str>,
    /// Picker state: hue (degrees), saturation and value (0..1). Kept as
    /// floats so the hue survives greys and small steps don't get rounded away.
    pub hsv: (f64, f64, f64),
    /// The text in the hex field of the picker.
    pub hex_input: String,
    /// The palette (and undo depth) before the picker's first change, for
    /// Esc to put back. Dropped once the keyboard is back in the list.
    edit_session: Option<(Palette, usize)>,
}

impl PaletteEditor {
    /// Starts editing `palette`; it also counts as the saved palette.
    pub fn new(palette: Palette) -> PaletteEditor {
        PaletteEditor {
            saved: palette.clone(),
            palette,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: None,
            hsv: (0.0, 0.0, 0.0),
            hex_input: String::new(),
            edit_session: None,
        }
    }

    /// Replaces the palette with one just loaded from disk (it is also the saved one).
    pub fn load(&mut self, p: Palette) {
        self.saved = p.clone();
        self.palette = p;
    }

    /// Replaces the palette with a blank one (it is also the saved one).
    pub fn start_blank(&mut self) {
        self.palette = Palette::blank();
        self.saved = self.palette.clone();
    }

    /// Remembers the current palette as the saved one.
    pub fn mark_saved(&mut self) {
        self.saved = self.palette.clone();
    }

    /// Points the picker at the colour of `key`.
    pub fn load_picker(&mut self, key: &'static str) {
        let c = self.palette.get(key);
        let (h, s, v) = color::rgb_to_hsv(c);
        self.hsv = (h.unwrap_or(self.hsv.0), s, v);
        self.hex_input = c.hex();
    }

    /// Ends the run of consecutive changes, so the next change is a new undo step.
    pub fn end_edit_run(&mut self) {
        self.last_edit = None;
    }

    fn push_undo(&mut self) {
        self.undo.push(self.palette.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Sets `key` from the picker. Consecutive changes to one key are a
    /// single undo step. `in_picker` is true when the keyboard is in the picker
    /// (not the list): the first change then starts an edit session.
    pub fn edit(&mut self, key: &'static str, c: Rgb, in_picker: bool) {
        if self.palette.get(key) == c {
            return;
        }
        // the first change while in the picker: remember the palette, so
        // Esc can put it back
        if self.edit_session.is_none() && in_picker {
            self.edit_session = Some((self.palette.clone(), self.undo.len()));
        }
        if self.last_edit != Some(key) {
            self.push_undo();
            self.last_edit = Some(key);
        }
        self.palette.set(key, c);
        self.hex_input = c.hex();
    }

    /// Sets `key` to `c` as its own undo step (paste, copy from another key).
    pub fn set_key(&mut self, key: &'static str, c: Rgb) {
        self.push_undo();
        self.last_edit = None;
        self.palette.set(key, c);
    }

    /// Sets the picker's hue, saturation and value (clamped) and edits `key` to match.
    pub fn set_hsv(&mut self, key: &'static str, h: f64, s: f64, v: f64, in_picker: bool) {
        self.hsv = (h.clamp(0.0, 360.0), s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
        let (h, s, v) = self.hsv;
        self.edit(key, color::hsv_to_rgb(h, s, v), in_picker);
    }

    /// Current saturation and lightness (HSL) of the colour of `key`.
    pub fn hsl(&self, key: &'static str) -> (f64, f64) {
        let (_, s, l) = color::rgb_to_hsl(self.palette.get(key));
        (s, l)
    }

    /// Sets saturation and lightness (HSL, clamped) of `key`, keeping the picker's hue.
    pub fn set_hsl(&mut self, key: &'static str, s: f64, l: f64, in_picker: bool) {
        let c = color::hsl_to_rgb(self.hsv.0, s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
        let (_, sv, v) = color::rgb_to_hsv(c);
        self.hsv.1 = sv;
        self.hsv.2 = v;
        self.edit(key, c, in_picker);
    }

    /// Pins `key` at its current colour, or unpins it so it follows its
    /// sources again. Its own undo step. Returns what the status line says
    /// for the change; `sources` is the text for "follows ... again".
    pub fn toggle_pin(&mut self, key: &'static str, sources: &str) -> String {
        self.push_undo();
        self.last_edit = None;
        let message = if self.palette.is_pinned(key) {
            self.palette.unpin(key);
            format!("{key} follows {sources} again")
        } else {
            let c = self.palette.get(key);
            self.palette.set(key, c);
            format!("{key} pinned")
        };
        self.load_picker(key);
        message
    }

    /// Goes back one undo step. Returns false when there is nothing to undo.
    pub fn undo(&mut self, key: &'static str) -> bool {
        let Some(p) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.palette, p));
        self.last_edit = None;
        self.load_picker(key);
        true
    }

    /// Goes forward one undo step. Returns false when there is nothing to redo.
    pub fn redo(&mut self, key: &'static str) -> bool {
        let Some(p) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.palette, p));
        self.last_edit = None;
        self.load_picker(key);
        true
    }

    /// Forgets all undo and redo steps.
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.last_edit = None;
    }

    /// True while an edit session is open (the picker changed something and
    /// the keyboard has not yet left it).
    pub fn in_edit_session(&self) -> bool {
        self.edit_session.is_some()
    }

    /// Ends the edit session. With `discard`, first puts back the colours from
    /// before the session; `key` is then the selected key. Returns true when
    /// colours were put back.
    pub fn end_session(&mut self, key: &'static str, discard: bool) -> bool {
        let session = self.edit_session.take();
        if discard
            && let Some((before, undo_len)) = session
            && before != self.palette
        {
            self.palette = before;
            self.undo.truncate(undo_len);
            self.redo.clear();
            self.last_edit = None;
            self.load_picker(key);
            return true;
        }
        false
    }
}
