//! Whole-page drawing: what fills the window for each page.
//!
//! `draw` in `ui/mod.rs` picks the arrangement for the window size; this
//! file draws the page inside it (Main, Download fonts, Settings) and the
//! "window too small" screen. The panes of the Main page live in `ui/panes/`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::layout::MainLayout;
use super::panes::colours::draw_list;
use super::panes::picker::draw_picker;
use super::panes::preview::draw_preview;
use super::widgets::to_u16;
use crate::app::App;
use crate::app::state::hit::{BorderId, Target};
use crate::app::state::nav::Page;
use crate::preview_term::types::Mode as PreviewMode;

/// The Main page: fonts, colour list, picker and preview, then the
/// draggable borders between them.
pub(super) fn draw_main_page(f: &mut Frame, app: &mut App, l: &MainLayout) {
    app.layout.wide = l.wide;
    crate::ui::page_bodies::fonts_column::draw_fonts(f, app, l.roles, l.font_list);
    if let Some(s) = l.sample {
        crate::ui::page_bodies::fonts_column::draw_sample(f, app, s);
    }
    draw_list(f, app, l.colour_list);
    if let Some(p) = l.picker {
        draw_picker(f, app, p);
    }
    // hidden: nothing drawn, and nvim and the shell keep their size
    if !l.preview.is_empty() {
        draw_preview(f, app, l.preview, l.wide);
    }
    draw_borders(f, app, l);
    app.layout.borders.clone_from(&l.borders);
}

/// The draggable borders: highlighted under the mouse and while dragged.
fn draw_borders(f: &mut Frame, app: &mut App, l: &MainLayout) {
    for b in &l.borders {
        if app.preview.mode == PreviewMode::Swatches && b.id == BorderId::PreviewSplit {
            continue;
        }
        let hovered = app.hits.add(b.hit, Target::Border(b.id));
        if hovered || app.layout.dragging == Some(b.id) {
            let buf = f.buffer_mut();
            for x in b.hit.x..b.hit.x + b.hit.width {
                for y in b.hit.y..b.hit.y + b.hit.height {
                    if let Some(c) = buf.cell_mut((x, y)) {
                        c.set_fg(Color::Yellow)
                            .set_style(Style::new().add_modifier(Modifier::BOLD));
                    }
                }
            }
        }
    }
}

/// The Download fonts and Settings pages (the body above the status line).
pub(super) fn draw_body_page(f: &mut Frame, app: &mut App, body: Rect) {
    if app.page == Page::Downloads {
        crate::ui::page_bodies::fonts_download::draw_downloads(f, app, body);
    } else if app.page == Page::Settings {
        // as edited; Save / Discard drawn with what's live
        let changes = app.draft_changes();
        app.show_draft(|app| crate::ui::page_bodies::settings::draw_settings(f, app, body));
        crate::ui::page_bodies::settings::draw_save_buttons(f, app, body, changes);
    } else {
        draw_page(f, app, body);
    }
}

/// The Download fonts page (filled in by phase 6).
fn draw_page(f: &mut Frame, app: &App, area: Rect) {
    let (title, text) = match app.page {
        Page::Downloads => (
            " Download fonts ",
            vec![
                "Searching and installing fonts from Google Fonts and Nerd Fonts comes here (plan phase 6).".to_string(),
            ],
        ),
        _ => (" Settings ", Vec::new()),
    };
    let lines: Vec<Line> = text
        .into_iter()
        .map(|t| Line::raw(format!(" {t}")))
        .collect();
    let block = Block::bordered()
        .title(title)
        .border_style(Style::new().yellow());
    f.render_widget(Paragraph::new(lines).block(block), area);
}
/// btop-style: the current size (red where it falls short) and what's needed.
pub(super) fn draw_too_small(f: &mut Frame, area: Rect, need_w: u16, need_h: u16) {
    let ok = |have: u16, need: u16| {
        if have >= need {
            Style::new().green()
        } else {
            Style::new().red().bold()
        }
    };
    let lines = vec![
        Line::styled("Window too small", Style::new().bold()),
        Line::from(vec![
            "Width = ".into(),
            Span::styled(area.width.to_string(), ok(area.width, need_w)),
            "  Height = ".into(),
            Span::styled(area.height.to_string(), ok(area.height, need_h)),
        ]),
        Line::raw(""),
        Line::styled("Needed:", Style::new().bold()),
        Line::raw(format!("Width = {need_w}  Height = {need_h}")),
    ];
    let h = to_u16(lines.len()).min(area.height);
    let r = Rect {
        x: area.x,
        y: area.y + (area.height - h) / 2,
        width: area.width,
        height: h,
    };
    f.render_widget(Paragraph::new(lines).centered(), r);
}
