//! Tests of the app lifecycle: saving, opening and forking themes, and quitting.

use super::state::nav::Pending;
use super::*;

#[test]
fn themes_save_open_and_fork() {
    let store = temp_store("save");
    let dir = store.dir.clone();
    crate::model::history::init_for_test(dir.join("history"));
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
