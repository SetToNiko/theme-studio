//! App's methods for the Behaviour tab's looks and colour rows.
//!
//! Turns look settings into colours, and sets a colour row from the list of
//! palette keys. This is another `impl App { ... }` block (see
//! `theme_actions.rs` for the same style). The tab's own data stays in
//! `settings_page/behaviour.rs`, which also tests these methods (it builds an `App`).

use ratatui::style::{Color, Modifier};

use crate::app::App;
use crate::app::state::hit::{Look, Looks, Underline};
use crate::app::state::nav::{BhRow, LookId, Overlay};
use crate::config::settings::{LookSettings, Settings};
use crate::settings_page::behaviour::theme_keys;

impl App {
    /// A key of the active system palette as a colour (the terminal's own
    /// colours when the palette can't be read).
    pub fn theme_colour(&self, key: &str) -> Color {
        if let Some(p) = &self.system_palette
            && theme_keys().contains(&key)
        {
            let c = p.get(key);
            return Color::Rgb(c.0, c.1, c.2);
        }
        match key {
            "red" | "bg_red" => Color::Red,
            "green" | "bg_green" => Color::Green,
            "yellow" | "bg_yellow" | "orange" => Color::Yellow,
            "blue" => Color::Blue,
            "purple" => Color::Magenta,
            "aqua" => Color::Cyan,
            "fg0" | "fg1" => Color::White,
            k if k.starts_with("grey") => Color::Gray,
            _ => Color::DarkGray,
        }
    }

    /// A look's settings as colours.
    pub fn look(&self, l: &LookSettings) -> Look {
        Look {
            bg: l.highlight.then(|| self.theme_colour(&l.highlight_colour)),
            underline: Underline::parse(&l.underline)
                .map(|u| (u, self.theme_colour(&l.underline_colour))),
            fg: l.text.then(|| self.theme_colour(&l.text_colour)),
            weight: match l.text_weight.as_str() {
                "bold" => Modifier::BOLD,
                "dim" => Modifier::DIM,
                _ => Modifier::empty(),
            },
        }
    }

    pub fn looks(&self, s: &Settings) -> Looks {
        Looks {
            hover: self.look(&s.hover.look),
            derived: self.look(&s.colours.derived_highlight),
            pane: self.theme_colour(&s.hover.pane_border),
            preview: self.theme_colour(&s.hover.preview_border),
        }
    }

    pub(super) fn look_settings(&mut self, id: LookId) -> &mut LookSettings {
        match id {
            LookId::Hover => &mut self.settings.hover.look,
            LookId::Derived => &mut self.settings.colours.derived_highlight,
        }
    }

    /// The key a colour row holds.
    pub(super) fn colour_setting(&mut self, row: BhRow) -> Option<&mut String> {
        Some(match row {
            BhRow::HighlightColour(id) => &mut self.look_settings(id).highlight_colour,
            BhRow::UnderlineColour(id) => &mut self.look_settings(id).underline_colour,
            BhRow::TextColour(id) => &mut self.look_settings(id).text_colour,
            BhRow::PaneBorder => &mut self.settings.hover.pane_border,
            BhRow::PreviewBorder => &mut self.settings.hover.preview_border,
            _ => return None,
        })
    }

    /// Sets a colour row to a palette key (from the key list).
    pub fn set_colour_setting(&mut self, row: BhRow, key: &str) {
        if let Some(v) = self.colour_setting(row) {
            *v = key.to_string();
        }
    }

    // Long function: one match over every key the list reacts to, one arm per key.
    /// Keys while the list of palette keys is open. Moving shows each key on
    /// the setting (in the draft); Enter keeps it, Esc puts `original` back.
    pub fn choose_key_key(
        &mut self,
        row: BhRow,
        mut sel: usize,
        original: String,
        k: ratatui::crossterm::event::KeyEvent,
    ) {
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};
        let keys = theme_keys();
        let last = keys.len() - 1;
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                return self.set_colour_setting(row, &original);
            }
            KeyCode::Enter => {
                self.overlay = Overlay::None;
                return self.set_colour_setting(row, keys[sel]);
            }
            KeyCode::Up | KeyCode::Char('k') if !ctrl => sel = sel.saturating_sub(1),
            KeyCode::Char('p') if ctrl => sel = sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') if !ctrl => sel = (sel + 1).min(last),
            KeyCode::Char('n') if ctrl => sel = (sel + 1).min(last),
            KeyCode::PageUp => sel = sel.saturating_sub(10),
            KeyCode::PageDown => sel = (sel + 10).min(last),
            KeyCode::Home | KeyCode::Char('g') => sel = 0,
            KeyCode::End | KeyCode::Char('G') => sel = last,
            _ => {}
        }
        // still open: preview the highlighted key
        self.set_colour_setting(row, keys[sel]);
        self.overlay = Overlay::ChooseKey { row, sel, original };
    }
}
