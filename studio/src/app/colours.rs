//! App's methods for the colour list rows and the text input of the colour picker.
//!
//! This is a second `impl App { ... }` block. Rust allows an `impl` of a type
//! in another file of the same crate. `app/mod.rs` has the other `App`
//! methods and the struct itself.
//!
//! The colour list shows a row per group and a row per colour of the open
//! groups ([`ColourRow`]). This file moves the list cursor, selects colours,
//! and handles the keys and pasted text that type a hex colour.

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::{App, ColourRow, Focus, Overlay, Side};
use crate::color::Rgb;
use crate::keymap::Action;
use crate::palette::GROUPS;

/// The order of Tab and Shift+Tab through the picker controls.
const FOCUS_ORDER: [Focus; 7] = [
    Focus::List,
    Focus::Square,
    Focus::Hue,
    Focus::SliderH,
    Focus::SliderS,
    Focus::SliderL,
    Focus::Hex,
];

impl App {
    /// Selects a colour without touching the list cursor.
    pub(super) fn select_key(&mut self, i: usize) {
        if i < self.keys.len() && i != self.sel {
            self.sel = i;
            self.editor.end_edit_run();
            self.load_picker();
        }
    }

    /// Selects a colour, opening its group and moving the list cursor to it.
    pub(super) fn select(&mut self, i: usize) {
        if i < self.keys.len() {
            self.select_key(i);
            let g = self.group_of(i);
            self.expanded_groups.insert(g);
            self.sync_row();
        }
    }

    /// The colour list as shown: a row per group, and the colours of open groups.
    pub fn colour_rows(&self) -> Vec<ColourRow> {
        let mut out = Vec::new();
        let mut key = 0;
        for (index, (_, keys)) in GROUPS.iter().enumerate() {
            let expanded = self.expanded_groups.contains(&index);
            out.push(ColourRow::Group { index, expanded });
            for _ in keys.iter() {
                if expanded {
                    out.push(ColourRow::Key(key));
                }
                key += 1;
            }
        }
        out
    }

    /// The index of the group that holds colour `key`.
    pub(super) fn group_of(&self, key: usize) -> usize {
        let mut start = 0;
        for (gi, (_, keys)) in GROUPS.iter().enumerate() {
            if key < start + keys.len() {
                return gi;
            }
            start += keys.len();
        }
        0
    }

    /// Moves the list cursor; landing on a colour selects it.
    pub(super) fn move_row(&mut self, d: i32) {
        let rows = self.colour_rows();
        self.row_sel = (self.row_sel as i32 + d).clamp(0, rows.len() as i32 - 1) as usize;
        if let ColourRow::Key(k) = rows[self.row_sel] {
            self.select_key(k);
        }
    }

    /// Puts the list cursor on the selected colour if its group is open, else
    /// on its group.
    pub(super) fn sync_row(&mut self) {
        let rows = self.colour_rows();
        let g = self.group_of(self.sel);
        self.row_sel = rows
            .iter()
            .position(|r| *r == ColourRow::Key(self.sel))
            .or_else(|| {
                rows.iter()
                    .position(|r| matches!(r, ColourRow::Group { index, .. } if *index == g))
            })
            .unwrap_or(0);
    }

    /// Opens or closes group `g` in the colour list.
    pub(super) fn set_group_open(&mut self, g: usize, open: bool) {
        if open {
            self.expanded_groups.insert(g);
        } else {
            self.expanded_groups.remove(&g);
        }
    }

    /// Keys while the palette menu above the list has the focus.
    fn palette_menu_action(&mut self, a: Action) {
        match a {
            Action::Down | Action::Back => self.layout.palette_menu_focused = false,
            Action::Confirm | Action::Right => self.open_overlay(),
            Action::Up => {
                self.focus_page_tabs(true);
            }
            _ => {}
        }
    }

    /// Up on the first row moves the focus to the palette menu. Down on the
    /// last row (picker hidden) moves it to the page tabs. True if the focus
    /// left the list, so the key is used up.
    fn leaves_list(&mut self, a: Action, last: i32) -> bool {
        match a {
            Action::Up if self.row_sel == 0 => {
                self.layout.palette_menu_focused = true;
                true
            }
            Action::Down
                if self.row_sel as i32 >= last
                    && !self.layout.picker_visible
                    && self.focus_page_tabs(false) =>
            {
                true
            }
            _ => false,
        }
    }

    /// Colour-list keys: open/close groups, move, start editing.
    // Long function: one match over every (action, row kind) pair of the colour list.
    pub(super) fn list_action(&mut self, a: Action) {
        let rows = self.colour_rows();
        let last = rows.len() as i32 - 1;
        let copying = matches!(self.overlay, Overlay::CopyFrom { .. });
        if self.layout.palette_menu_focused && !copying {
            self.palette_menu_action(a);
            return;
        }
        if !copying && self.leaves_list(a, last) {
            return;
        }
        match (a, rows.get(self.row_sel).copied()) {
            (Action::Up, _) => self.move_row(-1),
            (Action::Down, _) => self.move_row(1),
            (Action::Top, _) => self.move_row(-(self.row_sel as i32)),
            (Action::Bottom, _) => self.move_row(last - self.row_sel as i32),
            (Action::PageDown, _) => self.move_row(10),
            (Action::PageUp, _) => self.move_row(-10),
            (Action::Confirm, Some(ColourRow::Group { index, expanded })) => {
                self.set_group_open(index, !expanded)
            }
            (
                Action::Right,
                Some(ColourRow::Group {
                    index,
                    expanded: false,
                }),
            ) => self.set_group_open(index, true),
            (Action::Right, Some(ColourRow::Group { expanded: true, .. })) => self.move_row(1),
            (
                Action::Left,
                Some(ColourRow::Group {
                    index,
                    expanded: true,
                }),
            ) => self.set_group_open(index, false),
            (Action::Left, Some(ColourRow::Key(k))) => {
                let g = self.group_of(k);
                self.row_sel = rows
                    .iter()
                    .position(|r| matches!(r, ColourRow::Group { index, .. } if *index == g))
                    .unwrap_or(0);
            }
            (Action::Right | Action::Confirm, Some(ColourRow::Key(_))) => self.open_picker(),
            _ => {}
        }
    }

    /// Keys while the hex field has the focus: type, Enter applies, Esc restores.
    // Long function: one match over the 6 key kinds of the hex field (about 39 lines).
    pub(super) fn on_hex_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Enter => match Rgb::parse(&self.editor.hex_input) {
                Some(c) => {
                    self.editor.end_edit_run();
                    self.edit(c);
                    self.load_picker();
                    self.leave_picker(false);
                }
                None => self.messages.error(format!(
                    "Not a colour: {} (use #rrggbb)",
                    self.editor.hex_input
                )),
            },
            KeyCode::Esc => {
                self.load_picker();
                self.leave_picker(true);
            }
            KeyCode::Tab => self.cycle_focus(1),
            KeyCode::BackTab => self.cycle_focus(-1),
            KeyCode::Backspace => {
                self.editor.hex_input.pop();
            }
            KeyCode::Char(c) if c.is_ascii_hexdigit() || c == '#' => {
                if c == '#' {
                    self.editor.hex_input.clear();
                }
                if self.editor.hex_input.len() < 7 {
                    if self.editor.hex_input.is_empty() && c != '#' {
                        self.editor.hex_input.push('#');
                    }
                    self.editor.hex_input.push(c.to_ascii_lowercase());
                }
            }
            _ => {}
        }
    }

    /// Bracketed paste (Ctrl+Shift+V): a hex colour sets the selected key.
    pub(super) fn on_paste(&mut self, s: &str) {
        if self.side == Side::Fonts && matches!(self.overlay, Overlay::None) {
            if self.fonts.focus == crate::fonts::FontFocus::Search {
                self.fonts.search.push_str(s.trim());
                self.fonts.list_sel = 0;
            }
            return;
        }
        if let Some(id) = self.preview.focus {
            return self.preview.send_paste(id, s);
        }
        match (&mut self.overlay, Rgb::parse(s)) {
            (Overlay::SaveAs { input, .. }, _) => input.push_str(s.trim()),
            (Overlay::Open { filter, sel, .. }, _) => {
                filter.push_str(s.trim());
                *sel = 0;
            }
            (Overlay::None, Some(c)) => {
                let key = self.key();
                self.set_key(key, c);
                self.messages.info(format!("{key} = {}", c.hex()));
            }
            (Overlay::None, None) => self
                .messages
                .error(format!("Pasted text isn't a colour: {}", s.trim())),
            _ => {}
        }
    }

    /// Moves the focus `d` steps through the picker controls (wraps around).
    pub(super) fn cycle_focus(&mut self, d: i32) {
        let i = FOCUS_ORDER
            .iter()
            .position(|f| *f == self.focus)
            .unwrap_or(0) as i32;
        let n = FOCUS_ORDER.len() as i32;
        self.focus = FOCUS_ORDER[(i + d).rem_euclid(n) as usize];
        if self.focus != Focus::List {
            self.layout.picker_visible = true;
        }
        if self.focus == Focus::Hex {
            self.editor.hex_input = "#".into(); // typing replaces the colour; Esc restores it
        }
    }
}
