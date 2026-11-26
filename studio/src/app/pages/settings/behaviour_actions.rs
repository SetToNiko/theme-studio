//! App's methods for the Behaviour tab's keys: move, change a row's value, and the palette key list.
//!
//! This is another `impl App { ... }` block (see `theme_actions.rs` for the
//! same style). The tab's own data (the row list, which rows are visible,
//! `theme_keys`, `BhRow` helpers) stays in `settings_page/behaviour.rs`, which
//! also tests these methods (it builds an `App`).

use crate::app::App;
use crate::app::state::nav::{BhRow, Capture, Overlay};
use crate::keymap::action::Action;
use crate::model::palette::GROUPS;
use crate::settings_page::behaviour::{selectable_ends, theme_keys, visible_behaviour_rows};
use crate::settings_page::wrap;

impl App {
    /// The cursor is on the first (`up`) or last selectable row.
    pub fn behaviour_at_edge(&self, up: bool) -> bool {
        let (first, last) = selectable_ends(&visible_behaviour_rows(&self.settings_page.bh_closed));
        self.settings_page.bh_sel == if up { first } else { last }
    }

    /// A key on the Behaviour tab: moves the cursor, or changes the row's
    /// value (Left / Right step through it, Enter acts on it).
    pub(crate) fn run_behaviour_tab(&mut self, a: Action) {
        let rows = visible_behaviour_rows(&self.settings_page.bh_closed);
        if !rows[self.settings_page.bh_sel.min(rows.len() - 1)].selectable() {
            self.settings_page.bh_sel = selectable_ends(&rows).0;
        }
        let ok: Vec<bool> = rows.iter().map(|r| r.selectable()).collect();
        if crate::settings_page::move_over(&mut self.settings_page.bh_sel, &ok, a) {
            return;
        }
        let d = match a {
            Action::Left => -1,
            Action::Right | Action::Confirm => 1,
            _ => return,
        };
        let row = rows[self.settings_page.bh_sel];
        if let BhRow::Header(h) = row {
            self.toggle_group_header(h, a);
        } else if row.is_colour() {
            self.step_colour_row(row, a, d);
        } else {
            self.change_row_value(row, d);
        }
    }

    /// A group's title: Enter toggles it, right opens, left closes.
    fn toggle_group_header(&mut self, h: &'static str, a: Action) {
        let closed = &mut self.settings_page.bh_closed;
        if (a == Action::Confirm && !closed.contains(h)) || a == Action::Left {
            closed.insert(h);
        } else {
            closed.remove(h);
        }
    }

    /// A colour row: Enter opens the list of keys; left / right (`d` is
    /// -1 / 1) steps through it.
    fn step_colour_row(&mut self, row: BhRow, a: Action, d: i32) {
        let keys = theme_keys();
        let cur = self.colour_setting(row).cloned().unwrap_or_default();
        let i = keys.iter().position(|k| *k == cur).unwrap_or(0);
        if a == Action::Confirm {
            self.overlay = Overlay::ChooseKey {
                row,
                sel: i,
                original: cur,
            };
        } else {
            self.set_colour_setting(row, keys[wrap(i, d, keys.len())]);
        }
    }

    /// Changes the value of a row that is not a group title or a colour:
    /// sends it to the method of its kind (`d` is -1 for left, 1 for
    /// right or Enter).
    fn change_row_value(&mut self, row: BhRow, d: i32) {
        match row {
            BhRow::Leader => {
                self.overlay = Overlay::Capture {
                    what: Capture::Leader,
                    keys: Vec::new(),
                };
            }
            BhRow::Highlight(_) | BhRow::Underline(_) | BhRow::Text(_) | BhRow::TextWeight(_) => {
                self.change_look_row(row, d);
            }
            BhRow::SaveSizes | BhRow::ClearPreviews | BhRow::ResetSizes => self.press_button(row),
            BhRow::Tabs
            | BhRow::MessageSeconds
            | BhRow::SavedAs
            | BhRow::KeyHints
            | BhRow::MainPreview
            | BhRow::FontsPreview
            | BhRow::InnerTabs => self.change_layout_row(row, d),
            BhRow::GroupOpen(_) | BhRow::PickerAtStart | BhRow::PickerAfterEditing => {
                self.change_palette_row(row, d);
            }
            _ => self.change_fonts_row(row, d),
        }
    }

    /// A look's switches: highlight, underline, text, text weight.
    fn change_look_row(&mut self, row: BhRow, d: i32) {
        match row {
            BhRow::Highlight(id) => {
                let l = self.look_settings(id);
                l.highlight = !l.highlight;
            }
            BhRow::Underline(id) => {
                let l = self.look_settings(id);
                l.underline = cycle(&l.underline, &["off", "single", "double"], d);
            }
            BhRow::Text(id) => {
                let l = self.look_settings(id);
                l.text = !l.text;
            }
            BhRow::TextWeight(id) => {
                let l = self.look_settings(id);
                l.text_weight = cycle(&l.text_weight, &["normal", "bold", "dim"], d);
            }
            _ => {}
        }
    }

    /// The buttons: save or reset the pane sizes, delete the previews.
    fn press_button(&mut self, row: BhRow) {
        match row {
            BhRow::SaveSizes => {
                self.settings.layout.sizes = self.layout.sizes.clone();
                self.messages
                    .info("The current pane sizes become the default (when saved)");
            }
            BhRow::ClearPreviews => {
                let question = match self.prefetch.used {
                    Some(b) => format!(
                        "Delete the preview fonts ({})?",
                        crate::system::prefetch::size(b)
                    ),
                    None => "Delete the preview fonts?".into(),
                };
                self.overlay = Overlay::Ask {
                    question,
                    then: crate::app::state::nav::Deferred::ClearPreviews,
                };
            }
            BhRow::ResetSizes => {
                self.settings.layout.sizes = crate::config::settings::Sizes::default();
                self.messages
                    .info("Default pane sizes back to the shipped ones (applied when saved)");
            }
            _ => {}
        }
    }

    /// The Layout and Messages groups: tabs, key hints, previews, how long
    /// messages stay, where "Saved" shows.
    fn change_layout_row(&mut self, row: BhRow, d: i32) {
        let s = &mut self.settings;
        match row {
            BhRow::Tabs => s.layout.tabs = cycle(&s.layout.tabs, &["bottom", "top"], d),
            BhRow::MessageSeconds => {
                let opts = crate::config::settings::MessageSettings::SECONDS;
                let i = opts
                    .iter()
                    .position(|n| *n == s.messages.seconds)
                    .unwrap_or(2);
                s.messages.seconds = opts[wrap(i, d, opts.len())];
            }
            BhRow::SavedAs => s.messages.saved = cycle(&s.messages.saved, &["bar", "popup"], d),
            BhRow::KeyHints => {
                s.layout.key_hints = cycle(&s.layout.key_hints, &["shown", "hidden"], d);
            }
            BhRow::MainPreview => {
                s.layout.main_preview = cycle(&s.layout.main_preview, &["shown", "hidden"], d);
            }
            BhRow::FontsPreview => {
                s.layout.fonts_preview = cycle(&s.layout.fonts_preview, &["shown", "hidden"], d);
            }
            BhRow::InnerTabs => {
                s.layout.inner_tabs = cycle(&s.layout.inner_tabs, &["top", "bottom"], d);
            }
            _ => {}
        }
    }

    /// The Palette pane group: which groups start open, the picker.
    fn change_palette_row(&mut self, row: BhRow, d: i32) {
        let s = &mut self.settings;
        match row {
            BhRow::GroupOpen(i) => {
                let name = GROUPS[i].0;
                if s.colours
                    .open_groups
                    .iter()
                    .any(|g| g.eq_ignore_ascii_case(name))
                {
                    s.colours
                        .open_groups
                        .retain(|g| !g.eq_ignore_ascii_case(name));
                } else {
                    s.colours.open_groups.push(name.to_string());
                }
            }
            BhRow::PickerAtStart => {
                s.colours.picker_at_start =
                    cycle(&s.colours.picker_at_start, &["hidden", "shown"], d);
            }
            BhRow::PickerAfterEditing => {
                s.colours.picker_after_editing = cycle(
                    &s.colours.picker_after_editing,
                    &["hide-on-esc", "stay-open"],
                    d,
                );
            }
            _ => {}
        }
    }

    // Long function: one match over the Fonts group's rows, one short arm per row.
    /// The Fonts pane group: the sample, the font list's filters, prefetching.
    fn change_fonts_row(&mut self, row: BhRow, d: i32) {
        let s = &mut self.settings;
        match row {
            BhRow::SampleAtStart => {
                s.fonts.sample_at_start = cycle(&s.fonts.sample_at_start, &["shown", "hidden"], d);
            }
            BhRow::ListOpensToCurrent => {
                s.fonts.list_opens_to_current_font = !s.fonts.list_opens_to_current_font;
            }
            BhRow::Prefetch => s.fonts_page.prefetch = !s.fonts_page.prefetch,
            BhRow::PrefetchCategory(i) => {
                let cat = crate::config::settings::PREFETCH_CATEGORIES[i].0;
                let on = s.fonts_page.prefetches(cat);
                let cats = &mut s.fonts_page.prefetch_categories;
                if on {
                    cats.retain(|c| c != cat);
                } else {
                    cats.push(cat.to_string());
                }
            }
            BhRow::HideBitmap => s.font_list.hide_bitmap = !s.font_list.hide_bitmap,
            BhRow::HideSpecial => {
                s.font_list.hide_cjk_emoji_math = !s.font_list.hide_cjk_emoji_math;
            }
            BhRow::Spacing => {
                s.font_list.spacing = cycle(&s.font_list.spacing, &["any", "mono", "propo"], d);
            }
            BhRow::NerdFont => {
                s.font_list.nerd_font = cycle(
                    &s.font_list.nerd_font,
                    &[
                        "any",
                        "nerd-font",
                        "nerd-font-mono",
                        "nerd-font-propo",
                        "no-nerd-font",
                    ],
                    d,
                );
            }
            _ => {}
        }
    }

    /// Keys while the list of palette keys is open. Moving shows each key on
    /// the setting (in the draft); Enter keeps it, Esc puts `original` back.
    pub fn choose_colour_key_press(
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

/// The option after (`d` = 1) or before (`d` = -1) `cur` in `opts`, wrapping
/// round; the first option when `cur` is not in the list.
fn cycle(cur: &str, opts: &[&str], d: i32) -> String {
    let i = opts.iter().position(|o| *o == cur).unwrap_or(0);
    opts[wrap(i, d, opts.len())].to_string()
}
