//! Tests that stay until their target files are free: the drawing, the status line, the number keys, and the layout arrangement.

use crate::app::app_state::App;
use crate::app::startup::Start;
use crate::app::state::nav::{Focus, Page, Side};
use crate::app::test_support::*;
use crate::model::color::Rgb;
use crate::preview_term::types::PaneId;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

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
    t.draw(|f| crate::ui::screen::frame::draw(f, &mut app))
        .unwrap();
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

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one resize walk: each terminal size is checked against the layout the previous size left"
)]
fn layout_three_columns_then_preview_row_then_too_small() {
    use crate::ui::layout::{Arrangement, arrange};
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
