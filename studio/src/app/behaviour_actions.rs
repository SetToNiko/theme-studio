//! App's methods for the Behaviour tab: looks, colour rows, row values, keys.
//!
//! This is another `impl App { ... }` block (see `theme_actions.rs` for the
//! same style). The tab's own data (the row list, which rows are visible,
//! `theme_keys`, `BhRow` helpers) stays in `behaviour.rs`. The tests of
//! these methods are in the test module of `behaviour.rs` too (it builds an
//! `App`); this file only holds the methods that need the whole `App`.

use ratatui::style::{Color, Modifier};

use super::App;
use super::nav::{BhRow, Capture, LookId, Overlay};
use crate::behaviour::{selectable_ends, theme_keys, visible_behaviour_rows};
use crate::config::{LookSettings, Settings};
use crate::hit::{Look, Looks, Underline};
use crate::keymap::{Action, Key, display_seq};
use crate::palette::GROUPS;
use crate::settings::wrap;

impl App {
    // ----- looks -------------------------------------------------------------

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

    fn look_settings(&mut self, id: LookId) -> &mut LookSettings {
        match id {
            LookId::Hover => &mut self.settings.hover.look,
            LookId::Derived => &mut self.settings.colours.derived_highlight,
        }
    }

    /// The key a colour row holds.
    fn colour_setting(&mut self, row: BhRow) -> Option<&mut String> {
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

    // ----- rows ----------------------------------------------------------------

    // Long function: one match over every Behaviour row kind, one short arm per row.
    /// The value shown for a row.
    pub fn behaviour_value(&self, row: BhRow) -> String {
        let s = &self.settings;
        let yes = |b: bool| if b { "yes" } else { "no" }.to_string();
        let look = |id: LookId| match id {
            LookId::Hover => &s.hover.look,
            LookId::Derived => &s.colours.derived_highlight,
        };
        let none = Key::new(
            ratatui::crossterm::event::KeyCode::Null,
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        match row {
            BhRow::Gap | BhRow::Header(_) | BhRow::Sample(_) => String::new(),
            BhRow::Leader => display_seq(&[self.keymap.leader], none),
            BhRow::Tabs => s.layout.tabs.clone(),
            BhRow::InnerTabs => s.layout.inner_tabs.clone(),
            BhRow::KeyHints => s.layout.key_hints.clone(),
            BhRow::MainPreview => s.layout.main_preview.clone(),
            BhRow::FontsPreview => s.layout.fonts_preview.clone(),
            BhRow::SaveSizes | BhRow::ResetSizes => "Enter".into(),
            BhRow::MessageSeconds => match s.messages.seconds {
                0 => "until a key".into(),
                n => format!("{n} s"),
            },
            BhRow::SavedAs => s.messages.saved.clone(),
            BhRow::Highlight(id) => yes(look(id).highlight),
            BhRow::HighlightColour(id) => look(id).highlight_colour.clone(),
            BhRow::Underline(id) => look(id).underline.clone(),
            BhRow::UnderlineColour(id) => look(id).underline_colour.clone(),
            BhRow::Text(id) => yes(look(id).text),
            BhRow::TextColour(id) => look(id).text_colour.clone(),
            BhRow::TextWeight(id) => look(id).text_weight.clone(),
            BhRow::PaneBorder => s.hover.pane_border.clone(),
            BhRow::PreviewBorder => s.hover.preview_border.clone(),
            BhRow::GroupOpen(i) => yes(s
                .colours
                .open_groups
                .iter()
                .any(|g| g.eq_ignore_ascii_case(GROUPS[i].0))),
            BhRow::PickerAtStart => s.colours.picker_at_start.clone(),
            BhRow::PickerAfterEditing => s.colours.picker_after_editing.clone(),
            BhRow::SampleAtStart => s.fonts.sample_at_start.clone(),
            BhRow::ListOpensToCurrent => yes(s.fonts.list_opens_to_current_font),
            BhRow::HideBitmap => yes(s.font_list.hide_bitmap),
            BhRow::HideSpecial => yes(s.font_list.hide_cjk_emoji_math),
            BhRow::Spacing => s.font_list.spacing.clone(),
            BhRow::NerdFont => s.font_list.nerd_font.clone(),
            BhRow::Prefetch => yes(s.fonts_page.prefetch),
            BhRow::PrefetchCategory(i) => self.prefetch_category_value(i),
            // measured on a thread (prefetch.rs)
            BhRow::ClearPreviews => match self.prefetch.used {
                Some(b) => format!("Enter  ({} used)", crate::prefetch::size(b)),
                None => "Enter  (measuring…)".into(),
            },
        }
    }

    /// A prefetch category's value: "yes" or "no", and with the catalogue
    /// loaded, how many fonts it has and about how much space they take.
    fn prefetch_category_value(&self, i: usize) -> String {
        let (cat, _) = crate::config::PREFETCH_CATEGORIES[i];
        let on = if self.settings.fonts_page.prefetches(cat) {
            "yes"
        } else {
            "no"
        }
        .to_string();
        let Some(c) = &self.dl.catalogue else {
            return on;
        };
        let (n, get, keep) = crate::prefetch::estimate(c, cat);
        let fonts = if n == 1 { "font" } else { "fonts" };
        let size = crate::prefetch::size;
        if get == keep {
            format!("{on}  ({n} {fonts}, about {})", size(get))
        } else {
            format!(
                "{on}  ({n} {fonts}: {} to fetch, about {} kept)",
                size(get),
                size(keep)
            )
        }
    }

    // Long function: one match over every Behaviour row kind, one text per row.
    pub fn behaviour_label(row: BhRow) -> String {
        match row {
            BhRow::Gap => String::new(),
            BhRow::Header(h) => h.into(),
            BhRow::Sample(_) => "Sample".into(),
            BhRow::Leader => "Leader key (of the active keymap preset)".into(),
            BhRow::Tabs => "Page tabs (Main, Fonts, Settings)".into(),
            BhRow::InnerTabs => "Tabs inside a page (Keymaps, Behaviour, History)".into(),
            BhRow::KeyHints => "Key hints in the status line".into(),
            BhRow::MainPreview => "Main page preview (nvim, the shell) at start".into(),
            BhRow::FontsPreview => "Fonts page preview at start".into(),
            BhRow::SaveSizes => "Save the current pane sizes as the default".into(),
            BhRow::ResetSizes => "Pane sizes back to the shipped defaults".into(),
            BhRow::MessageSeconds => "Messages stay for (errors: until a key)".into(),
            BhRow::SavedAs => "\"Saved\" shows in (bar: the status line; popup: a box)".into(),
            BhRow::Highlight(_) => "Highlight the background".into(),
            BhRow::HighlightColour(_) => "Highlight colour".into(),
            BhRow::Underline(_) => "Underline".into(),
            BhRow::UnderlineColour(_) => "Underline colour".into(),
            BhRow::Text(_) => "Change the text".into(),
            BhRow::TextColour(_) => "Text colour".into(),
            BhRow::TextWeight(_) => "Text weight".into(),
            BhRow::PaneBorder => "Pane border colour".into(),
            BhRow::PreviewBorder => "Preview window frame colour".into(),
            BhRow::GroupOpen(i) => format!("Group \"{}\" open at start", GROUPS[i].0),
            BhRow::PickerAtStart => "Colour picker at start".into(),
            BhRow::PickerAfterEditing => "Colour picker after editing".into(),
            BhRow::SampleAtStart => "Sample at start".into(),
            BhRow::ListOpensToCurrent => "Installed fonts open at the role's current font".into(),
            BhRow::HideBitmap => "Hide bitmap fonts at start".into(),
            BhRow::HideSpecial => "Hide CJK / emoji / math fonts at start".into(),
            BhRow::Spacing => "Spacing filter at start".into(),
            BhRow::NerdFont => "Nerd Font filter at start".into(),
            BhRow::Prefetch => "Fetch previews ahead of time, a few at a time".into(),
            BhRow::PrefetchCategory(i) => format!("  {}", crate::config::PREFETCH_CATEGORIES[i].1),
            BhRow::ClearPreviews => "Delete the preview fonts (fetched again when needed)".into(),
        }
    }

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
        if crate::settings::move_over(&mut self.settings_page.bh_sel, &ok, a) {
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
                    Some(b) => format!("Delete the preview fonts ({})?", crate::prefetch::size(b)),
                    None => "Delete the preview fonts?".into(),
                };
                self.overlay = Overlay::Ask {
                    question,
                    then: crate::app::nav::Deferred::ClearPreviews,
                };
            }
            BhRow::ResetSizes => {
                self.settings.layout.sizes = crate::config::Sizes::default();
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
                let opts = crate::config::MessageSettings::SECONDS;
                let i = opts
                    .iter()
                    .position(|n| *n == s.messages.seconds)
                    .unwrap_or(2);
                s.messages.seconds = opts[wrap(i, d, opts.len())];
            }
            BhRow::SavedAs => s.messages.saved = cycle(&s.messages.saved, &["bar", "popup"], d),
            BhRow::KeyHints => {
                s.layout.key_hints = cycle(&s.layout.key_hints, &["shown", "hidden"], d)
            }
            BhRow::MainPreview => {
                s.layout.main_preview = cycle(&s.layout.main_preview, &["shown", "hidden"], d)
            }
            BhRow::FontsPreview => {
                s.layout.fonts_preview = cycle(&s.layout.fonts_preview, &["shown", "hidden"], d)
            }
            BhRow::InnerTabs => {
                s.layout.inner_tabs = cycle(&s.layout.inner_tabs, &["top", "bottom"], d)
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
                    cycle(&s.colours.picker_at_start, &["hidden", "shown"], d)
            }
            BhRow::PickerAfterEditing => {
                s.colours.picker_after_editing = cycle(
                    &s.colours.picker_after_editing,
                    &["hide-on-esc", "stay-open"],
                    d,
                )
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
                s.fonts.sample_at_start = cycle(&s.fonts.sample_at_start, &["shown", "hidden"], d)
            }
            BhRow::ListOpensToCurrent => {
                s.fonts.list_opens_to_current_font = !s.fonts.list_opens_to_current_font
            }
            BhRow::Prefetch => s.fonts_page.prefetch = !s.fonts_page.prefetch,
            BhRow::PrefetchCategory(i) => {
                let cat = crate::config::PREFETCH_CATEGORIES[i].0;
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
                s.font_list.hide_cjk_emoji_math = !s.font_list.hide_cjk_emoji_math
            }
            BhRow::Spacing => {
                s.font_list.spacing = cycle(&s.font_list.spacing, &["any", "mono", "propo"], d)
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
                )
            }
            _ => {}
        }
    }
}

/// The option after (`d` = 1) or before (`d` = -1) `cur` in `opts`, wrapping
/// round; the first option when `cur` is not in the list.
fn cycle(cur: &str, opts: &[&str], d: i32) -> String {
    let i = opts.iter().position(|o| *o == cur).unwrap_or(0);
    opts[wrap(i, d, opts.len())].to_string()
}
