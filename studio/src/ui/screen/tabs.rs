//! The page tabs and the theme name in one row.
//!
//! The row is on top of the window, or at the left of the status line. A click
//! on a tab opens its page, and a click on the theme name opens the theme list.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::app_state::App;
use crate::app::state::hit::{Target, hover};
use crate::app::state::nav::Page;
use crate::keymap::action::Action;
use crate::keymap::context::Context;
use crate::ui::widgets::{friendly, to_u16};

/// Draws the page tabs and the theme name in one row; returns the width used.
pub fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) -> u16 {
    let mut x = area.x;
    for (label, action, page) in [
        ("Main", Action::PageMain, Page::Main),
        ("Fonts", Action::PageDownloads, Page::Downloads),
        ("Settings", Action::PageSettings, Page::Settings),
    ] {
        x += draw_page_tab(f, app, area, x, (label, action, page));
    }
    x += draw_theme_tab(f, app, area, x);
    x - area.x
}

/// Draws one page tab at column `x`; returns its width.
// Long function (reason verified, phase 6): one tab: its text, width, style and click area depend on each other in this order.
fn draw_page_tab(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    x: u16,
    (label, action, page): (&str, Action, Page),
) -> u16 {
    let key = app
        .keymap
        .key_for(&[Context::Global], action)
        .map(|k| friendly(&k))
        .unwrap_or_default();
    // a dot on Settings while it has unsaved changes
    let dot = if page == Page::Settings && app.settings_page.draft.is_some() {
        " ●"
    } else {
        ""
    };
    let text = format!(" {key} {label}{dot} ");
    let width = to_u16(text.chars().count()).min((area.x + area.width).saturating_sub(x));
    let rect = Rect {
        x,
        y: area.y,
        width,
        height: 1,
    };
    // with the keyboard on the tabs, the current one is yellow (like a
    // page's own tabs)
    let mut style = match (app.page == page, app.layout.page_tabs_focused) {
        (true, true) => Style::new().black().on_yellow().bold(),
        (true, false) => Style::new().reversed().bold(),
        _ => Style::new().dark_gray(),
    };
    if app.hits.add(rect, Target::PageTab(page)) {
        style = hover(style);
    }
    f.render_widget(Paragraph::new(Span::styled(text, style)), rect);
    width
}

/// Draws the theme name and its unsaved dot at column `x` (a click opens the
/// theme list); returns its width, 0 when there is no room.
fn draw_theme_tab(f: &mut Frame, app: &mut App, area: Rect, x: u16) -> u16 {
    let dot = if app.theme_dirty() { " ●" } else { "" };
    let label = format!("  Theme: {}{dot} ", app.theme);
    let width = to_u16(label.chars().count()).min((area.x + area.width).saturating_sub(x));
    if width == 0 {
        return 0;
    }
    let rect = Rect {
        x,
        y: area.y,
        width,
        height: 1,
    };
    let mut line = Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("Theme: {}", app.theme), Style::new().bold()),
        Span::styled(dot, Style::new().yellow()),
        Span::raw(" "),
    ]);
    if app.hits.add(rect, Target::ThemeName) {
        line = crate::app::state::hit::hover_line(line, 1);
    }
    f.render_widget(Paragraph::new(line), rect);
    width
}
