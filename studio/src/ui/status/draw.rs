//! The status line: the page tabs, a message, and the key hints.
//!
//! `draw_status` puts the pieces side by side: the tabs (unless they are on
//! top), then the message at the left, then the key hints at the right. The
//! hints depend on where the focus is: `context.rs` picks the screen, and
//! `rows.rs` has one small function per screen that lists the actions to show.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::context::context_hints;
use super::fit::fit_hints;
use super::hint::{general_hints, hint};
use crate::app::App;
use crate::keymap::action::Action as A;
use crate::ui::screen::tabs::draw_tabs;
use crate::ui::widgets::{to_u16, truncate};

/// Draws the status line: tabs, then the message, then the key hints.
// Long function (reason verified, phase 6): the tabs, the message and the right-aligned hints share one row, and each width depends on the widths already used on it (tabs, then message, then hints).
pub fn draw_status(f: &mut Frame, app: &mut App, area: Rect) {
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

/// The status line without key hints: just the message, if any.
fn draw_message_only(f: &mut Frame, area: Rect, message: Option<Span<'static>>) {
    if let Some(m) = message {
        f.render_widget(Paragraph::new(Line::from(m)), area);
    }
}
