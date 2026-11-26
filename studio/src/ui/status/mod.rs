//! The status line: the page tabs, a message, and the key hints.
//!
//! `draw_status` puts the pieces side by side: the tabs (unless they are on
//! top), then the message at the left, then the key hints at the right. The
//! hints depend on where the focus is: `context_hints` picks the screen, and
//! `rows.rs` has one small function per screen that lists the actions to show.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::widgets::{friendly, to_u16};
use super::{draw_tabs, truncate};
use crate::app::App;
use crate::app::state::nav::{Focus, Page, Side};
use crate::fonts::types::FontFocus;
use crate::keymap::Context;
use crate::keymap::action::Action;
use crate::keymap::action::Action as A;

mod rows;
use rows::{colours_hints, downloads_hints, fonts_hints, settings_hints};

/// Hint rows: the actions whose keys are shown, and the label for them.
type Rows<'a> = Vec<(&'a [Action], &'static str)>;

/// Draws the status line: tabs, then the message, then the key hints.
// Long function (reason verified, phase 6): the tabs, the message and the right-aligned hints share one row, and each width depends on the widths already used on it (tabs, then message, then hints).
pub(super) fn draw_status(f: &mut Frame, app: &mut App, area: Rect) {
    // the page tabs start the line unless they're on top (settings: layout.tabs)
    let used = if app.settings.layout.tabs_on_top() {
        0
    } else {
        draw_tabs(f, app, area)
    };
    let area = Rect {
        x: area.x + used,
        width: area.width.saturating_sub(used),
        ..area
    };
    let app = &*app;
    // the message at the left (after the tabs), the key hints at the right
    let message = status_message(app, area.width);
    // hidden (settings: layout.key-hints, or the toggle): none
    if !app.layout.hints_visible {
        return draw_message_only(f, area, message);
    }
    let message_w = message.as_ref().map_or(0, Span::width);
    // a message keeps a space between it and the hints
    let room = usize::from(area.width).saturating_sub(message_w + usize::from(message_w > 0));
    let last = hint(app, &[A::Help], "keys").unwrap_or_default();
    let hints = fit_hints(&context_hints(app), &general_hints(app), &last, room);
    if let Some(message) = message {
        f.render_widget(Paragraph::new(Line::from(message)), area);
    }
    let hints_w = to_u16(hints.chars().count()).min(area.width);
    let right = Rect {
        x: area.right() - hints_w,
        width: hints_w,
        ..area
    };
    f.render_widget(
        Paragraph::new(Span::styled(hints, Style::new().dark_gray())),
        right,
    );
}

/// The message to show, cut to `width`: a status message, else a note about
/// what the mouse is over, else the preview fetch progress.
fn status_message(app: &App, width: u16) -> Option<Span<'static>> {
    let width = usize::from(width);
    if let Some((msg, is_error)) = &app.messages.message {
        let style = if *is_error {
            Style::new().red().bold()
        } else {
            Style::new().green()
        };
        return Some(Span::styled(truncate(&format!(" {msg} "), width), style));
    }
    // otherwise, a note about what the mouse is over
    if let Some(note) = &app.messages.hover_note {
        return Some(Span::styled(
            truncate(&format!(" {note} "), width),
            Style::new().dark_gray(),
        ));
    }
    // otherwise, previews being fetched ahead of time
    let prefetch = &app.prefetch;
    prefetch.active().then(|| {
        Span::styled(
            format!(" fetching previews {}/{} ", prefetch.done, prefetch.total),
            Style::new().dark_gray(),
        )
    })
}

/// One hint, such as `"j/k move"`: the keys bound to `actions`, then `label`.
/// `None` when none of the actions has a key.
fn hint(app: &App, actions: &[Action], label: &str) -> Option<String> {
    let ctx = app.contexts();
    let keys: Vec<String> = actions
        .iter()
        .filter_map(|a| app.keymap.key_for(&ctx, *a))
        .collect();
    let keys: Vec<String> = keys.iter().map(|k| friendly(k)).collect();
    (!keys.is_empty()).then(|| format!("{} {label}", keys.join("/")))
}

/// Turns rows into hints; rows without a key are left out.
fn row_hints(app: &App, rows: Rows) -> Vec<String> {
    rows.into_iter()
        .filter_map(|(actions, label)| hint(app, actions, label))
        .collect()
}

/// The hints for the current control, picked by where the focus is.
fn context_hints(app: &App) -> Vec<String> {
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

/// The general hints, after the control's own: next pane, panels, save,
/// apply, open. The main page's keys don't work on the other pages, so the
/// other pages have none.
fn general_hints(app: &App) -> Vec<String> {
    let main = app.page == Page::Main;
    [
        hint(app, &[A::FocusNext], "next").filter(|_| main),
        panels_hint(app).filter(|_| main),
        hint(app, &[A::Save], "save").filter(|_| main),
        hint(app, &[A::Apply], "apply").filter(|_| main),
        hint(app, &[A::Open], "open").filter(|_| main),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The panel keys as one range, such as `"Space 1-7 panels"`.
fn panels_hint(app: &App) -> Option<String> {
    match (hint(app, &[A::Panel(1)], ""), hint(app, &[A::Panel(7)], "")) {
        // "Space 1-7 panels" rather than "Space 1-Space 7 panels"
        (Some(a), Some(b)) => {
            let (a, b) = (a.trim(), b.trim());
            let shared = a.len().saturating_sub(1);
            Some(if a.len() == b.len() && a[..shared] == b[..shared] {
                format!("{a}-{} panels", &b[shared..])
            } else {
                format!("{a}-{b} panels")
            })
        }
        _ => None,
    }
}

/// The status line without key hints: just the message, if any.
fn draw_message_only(f: &mut Frame, area: Rect, message: Option<Span<'static>>) {
    if let Some(m) = message {
        f.render_widget(Paragraph::new(Line::from(m)), area);
    }
}

/// The hints that fit in `budget` columns, as one string: the context hints,
/// then the general ones after a `│`.
fn fill_hints(context: &[String], general: &[String], budget: usize) -> String {
    let mut out = String::from(" ");
    let mut general_started = false;
    for (i, hint) in context.iter().chain(general.iter()).enumerate() {
        let sep = if i == 0 {
            ""
        } else if i == context.len() && !general_started {
            general_started = true;
            " │ "
        } else {
            "  "
        };
        if out.chars().count() + sep.chars().count() + hint.chars().count() > budget {
            break;
        }
        out.push_str(sep);
        out.push_str(hint);
    }
    out
}

/// As many hints as fit in `room` columns: the ones for the current control
/// first, then the general ones, and `last` (the help key) always at the end.
fn fit_hints(context: &[String], general: &[String], last: &str, room: usize) -> String {
    let budget = room.saturating_sub(last.chars().count() + 3);
    let mut out = fill_hints(context, general, budget);
    if out.trim().is_empty() {
        out = format!(" {last} ");
    } else {
        out.push_str("  ");
        out.push_str(last);
        out.push(' ');
    }
    // not even the help key fits: no hints at all
    if out.chars().count() > room {
        String::new()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::fit_hints;

    #[test]
    fn hints_shorten_to_fit_and_keep_help_last() {
        let s = |v: &[&str]| v.iter().map(ToString::to_string).collect::<Vec<_>>();
        let ctx = s(&["j/k move", "Enter edit", "p pin"]);
        let general = s(&["Tab next", "1-4 panels", "s save", "a apply", "o open"]);
        let wide = fit_hints(&ctx, &general, "? keys", 200);
        assert_eq!(
            wide,
            " j/k move  Enter edit  p pin │ Tab next  1-4 panels  s save  a apply  o open  ? keys "
        );
        let narrow = fit_hints(&ctx, &general, "? keys", 40);
        assert!(narrow.chars().count() <= 40, "{narrow}");
        assert!(
            narrow.starts_with(" j/k move  Enter edit") && narrow.ends_with("? keys "),
            "{narrow}"
        );
        assert_eq!(fit_hints(&ctx, &general, "? keys", 10), " ? keys ");
        assert_eq!(fit_hints(&ctx, &general, "? keys", 5), "");
        for room in 0..120 {
            assert!(fit_hints(&ctx, &general, "? keys", room).chars().count() <= room);
        }
    }
}
