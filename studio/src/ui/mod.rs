//! Drawing. The studio's own chrome uses named terminal colours (so it
//! follows the system theme); only swatches and the preview use the palette
//! being edited.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::app::state::hit::{Target, hover};
use crate::app::state::nav::Page;
use crate::keymap::action::Action;
use crate::keymap::context::Context;
use crate::model::color::Rgb;
mod pages;
pub mod pane_chrome;
pub mod widgets;
use pages::{draw_body_page, draw_main_page, draw_too_small};
use pane_chrome::pane_border;
use widgets::{friendly, to_u16};
mod overlays;
use overlays::dialogs;
pub mod layout;
mod status;
use layout::{Arrangement, arrange};
use status::draw_status;
pub mod desktop_preview;
mod page_bodies;
mod panes;

/// Draws the whole window: the tabs, the page for the window size, the status
/// line, then the overlays on top.
// Long function (reason verified, phase 6): the order of the drawing calls is the order on screen: tabs, page, status, then the overlays on top.
pub fn draw(f: &mut Frame, app: &mut App) {
    reset_frame_state(app);
    let full = f.area();
    // tabs on top: their own row; at the bottom they share the status line
    let top = app.settings.layout.tabs_on_top();
    let (tabs_row, area) = if top && full.height > 1 {
        let [tabs, rest] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(full);
        (Some(tabs), rest)
    } else {
        (None, full)
    };
    let arrangement = arrange(
        area,
        app.page,
        app.layout.picker_visible,
        app.layout.sample_visible,
        app.layout.main_preview_visible,
        &app.layout.sizes,
    );
    if let Arrangement::TooSmall { need_w, need_h } = arrangement {
        return draw_too_small(f, full, need_w, need_h + u16::from(top));
    }
    if let Some(tabs) = tabs_row {
        draw_tabs(f, app, tabs);
    }
    match arrangement {
        Arrangement::TooSmall { .. } => {}
        Arrangement::Page { body, status } => {
            draw_body_page(f, app, body);
            draw_status(f, app, status);
        }
        Arrangement::Main(l) => {
            draw_main_page(f, app, &l);
            draw_status(f, app, l.status);
        }
    }
    dialogs::draw_overlay(f, app, full);
    overlays::draw_pending(f, app, full);
    overlays::draw_notice(f, app, full);
}

/// Clears what the last frame left in `app`: each draw call fills it again.
fn reset_frame_state(app: &mut App) {
    app.hits.clear();
    // images still waiting for a size to settle set this again
    app.redraw_at = None;
    // set again if the Palette pane is drawn (the palette list hangs from it)
    app.layout.palette_pane = None;
    app.messages.hover_note = None;
    crate::app::state::hit::set_looks(app.looks(&app.settings));
}

/// Draws the page tabs and the theme name in one row; returns the width used.
fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) -> u16 {
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

fn focused_block(app: &App, area: Rect, title: Line<'static>, focused: bool) -> Block<'static> {
    Block::bordered()
        .title(title)
        .border_style(pane_border(app, area, focused))
}

/// A swatch: `width` cells of background colour.
fn swatch(c: Rgb, width: usize) -> Span<'static> {
    Span::styled(" ".repeat(width), Style::new().bg(c.color()))
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>() + "…"
    }
}
