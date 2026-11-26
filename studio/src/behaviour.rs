//! The Settings page's Behaviour tab: settings.json's values, in groups
//! (Keys, Layout, Messages, Hover, Palette pane, Derived colours, Fonts pane).

use ratatui::style::{Color, Modifier};

use crate::app::App;
use crate::app::nav::Overlay;
use crate::config::{LookSettings, Settings};
use crate::hit::{Look, Looks, Underline};
use crate::keymap::{Action, Key, display_seq};
use crate::palette::GROUPS;
use crate::settings::{Capture, wrap};

/// Which highlight look a row sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookId {
    /// Clickable things under the mouse.
    Hover,
    /// The Palette pane's derivation relatives of the selected colour.
    Derived,
}

/// A row of the Behaviour tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BhRow {
    /// A blank line before a group (not selectable).
    Gap,
    /// A group's title (not selectable).
    Header(&'static str),
    /// A look as it's set (not selectable).
    Sample(LookId),
    Leader,
    Tabs,
    InnerTabs,
    KeyHints,
    MainPreview,
    FontsPreview,
    /// How long messages stay in the status line.
    MessageSeconds,
    /// Save confirmations: status line or a box.
    SavedAs,
    SaveSizes,
    ResetSizes,
    Highlight(LookId),
    HighlightColour(LookId),
    Underline(LookId),
    UnderlineColour(LookId),
    Text(LookId),
    TextColour(LookId),
    TextWeight(LookId),
    PaneBorder,
    PreviewBorder,
    GroupOpen(usize),
    PickerAtStart,
    PickerAfterEditing,
    SampleAtStart,
    ListOpensToCurrent,
    HideBitmap,
    HideSpecial,
    Spacing,
    NerdFont,
    /// Fetch Fonts page previews ahead of time.
    Prefetch,
    /// Whether a category (config::PREFETCH_CATEGORIES) is prefetched.
    PrefetchCategory(usize),
    /// Delete the preview fonts.
    ClearPreviews,
}

impl BhRow {
    /// Rows whose value is a key of the system palette.
    pub fn is_colour(self) -> bool {
        matches!(
            self,
            BhRow::HighlightColour(_)
                | BhRow::UnderlineColour(_)
                | BhRow::TextColour(_)
                | BhRow::PaneBorder
                | BhRow::PreviewBorder
        )
    }

    pub fn selectable(self) -> bool {
        !matches!(self, BhRow::Gap | BhRow::Sample(_))
    }

    /// A button rather than a value.
    pub fn is_button(self) -> bool {
        matches!(
            self,
            BhRow::SaveSizes | BhRow::ResetSizes | BhRow::ClearPreviews
        )
    }
}

fn look_rows(id: LookId) -> [BhRow; 7] {
    [
        BhRow::Highlight(id),
        BhRow::HighlightColour(id),
        BhRow::Underline(id),
        BhRow::UnderlineColour(id),
        BhRow::Text(id),
        BhRow::TextColour(id),
        BhRow::TextWeight(id),
    ]
}

pub fn behaviour_rows() -> Vec<BhRow> {
    use BhRow as R;
    let mut v = vec![R::Header("Keys"), R::Leader];
    v.extend([
        R::Header("Layout"),
        R::Tabs,
        R::InnerTabs,
        R::KeyHints,
        R::MainPreview,
        R::FontsPreview,
        R::SaveSizes,
        R::ResetSizes,
    ]);
    v.extend([R::Header("Messages"), R::MessageSeconds, R::SavedAs]);
    v.push(R::Header("Hover (everything clickable)"));
    v.extend(look_rows(LookId::Hover));
    v.extend([R::PaneBorder, R::PreviewBorder, R::Sample(LookId::Hover)]);
    v.push(R::Header("Palette pane"));
    v.extend((0..GROUPS.len()).map(R::GroupOpen));
    v.extend([R::PickerAtStart, R::PickerAfterEditing]);
    v.push(R::Header(
        "Derived colours (Palette pane: what the selected colour comes from or feeds)",
    ));
    v.extend(look_rows(LookId::Derived));
    v.push(R::Sample(LookId::Derived));
    v.push(R::Header("Fonts pane"));
    v.extend([
        R::SampleAtStart,
        R::ListOpensToCurrent,
        R::HideBitmap,
        R::HideSpecial,
        R::Spacing,
        R::NerdFont,
    ]);
    v.push(R::Header("Fonts page previews (fetched ahead of time)"));
    v.push(R::Prefetch);
    v.extend((0..crate::config::PREFETCH_CATEGORIES.len()).map(R::PrefetchCategory));
    v.push(R::ClearPreviews);
    // a blank line before each group's title but the first
    let mut out = Vec::new();
    for r in v {
        if matches!(r, R::Header(_)) && !out.is_empty() {
            out.push(R::Gap);
        }
        out.push(r);
    }
    out
}

/// The rows shown: a closed group keeps only its title.
pub fn visible_behaviour_rows(closed: &std::collections::HashSet<&'static str>) -> Vec<BhRow> {
    let mut shut = false;
    behaviour_rows()
        .into_iter()
        .filter(|r| match r {
            BhRow::Header(h) => {
                shut = closed.contains(h);
                true
            }
            BhRow::Gap => true,
            _ => !shut,
        })
        .collect()
}

/// The first and last selectable rows.
pub fn selectable_ends(rows: &[BhRow]) -> (usize, usize) {
    let first = rows.iter().position(|r| r.selectable()).unwrap_or(0);
    let last = rows.iter().rposition(|r| r.selectable()).unwrap_or(0);
    (first, last)
}

/// Every palette key, in the colour list's order (the choices for colours).
pub fn theme_keys() -> Vec<&'static str> {
    GROUPS
        .iter()
        .flat_map(|(_, keys)| keys.iter().copied())
        .collect()
}

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
            BhRow::PrefetchCategory(i) => {
                let (cat, _) = crate::config::PREFETCH_CATEGORIES[i];
                let on = yes(s.fonts_page.prefetches(cat));
                match &self.dl.catalogue {
                    Some(c) => {
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
                    None => on,
                }
            }
            // measured on a thread (prefetch.rs)
            BhRow::ClearPreviews => match self.prefetch.used {
                Some(b) => format!("Enter  ({} used)", crate::prefetch::size(b)),
                None => "Enter  (measuring…)".into(),
            },
        }
    }

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
        let cycle = |cur: &str, opts: &[&str]| -> String {
            let i = opts.iter().position(|o| *o == cur).unwrap_or(0);
            opts[wrap(i, d, opts.len())].to_string()
        };
        let row = rows[self.settings_page.bh_sel];
        // a group's title: Enter toggles it, right opens, left closes
        if let BhRow::Header(h) = row {
            let closed = &mut self.settings_page.bh_closed;
            if (a == Action::Confirm && !closed.contains(h)) || a == Action::Left {
                closed.insert(h);
            } else {
                closed.remove(h);
            }
            return;
        }
        // colours: Enter opens the list of keys; left/right steps through it
        if row.is_colour() {
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
            return;
        }
        match row {
            BhRow::Leader => {
                self.overlay = Overlay::Capture {
                    what: Capture::Leader,
                    keys: Vec::new(),
                };
            }
            BhRow::Highlight(id) => {
                let l = self.look_settings(id);
                l.highlight = !l.highlight;
            }
            BhRow::Underline(id) => {
                let l = self.look_settings(id);
                l.underline = cycle(&l.underline, &["off", "single", "double"]);
            }
            BhRow::Text(id) => {
                let l = self.look_settings(id);
                l.text = !l.text;
            }
            BhRow::TextWeight(id) => {
                let l = self.look_settings(id);
                l.text_weight = cycle(&l.text_weight, &["normal", "bold", "dim"]);
            }
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
                    then: crate::settings::Deferred::ClearPreviews,
                };
            }
            BhRow::ResetSizes => {
                self.settings.layout.sizes = crate::config::Sizes::default();
                self.messages
                    .info("Default pane sizes back to the shipped ones (applied when saved)");
            }
            _ => {
                let s = &mut self.settings;
                match row {
                    BhRow::Tabs => s.layout.tabs = cycle(&s.layout.tabs, &["bottom", "top"]),
                    BhRow::MessageSeconds => {
                        let opts = crate::config::MessageSettings::SECONDS;
                        let i = opts
                            .iter()
                            .position(|n| *n == s.messages.seconds)
                            .unwrap_or(2);
                        s.messages.seconds = opts[wrap(i, d, opts.len())];
                    }
                    BhRow::SavedAs => {
                        s.messages.saved = cycle(&s.messages.saved, &["bar", "popup"])
                    }
                    BhRow::KeyHints => {
                        s.layout.key_hints = cycle(&s.layout.key_hints, &["shown", "hidden"])
                    }
                    BhRow::MainPreview => {
                        s.layout.main_preview = cycle(&s.layout.main_preview, &["shown", "hidden"])
                    }
                    BhRow::FontsPreview => {
                        s.layout.fonts_preview =
                            cycle(&s.layout.fonts_preview, &["shown", "hidden"])
                    }
                    BhRow::InnerTabs => {
                        s.layout.inner_tabs = cycle(&s.layout.inner_tabs, &["top", "bottom"])
                    }
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
                            cycle(&s.colours.picker_at_start, &["hidden", "shown"])
                    }
                    BhRow::PickerAfterEditing => {
                        s.colours.picker_after_editing = cycle(
                            &s.colours.picker_after_editing,
                            &["hide-on-esc", "stay-open"],
                        )
                    }
                    BhRow::SampleAtStart => {
                        s.fonts.sample_at_start =
                            cycle(&s.fonts.sample_at_start, &["shown", "hidden"])
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
                        s.font_list.spacing = cycle(&s.font_list.spacing, &["any", "mono", "propo"])
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
                        )
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Start;
    use crate::color::Rgb;
    use crate::keymap::Keymap;
    use crate::palette::Palette;
    use crate::store::{Store, data_file, to_jq_string};
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use serde_json::{Map, Value};

    /// A store in a temp folder with one palette "One".
    fn temp_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!(
            "theme-studio-behaviour-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut m = Map::new();
        m.insert("One".into(), Value::Object(Palette::blank().to_json()));
        std::fs::write(
            dir.join("palettes.json"),
            to_jq_string(&data_file("palettes", m)),
        )
        .unwrap();
        Store::at(dir)
    }

    /// An app with the shipped defaults, in a temp folder (never ~/.config).
    fn test_app(tag: &str) -> App {
        let store = temp_store(tag);
        // settings_save (draft tests) writes history; keep it in the temp folder
        crate::history::init_for_test(store.dir.join("history"));
        let config = crate::config::Loaded {
            keymap: Keymap::vim(),
            presets: Keymap::builtin(),
            settings: Settings::default(),
            warnings: Vec::new(),
            dir: store.dir.join("studio-config"),
        };
        let mut app = App::with_config(store, Start::Named("One".into()), config).unwrap();
        app.preview.set_headless();
        app
    }

    /// Puts the Behaviour tab's cursor on `row` (as drawn, groups open).
    fn select(app: &mut App, row: BhRow) {
        let rows = visible_behaviour_rows(&app.settings_page.bh_closed);
        app.settings_page.bh_sel = rows.iter().position(|r| *r == row).unwrap();
    }

    /// One key on the Behaviour tab, as the Settings page runs it: an edit
    /// goes to the draft.
    fn run(app: &mut App, row: BhRow, a: Action) {
        select(app, row);
        app.with_draft(|x| x.run_behaviour_tab(a));
    }

    /// A row's value as the page shows it (the draft included).
    fn value(app: &mut App, row: BhRow) -> String {
        app.show_draft(|x| x.behaviour_value(row))
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn rows_have_a_blank_line_before_each_group_but_the_first() {
        let rows = behaviour_rows();
        assert_eq!(rows[0], BhRow::Header("Keys"));
        for (i, r) in rows.iter().enumerate() {
            if matches!(r, BhRow::Header(_)) && i > 0 {
                assert_eq!(rows[i - 1], BhRow::Gap, "row {i}");
            }
        }
        // one row for each palette group and each prefetch category
        for i in 0..GROUPS.len() {
            assert!(rows.contains(&BhRow::GroupOpen(i)));
        }
        for i in 0..crate::config::PREFETCH_CATEGORIES.len() {
            assert!(rows.contains(&BhRow::PrefetchCategory(i)));
        }
    }

    #[test]
    fn closed_group_keeps_only_its_title_and_gaps() {
        let mut closed = std::collections::HashSet::new();
        assert_eq!(visible_behaviour_rows(&closed), behaviour_rows());
        closed.insert("Layout");
        let rows = visible_behaviour_rows(&closed);
        assert!(rows.contains(&BhRow::Header("Layout")));
        assert!(!rows.contains(&BhRow::Tabs));
        assert!(!rows.contains(&BhRow::ResetSizes));
        // the next group is still there
        assert!(rows.contains(&BhRow::MessageSeconds));
    }

    #[test]
    fn selectable_rows_skip_gaps_and_samples_but_not_titles() {
        assert!(!BhRow::Gap.selectable());
        assert!(!BhRow::Sample(LookId::Hover).selectable());
        // a title is selectable: Enter opens and closes the group
        assert!(BhRow::Header("Keys").selectable());
        assert!(BhRow::Leader.selectable());
        let rows = behaviour_rows();
        let (first, last) = selectable_ends(&rows);
        assert_eq!(rows[first], BhRow::Header("Keys"));
        assert_eq!(rows[last], BhRow::ClearPreviews);
        assert_eq!(selectable_ends(&[]), (0, 0));
    }

    #[test]
    fn row_kinds_colour_and_button() {
        assert!(BhRow::PaneBorder.is_colour());
        assert!(BhRow::TextColour(LookId::Derived).is_colour());
        assert!(!BhRow::Text(LookId::Derived).is_colour());
        assert!(BhRow::SaveSizes.is_button());
        assert!(BhRow::ClearPreviews.is_button());
        assert!(!BhRow::Prefetch.is_button());
    }

    #[test]
    fn theme_keys_list_every_group_key_in_order() {
        let keys = theme_keys();
        let total: usize = GROUPS.iter().map(|(_, k)| k.len()).sum();
        assert_eq!(keys.len(), total);
        assert_eq!(keys[0], "bg0");
    }

    #[test]
    fn theme_colour_uses_terminal_colours_without_a_system_palette() {
        let mut app = test_app("colour-fallback");
        app.system_palette = None;
        assert_eq!(app.theme_colour("red"), Color::Red);
        assert_eq!(app.theme_colour("bg_red"), Color::Red);
        assert_eq!(app.theme_colour("orange"), Color::Yellow);
        assert_eq!(app.theme_colour("purple"), Color::Magenta);
        assert_eq!(app.theme_colour("aqua"), Color::Cyan);
        assert_eq!(app.theme_colour("fg1"), Color::White);
        assert_eq!(app.theme_colour("grey2"), Color::Gray);
        assert_eq!(app.theme_colour("bg0"), Color::DarkGray);
        assert_eq!(app.theme_colour("nonsense"), Color::DarkGray);
    }

    #[test]
    fn theme_colour_reads_the_system_palette_for_known_keys_only() {
        let mut app = test_app("colour-palette");
        let mut p = Palette::blank();
        p.set("red", Rgb(1, 2, 3));
        app.system_palette = Some(p);
        assert_eq!(app.theme_colour("red"), Color::Rgb(1, 2, 3));
        // a key that is not in the palette's key list falls back
        assert_eq!(app.theme_colour("nonsense"), Color::DarkGray);
    }

    #[test]
    fn look_turns_settings_into_colours_and_weight() {
        let mut app = test_app("look");
        app.system_palette = None;
        let mut l = LookSettings::default();
        // defaults: no background, single underline, yellow text, normal
        let look = app.look(&l);
        assert_eq!(look.bg, None);
        assert_eq!(look.underline, Some((Underline::Single, Color::Yellow)));
        assert_eq!(look.fg, Some(Color::Yellow));
        assert_eq!(look.weight, Modifier::empty());
        l.highlight = true;
        l.underline = "off".into();
        l.text = false;
        l.text_weight = "bold".into();
        let look = app.look(&l);
        assert_eq!(look.bg, Some(Color::DarkGray)); // bg3
        assert_eq!(look.underline, None);
        assert_eq!(look.fg, None);
        assert_eq!(look.weight, Modifier::BOLD);
        l.text_weight = "dim".into();
        assert_eq!(app.look(&l).weight, Modifier::DIM);
        // an unknown weight is plain
        l.text_weight = "heavy".into();
        assert_eq!(app.look(&l).weight, Modifier::empty());
    }

    #[test]
    fn looks_collects_hover_derived_and_border_colours() {
        let mut app = test_app("looks");
        app.system_palette = None;
        let s = app.settings.clone();
        let looks = app.looks(&s);
        assert_eq!(looks.hover, app.look(&s.hover.look));
        assert_eq!(looks.derived, app.look(&s.colours.derived_highlight));
        assert_eq!(looks.pane, app.theme_colour(&s.hover.pane_border));
        assert_eq!(looks.preview, app.theme_colour(&s.hover.preview_border));
    }

    #[test]
    fn set_colour_setting_changes_colour_rows_and_ignores_others() {
        let mut app = test_app("set-colour");
        app.set_colour_setting(BhRow::PaneBorder, "aqua");
        assert_eq!(app.settings.hover.pane_border, "aqua");
        app.set_colour_setting(BhRow::TextColour(LookId::Derived), "blue");
        assert_eq!(app.settings.colours.derived_highlight.text_colour, "blue");
        let before = app.settings.clone();
        app.set_colour_setting(BhRow::Tabs, "aqua");
        assert_eq!(app.settings.layout.tabs, before.layout.tabs);
    }

    #[test]
    fn choose_key_moves_preview_the_key_and_enter_keeps_it() {
        let mut app = test_app("choose-enter");
        let keys = theme_keys();
        let row = BhRow::PaneBorder;
        let original = app.settings.hover.pane_border.clone();
        app.choose_key_key(row, 0, original.clone(), key(KeyCode::Down));
        assert_eq!(app.settings.hover.pane_border, keys[1]);
        assert!(matches!(app.overlay, Overlay::ChooseKey { sel: 1, .. }));
        app.choose_key_key(row, 1, original, key(KeyCode::Enter));
        assert_eq!(app.settings.hover.pane_border, keys[1]);
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn choose_key_esc_puts_the_original_back() {
        let mut app = test_app("choose-esc");
        let original = app.settings.hover.pane_border.clone();
        app.choose_key_key(BhRow::PaneBorder, 3, original.clone(), key(KeyCode::Down));
        assert_ne!(app.settings.hover.pane_border, original);
        app.choose_key_key(BhRow::PaneBorder, 4, original.clone(), key(KeyCode::Esc));
        assert_eq!(app.settings.hover.pane_border, original);
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn choose_key_navigation_clamps_at_both_ends() {
        let mut app = test_app("choose-clamp");
        let keys = theme_keys();
        let last = keys.len() - 1;
        let row = BhRow::PaneBorder;
        app.choose_key_key(row, 0, String::new(), key(KeyCode::Up));
        assert!(matches!(app.overlay, Overlay::ChooseKey { sel: 0, .. }));
        app.choose_key_key(row, last, String::new(), key(KeyCode::Down));
        assert!(matches!(app.overlay, Overlay::ChooseKey { sel, .. } if sel == last));
        app.choose_key_key(row, 0, String::new(), key(KeyCode::PageDown));
        assert!(matches!(app.overlay, Overlay::ChooseKey { sel: 10, .. }));
        app.choose_key_key(row, 5, String::new(), key(KeyCode::PageUp));
        assert!(matches!(app.overlay, Overlay::ChooseKey { sel: 0, .. }));
        app.choose_key_key(row, 5, String::new(), key(KeyCode::End));
        assert_eq!(app.settings.hover.pane_border, keys[last]);
        app.choose_key_key(row, 5, String::new(), key(KeyCode::Char('g')));
        assert_eq!(app.settings.hover.pane_border, keys[0]);
        // vim and emacs keys
        app.choose_key_key(row, 5, String::new(), key(KeyCode::Char('j')));
        assert!(matches!(app.overlay, Overlay::ChooseKey { sel: 6, .. }));
        let ctrl_p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        app.choose_key_key(row, 5, String::new(), ctrl_p);
        assert!(matches!(app.overlay, Overlay::ChooseKey { sel: 4, .. }));
    }

    #[test]
    fn behaviour_labels_name_each_row() {
        assert_eq!(App::behaviour_label(BhRow::Gap), "");
        assert_eq!(App::behaviour_label(BhRow::Header("Keys")), "Keys");
        assert_eq!(App::behaviour_label(BhRow::Sample(LookId::Hover)), "Sample");
        assert_eq!(
            App::behaviour_label(BhRow::Tabs),
            "Page tabs (Main, Fonts, Settings)"
        );
        assert_eq!(
            App::behaviour_label(BhRow::GroupOpen(0)),
            "Group \"Base\" open at start"
        );
        assert_eq!(
            App::behaviour_label(BhRow::PrefetchCategory(0)),
            "  Google monospace"
        );
    }

    #[test]
    fn behaviour_values_show_the_default_settings() {
        let mut app = test_app("values");
        let d = Settings::default();
        assert_eq!(value(&mut app, BhRow::Header("Keys")), "");
        assert_eq!(value(&mut app, BhRow::Tabs), d.layout.tabs);
        assert_eq!(value(&mut app, BhRow::SaveSizes), "Enter");
        assert_eq!(value(&mut app, BhRow::ResetSizes), "Enter");
        assert_eq!(
            value(&mut app, BhRow::Highlight(LookId::Hover)),
            if d.hover.look.highlight { "yes" } else { "no" }
        );
        assert_eq!(value(&mut app, BhRow::PaneBorder), d.hover.pane_border);
        assert_eq!(
            value(&mut app, BhRow::Underline(LookId::Derived)),
            d.colours.derived_highlight.underline
        );
        assert_eq!(value(&mut app, BhRow::Spacing), d.font_list.spacing);
        // the leader is shown as its key
        assert_eq!(
            value(&mut app, BhRow::Leader),
            display_seq(
                &[app.keymap.leader],
                Key::new(KeyCode::Null, KeyModifiers::NONE)
            )
        );
    }

    #[test]
    fn message_seconds_value_zero_means_until_a_key() {
        let mut app = test_app("seconds");
        app.settings.messages.seconds = 0;
        assert_eq!(value(&mut app, BhRow::MessageSeconds), "until a key");
        app.settings.messages.seconds = 5;
        assert_eq!(value(&mut app, BhRow::MessageSeconds), "5 s");
    }

    #[test]
    fn group_open_value_ignores_case() {
        let mut app = test_app("group-open");
        app.settings.colours.open_groups = vec!["bAsE".into()];
        assert_eq!(value(&mut app, BhRow::GroupOpen(0)), "yes");
        assert_eq!(value(&mut app, BhRow::GroupOpen(1)), "no");
    }

    #[test]
    fn clear_previews_value_shows_measuring_until_the_size_is_known() {
        let mut app = test_app("clear-value");
        app.prefetch.used = None;
        assert_eq!(value(&mut app, BhRow::ClearPreviews), "Enter  (measuring…)");
        app.prefetch.used = Some(2_500_000);
        assert_eq!(
            value(&mut app, BhRow::ClearPreviews),
            "Enter  (2.5 MB used)"
        );
        app.prefetch.used = Some(1);
        assert_eq!(value(&mut app, BhRow::ClearPreviews), "Enter  (1 KB used)");
    }

    #[test]
    fn prefetch_category_value_has_no_estimate_without_a_catalogue() {
        let mut app = test_app("prefetch-value");
        app.dl.catalogue = None;
        app.settings.fonts_page.prefetch_categories = vec!["serif".into()];
        assert_eq!(value(&mut app, BhRow::PrefetchCategory(2)), "yes");
        assert_eq!(value(&mut app, BhRow::PrefetchCategory(0)), "no");
    }

    #[test]
    fn behaviour_edits_go_to_the_draft_not_the_live_settings() {
        let mut app = test_app("draft-edit");
        let before = app.settings.hover.look.highlight;
        run(&mut app, BhRow::Highlight(LookId::Hover), Action::Confirm);
        assert_eq!(app.settings.hover.look.highlight, before, "not live yet");
        assert!(app.settings_page.draft.is_some());
        let shown = app.show_draft(|a| a.settings.hover.look.highlight);
        assert_eq!(shown, !before);
    }

    #[test]
    fn toggles_flip_on_confirm_and_cycles_wrap_both_ways() {
        let mut app = test_app("cycles");
        // a toggle
        let before = value(&mut app, BhRow::HideBitmap);
        run(&mut app, BhRow::HideBitmap, Action::Confirm);
        assert_ne!(value(&mut app, BhRow::HideBitmap), before);
        // underline: off, single, double; Right goes on, Left goes back
        let id = LookId::Hover;
        let u = BhRow::Underline(id);
        run(&mut app, u, Action::Confirm);
        assert_eq!(value(&mut app, u), "double");
        run(&mut app, u, Action::Right);
        assert_eq!(value(&mut app, u), "off");
        run(&mut app, u, Action::Left);
        assert_eq!(value(&mut app, u), "double");
        // text weight and tab position
        let w = BhRow::TextWeight(id);
        run(&mut app, w, Action::Right);
        assert_eq!(value(&mut app, w), "bold");
        run(&mut app, BhRow::Tabs, Action::Right);
        assert_eq!(value(&mut app, BhRow::Tabs), "top");
        run(&mut app, BhRow::Tabs, Action::Right);
        assert_eq!(value(&mut app, BhRow::Tabs), "bottom");
    }

    #[test]
    fn a_value_outside_its_options_cycles_from_the_first_option() {
        let mut app = test_app("cycle-unknown");
        app.settings.layout.tabs = "sideways".into();
        run(&mut app, BhRow::Tabs, Action::Right);
        // unknown counts as the first option ("bottom"), so Right gives "top"
        assert_eq!(value(&mut app, BhRow::Tabs), "top");
    }

    #[test]
    fn message_seconds_cycle_through_the_options() {
        let mut app = test_app("seconds-cycle");
        app.settings.messages.seconds = 5;
        run(&mut app, BhRow::MessageSeconds, Action::Right);
        assert_eq!(value(&mut app, BhRow::MessageSeconds), "10 s");
        run(&mut app, BhRow::MessageSeconds, Action::Right);
        assert_eq!(value(&mut app, BhRow::MessageSeconds), "until a key");
        run(&mut app, BhRow::MessageSeconds, Action::Right);
        assert_eq!(value(&mut app, BhRow::MessageSeconds), "1 s");
        run(&mut app, BhRow::MessageSeconds, Action::Left);
        assert_eq!(value(&mut app, BhRow::MessageSeconds), "until a key");
    }

    #[test]
    fn group_open_toggles_the_group_name_in_the_list() {
        let mut app = test_app("group-toggle");
        app.settings.colours.open_groups = vec!["base".into()];
        run(&mut app, BhRow::GroupOpen(0), Action::Confirm);
        assert_eq!(value(&mut app, BhRow::GroupOpen(0)), "no");
        run(&mut app, BhRow::GroupOpen(0), Action::Confirm);
        assert_eq!(value(&mut app, BhRow::GroupOpen(0)), "yes");
        let open = app.show_draft(|a| a.settings.colours.open_groups.clone());
        // closing removed "base"; opening added the title's own spelling
        assert_eq!(open, vec!["Base".to_string()]);
    }

    #[test]
    fn prefetch_category_toggles_in_the_list() {
        let mut app = test_app("prefetch-toggle");
        app.settings.fonts_page.prefetch_categories = vec!["serif".into()];
        run(&mut app, BhRow::PrefetchCategory(2), Action::Confirm);
        assert_eq!(value(&mut app, BhRow::PrefetchCategory(2)), "no");
        run(&mut app, BhRow::PrefetchCategory(0), Action::Confirm);
        assert_eq!(value(&mut app, BhRow::PrefetchCategory(0)), "yes");
    }

    #[test]
    fn group_title_confirm_toggles_right_opens_left_closes() {
        let mut app = test_app("title");
        let h = BhRow::Header("Layout");
        run(&mut app, h, Action::Confirm);
        assert!(app.settings_page.bh_closed.contains("Layout"));
        // while closed the row is still on the title; Confirm opens it again
        let rows = visible_behaviour_rows(&app.settings_page.bh_closed);
        assert!(!rows.contains(&BhRow::Tabs));
        run(&mut app, h, Action::Confirm);
        assert!(!app.settings_page.bh_closed.contains("Layout"));
        run(&mut app, h, Action::Left);
        assert!(app.settings_page.bh_closed.contains("Layout"));
        // Left on a closed group keeps it closed; Right opens it
        run(&mut app, h, Action::Left);
        assert!(app.settings_page.bh_closed.contains("Layout"));
        run(&mut app, h, Action::Right);
        assert!(!app.settings_page.bh_closed.contains("Layout"));
        // titles are not settings: no draft
        assert!(app.settings_page.draft.is_none());
    }

    #[test]
    fn colour_row_confirm_opens_the_key_list_and_arrows_step_the_key() {
        let mut app = test_app("colour-row");
        let keys = theme_keys();
        app.settings.hover.pane_border = "bg0".into();
        run(&mut app, BhRow::PaneBorder, Action::Confirm);
        match &app.overlay {
            Overlay::ChooseKey { row, sel, original } => {
                assert_eq!(*row, BhRow::PaneBorder);
                assert_eq!(*sel, 0);
                assert_eq!(original, "bg0");
            }
            _ => panic!("expected the key list"),
        }
        app.overlay = Overlay::None;
        run(&mut app, BhRow::PaneBorder, Action::Right);
        assert_eq!(value(&mut app, BhRow::PaneBorder), keys[1]);
        // Left from the first key wraps to the last
        app.settings.hover.pane_border = "bg0".into();
        app.settings_page.draft = None;
        run(&mut app, BhRow::PaneBorder, Action::Left);
        assert_eq!(value(&mut app, BhRow::PaneBorder), *keys.last().unwrap());
    }

    #[test]
    fn leader_confirm_starts_capturing_a_key() {
        let mut app = test_app("leader");
        run(&mut app, BhRow::Leader, Action::Confirm);
        assert!(matches!(
            app.overlay,
            Overlay::Capture {
                what: Capture::Leader,
                ..
            }
        ));
    }

    #[test]
    fn save_sizes_and_reset_sizes_change_the_default_sizes() {
        let mut app = test_app("sizes");
        app.layout.sizes.fonts_width = 0.31;
        run(&mut app, BhRow::SaveSizes, Action::Confirm);
        assert_eq!(
            app.show_draft(|a| a.settings.layout.sizes.fonts_width),
            0.31
        );
        assert!(
            app.messages
                .message
                .as_ref()
                .is_some_and(|(m, err)| !*err && m.contains("pane sizes become the default"))
        );
        run(&mut app, BhRow::ResetSizes, Action::Confirm);
        assert_eq!(
            app.show_draft(|a| a.settings.layout.sizes.clone()),
            crate::config::Sizes::default()
        );
        // the running layout keeps its sizes until saved
        assert_eq!(app.layout.sizes.fonts_width, 0.31);
    }

    #[test]
    fn clear_previews_asks_first_and_names_the_size_when_known() {
        let mut app = test_app("clear");
        app.prefetch.used = None;
        run(&mut app, BhRow::ClearPreviews, Action::Confirm);
        assert!(matches!(
            &app.overlay,
            Overlay::Ask { question, .. } if question == "Delete the preview fonts?"
        ));
        app.prefetch.used = Some(3_000_000);
        run(&mut app, BhRow::ClearPreviews, Action::Confirm);
        assert!(matches!(
            &app.overlay,
            Overlay::Ask { question, .. } if question == "Delete the preview fonts (3.0 MB)?"
        ));
        assert!(app.settings_page.draft.is_none());
    }

    #[test]
    fn moving_skips_gaps_and_a_cursor_on_a_gap_jumps_to_the_first_row() {
        let mut app = test_app("move");
        select(&mut app, BhRow::Leader);
        let leader = app.settings_page.bh_sel;
        app.run_behaviour_tab(Action::Down);
        // the next selectable row is the "Layout" title, after a gap
        let rows = visible_behaviour_rows(&app.settings_page.bh_closed);
        assert_eq!(rows[app.settings_page.bh_sel], BhRow::Header("Layout"));
        assert_eq!(app.settings_page.bh_sel, leader + 2);
        // a cursor left on a gap goes to the first selectable row
        app.settings_page.bh_sel = leader + 1;
        app.run_behaviour_tab(Action::Confirm);
        assert_eq!(app.settings_page.bh_sel, 0);
    }

    #[test]
    fn behaviour_at_edge_is_true_on_the_first_and_last_selectable_rows() {
        let mut app = test_app("edge");
        select(&mut app, BhRow::Header("Keys"));
        assert!(app.behaviour_at_edge(true));
        assert!(!app.behaviour_at_edge(false));
        select(&mut app, BhRow::ClearPreviews);
        assert!(app.behaviour_at_edge(false));
        assert!(!app.behaviour_at_edge(true));
    }
}
