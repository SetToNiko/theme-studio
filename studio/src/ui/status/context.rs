//! The hints for the current control, picked by where the focus is.

use super::hint::hint;
use super::rows::{colours_hints, downloads_hints, fonts_hints, settings_hints};
use crate::app::app_state::App;
use crate::app::state::nav::{Focus, Page, Side};
use crate::fonts::types::FontFocus;
use crate::keymap::action::Action as A;
use crate::keymap::context::Context;
use crate::ui::widgets::friendly;

/// The hints for the current control, picked by where the focus is.
pub(super) fn context_hints(app: &App) -> Vec<String> {
    let fonts = app.side == Side::Fonts;
    if app.layout.resize_mode {
        resize_hints(app)
    } else if app.layout.page_tabs_focused {
        page_tabs_hints(app)
    } else if app.page == Page::Main && app.layout.palette_menu_focused && app.side == Side::Colours
    {
        palette_menu_hints(app)
    } else if app.page == Page::Downloads {
        downloads_hints(app)
    } else if app.page == Page::Settings {
        settings_hints(app)
    } else if fonts && app.fonts.focus == FontFocus::Search {
        vec!["type to search".into(), "Enter/Esc done".into()]
    } else if app.preview.focus.is_some() {
        preview_focus_hints(app)
    } else if !fonts && app.focus == Focus::Hex {
        hex_hints()
    } else if fonts {
        fonts_hints(app)
    } else {
        colours_hints(app)
    }
}

/// Hints while the Palette dropdown row has the keyboard.
fn palette_menu_hints(app: &App) -> Vec<String> {
    [
        hint(app, &[A::Confirm], "palettes"),
        hint(app, &[A::Down], "colours"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Hints while a hex colour is typed.
fn hex_hints() -> Vec<String> {
    vec![
        "type #rrggbb".into(),
        "Enter set".into(),
        "Esc cancel".into(),
    ]
}

/// Hints while the page tabs have the keyboard.
fn page_tabs_hints(app: &App) -> Vec<String> {
    let back = if app.settings.layout.tabs_on_top() {
        A::Down
    } else {
        A::Up
    };
    [
        hint(app, &[A::Left, A::Right], "pages"),
        hint(app, &[back], "back to the page"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Hints while the keys go to a preview window.
fn preview_focus_hints(app: &App) -> Vec<String> {
    let out = app
        .keymap
        .key_for(&[Context::Global], A::PreviewCycle)
        .unwrap_or_default();
    vec![
        "keys go to the preview window (Ctrl+C too)".into(),
        format!("{} next window / back", friendly(&out)),
    ]
}

/// Hints for resize mode: the keys come from the resize context only.
fn resize_hints(app: &App) -> Vec<String> {
    let k = |a| {
        app.keymap
            .key_for(&[Context::Resize], a)
            .map(|s| friendly(&s))
            .unwrap_or_default()
    };
    vec![
        "RESIZE".into(),
        format!("{}/{} side edge", k(A::Left), k(A::Right)),
        format!("{}/{} lower edge", k(A::Down), k(A::Up)),
        "capitals by 5".into(),
        format!("{} reset", k(A::ResetSizes)),
        format!("{} done", k(A::Back)),
    ]
}
