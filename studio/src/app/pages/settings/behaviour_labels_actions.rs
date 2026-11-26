//! App's methods for the Behaviour tab's row texts: the value and the label.
//!
//! This is another `impl App { ... }` block (see `theme_actions.rs` for the
//! same style). The tab's own data stays in `settings_page/behaviour.rs`,
//! which also tests these methods (it builds an `App`).

use crate::app::App;
use crate::app::state::nav::{BhRow, LookId};
use crate::keymap::key::{Key, display_seq};
use crate::model::palette::GROUPS;

impl App {
    // Long function (reason verified, phase 6): one match over every Behaviour row kind, one short arm per row.
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
            // measured on a thread (system/prefetch.rs)
            BhRow::ClearPreviews => match self.prefetch.used {
                Some(b) => format!("Enter  ({} used)", crate::system::prefetch::size(b)),
                None => "Enter  (measuring…)".into(),
            },
        }
    }

    /// A prefetch category's value: "yes" or "no", and with the catalogue
    /// loaded, how many fonts it has and about how much space they take.
    fn prefetch_category_value(&self, i: usize) -> String {
        let (cat, _) = crate::config::settings::PREFETCH_CATEGORIES[i];
        let on = if self.settings.fonts_page.prefetches(cat) {
            "yes"
        } else {
            "no"
        }
        .to_string();
        let Some(c) = &self.dl.catalogue else {
            return on;
        };
        let (n, get, keep) = crate::system::prefetch::estimate(c, cat);
        let fonts = if n == 1 { "font" } else { "fonts" };
        let size = crate::system::prefetch::size;
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

    // Long function (reason verified, phase 6): one match over every Behaviour row kind, one text per row.
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
            BhRow::PrefetchCategory(i) => {
                format!("  {}", crate::config::settings::PREFETCH_CATEGORIES[i].1)
            }
            BhRow::ClearPreviews => "Delete the preview fonts (fetched again when needed)".into(),
        }
    }
}
