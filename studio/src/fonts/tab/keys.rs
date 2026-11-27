//! Keys and clicks of the Fonts tab: the actions in the roles and in the
//! font list, typing into the search box, and the size of a role.

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::keymap::action::Action;

use crate::fonts::tab::state::FontsTab;
use crate::fonts::types::{FontFocus, Row};

impl FontsTab {
    /// Mouse: a click on one of the three roles.
    pub fn click_role(&mut self, role: usize) {
        if role < 3 {
            self.slot = role;
            self.focus = FontFocus::Slots;
        }
    }

    /// h / l on a role: the theme's size for it. The app saves and applies
    /// it once the keys have been still for a moment (`sized_at`).
    pub fn resize(&mut self, d: i32) {
        let f = &mut self.preset.fonts[self.slot];
        f.size = u32::try_from((i64::from(f.size) + i64::from(d)).clamp(4, 72)).unwrap_or(72);
        self.sized_at = Some(std::time::Instant::now());
    }

    /// Typing into the search box (the keymap doesn't apply while typing).
    pub fn on_search_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Down => self.focus = FontFocus::List,
            KeyCode::Backspace => {
                self.search.pop();
                self.list_sel = 0;
            }
            KeyCode::Char(c) => {
                self.search.push(c);
                self.list_sel = 0;
            }
            _ => {}
        }
    }

    /// Runs an action in the roles or the font list; returns false if it
    /// means nothing there.
    pub fn act(&mut self, a: Action) -> bool {
        match self.focus {
            FontFocus::Search | FontFocus::Sample => false,
            FontFocus::List => self.act_in_list(a),
            FontFocus::Slots => self.act_in_slots(a),
        }
    }

    /// An action in the font list; returns false if it means nothing there.
    // Long function (reason verified, phase 6): one match over the list actions, which share the row count and the cursor.
    fn act_in_list(&mut self, a: Action) -> bool {
        let last = self.rows().len().saturating_sub(1);
        match a {
            Action::Down => self.list_sel = (self.list_sel + 1).min(last),
            Action::Up => self.list_sel = self.list_sel.saturating_sub(1),
            Action::Top => self.list_sel = 0,
            Action::Bottom => self.list_sel = last,
            Action::PageDown => self.list_sel = (self.list_sel + 10).min(last),
            Action::PageUp => self.list_sel = self.list_sel.saturating_sub(10),
            Action::Search => self.focus = FontFocus::Search,
            Action::Right => match self.rows().get(self.list_sel) {
                Some(Row::Group {
                    expanded: false, ..
                }) => self.set_expanded(true),
                // already open: step into it
                Some(Row::Group { expanded: true, .. }) => {
                    self.list_sel = (self.list_sel + 1).min(last);
                }
                _ => {}
            },
            Action::Left => self.go_up(),
            Action::Confirm => self.activate(),
            Action::Back => {
                self.end_pick();
                self.focus = FontFocus::Slots;
            }
            Action::Filter(c) => {
                self.filters.toggle(c);
                self.clamp_sel();
            }
            _ => return false,
        }
        true
    }

    /// An action on the roles; returns false if it means nothing there.
    fn act_in_slots(&mut self, a: Action) -> bool {
        match a {
            Action::Down => self.slot = (self.slot + 1).min(2),
            Action::Up => self.slot = self.slot.saturating_sub(1),
            Action::Right | Action::SizeUp => self.resize(1),
            Action::Left | Action::SizeDown => self.resize(-1),
            Action::RightBig => self.resize(4),
            Action::LeftBig => self.resize(-4),
            Action::Confirm | Action::PickFont => self.start_pick(),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::tab::list::tests::labels;
    use crate::fonts::tab::state::tests::{font, tab_with};

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(
            KeyCode::Char(c),
            ratatui::crossterm::event::KeyModifiers::NONE,
        )
    }

    fn special(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, ratatui::crossterm::event::KeyModifiers::NONE)
    }

    /// A key as the app routes it: typing while searching, otherwise the
    /// Vim keymap's action for the font list or roles.
    fn press(t: &mut FontsTab, k: KeyEvent) {
        use crate::keymap::key::Key;
        use crate::keymap::{context::Context, lookup::Lookup, map::Keymap};
        if t.focus == FontFocus::Search {
            return t.on_search_key(k);
        }
        let pane = if t.focus == FontFocus::Slots {
            Context::FontRoles
        } else {
            Context::FontList
        };
        if let Lookup::Run(a) = Keymap::vim().lookup(
            &[pane, Context::Fonts, Context::Global],
            &[Key::from_event(k)],
        ) {
            t.act(a);
        }
    }

    #[test]
    // Long function (reason verified, phase 6): one scenario in order (open, filter, search, pick) on one tab; each step needs the state of the step before.
    fn grouped_list_opens_filters_searches_and_picks() {
        let mut t = tab_with(vec![
            font("DejaVu Sans", false, false),
            font("DejaVu Sans Mono", true, false),
            font("Fixed", true, false),
            font("JetBrainsMono Nerd Font", true, true),
            font("JetBrainsMono Nerd Font Mono", true, true),
            font("JetBrainsMono Nerd Font Propo", false, true),
            font("Noto Sans CJK JP", false, false),
            font("Noto Sans CJK KR", false, false),
        ]);
        // bitmap and CJK fonts are hidden at first; groups start closed
        assert_eq!(labels(&t), ["▸DejaVu 2", "▸JetBrains Mono 3"]);
        // opening the list shows where the role's font is
        t.open_list();
        assert_eq!(
            labels(&t),
            [
                "▸DejaVu 2",
                "▾JetBrains Mono 3",
                "  JetBrainsMono Nerd Font",
                "  JetBrainsMono Nerd Font Mono",
                "  JetBrainsMono Nerd Font Propo",
            ]
        );
        assert_eq!(t.list_sel, 2);
        // browsing previews the highlighted font without setting it
        press(&mut t, key('j'));
        assert_eq!(t.shown_fonts()[0].family, "JetBrainsMono Nerd Font Mono");
        assert_eq!(t.preset.fonts[0].family, "JetBrainsMono Nerd Font");
        // h goes up to the group, again closes it
        press(&mut t, key('h'));
        assert_eq!(t.list_sel, 1);
        press(&mut t, key('h'));
        assert_eq!(labels(&t), ["▸DejaVu 2", "▸JetBrains Mono 3"]);
        // filters: NF Mono only, then show the hidden kinds
        press(&mut t, key('M'));
        // a group left with one font shows the font itself
        assert_eq!(labels(&t), ["JetBrainsMono Nerd Font Mono"]);
        press(&mut t, key('M'));
        press(&mut t, key('b'));
        press(&mut t, key('c'));
        press(&mut t, key('p'));
        // (only Noto Sans CJK fonts here, so that is the shared name)
        assert_eq!(
            labels(&t),
            [
                "DejaVu Sans",
                "JetBrainsMono Nerd Font Propo",
                "▸Noto Sans CJK 2"
            ]
        );
        press(&mut t, key('p'));
        assert!(labels(&t).contains(&"Fixed".to_string()));
        // l opens a group, l again steps in, Enter uses the font
        t.list_sel = 0;
        press(&mut t, key('l'));
        press(&mut t, key('l'));
        press(&mut t, special(KeyCode::Enter));
        assert_eq!(t.preset.fonts[t.slot].family, "DejaVu Sans");
        assert_eq!(t.focus, FontFocus::Slots);
        // searching shows a flat list of matches
        t.open_list();
        press(&mut t, key('/'));
        for c in "font mono".chars() {
            press(&mut t, key(c));
        }
        assert_eq!(labels(&t), ["JetBrainsMono Nerd Font Mono"]);
        press(&mut t, special(KeyCode::Enter));
        press(&mut t, special(KeyCode::Enter));
        assert_eq!(
            t.preset.fonts[t.slot].family,
            "JetBrainsMono Nerd Font Mono"
        );
        assert!(t.dirty());
    }
}
