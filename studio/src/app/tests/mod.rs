//! Tests for the app, one file per area, with the shared set-up helpers.
//!
//! - `palette.rs`: palette editing (picker, pinning, derived colours, palette list)
//! - `layout.rs`: window sizes, panes, borders, number keys, status line, toggles
//! - `input.rs`: leader keys, mouse hover and clicks, hover looks
//! - `pages_settings.rs`: the Settings page (tabs, behaviour rows, keymap presets, guide)
//! - `pages_fonts.rs`: the Fonts page (roles, families, presets, sizes)
//! - `font_downloads.rs`: font downloads, installed-font previews, prefetching, the tournament
//! - `lifecycle.rs`: saving, opening and forking themes, quitting
//!
//! This file holds the helpers that more than one file uses. A child file gets
//! them with `use super::*;`.

use crate::app::app_state::App;
use crate::app::startup::Start;
use crate::app::state::nav::{Focus, Overlay, Page, Side};
use crate::keymap::key::Key;
use crate::keymap::map::Keymap;
use crate::model::color::Rgb;
use crate::model::palette::{GROUPS, Palette};
use crate::model::store::Store;
use crate::model::theme::ThemeStore;
use ratatui::layout::Rect;
use std::time::Instant;

use crate::model::store::to_jq_string;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use serde_json::{Map, Value};

mod font_downloads;
mod input;
mod layout;
mod lifecycle;
mod pages_fonts;
mod pages_settings;
mod palette;

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
        to_jq_string(&crate::model::store::data_file("palettes", m)),
    )
    .unwrap();
    Store::at(dir)
}

/// An app whose preview never starts nvim, the shell or yazi.
fn test_app(store: Store, start: Start) -> App {
    // the shipped defaults, never your ~/.config/theme-studio
    let config = crate::config::load::Loaded {
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
    t.draw(|f| crate::ui::screen::frame::draw(f, app)).unwrap();
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

fn mouse(app: &mut App, kind: MouseEventKind, x: u16, y: u16) {
    app.on_mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    });
}

/// Clicks the middle of wherever `t` was last drawn.
fn click_on(app: &mut App, t: crate::app::state::hit::Target) {
    let r = app
        .hits
        .rect_of(t)
        .unwrap_or_else(|| panic!("{t:?} not on screen"));
    let (x, y) = (r.x + r.width / 2, r.y + r.height / 2);
    mouse(app, MouseEventKind::Moved, x, y);
    mouse(app, MouseEventKind::Down(MouseButton::Left), x, y);
    mouse(app, MouseEventKind::Up(MouseButton::Left), x, y);
}
