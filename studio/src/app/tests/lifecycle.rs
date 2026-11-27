//! Tests of the app lifecycle.
//!
//! The test `themes_save_open_and_fork` is now in `app/pages/themes/theme_actions.rs`.
//! Step P3.11 moves the Ctrl+C test to `app/input/actions.rs`.

use crate::app::startup::Start;
use crate::app::state::nav::Pending;
use crate::app::state::nav::{Focus, Overlay, Side};
use crate::app::test_support::*;
use crate::keymap::map::Keymap;
use crate::model::color::Rgb;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
