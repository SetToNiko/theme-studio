//! Tests of input: the leader keys, mouse hover and clicks, and hover looks.

use super::state::nav::ColourRow;
use super::*;
use crate::preview_term::types::PaneId;

#[test]
fn leader_sequences_popup_help_and_hints() {
    let store = temp_store("leader");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    // the hints come from the keymap
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("j/k move") && screen.contains("Enter edit") && screen.contains("? keys"),
        "{screen}"
    );
    // Space waits and shows what can follow
    press(&mut app, " ");
    assert_eq!(app.pending_keys.len(), 1);
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("<leader>") && screen.contains("Esc cancels"),
        "{screen}"
    );
    // Esc cancels without doing anything
    press(&mut app, "\x1b");
    assert_eq!(app.pending_keys, []);
    // <leader>sp saves the palette from anywhere
    press(&mut app, "#abcdef\n");
    assert!(app.dirty());
    press(&mut app, " 1"); // the fonts column
    press(&mut app, " sp");
    assert!(!app.dirty(), "saved from the Fonts tab");
    assert_eq!(
        Store::at(dir).load("One").unwrap().get("bg0"),
        Rgb(0xab, 0xcd, 0xef)
    );
    // an unbound sequence does nothing and clears
    press(&mut app, " zz");
    assert_eq!(app.pending_keys, []);
    // the help screen lists the real bindings
    press(&mut app, "?");
    let screen = render(&mut app, 160, 45);

    assert!(
        screen.contains("<leader>sp") && screen.contains("save palette"),
        "{screen}"
    );
    assert!(screen.contains("Vim preset"));
}
#[test]
fn hover_highlights_and_clicks_through_the_click_map() {
    use crate::app::state::hit::Target as T;
    let mut app = test_app(temp_store("mouse"), Start::Named("One".into()));
    let mut t = Terminal::new(TestBackend::new(200, 50)).unwrap();
    let mut draw = |app: &mut App| {
        t.draw(|f| crate::ui::draw(f, app)).unwrap();
        t.backend().buffer().clone()
    };
    draw(&mut app);
    // hovering a tab highlights it; clicking it opens the page
    let tab = app.hits.rect_of(T::PageTab(Page::Settings)).unwrap();
    mouse(&mut app, MouseEventKind::Moved, tab.x + 1, tab.y);
    let buf = draw(&mut app);
    // the default look: underlined, in the theme's yellow
    let cell = &buf[(tab.x + 1, tab.y)];
    assert!(
        cell.modifier.contains(ratatui::style::Modifier::UNDERLINED),
        "hover"
    );
    assert_eq!(cell.fg, app.theme_colour("yellow"), "hover");
    assert_eq!(app.hits.hovered(), Some(T::PageTab(Page::Settings)));
    click_on(&mut app, T::PageTab(Page::Settings));
    assert_eq!(app.page, Page::Settings);
    draw(&mut app);
    click_on(&mut app, T::PageTab(Page::Main));
    assert_eq!(app.page, Page::Main);
    draw(&mut app);
    // a group row: select, then open
    let backgrounds = app
        .colour_rows()
        .iter()
        .position(|r| matches!(r, ColourRow::Group { index: 1, .. }))
        .unwrap();
    click_on(&mut app, T::ColourRow(backgrounds));
    click_on(&mut app, T::ColourRow(backgrounds));
    assert!(app.expanded_groups.contains(&1));
    draw(&mut app);
    // a colour row twice opens the picker; dragging in the square edits
    click_on(&mut app, T::ColourRow(1)); // bg0
    click_on(&mut app, T::ColourRow(1));
    assert!(app.layout.picker_visible && app.focus == Focus::Square);
    draw(&mut app);
    let before = app.editor.palette.get("bg0");
    let sq = app.hits.rect_of(T::Square).unwrap();
    mouse(
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        sq.x + sq.width - 1,
        sq.y,
    );
    mouse(
        &mut app,
        MouseEventKind::Drag(MouseButton::Left),
        sq.x + sq.width - 2,
        sq.y,
    );
    assert_ne!(app.editor.palette.get("bg0"), before);
    press(&mut app, "\n"); // keep it
    draw(&mut app);
    // a pane title focuses the pane
    click_on(&mut app, T::PaneTitle(1));
    assert_eq!(app.current_pane(), 1);
    // a preview window takes the keyboard
    click_on(&mut app, T::Preview(PaneId::Nvim));
    assert_eq!(app.preview.focus, Some(PaneId::Nvim));
    // clicking the studio again gives it back
    draw(&mut app);
    click_on(&mut app, T::PaneTitle(4));
    assert_eq!(app.preview.focus, None);
    // pop-up buttons: the unsaved-changes prompt's Esc
    press(&mut app, "q");
    assert!(matches!(app.overlay, Overlay::Confirm(_)));
    draw(&mut app);
    click_on(&mut app, T::ConfirmChoice('n'));
    assert!(matches!(app.overlay, Overlay::None));
    // tabs on top: their own first row
    app.settings.layout.tabs = "top".into();
    let buf = draw(&mut app);
    let row0: String = (0..40).map(|x| buf[(x, 0)].symbol().to_string()).collect();
    assert!(row0.contains("F1 Main"), "{row0}");
    let last: String = (0..60).map(|x| buf[(x, 49)].symbol().to_string()).collect();
    assert!(!last.contains("F1 Main"), "{last}");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one scenario: set the hover look, save it, then check each pane border in turn"
)]
fn hover_look_is_configurable_and_pane_borders_light_up() {
    use crate::app::state::hit::{Target as T, Underline};
    use crate::app::state::nav::{BhRow, LookId};
    use crate::settings_page::behaviour::behaviour_rows;
    use ratatui::style::{Color, Modifier};
    let store = temp_store("hover-look");
    let dir = store.dir.clone();
    crate::model::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    // a system palette whose keys we know
    let mut sys = Palette::blank();
    sys.set("aqua", Rgb(1, 2, 3));
    sys.set("bg3", Rgb(4, 5, 6));
    sys.set("purple", Rgb(7, 8, 9));
    app.system_palette = Some(sys);
    assert_eq!(app.theme_colour("aqua"), Color::Rgb(1, 2, 3));
    // the mouse over the colour list (without the keyboard: focus is
    // yellow) lights its border in the pane colour
    app.side = Side::Fonts;
    let mut t = Terminal::new(TestBackend::new(200, 50)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let row = app.hits.rect_of(T::ColourRow(1)).unwrap();
    mouse(&mut app, MouseEventKind::Moved, row.x + 2, row.y);
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let buf = t.backend().buffer().clone();
    assert_eq!(
        buf[(row.x - 1, row.y)].fg,
        Color::Rgb(1, 2, 3),
        "pane border in aqua"
    );
    // Behaviour tab: highlight on, in bg3; curly underline; bold text
    app.page = Page::Settings;
    app.settings_page.tab = crate::settings_page::state::Tab::Behaviour;
    let rows = behaviour_rows();
    let at = |r: BhRow| rows.iter().position(|x| *x == r).unwrap();
    app.settings_page.bh_sel = at(BhRow::Highlight(LookId::Hover));
    press(&mut app, "l");
    app.settings_page.bh_sel = at(BhRow::HighlightColour(LookId::Hover));
    press(&mut app, "\n");
    assert!(matches!(app.overlay, Overlay::ChooseKey { .. }));
    let screen = render(&mut app, 160, 45);
    assert!(screen.contains("Colour from the active theme"), "{screen}");
    // the list: move to bg3 and choose it
    let target = crate::settings_page::behaviour::theme_keys()
        .iter()
        .position(|k| *k == "bg3")
        .unwrap();
    // moving previews; Esc puts the old colour back
    press(&mut app, "j");
    assert!(
        matches!(app.overlay, Overlay::ChooseKey { .. }),
        "stays open"
    );
    assert_ne!(
        app.show_draft(|a| a.settings.hover.look.highlight_colour.clone()),
        "bg3"
    );
    press(&mut app, "\x1b");
    assert!(matches!(app.overlay, Overlay::None));
    let before = app.show_draft(|a| a.settings.hover.look.highlight_colour.clone());
    press(&mut app, "\n");
    // move to bg3 with the keys, Enter keeps it
    let at_now = crate::settings_page::behaviour::theme_keys()
        .iter()
        .position(|k| *k == before)
        .unwrap();
    let steps = if target >= at_now {
        "j".repeat(target - at_now)
    } else {
        "k".repeat(at_now - target)
    };
    press(&mut app, &steps);
    press(&mut app, "\n");
    assert_eq!(
        app.show_draft(|a| a.settings.hover.look.highlight_colour.clone()),
        "bg3"
    );
    app.settings_page.bh_sel = at(BhRow::Underline(LookId::Hover));
    press(&mut app, "l"); // single → double
    app.settings_page.bh_sel = at(BhRow::TextWeight(LookId::Hover));
    press(&mut app, "l");
    // the derived-colour highlight is its own look
    app.settings_page.bh_sel = at(BhRow::Underline(LookId::Derived));
    press(&mut app, "l"); // off → single
    press(&mut app, "s"); // save
    let looks = app.looks(&app.settings);
    assert_eq!(looks.hover.bg, Some(Color::Rgb(4, 5, 6)));
    assert_eq!(looks.hover.underline.map(|u| u.0), Some(Underline::Double));
    assert_eq!(looks.hover.weight, Modifier::BOLD);
    assert_eq!(
        looks.derived.underline.map(|u| u.0),
        Some(Underline::Single)
    );
    assert_eq!(looks.preview, Color::Rgb(7, 8, 9));
    // drawn: the marker that out::Out turns into a double underline
    crate::app::state::hit::set_looks(looks);
    let s = crate::app::state::hit::hover(ratatui::style::Style::new());
    assert!(
        s.add_modifier
            .contains(Modifier::RAPID_BLINK | Modifier::BOLD)
    );
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Sample:") && screen.contains("Derived colours"),
        "{screen}"
    );
    // saved
    let saved = crate::config::load(&dir.join("studio-config"), false);
    assert_eq!(saved.settings.hover.look.underline, "double");
    assert_eq!(saved.settings.colours.derived_highlight.underline, "single");
    crate::app::state::hit::set_looks(crate::app::state::hit::Looks::default());
    std::fs::remove_dir_all(dir).unwrap();
}
