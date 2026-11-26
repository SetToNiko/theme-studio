//! Drawing the Download fonts page (F2): tabs, search and category chips,
//! the list, and the highlighted font's details with Install / Remove.
//!
//! The details box is in `fonts_download_details.rs` and the preview in
//! `fonts_download_preview.rs`.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use serde_json::{Map, Value};

use super::fonts_download_details::draw_details;
use super::fonts_download_preview::draw_preview;
use crate::app::App;
use crate::app::state::hit::{Target, hover, hover_line, hover_row};
use crate::fonts::download_page::{CATEGORIES, DL_TABS, DlRow, DlStatus, DlTab, mb};
use crate::ui::truncate;
use crate::ui::widgets::{to_u8, to_u16};

/// The status of a font as a short coloured text ("✓ installed", "↓ 40%").
pub(super) fn status_span(status: &DlStatus) -> Span<'static> {
    match status {
        DlStatus::None => Span::raw(""),
        DlStatus::Downloading(done, total) if *total > 0 => {
            Span::styled(format!("↓ {}%", done * 100 / total), Style::new().yellow())
        }
        DlStatus::Downloading(done, _) => {
            Span::styled(format!("↓ {}", mb(*done)), Style::new().yellow())
        }
        DlStatus::Listed => Span::styled("✓ installed", Style::new().green()),
        DlStatus::System => Span::styled("on the system", Style::new().dark_gray()),
    }
}

/// Splits the page inside into the tab row and the body: the tabs go on top
/// or at the bottom (the in-page tabs setting).
fn tabs_and_body(app: &App, inner: Rect) -> (Rect, Rect) {
    if app.settings.layout.inner_tabs_on_top() {
        let [tabs, _, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(inner);
        (tabs, body)
    } else {
        let [body, _, tabs] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);
        (tabs, body)
    }
}

/// Where the list, the details and (when shown) the preview go in `body`.
///
/// Wide: the list, then a column of details over the preview; narrow: all
/// three stacked. With the preview hidden, the list and the details.
fn body_areas(body: Rect, preview_visible: bool) -> (Rect, Rect, Option<Rect>) {
    let wide = body.width >= 100;
    match (preview_visible, wide) {
        (false, true) => {
            let [list, details] =
                Layout::horizontal([Constraint::Min(44), Constraint::Percentage(55)]).areas(body);
            (list, details, None)
        }
        (false, false) => {
            let [list, details] =
                Layout::vertical([Constraint::Min(5), Constraint::Length(9)]).areas(body);
            (list, details, None)
        }
        (true, true) => {
            let [list, right] =
                Layout::horizontal([Constraint::Min(44), Constraint::Percentage(55)]).areas(body);
            let [details, preview] =
                Layout::vertical([Constraint::Length(12), Constraint::Min(4)]).areas(right);
            (list, details, Some(preview))
        }
        (true, false) => {
            let [list, details, preview] = Layout::vertical([
                Constraint::Min(5),
                Constraint::Length(9),
                Constraint::Min(6),
            ])
            .areas(body);
            (list, details, Some(preview))
        }
    }
}

/// Draws the Download fonts page into `area`.
pub fn draw_downloads(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::bordered()
        .title(" Download fonts ")
        .border_style(Style::new().yellow());
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 6 {
        return;
    }
    let (tabs, body) = tabs_and_body(app, inner);
    draw_tabs(f, app, tabs);
    if app.dl.tab == DlTab::Tournament {
        return super::tournament_tab::draw_tournament(f, app, body);
    }
    let (list, details, preview) = body_areas(body, app.layout.fonts_preview_visible);
    draw_list(f, app, list);
    draw_details(f, app, details);
    if let Some(preview) = preview {
        draw_preview(f, app, preview);
    }
}

fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let mut x = area.x + 1;
    for (index, (tab, name)) in DL_TABS.iter().enumerate() {
        let text = format!(" {name} ");
        let width = to_u16(text.chars().count());
        let rect = Rect {
            x,
            y: area.y,
            width: width.min(area.right().saturating_sub(x)),
            height: 1,
        };
        let mut style = match (
            *tab == app.dl.tab,
            app.dl.tabs_focused && !app.layout.page_tabs_focused,
        ) {
            (true, true) => Style::new().black().on_yellow().bold(),
            (true, false) => Style::new().reversed().bold(),
            _ => Style::new().dark_gray(),
        };
        let mut line = Line::from(Span::styled(text, style));
        if app.hits.add(rect, Target::DlTab(to_u8(index))) {
            style = hover_row(style);
            line = hover_line(line, 0);
        }
        f.render_widget(Paragraph::new(line).style(style), rect);
        x += width + 1;
    }
}

/// The search line (" / search" and the number of fonts) on the first row of
/// `area`; the search text is clickable.
// Long function: the search text, the count text and the click area are built from the same `app.dl` state in one row.
fn draw_search_line(f: &mut Frame, app: &mut App, area: Rect, row_count: usize) {
    let count = match (&app.dl.catalogue, app.dl.tab) {
        (None, DlTab::Google | DlTab::Nerd) => " loading the font lists…".to_string(),
        (Some(c), DlTab::Nerd) if !c.nerd_tag.is_empty() => {
            format!("   {row_count} fonts   release {}", c.nerd_tag)
        }
        _ => format!("   {row_count} fonts"),
    };
    let search = if app.dl.searching || !app.dl.search.is_empty() {
        format!(
            " / {}{}",
            app.dl.search,
            if app.dl.searching { "▏" } else { "" }
        )
    } else {
        " / search".to_string()
    };
    let search_area = Rect {
        height: 1,
        width: to_u16(search.chars().count()).min(area.width),
        ..area
    };
    let mut style = if app.dl.searching {
        Style::new().yellow()
    } else {
        Style::new().dark_gray()
    };
    if app.hits.add(search_area, Target::DlSearch) {
        style = hover(style);
    }
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(search, style),
            Span::styled(count, Style::new().dark_gray()),
        ])),
        Rect { height: 1, ..area },
    );
}

/// The category chips of the Google tab on row `y`; each is clickable.
fn draw_category_chips(f: &mut Frame, app: &mut App, area: Rect, y: u16) {
    let mut x = area.x + 1;
    for (index, (label, _)) in CATEGORIES.iter().enumerate() {
        let text = format!("[{label}]");
        let width = to_u16(text.chars().count());
        let rect = Rect {
            x,
            y,
            width: width.min(area.right().saturating_sub(x)),
            height: 1,
        };
        let mut style = if index == app.dl.category {
            Style::new().black().on_cyan()
        } else {
            Style::new().dark_gray()
        };
        if app.hits.add(rect, Target::DlChip(to_u8(index))) {
            style = hover(style);
        }
        f.render_widget(Paragraph::new(Span::styled(text, style)), rect);
        x += width + 1;
    }
}

/// One row of the list: name, kind and status, clickable. The cursor row is
/// highlighted when the list has the keyboard.
// Long function: one row picks its texts, its cursor or hover style and its click area in order; the style depends on the click-area result.
fn draw_list_row(
    f: &mut Frame,
    app: &mut App,
    rect: Rect,
    (index, row): (usize, &DlRow),
    listed: &Map<String, Value>,
    name_w: usize,
) {
    let (name, kind, status) = match row {
        DlRow::Entry(n) => {
            let entry = &app.dl_entries()[*n];
            (
                entry.family.clone(),
                entry.category.clone(),
                status_span(&app.dl_status(entry, listed)),
            )
        }
        DlRow::Listed(name) => {
            let source = listed
                .get(name)
                .and_then(|v| v.get("source"))
                .and_then(Value::as_str)
                .unwrap_or("");
            (
                name.clone(),
                source.to_string(),
                status_span(&DlStatus::Listed),
            )
        }
    };
    let mut line = Line::from(vec![
        Span::raw(format!(" {:<name_w$}", truncate(&name, name_w))),
        Span::styled(format!(" {kind:<12}"), Style::new().dark_gray()),
        status,
    ]);
    let on = index == app.dl.sel && !app.dl.tabs_focused && !app.layout.page_tabs_focused;
    let mut style = if on {
        Style::new().reversed()
    } else {
        Style::new()
    };
    if app.hits.add(rect, Target::DlRow(index)) && !on {
        style = hover_row(style);
        line = hover_line(line, 0);
    }
    // the cursor covers the text, not padding or swatches
    let (line, style) = if on {
        (
            crate::app::state::hit::cursor_line(line, style),
            Style::new(),
        )
    } else {
        (line, style)
    };
    f.render_widget(Paragraph::new(line).style(style), rect);
}

// Long function: search line, chips, error and list rows are stacked from one running `y`, and the scroll window uses the list height that results.
fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = app.dl_rows();
    // search line
    draw_search_line(f, app, area, rows.len());
    let mut y = area.y + 1;
    // category chips (Google)
    if app.dl.tab == DlTab::Google {
        draw_category_chips(f, app, area, y);
        y += 1;
    }
    if let Some(error) = &app.dl.load_error {
        f.render_widget(
            Paragraph::new(format!(" Couldn't load the font lists: {error}"))
                .red()
                .wrap(Wrap { trim: false }),
            Rect {
                y,
                height: 3.min(area.bottom().saturating_sub(y)),
                ..area
            },
        );
        y += 3;
    }
    let list = Rect {
        y,
        height: area.bottom().saturating_sub(y),
        ..area
    };
    let height = usize::from(list.height);
    app.dl.sel = app.dl.sel.min(rows.len().saturating_sub(1));
    let sel = app.dl.sel;
    if sel < app.dl.scroll {
        app.dl.scroll = sel;
    } else if height > 0 && sel >= app.dl.scroll + height {
        app.dl.scroll = sel + 1 - height;
    }
    let listed = app.dl_listed();
    let name_w = usize::from(list.width).saturating_sub(30).clamp(16, 40);
    for (index, row) in rows.iter().enumerate().skip(app.dl.scroll).take(height) {
        let rect = Rect {
            y: list.y + to_u16(index - app.dl.scroll),
            height: 1,
            ..list
        };
        draw_list_row(f, app, rect, (index, row), &listed, name_w);
    }
    if rows.is_empty() && app.dl.catalogue.is_some() {
        let text = if app.dl.tab == DlTab::Installed {
            " Nothing installed by the studio yet."
        } else {
            " No fonts match."
        };
        f.render_widget(Paragraph::new(text).dark_gray(), Rect { height: 1, ..list });
    }
}
