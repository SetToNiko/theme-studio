//! The Settings page's Behaviour tab: settings.json's values, in groups
//! (Keys, Layout, Messages, Hover, Palette pane, Derived colours, Fonts pane).
//!
//! The test module below also tests the `App` methods of
//! `app/behaviour_actions.rs` (it builds an `App`).

use crate::app::state::nav::{BhRow, LookId};
use crate::palette::GROUPS;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::app::Start;
    use crate::app::state::nav::{Capture, Overlay};
    use crate::color::Rgb;
    use crate::config::{LookSettings, Settings};
    use crate::hit::Underline;
    use crate::keymap::Keymap;
    use crate::keymap::{Action, Key, display_seq};
    use crate::palette::Palette;
    use crate::store::{Store, data_file, to_jq_string};
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::style::{Color, Modifier};
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
