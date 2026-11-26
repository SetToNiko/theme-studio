//! Tests for the app: key handling, panes, palette editing, fonts, and the tour.

use super::state::nav::{ColourRow, Pending};
use super::*;

use crate::keymap::action::Action;

use crate::keymap::Context;
use crate::preview::PaneId;
use crate::store::to_jq_string;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use serde_json::{Map, Value};

/// A store in a temp folder with palettes "One" and "Two".
fn temp_store(tag: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("theme-studio-app-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut two = Palette::blank();
    two.set("red", Rgb(200, 0, 0));
    let mut m = Map::new();
    m.insert("One".into(), Value::Object(Palette::blank().to_json()));
    m.insert("Two".into(), Value::Object(two.to_json()));
    std::fs::write(
        dir.join("palettes.json"),
        to_jq_string(&crate::store::data_file("palettes", m)),
    )
    .unwrap();
    Store::at(dir)
}

/// An app whose preview never starts nvim, the shell or yazi.
fn test_app(store: Store, start: Start) -> App {
    // the shipped defaults, never your ~/.config/theme-studio
    let config = crate::config::Loaded {
        keymap: Keymap::vim(),
        presets: Keymap::builtin(),
        settings: crate::config::settings::Settings::default(),
        warnings: Vec::new(),
        // a folder inside the test's own temp store
        dir: store.dir.join("studio-config"),
    };
    let mut app = App::with_config(store, start, config).unwrap();
    app.preview.set_headless();
    app
}

fn press(app: &mut App, keys: &str) {
    for c in keys.chars() {
        let code = match c {
            '\n' => KeyCode::Enter,
            '\t' => KeyCode::Tab,
            '\x1b' => KeyCode::Esc,
            c => KeyCode::Char(c),
        };
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }
}

fn render(app: &mut App, w: u16, h: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| crate::ui::draw(f, app)).unwrap();
    let buf = t.backend().buffer().clone();
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

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
fn themes_save_open_and_fork() {
    let store = temp_store("save");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    let themes = || {
        ThemeStore { dir: dir.clone() }
            .load_all()
            .into_iter()
            .map(|(n, _)| n)
            .collect::<Vec<_>>()
    };
    // s on a theme that was never saved asks for its name
    press(&mut app, "#abcdef\n");
    assert!(app.theme_dirty());
    if app.theme_new {
        press(&mut app, "s");
        assert!(matches!(app.overlay, Overlay::SaveAs { .. }));
        press(&mut app, "Work\n");
    } else {
        // (made at start from the active parts) save as Work
        press(&mut app, "S");
        for _ in 0..app.theme.len() {
            app.on_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        }
        press(&mut app, "Work\n");
    }
    assert_eq!(app.theme, "Work");
    assert!(!app.theme_dirty() && !app.dirty());
    assert!(themes().contains(&"Work".to_string()));
    assert_eq!(
        Store::at(dir.clone()).load(&app.name).unwrap().get("bg0"),
        Rgb(0xab, 0xcd, 0xef)
    );
    // <leader>op swaps another palette into the theme: unsaved
    press(&mut app, " opTwo\n");
    assert_eq!(app.name, "Two");
    assert_eq!(app.editor.palette.get("red"), Rgb(200, 0, 0));
    assert!(app.theme_dirty() && !app.dirty());
    assert!(render(&mut app, 160, 45).contains("Theme: Work ●"));
    // an edited palette asks before another is opened
    press(&mut app, "#111111\n");
    press(&mut app, " opOne\n");
    assert!(matches!(app.overlay, Overlay::Confirm(Pending::Open(_))));
    press(&mut app, "\x1b");
    assert_eq!(app.name, "Two");
    // save as a new name forks the edited palette under that name
    press(&mut app, "S");
    for _ in 0.."Work".len() {
        app.on_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    }
    press(&mut app, "Play\n");
    assert_eq!((app.theme.as_str(), app.name.as_str()), ("Play", "Play"));
    let names: Vec<String> = Store::at(dir.clone())
        .load_all()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert!(
        names.ends_with(&["Two".to_string(), "Play".to_string()]),
        "{names:?}"
    );
    assert_eq!(
        Store::at(dir.clone()).load("Two").unwrap().get("bg0"),
        Palette::blank().get("bg0"),
        "Two kept"
    );
    assert!(themes().ends_with(&["Work".to_string(), "Play".to_string()]));
    // o opens a theme: Work's palette comes back
    press(&mut app, "oWork\n");
    assert_eq!(app.theme, "Work");
    assert_ne!(app.name, "Play");
    // n: a new theme, a copy of this one
    press(&mut app, "n");
    assert!(app.theme_new && app.theme == "untitled");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn renders_at_full_size_and_warns_when_small() {
    let mut app = test_app(temp_store("draw"), Start::Named("One".into()));
    let screen = render(&mut app, 160, 45);
    assert!(screen.contains("Theme: "), "{screen}");
    // only Base is open; the other groups are rows that open
    assert!(
        screen.contains("▾ Base") && screen.contains("▸ Backgrounds"),
        "{screen}"
    );
    assert!(
        !screen.contains("bg_statusline1"),
        "Backgrounds starts closed"
    );
    assert!(!screen.contains("hex #1e1e22"), "the picker starts hidden");
    // contrast column fits: e.g. "fg0 ... 11.8:1"
    let fg_row = screen.lines().find(|l| l.contains(" fg0 ")).unwrap();
    assert!(fg_row.contains(":1"), "contrast cut off: {fg_row}");
    // the last row is the last group: open it and go to its last colour
    press(&mut app, "G\nG");
    assert!(render(&mut app, 160, 45).contains("bg_diff_blue"));
    // editing a colour shows the picker; Esc hides it again
    press(&mut app, "gj\n"); // g is the Base group row; j the first colour
    assert!(render(&mut app, 160, 45).contains("hex"));
    assert!(app.layout.picker_visible);
    press(&mut app, "\x1b");
    assert!(!app.layout.picker_visible);
    // <leader>cp toggles it, <leader>fs the font sample
    press(&mut app, " cp");
    assert!(app.layout.picker_visible);
    press(&mut app, " cp");
    assert!(!app.layout.picker_visible);
    press(&mut app, " 1 fs");
    assert!(!app.layout.sample_visible);
    press(&mut app, " 4");
    // too small: what's needed
    press(&mut app, "o?");
    let small = render(&mut app, 80, 20);
    assert!(small.contains("Window too small"));
    assert!(small.contains("Width = 80  Height = 20"));
    assert!(small.contains("Width = 98"), "{small}");
    // every pop-up draws without panicking
    for keys in ["\x1b?", "\x1bS", "\x1b\x1bc", "\x1b"] {
        press(&mut app, keys);
        render(&mut app, 160, 45);
    }
    // the other pages
    for page in [2, 3, 1] {
        app.on_key(KeyEvent::new(KeyCode::F(page), KeyModifiers::NONE));
        render(&mut app, 160, 45);
    }
}

#[test]
fn number_keys_jump_between_panels() {
    let mut app = test_app(temp_store("panels"), Start::Named("One".into()));
    render(&mut app, 200, 50); // lays out the preview windows
    assert!(!app.layout.picker_visible, "the picker starts hidden");
    press(&mut app, " 5");
    assert_eq!((app.side, app.focus), (Side::Colours, Focus::Square));
    assert!(app.layout.picker_visible, "5 shows it");
    press(&mut app, " 4");
    assert_eq!(app.focus, Focus::List);
    assert!(!app.layout.picker_visible, "leaving it hides it again");
    press(&mut app, " 1");
    assert_eq!(
        (app.side, app.fonts.focus),
        (Side::Fonts, crate::fonts::types::FontFocus::Slots)
    );
    press(&mut app, " 2");
    assert_eq!(app.fonts.focus, crate::fonts::types::FontFocus::List);
    press(&mut app, " 3");
    assert_eq!(app.fonts.focus, crate::fonts::types::FontFocus::Sample);
    press(&mut app, " 6");
    assert_eq!(app.preview.focus, Some(PaneId::Nvim));
    // inside a preview window digits are typed into it, F12 comes back out
    press(&mut app, "1");
    assert_eq!(app.preview.focus, Some(PaneId::Nvim));
    while app.preview.focus.is_some() {
        app.on_key(KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE));
    }
    press(&mut app, " 4Y 7");
    assert_eq!(app.preview.focus, Some(PaneId::Yazi));
    app.on_key(KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE));
    // <C-w>: across and down between the fonts and colour columns
    press(&mut app, " 1");
    app.on_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
    press(&mut app, "l");
    assert_eq!(app.current_pane(), 4);
    app.on_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
    press(&mut app, "h");
    app.on_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
    press(&mut app, "j");
    assert_eq!(app.current_pane(), 2);
    // digits still type into the hex box
    press(&mut app, " 4#123456\n");
    assert_eq!(app.editor.palette.get("bg0"), Rgb(0x12, 0x34, 0x56));
    // titles carry the numbers
    press(&mut app, " 5");
    let screen = render(&mut app, 200, 50);
    for t in [
        "¹Fonts",
        "²Installed",
        "³Sample",
        "⁴Palette",
        "⁵Colour",
        "⁶nvim",
        "⁷yazi",
    ] {
        assert!(screen.contains(t), "missing {t}");
    }
}

#[test]
fn fonts_tab_edits_saves_and_opens_presets() {
    let store = temp_store("fonts");
    let dir = store.dir.clone();
    std::fs::write(
        dir.join("fonts.json"),
        "{\n  \"Base\": {\n    \"mono\": {\n      \"family\": \"A\",\n      \"size\": 14\n    },\n    \"ui\": {\n      \"family\": \"B\",\n      \"size\": 9\n    }\n  }\n}\n",
    )
    .unwrap();
    let mut app = test_app(store, Start::Named("One".into()));
    app.fonts.store.dir = dir.clone();
    app.fonts.open("Base").unwrap();
    press(&mut app, " 1"); // the font roles
    assert_eq!(app.side, Side::Fonts);
    app.fonts.set_installed(vec![
        crate::fonts::types::Installed {
            family: "A".into(),
            mono: true,
            nerd: true,
            bitmap: false,
            special: false,
        },
        crate::fonts::types::Installed {
            family: "B".into(),
            mono: false,
            nerd: false,
            bitmap: false,
            special: false,
        },
        crate::fonts::types::Installed {
            family: "C Mono".into(),
            mono: true,
            nerd: false,
            bitmap: false,
            special: false,
        },
        crate::fonts::types::Installed {
            family: "C Nerd Font Propo".into(),
            mono: false,
            nerd: true,
            bitmap: false,
            special: false,
        },
    ]);
    // A is no Nerd Font Mono: the role says so, with the fix key
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("⚠ not a Nerd Font Mono · r fixes"),
        "{screen}"
    );
    // Icons (third role): Enter lists only the Nerd Font Propo fonts
    press(&mut app, "jj\n");
    assert_eq!(app.fonts.picking, Some(2));
    assert_eq!(app.fonts.rows().len(), 1);
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Nerd Font Propo]") && screen.contains("picking Icons"),
        "{screen}"
    );
    // the chips that would widen it are locked (and hidden)
    press(&mut app, "n");
    assert_eq!(app.fonts.rows().len(), 1, "n is locked while picking");
    // Enter picks it (saved and applied: font-switch.sh isn't in the
    // temp folder, so applying reports it), two sizes bigger
    press(&mut app, "\n");
    assert_eq!(app.fonts.picking, None);
    assert_eq!(app.fonts.filters.need, None, "the user's filters are back");
    assert!(!app.fonts.dirty(), "saved straight away");
    press(&mut app, "ll");
    assert_eq!(app.fonts.preset.fonts[2].family, "C Nerd Font Propo");
    // the sizes are the theme's, not the preset's: the new theme's 11, +2
    assert_eq!(app.fonts.preset.fonts[2].size, 13);
    // number keys: 2 opens the list, 1 goes back
    press(&mut app, " 2");
    assert_eq!(app.fonts.focus, crate::fonts::types::FontFocus::List);
    press(&mut app, " 1");
    // a size isn't the preset's: it stays saved, and opening the preset
    // again doesn't ask, and keeps the sizes
    assert!(!app.fonts.dirty());
    press(&mut app, " ofBase\n");
    assert!(!matches!(
        app.overlay,
        Overlay::Confirm(Pending::OpenFonts(_))
    ));
    assert_eq!(app.fonts.preset.fonts[2].size, 13);
    // <leader>sf saves the preset in place, in jq's format, families only
    press(&mut app, " sf");
    assert!(!app.fonts.dirty());
    let text = std::fs::read_to_string(dir.join("fonts.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["version"], 4, "saved as version 4");
    assert_eq!(
        v["presets"]["Base"]["icons"],
        serde_json::json!({"family": "C Nerd Font Propo"})
    );
    assert_eq!(text, to_jq_string(&v));
    // the palette tab didn't change, and its keys still work after F1
    assert!(!app.dirty());
    press(&mut app, " 5");
    assert_eq!((app.side, app.focus), (Side::Colours, Focus::Square));
    assert!(app.layout.picker_visible);
    // the Fonts tab draws at every size (no image support in tests)
    let screen = render(&mut app, 160, 45);
    for t in [
        "¹Fonts",
        "²Installed fonts",
        "F2 Fonts",
        "Sample",
        "C Nerd Font Propo",
    ] {
        assert!(screen.contains(t), "missing {t}");
    }
    for w in (1..=200).step_by(7) {
        for h in (1..=70).step_by(5) {
            render(&mut app, w, h);
        }
    }
}

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
    assert!(app.pending_keys.is_empty());
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
    assert!(app.pending_keys.is_empty());
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
fn narrow_layout_drops_the_font_sample_and_preview_capture_is_magenta() {
    let mut app = test_app(temp_store("narrow"), Start::Named("One".into()));
    let wide = render(&mut app, 200, 50);
    assert!(wide.contains("Sample"));
    let narrow = render(&mut app, 137, 50);
    assert!(
        !narrow.contains("Sample"),
        "no sample under the narrow layout"
    );
    press(&mut app, " 3");
    assert_ne!(app.current_pane(), 3, "the hidden sample can't be focused");
    // while nvim has the keyboard, the preview frame is magenta
    press(&mut app, " 6");
    let mut t = Terminal::new(TestBackend::new(200, 50)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let buf = t.backend().buffer().clone();
    let magenta = (0..200)
        .flat_map(|x| (0..50).map(move |y| (x, y)))
        .any(|(x, y)| {
            buf[(x, y)].symbol() == "─" && buf[(x, y)].fg == ratatui::style::Color::Magenta
        });
    assert!(
        magenta,
        "preview frame drawn in magenta while capturing keys"
    );
}

fn mouse(app: &mut App, kind: MouseEventKind, x: u16, y: u16) {
    app.on_mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    });
}

/// Clicks the middle of wherever `t` was last drawn.
fn click_on(app: &mut App, t: crate::hit::Target) {
    let r = app
        .hits
        .rect_of(t)
        .unwrap_or_else(|| panic!("{t:?} not on screen"));
    let (x, y) = (r.x + r.width / 2, r.y + r.height / 2);
    mouse(app, MouseEventKind::Moved, x, y);
    mouse(app, MouseEventKind::Down(MouseButton::Left), x, y);
    mouse(app, MouseEventKind::Up(MouseButton::Left), x, y);
}

#[test]
fn hover_highlights_and_clicks_through_the_click_map() {
    use crate::hit::Target as T;
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
fn borders_drag_and_resize_mode_moves_edges() {
    use crate::hit::{BorderId as B, Target as T};
    let mut app = test_app(temp_store("resize"), Start::Named("One".into()));
    render(&mut app, 200, 50);
    // drag the border between fonts and colours 10 columns right
    let b = app.hits.rect_of(T::Border(B::FontsColours)).unwrap();
    mouse(
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        b.x,
        b.y + 5,
    );
    assert_eq!(app.layout.dragging, Some(B::FontsColours));
    mouse(
        &mut app,
        MouseEventKind::Drag(MouseButton::Left),
        b.x + 10,
        b.y + 5,
    );
    mouse(
        &mut app,
        MouseEventKind::Up(MouseButton::Left),
        b.x + 10,
        b.y + 5,
    );
    assert_eq!(app.layout.dragging, None);
    render(&mut app, 200, 50);
    assert_eq!(
        app.hits.rect_of(T::Border(B::FontsColours)).unwrap().x,
        b.x + 10
    );
    // resize mode on the colour list: l moves its right edge, = resets, Esc leaves
    press(&mut app, " 4 r");
    assert!(app.layout.resize_mode);
    let before = app.hits.rect_of(T::Border(B::ColoursPreview)).unwrap().x;
    press(&mut app, "lll");
    render(&mut app, 200, 50);
    assert_eq!(
        app.hits.rect_of(T::Border(B::ColoursPreview)).unwrap().x,
        before + 3
    );
    press(&mut app, "H");
    render(&mut app, 200, 50);
    assert_eq!(
        app.hits.rect_of(T::Border(B::ColoursPreview)).unwrap().x,
        before - 2
    );
    // nvim / shell split from the preview panes isn't reachable from the list; j
    // here moves the list/picker border, which only exists with the picker shown
    press(&mut app, "j");
    press(&mut app, "=");
    render(&mut app, 200, 50);
    assert_eq!(app.layout.sizes, crate::config::settings::Sizes::default());
    press(&mut app, "\x1b");
    assert!(!app.layout.resize_mode);
    // keys work normally again
    press(&mut app, "j");
    assert_eq!(app.row_sel, 2);
}

#[test]
fn extreme_sizes_render_at_every_window_size() {
    let mut app = test_app(temp_store("extreme"), Start::Named("One".into()));
    app.layout.picker_visible = true;
    for share in [0.0, 0.02, 0.5, 0.98, 1.0] {
        app.layout.sizes = crate::config::settings::Sizes {
            fonts_width: share,
            colours_width: share,
            sample_height: share,
            picker_height: share,
            preview_split: share,
            narrow_preview_height: share,
            narrow_fonts_width: share,
        };
        for w in (90..=260).step_by(17) {
            for h in (20..=70).step_by(9) {
                render(&mut app, w, h);
            }
        }
    }
}

#[test]
fn layout_three_columns_then_preview_row_then_too_small() {
    use crate::ui::{Arrangement, arrange};
    let r = |w, h| Rect::new(0, 0, w, h);
    // wide: fonts | colours | preview, the picker hidden, the sample shown
    let Arrangement::Main(l) = arrange(
        r(200, 50),
        Page::Main,
        false,
        true,
        true,
        &crate::config::settings::Sizes::default(),
    ) else {
        panic!()
    };
    assert!(l.wide);
    assert!(l.roles.x < l.colour_list.x && l.colour_list.x < l.preview.x);
    assert_eq!(
        (l.preview.y, l.preview.height),
        (0, 49),
        "preview is a full-height column"
    );
    assert!(l.picker.is_none() && l.sample.is_some());
    let s = l.sample.unwrap();
    assert!(
        l.roles.y < l.font_list.y && l.font_list.y < s.y,
        "roles, list, sample top to bottom"
    );
    // picker shown: under the list in the same column
    let Arrangement::Main(l) = arrange(
        r(200, 50),
        Page::Main,
        true,
        true,
        true,
        &crate::config::settings::Sizes::default(),
    ) else {
        panic!()
    };
    let p = l.picker.unwrap();
    assert_eq!(p.x, l.colour_list.x);
    assert!(p.y > l.colour_list.y);
    // narrow (your portrait monitor): columns on top, preview row underneath
    let Arrangement::Main(l) = arrange(
        r(137, 60),
        Page::Main,
        true,
        true,
        true,
        &crate::config::settings::Sizes::default(),
    ) else {
        panic!()
    };
    assert!(!l.wide);
    assert_eq!((l.preview.x, l.preview.width), (0, 137));
    assert!(l.preview.y > l.roles.y && l.preview.y > l.colour_list.y);
    assert_eq!(l.roles.y, l.colour_list.y, "fonts and colours side by side");
    // too small, and what's needed (less height when things are hidden)
    assert_eq!(
        arrange(
            r(90, 60),
            Page::Main,
            false,
            false,
            true,
            &crate::config::settings::Sizes::default()
        ),
        Arrangement::TooSmall {
            need_w: 98,
            need_h: 30
        }
    );
    assert_eq!(
        arrange(
            r(137, 20),
            Page::Main,
            true,
            true,
            true,
            &crate::config::settings::Sizes::default()
        ),
        Arrangement::TooSmall {
            need_w: 98,
            need_h: 35
        }
    );
    assert_eq!(
        arrange(
            r(160, 20),
            Page::Main,
            true,
            true,
            true,
            &crate::config::settings::Sizes::default()
        ),
        Arrangement::TooSmall {
            need_w: 148,
            need_h: 24
        }
    );
    // other pages need little room
    assert!(matches!(
        arrange(
            r(60, 20),
            Page::Settings,
            true,
            true,
            true,
            &crate::config::settings::Sizes::default()
        ),
        Arrangement::Page { .. }
    ));
}

/// Dragging the window through every size never crashes the drawing
/// (tests build in debug mode, where integer underflow panics).
#[test]
fn every_window_size_renders() {
    let mut app = test_app(temp_store("sizes"), Start::Named("One".into()));
    for w in (1..=220).step_by(3) {
        for h in (1..=80).step_by(2) {
            render(&mut app, w, h);
        }
    }
    // and with the picker focused and the selection near the end of the list
    press(&mut app, "G\t");
    for w in [52, 53, 111, 112, 113] {
        for h in [22, 23, 34, 35, 36, 60] {
            render(&mut app, w, h);
        }
    }
}

#[test]
fn settings_changes_wait_for_save() {
    use crate::settings::KmRow;
    let store = temp_store("settings-keys");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    app.on_key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
    assert_eq!(app.page, Page::Settings);
    assert_eq!(app.contexts()[0], Context::Settings);
    let row_of = |app: &mut App, r: KmRow| {
        app.show_draft(|a| a.keymap_rows().iter().position(|x| *x == r).unwrap())
    };
    let draft_keys =
        |app: &mut App, c: Context, a: Action| app.show_draft(|x| x.keymap.keys_of(c, a));
    let on_disk = |dir: &std::path::Path| crate::config::load(&dir.join("studio-config"), false);
    let q = Key::parse_seq("Q", app.keymap.leader).unwrap();
    // Enter on "quit" records new keys: Q, then Enter keeps them, unsaved
    app.settings_page.km_sel = row_of(&mut app, KmRow::Binding(Context::Global, Action::Quit));
    press(&mut app, "\n");
    assert!(matches!(app.overlay, Overlay::Capture { .. }));
    press(&mut app, "Q");
    assert!(render(&mut app, 160, 45).contains("Q▏"));
    press(&mut app, "\n");
    assert_eq!(
        draft_keys(&mut app, Context::Global, Action::Quit),
        vec![q.clone()]
    );
    assert_ne!(
        app.keymap.keys_of(Context::Global, Action::Quit),
        vec![q.clone()],
        "not live yet"
    );
    assert_ne!(
        on_disk(&dir).keymap.keys_of(Context::Global, Action::Quit),
        vec![q.clone()]
    );
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("● 1 unsaved")
            && screen.contains("s Save")
            && screen.contains("Settings ●"),
        "{screen}"
    );
    // s saves: live and on disk
    press(&mut app, "s");
    assert!(app.settings_page.draft.is_none());
    assert_eq!(
        app.keymap.keys_of(Context::Global, Action::Quit),
        vec![q.clone()]
    );
    assert_eq!(
        on_disk(&dir).keymap.keys_of(Context::Global, Action::Quit),
        vec![q]
    );
    // binding pin to y (copy-hex in Colours) warns; D discards
    app.settings_page.km_sel = row_of(&mut app, KmRow::Binding(Context::Colours, Action::Pin));
    press(&mut app, "ay\n");
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, err)| *err && m.contains("is also")),
        "{:?}",
        app.messages.message
    );
    assert_eq!(draft_keys(&mut app, Context::Colours, Action::Pin).len(), 2);
    press(&mut app, "D");
    assert!(app.settings_page.draft.is_none());
    assert_eq!(app.keymap.keys_of(Context::Colours, Action::Pin).len(), 1);
    // d unbinds, s saves; <leader>u undoes that save, again redoes
    press(&mut app, "ds");
    assert!(app.keymap.keys_of(Context::Colours, Action::Pin).is_empty());
    press(&mut app, " u");
    assert_eq!(app.keymap.keys_of(Context::Colours, Action::Pin).len(), 1);
    press(&mut app, " u");
    assert!(app.keymap.keys_of(Context::Colours, Action::Pin).is_empty());
    // Behaviour tab: the leader, by pressing it
    press(&mut app, "]");
    app.settings_page.bh_sel = 1;
    press(&mut app, "\n,");
    let comma = Key::new(KeyCode::Char(','), KeyModifiers::NONE);
    assert_eq!(app.show_draft(|a| a.keymap.leader), comma);
    assert_ne!(app.keymap.leader, comma, "not live yet");
    // quitting asks, and "save first" saves the settings
    press(&mut app, "Q"); // quit is Q now
    assert!(matches!(app.overlay, Overlay::Confirm(Pending::Quit)));
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Unsaved: ") && screen.contains("settings."),
        "{screen}"
    );
    press(&mut app, "s");
    assert!(app.quit);
    assert_eq!(app.keymap.leader, comma);
    assert_eq!(on_disk(&dir).keymap.leader, comma);
    assert_eq!(
        app.keymap.keys_of(Context::Global, Action::SavePalette),
        vec![Key::parse_seq(",sp", comma).unwrap()]
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn settings_behaviour_tab_edits_then_saves() {
    use crate::app::state::nav::BhRow;
    use crate::behaviour::behaviour_rows;
    let store = temp_store("settings-behaviour");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    app.page = Page::Settings;
    render(&mut app, 160, 45);
    click_on(&mut app, crate::hit::Target::SettingsTab(1));
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Page tabs") && screen.contains("‹ bottom ›"),
        "{screen}"
    );
    let rows = behaviour_rows();
    let on_disk = |dir: &std::path::Path| crate::config::load(&dir.join("studio-config"), false);
    app.settings_page.bh_sel = rows.iter().position(|r| *r == BhRow::Tabs).unwrap();
    press(&mut app, "l");
    assert_eq!(app.show_draft(|a| a.settings.layout.tabs.clone()), "top");
    assert_eq!(app.settings.layout.tabs, "bottom", "not live yet");
    // changing it back leaves nothing unsaved
    render(&mut app, 160, 45);
    let i = app.settings_page.bh_sel;
    click_on(&mut app, crate::hit::Target::SettingsRow(i));
    assert!(app.settings_page.draft.is_none());
    // again, then the Save button
    press(&mut app, "l");
    render(&mut app, 160, 45);
    click_on(&mut app, crate::hit::Target::SettingsButton('s'));
    assert_eq!(app.settings.layout.tabs, "top");
    assert_eq!(on_disk(&dir).settings.layout.tabs, "top");
    // the current pane sizes become the default when saved
    app.layout.sizes.fonts_width = 0.4;
    app.settings_page.bh_sel = rows.iter().position(|r| *r == BhRow::SaveSizes).unwrap();
    press(&mut app, "\ns");
    assert_eq!(on_disk(&dir).settings.layout.sizes.fonts_width, 0.4);
    // the Discard button drops changes
    app.settings_page.bh_sel = rows.iter().position(|r| *r == BhRow::Tabs).unwrap();
    press(&mut app, "l");
    render(&mut app, 160, 45);
    click_on(&mut app, crate::hit::Target::SettingsButton('d'));
    assert!(app.settings_page.draft.is_none());
    assert_eq!(app.settings.layout.tabs, "top");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn settings_history_tab_previews_restores_and_names_versions() {
    use crate::settings::DiffLine;
    let store = temp_store("settings-history");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    let before = app.editor.palette.get("red");
    app.editor.palette.set("red", Rgb(1, 2, 3));
    app.save(false);
    app.page = Page::Settings;
    render(&mut app, 160, 45);
    click_on(&mut app, crate::hit::Target::SettingsTab(2));
    // palettes.json in the file list
    app.settings_page.hist_file = crate::settings::FILES
        .iter()
        .position(|f| *f == "palettes.json")
        .unwrap();
    assert_eq!(app.settings_page.tab, crate::settings::Tab::History);
    let versions = app.versions();
    assert_eq!(versions.len(), 1);
    // red, and the colours derived from it
    assert!(versions[0].1.summary.starts_with("One: ") && versions[0].1.summary.contains(", red,"));
    // the preview: red as it is now → as it was
    let view = app.diff_view();
    assert!(view.lines.contains(&DiffLine::Changed {
        depth: 2,
        key: "red".into(),
        now: "#010203".into(),
        then: before.hex(),
    }));
    let (now, then) = view.palette.unwrap();
    assert!(now.contains(&("red".into(), Rgb(1, 2, 3))) && then.contains(&("red".into(), before)));
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Restore preview") && screen.contains("One: bg_visual_red"),
        "{screen}"
    );
    render(&mut app, 100, 30); // stacked
    // A shows everything
    press(&mut app, "A");
    assert!(app.diff_view().lines.len() > 10);
    // r asks, y restores (and what was there goes to the history)
    press(&mut app, "r");
    assert!(matches!(app.overlay, Overlay::Ask { .. }));
    render(&mut app, 160, 45);
    click_on(&mut app, crate::hit::Target::ConfirmChoice('y'));
    assert_eq!(app.editor.palette.get("red"), before);
    assert!(!app.dirty());
    assert_eq!(
        Store::at(dir.clone()).load("One").unwrap().get("red"),
        before
    );
    assert_eq!(app.versions().len(), 2);
    // <leader>u undoes the restore
    press(&mut app, " u");
    assert_eq!(app.editor.palette.get("red"), Rgb(1, 2, 3));
    // n names the current file; named versions come first, d deletes them
    press(&mut app, "nmy reds\n");
    let versions = app.versions();
    assert_eq!(versions[0].1.name.as_deref(), Some("my reds"));
    app.settings_page.hist_sel = 1;
    press(&mut app, "d");
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("Only named"))
    );
    app.settings_page.hist_sel = 0;
    press(&mut app, "dy");
    assert!(app.versions().iter().all(|(_, s)| s.name.is_none()));
    // other files: the list follows
    press(&mut app, "hj");
    assert_eq!(
        crate::settings::FILES[app.settings_page.hist_file],
        "fonts.json"
    );
    assert!(app.versions().is_empty());
    assert!(render(&mut app, 160, 45).contains("No earlier versions yet"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ctrl_c_quits_unless_the_keymap_uses_it() {
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let mut app = test_app(temp_store("ctrl-c"), Start::Named("One".into()));
    app.editor.palette.set("red", Rgb(1, 2, 3));
    app.on_key(ctrl_c);
    assert!(
        matches!(app.overlay, Overlay::Confirm(Pending::Quit)),
        "Vim: asks, then quits"
    );
    app.overlay = Overlay::None;
    // Emacs: C-c is the leader; C-c C-c applies
    app.keymap = Keymap::emacs();
    app.on_key(ctrl_c);
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.pending_keys.len(), 1);
    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.pending_keys.clear();
    // VS Code: Ctrl+C copies the colour in the colours pane
    app.keymap = Keymap::vscode();
    app.side = Side::Colours;
    app.focus = Focus::List;
    app.on_key(ctrl_c);
    assert!(matches!(app.overlay, Overlay::None), "copied, not quit");
    std::fs::remove_dir_all(app.store.dir.clone()).unwrap();
}

#[test]
fn hover_look_is_configurable_and_pane_borders_light_up() {
    use crate::app::state::nav::{BhRow, LookId};
    use crate::behaviour::behaviour_rows;
    use crate::hit::{Target as T, Underline};
    use ratatui::style::{Color, Modifier};
    let store = temp_store("hover-look");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
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
    app.settings_page.tab = crate::settings::Tab::Behaviour;
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
    let target = crate::behaviour::theme_keys()
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
    let at_now = crate::behaviour::theme_keys()
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
    crate::hit::set_looks(looks);
    let s = crate::hit::hover(ratatui::style::Style::new());
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
    crate::hit::set_looks(Default::default());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn selecting_a_colour_lights_up_its_derivation_relatives() {
    use crate::hit::Target as T;
    let mut app = test_app(temp_store("related"), Start::Named("One".into()));
    let lit = |app: &mut App, key: &str| -> bool {
        let mut t = Terminal::new(TestBackend::new(200, 60)).unwrap();
        t.draw(|f| crate::ui::draw(f, app)).unwrap();
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
        t.draw(|f| crate::ui::draw(f, app)).unwrap();
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
fn settings_tabs_take_the_keyboard_at_the_list_edge() {
    let store = temp_store("tab-focus");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    app.page = Page::Settings;
    // top (the default): up from the first row reaches the tabs
    press(&mut app, "k");
    assert!(app.settings_page.tabs_focused);
    press(&mut app, "l");
    assert_eq!(app.settings_page.tab, crate::settings::Tab::Behaviour);
    assert!(render(&mut app, 160, 45).contains("back to the list"));
    press(&mut app, "j");
    assert!(!app.settings_page.tabs_focused);
    // the first row is the first group's title (groups fold open / shut)
    assert_eq!(
        crate::behaviour::behaviour_rows()[app.settings_page.bh_sel],
        crate::app::state::nav::BhRow::Header("Keys")
    );
    press(&mut app, "jk");
    assert!(!app.settings_page.tabs_focused);
    // tabs at the bottom: down from the last row reaches them
    app.settings.layout.inner_tabs = "bottom".into();
    press(&mut app, "G");
    assert!(!app.settings_page.tabs_focused);
    press(&mut app, "j");
    assert!(app.settings_page.tabs_focused);
    press(&mut app, "l");
    assert_eq!(app.settings_page.tab, crate::settings::Tab::History);
    press(&mut app, "k");
    assert!(!app.settings_page.tabs_focused);
    let screen = render(&mut app, 160, 45);
    let lines: Vec<&str> = screen.lines().collect();
    assert!(
        lines[lines.len() - 3].contains("Keymaps"),
        "tabs at the bottom:\n{screen}"
    );
    std::fs::remove_dir_all(dir).unwrap();
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
fn preset_browser_previews_loads_creates_and_deletes() {
    use crate::hit::Target as T;
    let store = temp_store("presets");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    app.page = Page::Settings;
    let screen = render(&mut app, 160, 45);
    assert!(screen.contains("[ Vim ▾ ]"), "{screen}");
    // Enter on the preset button opens the list, on the active preset
    app.settings_page.km_sel = 0;
    press(&mut app, "\n");
    assert_eq!(app.settings_page.browse, Some(0));
    assert!(render(&mut app, 160, 45).contains("Vim (the current keymap)"));
    // j highlights Helix: the left side shows what would change
    press(&mut app, "j");
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Vim → Helix:") && screen.contains("differ"),
        "{screen}"
    );
    assert!(
        screen.contains("redo") && !screen.contains("main page"),
        "only differences:\n{screen}"
    );
    press(&mut app, "A");
    assert!(
        render(&mut app, 160, 45).contains("main page"),
        "everything"
    );
    // n makes a new preset from the highlighted one
    press(&mut app, "nMy Helix\n");
    let names = |app: &mut App| {
        app.show_draft(|a| a.presets.iter().map(|p| p.name.clone()).collect::<Vec<_>>())
    };
    assert!(names(&mut app).contains(&"My Helix".to_string()));
    let mine = names(&mut app)
        .iter()
        .position(|n| n == "My Helix")
        .unwrap();
    assert_eq!(app.settings_page.browse, Some(mine));
    // Enter loads it (unsaved); s saves
    press(&mut app, "\n");
    assert_eq!(app.settings_page.browse, None);
    assert_eq!(app.show_draft(|a| a.keymap.name.clone()), "My Helix");
    assert_eq!(app.keymap.name, "Vim", "not live yet");
    press(&mut app, "s");
    assert_eq!(app.keymap.name, "My Helix");
    let saved = crate::config::load(&dir.join("studio-config"), false);
    assert_eq!(saved.keymap.name, "My Helix");
    // shipped presets can't be deleted; yours can (asks first)
    app.settings_page.km_sel = 0;
    press(&mut app, "\n");
    assert_eq!(
        app.settings_page.browse,
        Some(mine),
        "opens on the active one"
    );
    press(&mut app, "ggd"); // Helix keys are live now: gg, ge
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("can't be deleted"))
    );
    press(&mut app, "ge");
    assert_eq!(app.settings_page.browse, Some(mine));
    press(&mut app, "dy");
    assert!(!names(&mut app).contains(&"My Helix".to_string()));
    assert_eq!(
        app.show_draft(|a| a.keymap.name.clone()),
        "Vim",
        "the active one went: Vim"
    );
    press(&mut app, "s");
    assert!(
        !crate::config::load(&dir.join("studio-config"), false)
            .presets
            .iter()
            .any(|p| p.name == "My Helix")
    );
    // the mouse: a click highlights, a second loads
    render(&mut app, 160, 45);
    click_on(&mut app, T::PresetRow(2));
    assert_eq!(app.settings_page.browse, Some(2));
    render(&mut app, 160, 45);
    click_on(&mut app, T::PresetRow(2));
    assert_eq!(app.show_draft(|a| a.keymap.name.clone()), "Emacs");
    press(&mut app, "D");
    // Esc closes the list first, then leaves the page
    app.settings_page.km_sel = 0;
    press(&mut app, "\n\x1b");
    assert_eq!((app.settings_page.browse, app.page), (None, Page::Settings));
    press(&mut app, "\x1b");
    assert_eq!(app.page, Page::Main);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn settings_groups_fold_and_only_names_are_underlined() {
    use crate::app::state::nav::BhRow;
    use crate::behaviour::visible_behaviour_rows;
    use crate::hit::Target as T;
    use crate::settings::KmRow;
    let mut app = test_app(temp_store("fold"), Start::Named("One".into()));
    app.page = Page::Settings;
    // Keymaps: Enter on a group's title closes it, again opens it
    let n = app.keymap_rows().len();
    let g = app
        .keymap_rows()
        .iter()
        .position(|r| *r == KmRow::Header(Context::Global))
        .unwrap();
    app.settings_page.km_sel = g;
    press(&mut app, "\n");
    assert!(app.keymap_rows().len() < n);
    assert!(render(&mut app, 160, 45).contains("▸ Everywhere"));
    press(&mut app, "l");
    assert_eq!(app.keymap_rows().len(), n);
    // the cursor stops on titles but skips blank lines
    press(&mut app, "k");
    assert_eq!(app.keymap_rows()[app.settings_page.km_sel], KmRow::Preset);
    // Behaviour: a click on the selected title folds it
    app.settings_page.tab = crate::settings::Tab::Behaviour;
    render(&mut app, 160, 45);
    let rows = visible_behaviour_rows(&app.settings_page.bh_closed);
    let hover = rows
        .iter()
        .position(|r| *r == BhRow::Header("Hover (everything clickable)"))
        .unwrap();
    app.settings_page.bh_sel = hover;
    render(&mut app, 160, 45);
    click_on(&mut app, T::SettingsRow(hover));
    assert!(
        app.settings_page
            .bh_closed
            .contains("Hover (everything clickable)")
    );
    assert!(visible_behaviour_rows(&app.settings_page.bh_closed).len() < rows.len());
    // hover: only the option's name is underlined, not the row
    let mut t = Terminal::new(TestBackend::new(160, 45)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let leader = visible_behaviour_rows(&app.settings_page.bh_closed)
        .iter()
        .position(|r| *r == BhRow::Leader)
        .unwrap();
    let r = app.hits.rect_of(T::SettingsRow(leader)).unwrap();
    mouse(&mut app, MouseEventKind::Moved, r.x + 5, r.y);
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let buf = t.backend().buffer().clone();
    let under = |x: u16| {
        buf[(x, r.y)]
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    };
    let row: String = (r.x..r.right())
        .map(|x| buf[(x, r.y)].symbol().to_string())
        .collect();
    let start = r.x + row.find("Leader").unwrap() as u16;
    assert!(under(start) && under(start + 10), "the name");
    assert!(
        !under(r.x) && !under(start + 45) && !under(start + 52),
        "not the padding or value:\n{row}"
    );
    std::fs::remove_dir_all(app.store.dir.clone()).unwrap();
}

#[test]
fn download_page_lists_searches_and_removes() {
    use crate::download::{Catalogue, Entry, Source};
    use crate::fonts_dl::{DlRow, DlStatus, DlTab};
    use crate::hit::Target as T;
    let store = temp_store("dl");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    let entry = |id: &str, family: &str, cat: &str, source: Source| Entry {
        id: id.into(),
        family: family.into(),
        category: cat.into(),
        license: "OFL-1.1".into(),
        weights: vec![400, 700],
        italic: false,
        variable: false,
        source,
        subset: "latin".into(),
    };
    app.dl.catalogue = Some(Catalogue {
        google: vec![
            entry("inter", "Inter", "sans-serif", Source::Google),
            entry("roboto-mono", "Roboto Mono", "monospace", Source::Google),
            entry("zen-loop", "Zen Loop", "display", Source::Google),
        ],
        nerd: vec![entry(
            "nerd-heavydata",
            "HeavyData Nerd Font",
            "nerd font",
            Source::Nerd {
                asset: "HeavyData".into(),
                url: "u".into(),
                size: 1,
            },
        )],
        nerd_tag: "v3.5.1".into(),
    });
    app.on_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
    assert_eq!(app.page, Page::Downloads);
    assert_eq!(app.contexts()[0], Context::Downloads);
    let screen = render(&mut app, 160, 45);
    assert!(
        screen.contains("Google Fonts")
            && screen.contains("Roboto Mono")
            && screen.contains("3 fonts"),
        "{screen}"
    );
    // a category: right steps to Sans
    press(&mut app, "l");
    assert_eq!(app.dl_rows(), vec![DlRow::Entry(0)]);
    press(&mut app, "h");
    // search: / then typing; Enter leaves the box
    press(&mut app, "/mono\n");
    assert_eq!(app.dl_rows(), vec![DlRow::Entry(1)]);
    assert!(render(&mut app, 160, 45).contains("Install"));
    // the preview: Roboto Mono shows code; t / L / + change it
    let screen = render(&mut app, 160, 45);
    assert!(screen.contains("Preview  Code ▾  Rust ▾  12pt"), "{screen}");
    press(&mut app, "L+");
    assert!(render(&mut app, 160, 45).contains("Preview  Code ▾  Python ▾  13pt"));
    // > pages through the code, < comes back
    press(&mut app, ">");
    let pv = app.dl.preview.as_ref().unwrap();
    assert!(pv.page > 0 && pv.page == pv.step, "{} {}", pv.page, pv.step);
    press(&mut app, "<");
    assert_eq!(app.dl.preview.as_ref().unwrap().page, 0);
    // the wheel over the preview moves three lines
    render(&mut app, 160, 45);
    let r = app.hits.rect_of(crate::hit::Target::DlPreview).unwrap();
    mouse(&mut app, MouseEventKind::ScrollDown, r.x + 2, r.y + 2);
    assert_eq!(app.dl.preview.as_ref().unwrap().page, 3);
    // the language dropdown: a click opens it on the current one,
    // a click on a row chooses (and starts at the top)
    click_on(&mut app, crate::hit::Target::PreviewMenu(1));
    assert!(matches!(
        app.overlay,
        Overlay::Menu {
            menu: 1,
            sel: 1,
            ..
        }
    ));
    assert!(render(&mut app, 160, 45).contains(" Haskell "));
    click_on(&mut app, crate::hit::Target::OverlayRow(10));
    assert_eq!(app.dl.preview.as_ref().unwrap().lang, 10);
    assert_eq!(app.dl.preview.as_ref().unwrap().page, 0);
    assert!(render(&mut app, 160, 45).contains("Code ▾  Haskell ▾"));
    // the kind dropdown: keys move, Enter chooses; a click outside closes
    click_on(&mut app, crate::hit::Target::PreviewMenu(0));
    press(&mut app, "j\n");
    assert!(matches!(app.overlay, Overlay::None));
    assert!(render(&mut app, 160, 45).contains("Preview  Icons ▾  13pt"));
    click_on(&mut app, crate::hit::Target::PreviewMenu(0));
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 1);
    assert!(matches!(app.overlay, Overlay::None));
    app.dl.preview.as_mut().unwrap().kind = Some(crate::samples::Kind::Code);
    press(&mut app, "t");
    assert!(render(&mut app, 160, 45).contains("Preview  Icons"));
    // a failed fetch says so
    app.dl
        .preview
        .as_mut()
        .unwrap()
        .families
        .insert("roboto-mono".into(), Err("offline".into()));
    assert!(render(&mut app, 160, 45).contains("Couldn't fetch it: offline"));
    app.dl.search.clear();
    // something in the list counts as installed and can be removed
    let list = app.installed_list_path();
    let mut m = serde_json::Map::new();
    m.insert(
        "Zen Loop".into(),
        serde_json::json!({"source": "google", "id": "zen-loop"}),
    );
    crate::download::write_list(&list, m).unwrap();

    let zen = app.dl_entries()[2].clone();
    let listed = crate::download::read_list(&list);
    assert_eq!(app.dl_status(&zen, &listed), DlStatus::Listed);
    // the Installed tab (a click) lists it; d removes it
    render(&mut app, 160, 45);
    click_on(&mut app, T::DlTab(2));
    assert_eq!(app.dl.tab, DlTab::Installed);
    assert_eq!(app.dl_rows(), vec![DlRow::Listed("Zen Loop".into())]);
    press(&mut app, "d");
    assert!(crate::download::read_list(&list).is_empty());
    assert!(app.dl_rows().is_empty());
    // Nerd Fonts tab, with the release (past the Tournament tab)
    press(&mut app, "]");
    assert_eq!(app.dl.tab, DlTab::Tournament);
    press(&mut app, "]");
    assert_eq!(app.dl.tab, DlTab::Google, "wraps around");
    press(&mut app, "]");
    assert!(render(&mut app, 160, 45).contains("release v3.5.1"));
    // up from the first row reaches the tabs
    press(&mut app, "k");
    assert!(app.dl.tabs_focused);
    press(&mut app, "j\x1b");
    assert_eq!(app.page, Page::Main);
    render(&mut app, 100, 40); // narrow
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_family_fills_every_role() {
    use crate::fonts::types::{Installed, Row};
    let store = temp_store("family");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.fonts.store.dir = dir.clone();
    let f = |family: &str, mono: bool, nerd: bool| Installed {
        family: family.into(),
        mono,
        nerd,
        bitmap: false,
        special: false,
    };
    app.fonts.set_installed(vec![
        f("JetBrains Mono", true, false),
        f("JetBrains Mono NL", true, false),
        f("JetBrainsMono Nerd Font", true, true),
        f("JetBrainsMono Nerd Font Mono", true, true),
        f("JetBrainsMono Nerd Font Propo", false, true),
        f("Inter", false, false),
    ]);
    press(&mut app, " 2");
    // the family's group row, then F
    let g = app
        .fonts
        .rows()
        .iter()
        .position(|r| matches!(r, Row::Group { name, .. } if name.starts_with("JetBrains")))
        .unwrap();
    app.fonts.list_sel = g;
    press(&mut app, "F");
    let fams: Vec<&str> = app
        .fonts
        .preset
        .fonts
        .iter()
        .map(|f| f.family.as_str())
        .collect();
    assert_eq!(
        fams,
        [
            "JetBrainsMono Nerd Font Mono",
            "JetBrainsMono Nerd Font Propo",
            "JetBrainsMono Nerd Font Propo"
        ]
    );
    assert!(!app.fonts.dirty(), "saved straight away");
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("From JetBrains")),
        "{:?}",
        app.messages.message
    );
    // a font in no group: the family is its own name, so Inter for all
    let i = app
        .fonts
        .installed
        .iter()
        .position(|f| f.family == "Inter")
        .unwrap();
    let inter = app
        .fonts
        .rows()
        .iter()
        .position(|r| matches!(r, Row::Font { index, .. } if *index == i))
        .unwrap();
    app.fonts.focus = crate::fonts::types::FontFocus::List;
    app.fonts.list_sel = inter;
    press(&mut app, "F");
    let fams: Vec<&str> = app
        .fonts
        .preset
        .fonts
        .iter()
        .map(|f| f.family.as_str())
        .collect();
    // Monospace and Icons keep theirs: Inter has no Nerd Font faces (the
    // test catalogue has no Nerd Font of it either, so nothing is offered)
    assert_eq!(
        fams,
        [
            "JetBrainsMono Nerd Font Mono",
            "Inter",
            "JetBrainsMono Nerd Font Propo"
        ]
    );
    assert!(!matches!(app.overlay, Overlay::Ask { .. }));
    assert!(
        app.messages
            .message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("Monospace, Icons kept")),
        "{:?}",
        app.messages.message
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_family_without_fitting_faces_offers_its_nerd_font() {
    use crate::download::{Catalogue, Entry, Source};
    use crate::fonts::types::{Installed, Row};
    let store = temp_store("offer");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.fonts.store.dir = dir.clone();
    app.fonts.set_installed(vec![Installed {
        family: "Fira Code".into(),
        mono: true,
        nerd: false,
        bitmap: false,
        special: false,
    }]);
    app.dl.catalogue = Some(Catalogue {
        nerd: vec![Entry {
            id: "nerd-firacode".into(),
            family: "FiraCode Nerd Font".into(),
            category: "nerd font".into(),
            license: String::new(),
            weights: Vec::new(),
            italic: false,
            variable: false,
            source: Source::Nerd {
                asset: "FiraCode".into(),
                url: "https://x/FiraCode.tar.xz".into(),
                size: 12_000_000,
            },
            subset: String::new(),
        }],
        ..Default::default()
    });
    press(&mut app, " 2");
    app.fonts.list_sel = app
        .fonts
        .rows()
        .iter()
        .position(|r| matches!(r, Row::Font { .. }))
        .unwrap();
    let before = app.fonts.preset.fonts.clone();
    press(&mut app, "F");
    assert_eq!(
        app.fonts.preset.fonts, before,
        "nothing fits: every role kept"
    );
    match &app.overlay {
        Overlay::Ask { question, then } => {
            assert_eq!(
                question,
                "Fira Code has no Nerd Font Mono, proportional or Nerd Font Propo face installed. Download FiraCode Nerd Font (12.0 MB)?"
            );
            assert_eq!(
                *then,
                crate::app::state::nav::Deferred::NerdForRoles {
                    id: "nerd-firacode".into(),
                    group: "Fira Code".into(),
                    roles: vec![0, 1, 2],
                }
            );
        }
        _ => panic!("no offer"),
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn sizes_belong_to_the_theme_and_are_saved_when_the_keys_stop() {
    let store = temp_store("sizes");
    let dir = store.dir.clone();
    std::fs::write(
        dir.join("fonts.json"),
        r#"{"version": 4, "presets": {"Base": {"monospace": {"family": "A"}, "proportional": {"family": "B"}, "icons": {"family": "C"}}, "Other": {"monospace": {"family": "D"}, "proportional": {"family": "E"}, "icons": {"family": "F"}}}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("themes.json"),
        r#"{"version": 2, "themes": {"T": {"palette": "One", "fonts": "Base", "sizes": {"monospace": 14, "proportional": 9, "icons": 9}}}}"#,
    )
    .unwrap();
    let mut app = test_app(store, Start::Named("T".into()));
    app.fonts.store.dir = dir.clone();
    app.themes.dir = dir.clone();
    app.open_theme("T");
    assert_eq!(app.fonts.preset.sizes(), [14, 9, 9], "the theme's sizes");
    // l on Icons: the size changes at once, and is saved and applied
    // once the keys stop (the run loop calls apply_sizes after 400 ms)
    press(&mut app, " 1");
    app.fonts.slot = 2;
    press(&mut app, "ll");
    assert_eq!(app.fonts.preset.sizes(), [14, 9, 11]);
    assert!(app.fonts.sized_at.is_some());
    assert!(!app.fonts.dirty(), "the preset isn't changed by a size");
    app.apply_sizes();
    assert!(app.fonts.sized_at.is_none());
    assert_eq!(app.themes.get("T").unwrap().sizes, Some([14, 9, 11]));
    assert!(!app.theme_dirty(), "saved: nothing left unsaved");
    // the preset file has no sizes
    let f = std::fs::read_to_string(dir.join("fonts.json")).unwrap();
    assert!(!f.contains("size"));
    // swapping in another preset keeps the theme's sizes
    app.fonts.open("Other").unwrap();
    assert_eq!(app.fonts.preset.sizes(), [14, 9, 11]);
    assert_eq!(app.fonts.preset.fonts[0].family, "D");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_readme_tab_renders_scrolls_and_jumps_between_headings() {
    let store = temp_store("readme");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    // the guide isn't next to the temp configs: give the tab its text
    let mut md = String::from("# Guide\n\nFirst part.\n\n");
    for i in 0..60 {
        md += &format!("- line {i}\n");
    }
    md += "\n## Second\n\n| a | b |\n| - | - |\n| `x` | y |\n";
    app.settings_page.readme = Some((None, Ok(md)));
    app.page = Page::Settings;
    press(&mut app, "]]]");
    assert_eq!(app.settings_page.tab, crate::settings::Tab::Readme);
    let screen = render(&mut app, 120, 30);
    assert!(
        screen.contains("README") && screen.contains("Guide") && screen.contains("• line 0"),
        "{screen}"
    );
    assert!(
        screen.contains("} /{ headings") || screen.contains("}/{ headings"),
        "{screen}"
    );
    press(&mut app, "}");
    let second = app.settings_page.readme_scroll;
    assert!(second > 60);
    let screen = render(&mut app, 120, 30);
    assert!(
        screen.lines().nth(3).is_some_and(|l| l.contains("Second")),
        "{screen}"
    );
    press(&mut app, "{");
    assert_eq!(app.settings_page.readme_scroll, 0);
    // at the top, up goes on to the tab strip
    press(&mut app, "k");
    assert!(app.settings_page.tabs_focused);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn messages_time_out_and_saves_can_pop_up() {
    let store = temp_store("messages");
    let dir = store.dir.clone();
    crate::history::init_for_test(dir.join("history"));
    let mut app = test_app(store, Start::Named("One".into()));
    let ago = |s: u64| Some(Instant::now() - Duration::from_secs(s));
    // info messages go after settings.messages.seconds (3 by default)
    app.messages.info("hello");
    assert!(!app.messages.expire(app.settings.messages.seconds));
    app.messages.message_at = ago(3);
    assert!(app.messages.expire(app.settings.messages.seconds));
    assert_eq!(app.messages.message, None);
    // errors stay until a key
    app.messages.error("bad");
    assert!(!app.messages.expire(app.settings.messages.seconds));
    assert!(app.messages.message.is_some());
    // 0: until a key
    app.settings.messages.seconds = 0;
    app.messages.info("stays");
    app.messages.message_at = ago(100);
    assert!(!app.messages.expire(app.settings.messages.seconds));
    app.settings.messages.seconds = 3;
    // saving the settings says "Saved"; switched to popup, that save
    // already shows in a box
    app.with_draft(|a| a.settings.messages.saved = "popup".into());
    app.settings_save();
    assert_eq!(app.messages.notice.as_deref(), Some("Saved"));
    assert_eq!(app.messages.message, None);
    assert!(render(&mut app, 160, 45).contains("press any key to dismiss"));
    // a key closes it and does nothing else
    let sel = app.sel;
    press(&mut app, "j");
    assert_eq!(app.messages.notice, None);
    assert_eq!(app.sel, sel);
    // so does a click
    app.notify_saved("Saved \"One\"");
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 1);
    assert_eq!(app.messages.notice, None);
    // bar: the status line, like any message
    app.settings.messages.saved = "bar".into();
    app.notify_saved("Saved");
    assert_eq!(app.messages.message, Some(("Saved".into(), false)));
    assert_eq!(app.messages.notice, None);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn status_line_hints_sit_at_the_right() {
    let store = temp_store("hints");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let last_row = |app: &mut App| render(app, 160, 45).lines().last().unwrap().to_string();
    // tabs at the bottom: tabs and theme at the left, hints at the right end
    let row = last_row(&mut app);
    assert!(row.trim_end().ends_with("? keys"), "{row}");
    assert!(row.contains("F1"));
    // a message sits after the tabs; the hints stay at the right
    app.messages.info("Nothing to undo");
    let row = last_row(&mut app);
    let (msg, keys) = (
        row.find("Nothing to undo").unwrap(),
        row.find("? keys").unwrap(),
    );
    assert!(row.find("F1").unwrap() < msg && msg < keys, "{row}");
    assert!(row.trim_end().ends_with("? keys"), "{row}");
    // tabs on top: the bottom line keeps the message and the hints
    app.settings.layout.tabs = "top".into();
    let screen = render(&mut app, 160, 45);
    let (top, bottom) = (
        screen.lines().next().unwrap(),
        screen.lines().last().unwrap(),
    );
    assert!(top.contains("F1") && !top.contains("? keys"), "{top}");
    assert!(bottom.starts_with(" Nothing to undo"), "{bottom}");
    assert!(bottom.trim_end().ends_with("? keys"), "{bottom}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn palette_list_hangs_from_its_row_and_previews() {
    use crate::hit::Target as T;
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
fn behaviour_tab_sets_up_prefetching() {
    use crate::app::state::nav::BhRow;
    use crate::behaviour::behaviour_rows;
    let store = temp_store("prefetch-rows");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.settings_page.bh_closed.clear();
    let at = |r: BhRow| behaviour_rows().iter().position(|x| *x == r).unwrap();
    let run = |app: &mut App, r: BhRow| {
        app.with_draft(|a| {
            a.settings_page.bh_sel = at(r);
            a.run_behaviour_tab(Action::Confirm);
        })
    };
    // prefetching: off by default, monospace chosen; edits wait for Save
    assert!(!app.settings.fonts_page.prefetch);
    run(&mut app, BhRow::Prefetch);
    run(&mut app, BhRow::PrefetchCategory(5));
    assert!(!app.settings.fonts_page.prefetch, "not live before Save");
    app.settings_save();
    assert!(app.settings.fonts_page.prefetch);
    assert_eq!(
        app.settings.fonts_page.prefetch_categories,
        ["monospace", "nerd font"]
    );
    // with the catalogue, each category shows what it takes
    let nerd = crate::download::Source::Nerd {
        asset: "Hack".into(),
        url: String::new(),
        size: 40_000_000,
    };
    let mut hack = crate::download::Entry {
        id: "hack".into(),
        family: "Hack".into(),
        category: "nerd font".into(),
        license: String::new(),
        weights: vec![400],
        italic: false,
        variable: false,
        source: nerd,
        subset: "latin".into(),
    };
    app.dl.catalogue = Some(crate::download::Catalogue {
        nerd: vec![hack.clone()],
        ..Default::default()
    });
    assert_eq!(
        app.behaviour_value(BhRow::PrefetchCategory(5)),
        "yes  (1 font: 40.0 MB to fetch, about 2.5 MB kept)"
    );
    hack.category = "monospace".into();
    hack.source = crate::download::Source::Google;
    app.dl.catalogue.as_mut().unwrap().google = vec![hack];
    assert_eq!(
        app.behaviour_value(BhRow::PrefetchCategory(0)),
        "yes  (1 font, about 100 KB)"
    );
    // Clear asks first (the answer isn't given here: it would delete
    // the real folder)
    run(&mut app, BhRow::ClearPreviews);
    assert!(matches!(
        app.overlay,
        Overlay::Ask {
            then: crate::app::state::nav::Deferred::ClearPreviews,
            ..
        }
    ));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn hints_and_previews_toggle_per_page() {
    let store = temp_store("toggles");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let screen = |app: &mut App| render(app, 160, 45);
    // Main: <leader>tp hides the preview; the columns take its room
    assert!(screen(&mut app).contains("Preview"));
    press(&mut app, " tp");
    assert!(!app.layout.main_preview_visible);
    let s = screen(&mut app);
    assert!(!s.contains("Space 6 / Space 7"), "{s}");
    let pane = app.layout.palette_pane.unwrap();
    assert_eq!(pane.x + pane.width, 160, "colours reach the right edge");
    // jumping to nvim shows it again
    press(&mut app, " 6");
    assert!(app.layout.main_preview_visible);
    // (nvim has the keyboard now: take it back)
    app.preview.focus = None;
    press(&mut app, " tp");
    // the Fonts page's preview is its own
    app.on_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
    assert!(app.layout.fonts_preview_visible && !app.layout.main_preview_visible);
    press(&mut app, " tp");
    assert!(!app.layout.fonts_preview_visible && !app.layout.main_preview_visible);
    // <leader>th hides the key hints anywhere; messages still show
    assert!(screen(&mut app).contains("? keys"));
    press(&mut app, " th");
    let s = screen(&mut app);
    assert!(!s.contains("? keys"), "{s}");
    app.messages.info("Hello");
    assert!(screen(&mut app).contains("Hello"));
    // the settings say how a run starts
    let mut config = crate::config::settings::Settings::default();
    config.layout.key_hints = "hidden".into();
    config.layout.main_preview = "hidden".into();
    let loaded = crate::config::Loaded {
        keymap: Keymap::vim(),
        presets: Keymap::builtin(),
        settings: config,
        warnings: Vec::new(),
        dir: dir.join("studio-config"),
    };
    let mut app2 =
        App::with_config(Store::at(dir.clone()), Start::Named("One".into()), loaded).unwrap();
    app2.preview.set_headless();
    assert!(
        !app2.layout.hints_visible
            && !app2.layout.main_preview_visible
            && app2.layout.fonts_preview_visible
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn behaviour_cursor_keeps_the_swatch() {
    use crate::app::state::nav::{BhRow, LookId};
    use crate::behaviour::visible_behaviour_rows;
    use ratatui::style::Modifier;
    let store = temp_store("cursor-swatch");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    app.page = Page::Settings;
    app.settings_page.tab = crate::settings::Tab::Behaviour;
    app.settings_page.bh_closed.clear();
    let row = BhRow::HighlightColour(LookId::Hover);
    app.settings_page.bh_sel = visible_behaviour_rows(&app.settings_page.bh_closed)
        .iter()
        .position(|r| *r == row)
        .unwrap();
    let mut t = Terminal::new(TestBackend::new(160, 60)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let buf = t.backend().buffer().clone();
    let label = App::behaviour_label(row);
    let y = (0..60)
        .find(|&y| {
            let s: String = (0..160).map(|x| buf[(x, y)].symbol().to_string()).collect();
            s.contains(&label) && s.contains("██")
        })
        .expect("the row is drawn");
    let cells: Vec<_> = (0..160).map(|x| buf[(x, y)].clone()).collect();
    let reversed = |c: &ratatui::buffer::Cell| c.modifier.contains(Modifier::REVERSED);
    // the swatch keeps its colour and isn't reversed
    let swatch = cells.iter().find(|c| c.symbol() == "█").unwrap();
    assert!(!reversed(swatch));
    assert_eq!(
        swatch.fg,
        app.theme_colour(&app.settings.hover.look.highlight_colour)
    );
    // the label is reversed, the padding after it isn't
    let start = (0..160).find(|&x| buf[(x, y)].symbol() == "H").unwrap() as usize;
    assert!(reversed(&cells[start]));
    let end = start + label.chars().count();
    assert!(!reversed(&cells[end + 1]), "padding after the label");
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

#[test]
fn installed_fonts_preview_from_their_files() {
    // a real font file to stand in (skipped without one, e.g. in the
    // Nix build's sandbox)
    let Some(file) = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", "monospace"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .filter(|f| f.ends_with(".ttf") || f.ends_with(".otf"))
    else {
        eprintln!("skipped: no font file");
        return;
    };
    let store = temp_store("installed-preview");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let fonts = crate::paths::installed_fonts_dir().join("test-mono");
    std::fs::create_dir_all(&fonts).unwrap();
    let ext = if file.ends_with(".otf") { "otf" } else { "ttf" };
    std::fs::copy(&file, fonts.join(format!("TestMono-Regular.{ext}"))).unwrap();
    let list = app.installed_list_path();
    std::fs::create_dir_all(list.parent().unwrap()).unwrap();
    std::fs::write(
        &list,
        r#"{"version": 1, "fonts": {"Test Mono": {"source": "fontsource", "id": "test-mono"}}}"#,
    )
    .unwrap();
    app.page = Page::Downloads;
    app.dl.tab = crate::fonts_dl::DlTab::Installed;
    app.dl.listed = None;
    app.dl.sel = 0;
    // the highlighted installed font is the preview's, with code by default
    let e = app.preview_entry().unwrap();
    assert_eq!(
        (e.id.as_str(), e.category.as_str()),
        ("test-mono", "monospace")
    );
    // its family comes from its files, without fetching anything
    let start = std::time::Instant::now();
    loop {
        app.poll_downloads();
        if let Some(r) = app
            .dl
            .preview
            .as_ref()
            .and_then(|p| p.families.get("test-mono"))
        {
            assert!(r.as_ref().is_ok_and(|f| !f.is_empty()), "{r:?}");
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "no family name");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::fs::remove_dir_all(fonts).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn prefetching_keeps_the_main_loop_quick() {
    use crate::download::{Catalogue, Entry, Msg, Source};
    use std::sync::mpsc::Sender;
    // downloads answer at once (no network)
    fn stub(e: Entry, tx: Sender<Msg>) {
        let _ = tx.send(Msg::Preview {
            id: e.id,
            result: Ok(e.family),
        });
    }
    let store = temp_store("prefetch-loop");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let entry = |i: usize, category: &str, source: Source| Entry {
        id: format!("font-{i}"),
        family: format!("Font {i}"),
        category: category.into(),
        license: String::new(),
        weights: vec![400],
        italic: false,
        variable: false,
        source,
        subset: "latin".into(),
    };
    let nerd = |i| Source::Nerd {
        asset: format!("N{i}"),
        url: String::new(),
        size: 1,
    };
    app.dl.catalogue = Some(Catalogue {
        google: (0..2000)
            .map(|i| entry(i, "monospace", Source::Google))
            .collect(),
        nerd: (2000..2005)
            .map(|i| entry(i, "nerd font", nerd(i)))
            .collect(),
        nerd_tag: String::new(),
    });
    app.settings.fonts_page.prefetch = true;
    app.settings.fonts_page.prefetch_categories = vec!["monospace".into(), "nerd font".into()];
    app.prefetch.fetch = Some(stub);
    let (start, mut slowest) = (Instant::now(), Duration::ZERO);
    while app.prefetch.done < 2005 || app.prefetch.active() {
        let t = Instant::now();
        app.poll_downloads();
        app.poll_prefetch();
        slowest = slowest.max(t.elapsed());
        assert!(
            app.prefetch.nerd_in_flight.len() <= 1,
            "archives one at a time"
        );
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "stuck at {}",
            app.prefetch.done
        );
    }
    assert_eq!((app.prefetch.done, app.prefetch.total), (2005, 2005));
    // a tick never takes long (generous for a debug build)
    assert!(
        slowest < Duration::from_millis(20),
        "a tick took {slowest:?}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_tournament_of_installed_fonts_from_setup_to_results() {
    use crate::tour_page::Stage;
    let store = temp_store("tournament");
    let dir = store.dir.clone();
    let mut app = test_app(store, Start::Named("One".into()));
    let _ = std::fs::remove_dir_all(crate::tournament::dir());
    app.fonts.installed = (0..5)
        .map(|i| crate::fonts::types::Installed {
            family: format!("Test Mono {i}"),
            mono: true,
            nerd: false,
            bitmap: false,
            special: false,
        })
        .collect();
    app.page = Page::Downloads;
    app.dl.tab = crate::fonts_dl::DlTab::Tournament;
    let screen = |app: &mut App| render(app, 160, 45);
    assert!(screen(&mut app).contains("Installed monospaced"));
    // the installed monospaced pool: what it takes, then it starts
    app.tour().sel = 6;
    press(&mut app, "\n");
    let Overlay::Ask { question, .. } = &app.overlay else {
        panic!("asks first")
    };
    assert!(
        question.contains("5 fonts, 4 picks; nothing to fetch"),
        "{question}"
    );
    press(&mut app, "y");
    assert_eq!(app.tour().stage, Stage::Fetching);
    app.poll_tournament();
    assert_eq!(app.tour().stage, Stage::Playing);
    let s = screen(&mut app);
    assert!(s.contains("round 1 of 3") && s.contains("← A"), "{s}");
    assert!(!s.contains("Test Mono"), "names hidden");
    press(&mut app, "n");
    assert!(screen(&mut app).contains("Test Mono"));
    // picks: four in all; u takes one back
    press(&mut app, "hl");
    press(&mut app, "u");
    assert_eq!(app.tour().t.as_ref().unwrap().picks.len(), 1);
    press(&mut app, "lhh");
    assert_eq!(app.tour().stage, Stage::Results);
    let s = screen(&mut app);
    assert!(
        s.contains("the top 10") && s.contains("1st") && s.contains("2nd"),
        "{s}"
    );
    // the bracket
    press(&mut app, "b");
    assert!(screen(&mut app).contains("Round 1 (5 fonts)"));
    press(&mut app, "\x1b");
    assert!(!app.tour().bracket);
    // back to the setup: the result is listed
    press(&mut app, "\x1b");
    assert_eq!(app.tour().stage, Stage::Setup);
    assert!(screen(&mut app).contains("won by Test Mono"));
    assert_eq!(app.tour_past().len(), 1);
    let _ = std::fs::remove_dir_all(crate::tournament::dir());
    std::fs::remove_dir_all(dir).unwrap();
}

/// End to end against the real data and commands: re-picks the active
/// preset's current fonts through the studio, which saves (unchanged) and
/// applies them. Run with NO_RESTART=1 so i3 and Ghostty aren't restarted:
/// `THEME_STUDIO_REAL_SWITCH=1 NO_RESTART=1 cargo test studio_applies_fonts --
/// --ignored --nocapture` (without the first, tests never run the scripts).
#[test]
#[ignore]
fn studio_applies_fonts_like_font_switch() {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let store = Store::locate();
    eprintln!(
        "data {} share {}",
        store.dir.display(),
        store.share.display()
    );
    let config = crate::config::load(&crate::config::dir(), false);
    let mut app = App::with_config(store, Start::Active, config).unwrap();
    app.preview.set_headless();
    app.fonts.refresh_installed();
    let key = |app: &mut App, c: KeyCode| app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
    for role in 0..3 {
        app.focus_pane(1);
        app.fonts.focus = crate::fonts::types::FontFocus::Slots;
        app.fonts.slot = role;
        let before = app.fonts.preset.fonts[role].clone();
        key(&mut app, KeyCode::Enter); // the list, at the current font
        eprintln!(
            "role {role}: {} {}  highlighted {:?}",
            before.family,
            before.size,
            app.fonts
                .highlighted()
                .map(|i| app.fonts.installed[i].family.clone())
        );
        key(&mut app, KeyCode::Enter); // use it
        eprintln!(
            "  -> {:?}  message {:?}",
            app.fonts.preset.fonts[role], app.messages.message
        );
    }
}
