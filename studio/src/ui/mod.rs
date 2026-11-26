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
use crate::keymap::Context;
use crate::keymap::action::Action;
use crate::model::color::Rgb;
mod pages;
pub mod pane_chrome;
pub mod widgets;
use pages::{draw_body_page, draw_main_page, draw_too_small};
use pane_chrome::pane_border;
use widgets::friendly;
mod overlays;
use overlays::dialogs;
pub mod layout;
mod status;
use layout::{Arrangement, arrange};
use status::draw_status;
pub mod desktop_preview;
mod page_bodies;
mod panes;

pub fn draw(f: &mut Frame, app: &mut App) {
    app.hits.clear();
    // images still waiting for a size to settle set this again
    app.redraw_at = None;
    // set again if the Palette pane is drawn (the palette list hangs from it)
    app.layout.palette_pane = None;
    app.messages.hover_note = None;
    crate::app::state::hit::set_looks(app.looks(&app.settings));
    let full = f.area();
    // tabs on top: their own row; at the bottom they share the status line
    let top = app.settings.layout.tabs_on_top();
    let (tabs_row, area) = if top && full.height > 1 {
        let [t, rest] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(full);
        (Some(t), rest)
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
        return draw_too_small(f, full, need_w, need_h + top as u16);
    }
    if let Some(t) = tabs_row {
        draw_tabs(f, app, t);
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

fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) -> u16 {
    let mut x = area.x;
    for (label, action, page) in [
        ("Main", Action::PageMain, Page::Main),
        ("Fonts", Action::PageDownloads, Page::Downloads),
        ("Settings", Action::PageSettings, Page::Settings),
    ] {
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
        let w = (text.chars().count() as u16).min((area.x + area.width).saturating_sub(x));
        let r = Rect {
            x,
            y: area.y,
            width: w,
            height: 1,
        };
        // with the keyboard on the tabs, the current one is yellow (like a
        // page's own tabs)
        let mut style = match (app.page == page, app.layout.page_tabs_focused) {
            (true, true) => Style::new().black().on_yellow().bold(),
            (true, false) => Style::new().reversed().bold(),
            _ => Style::new().dark_gray(),
        };
        if app.hits.add(r, Target::PageTab(page)) {
            style = hover(style);
        }
        f.render_widget(Paragraph::new(Span::styled(text, style)), r);
        x += w;
    }
    // the theme: its name and unsaved dot (a click opens the theme list)
    let dot = if app.theme_dirty() { " ●" } else { "" };
    let label = format!("  Theme: {}{dot} ", app.theme);
    let w = (label.chars().count() as u16).min((area.x + area.width).saturating_sub(x));
    if w > 0 {
        let r = Rect {
            x,
            y: area.y,
            width: w,
            height: 1,
        };
        let mut line = Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("Theme: {}", app.theme), Style::new().bold()),
            Span::styled(dot, Style::new().yellow()),
            Span::raw(" "),
        ]);
        if app.hits.add(r, Target::ThemeName) {
            line = crate::app::state::hit::hover_line(line, 1);
        }
        f.render_widget(Paragraph::new(line), r);
        x += w;
    }
    x - area.x
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
