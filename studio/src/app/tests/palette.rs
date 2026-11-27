//! Tests of palette editing: the picker, pinning, derived colours, and the palette list.

use super::state::nav::{ColourRow, Pending};
use super::*;

#[test]
fn picker_edits_follow_through_derived_keys_and_undo() {
    let mut app = test_app(temp_store("edit"), Start::Named("One".into()));
    assert_eq!(app.key(), "bg0");
    let grey_before = app.editor.palette.get("grey1");
    // bg0: into the square, raise value a lot
    press(&mut app, "\nKKK");
    let bg = app.editor.palette.get("bg0");
    assert_ne!(bg, Palette::blank().get("bg0"));
    assert_ne!(
        app.editor.palette.get("grey1"),
        grey_before,
        "grey1 follows bg0"
    );
    assert!(app.dirty());
    // one undo step for the whole run of picker moves
    press(&mut app, "u");
    assert_eq!(app.editor.palette, Palette::blank());
    press(&mut app, "\x18"); // not a binding: ignored
    app.on_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
    assert_eq!(app.editor.palette.get("bg0"), bg);
}

#[test]
fn pinning_hex_entry_paste_and_copy_from() {
    let mut app = test_app(temp_store("pin"), Start::Named("One".into()));
    // select grey1 (in the Greys group) and type a hex colour
    let i = app.keys.iter().position(|k| *k == "grey1").unwrap();
    app.select(i);
    press(&mut app, "#123456\n");
    assert_eq!(app.editor.palette.get("grey1"), Rgb(0x12, 0x34, 0x56));
    assert!(app.editor.palette.is_pinned("grey1"));
    // a pinned key ignores base changes; p lets it follow again
    app.select(0);
    app.on_paste("#000000");
    assert_eq!(app.editor.palette.get("bg0"), Rgb(0, 0, 0));
    assert_eq!(app.editor.palette.get("grey1"), Rgb(0x12, 0x34, 0x56));
    app.select(i);
    press(&mut app, "p");
    assert!(!app.editor.palette.is_pinned("grey1"));
    assert_ne!(app.editor.palette.get("grey1"), Rgb(0x12, 0x34, 0x56));
    // copy red into grey1: c, move to red, Enter
    press(&mut app, "c");
    let red = app.keys.iter().position(|k| *k == "red").unwrap();
    while app.sel > red {
        press(&mut app, "k");
    }
    press(&mut app, "\n");
    assert_eq!(app.key(), "grey1");
    assert_eq!(
        app.editor.palette.get("grey1"),
        app.editor.palette.get("red")
    );
}

#[test]
fn esc_discards_picker_changes_enter_keeps_them() {
    let mut app = test_app(temp_store("discard"), Start::Named("One".into()));
    let before = app.editor.palette.clone();
    // edit bg0 in the square, then Esc: everything as it was, nothing to undo
    press(&mut app, "\nKK");
    assert_ne!(app.editor.palette, before);
    assert_ne!(
        app.editor.palette.get("grey1"),
        before.get("grey1"),
        "derived keys followed"
    );
    press(&mut app, "\x1b");
    assert_eq!(app.editor.palette, before);
    assert!(!app.dirty() && !app.layout.picker_visible);
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("Discarded"))
    );
    press(&mut app, "u");
    assert_eq!(
        app.editor.palette, before,
        "the discarded steps aren't undoable"
    );
    // the same edit, then Enter: kept (and undoable)
    press(&mut app, "\nKK\n");
    assert_ne!(app.editor.palette, before);
    assert!(app.dirty() && app.focus == Focus::List);
    let kept = app.editor.palette.clone();
    // the hex box: Esc discards whatever the session changed
    press(&mut app, "\nJ#abcdef");
    press(&mut app, "\x1b");
    assert_eq!(app.editor.palette, kept);
    // only Enter keeps: every other way out of the picker discards
    for leave in [" 4", " 1", " cp"] {
        press(&mut app, "\nJ");
        assert_ne!(app.editor.palette, kept);
        press(&mut app, leave);
        assert_eq!(app.editor.palette, kept, "left with {leave:?}");
        press(&mut app, " 4");
    }
    // the hex box: Tab past the last control (back to the list) discards too
    press(&mut app, "\n#abcdef\t");
    assert_eq!(app.editor.palette, kept);
}

#[test]
fn selecting_a_colour_lights_up_its_derivation_relatives() {
    use crate::app::state::hit::Target as T;
    let mut app = test_app(temp_store("related"), Start::Named("One".into()));
    let lit = |app: &mut App, key: &str| -> bool {
        let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
        t.draw(|f| crate::ui::screen::frame::draw(f, app)).unwrap();
        let i = app.keys.iter().position(|k| *k == key).unwrap();
        let Some(row) = app
            .colour_rows()
            .iter()
            .position(|r| *r == ColourRow::Key(i))
        else {
            return false;
        };
        let r = app.hits.rect_of(T::ColourRow(row)).unwrap();
        let buf = t.backend().buffer().clone();
        buf[(r.x + 5, r.y)].bg == app.looks(&app.settings).derived.bg.unwrap()
    };
    let group_lit = |app: &mut App, name: &str| -> bool {
        let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
        t.draw(|f| crate::ui::screen::frame::draw(f, app)).unwrap();
        let g = GROUPS.iter().position(|(n, _)| *n == name).unwrap();
        let row = app
            .colour_rows()
            .iter()
            .position(|r| matches!(r, ColourRow::Group { index, .. } if *index == g))
            .unwrap();
        let r = app.hits.rect_of(T::ColourRow(row)).unwrap();
        let buf = t.backend().buffer().clone();
        buf[(r.x + 4, r.y)].bg == app.looks(&app.settings).derived.bg.unwrap()
    };
    let select = |app: &mut App, key: &str| {
        let i = app.keys.iter().position(|k| *k == key).unwrap();
        app.select_key(i);
    };
    // a base colour: what's derived from it lights up (its closed group)
    select(&mut app, "red");
    assert!(group_lit(&mut app, "Tinted backgrounds"));
    assert!(!lit(&mut app, "green"));
    // a derived colour: its sources light up
    let g = GROUPS
        .iter()
        .position(|(n, _)| *n == "Tinted backgrounds")
        .unwrap();
    app.set_group_open(g, true);
    select(&mut app, "bg_visual_red");
    assert!(lit(&mut app, "bg0") && lit(&mut app, "red"));
    assert!(!lit(&mut app, "green"));
    // pinned (setting a derived key by hand pins it): nothing lights up
    app.editor.palette.set("bg_visual_red", Rgb(9, 9, 9));
    assert!(!lit(&mut app, "red"));
    std::fs::remove_dir_all(app.store.dir.clone()).unwrap();
}

#[test]
fn editing_a_base_colour_moves_its_unpinned_derived_colours_live() {
    let mut app = test_app(temp_store("live-derive"), Start::Named("One".into()));
    let i = app.keys.iter().position(|k| *k == "red").unwrap();
    app.select_key(i);
    app.open_picker();
    let before = app.editor.palette.get("bg_visual_red");
    app.editor.palette.set("bg_diff_red", Rgb(1, 1, 1)); // pinned now
    press(&mut app, "lll"); // in the picker, before Enter
    assert!(app.editor.in_edit_session(), "not kept yet");
    assert_ne!(
        app.editor.palette.get("bg_visual_red"),
        before,
        "derived follows live"
    );
    assert_eq!(
        app.editor.palette.get("bg_diff_red"),
        Rgb(1, 1, 1),
        "pinned stays"
    );
    std::fs::remove_dir_all(app.store.dir.clone()).unwrap();
}

#[test]
fn palette_list_hangs_from_its_row_and_previews() {
    use crate::app::state::hit::Target as T;
    let store = temp_store("palette-menu");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let one_red = app.editor.palette.get("red");
    let down = |app: &mut App| app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    // the title names it; the dropdown is the pane's first row
    let s = render(&mut app, 160, 45);
    assert!(s.contains("Palette: One ") && s.contains(" One ▾ "), "{s}");
    // a click on the row opens the list, hanging under it
    click_on(&mut app, T::PaletteMenu);
    assert!(matches!(app.overlay, Overlay::Open { .. }));
    render(&mut app, 160, 45);
    let pane = app.layout.palette_pane.unwrap();
    assert_eq!(app.hits.rect_of(T::OverlayRow(0)).unwrap().y, pane.y + 5);
    // moving shows the highlighted palette; it isn't an unsaved edit
    down(&mut app);
    assert_eq!(app.editor.palette.get("red"), Rgb(200, 0, 0));
    assert_eq!(app.name, "One");
    assert!(!app.dirty());
    assert!(render(&mut app, 160, 45).contains("Palette: One "));
    // Esc puts it back
    press(&mut app, "\x1b");
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.editor.palette.get("red"), one_red);
    assert!(app.open_original.is_none());
    // <leader>op opens the same list; Enter keeps the highlighted one
    press(&mut app, " op");
    down(&mut app);
    press(&mut app, "\n");
    assert_eq!(app.name, "Two");
    assert_eq!(app.editor.palette.get("red"), Rgb(200, 0, 0));
    assert!(!app.dirty());
    // with unsaved edits, choosing still asks first (about the real palette)
    press(&mut app, "#111111\n");
    assert!(app.dirty());
    let edited = app.editor.palette.clone();
    press(&mut app, " op");
    app.on_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_ne!(app.editor.palette, edited);
    assert!(app.dirty());
    press(&mut app, "\n");
    assert!(matches!(app.overlay, Overlay::Confirm(Pending::Open(_))));
    assert_eq!(app.editor.palette, edited);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn keyboard_reaches_the_palette_dropdown_and_the_page_tabs() {
    let store = temp_store("nav-up");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.settings.layout.tabs = "top".into();
    let key = |app: &mut App, c: KeyCode| app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
    app.focus_pane(4);
    // the cursor starts on the first colour, under the Base group
    key(&mut app, KeyCode::Up);
    assert_eq!(app.row_sel, 0);
    // up from the first row: the dropdown row; Enter opens the list
    key(&mut app, KeyCode::Up);
    assert!(app.layout.palette_menu_focused);
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::Open { .. }));
    key(&mut app, KeyCode::Esc);
    assert!(app.layout.palette_menu_focused, "back on the row");
    // up again: the page tabs; right changes pages, the keyboard stays
    key(&mut app, KeyCode::Up);
    assert!(app.layout.page_tabs_focused);
    let s = render(&mut app, 160, 45);
    assert!(s.contains("F1 Main"), "{s}");
    key(&mut app, KeyCode::Right);
    assert_eq!(app.page, Page::Downloads);
    assert!(app.layout.page_tabs_focused);
    // down: into the page (its tabs are on top, so the list)
    key(&mut app, KeyCode::Down);
    assert!(!app.layout.page_tabs_focused && !app.dl.tabs_focused);
    // up: the page's tabs, then the page tabs; left back to Main
    key(&mut app, KeyCode::Up);
    assert!(app.dl.tabs_focused);
    key(&mut app, KeyCode::Up);
    assert!(app.layout.page_tabs_focused);
    key(&mut app, KeyCode::Left);
    assert_eq!(app.page, Page::Main);
    key(&mut app, KeyCode::Down);
    assert!(!app.layout.page_tabs_focused && app.layout.palette_menu_focused);
    key(&mut app, KeyCode::Down);
    assert!(!app.layout.palette_menu_focused && app.row_sel == 0);
    // tabs at the bottom: up from the dropdown row goes nowhere
    app.settings.layout.tabs = "bottom".into();
    key(&mut app, KeyCode::Up);
    key(&mut app, KeyCode::Up);
    assert!(app.layout.palette_menu_focused && !app.layout.page_tabs_focused);
    // a click anywhere takes the keyboard off the page tabs
    app.layout.page_tabs_focused = true;
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 1);
    assert!(!app.layout.page_tabs_focused);
    std::fs::remove_dir_all(dir).unwrap();
}
