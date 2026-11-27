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
use super::rows::{RowLook, draw_selectable_row, follow, row_rect};
use crate::app::app_state::App;
use crate::app::state::hit::{Target, hover, hover_line, hover_row};
use crate::fonts::pages::download::{CATEGORIES, DL_TABS, DlRow, DlStatus, DlTab, mb};
use crate::ui::widgets::truncate;
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

/// The count text after the search: how many fonts are listed, or that the lists load.
fn count_text(app: &App, row_count: usize) -> String {
    match (&app.dl.catalogue, app.dl.tab) {
        (None, DlTab::Google | DlTab::Nerd) => " loading the font lists…".to_string(),
        (Some(c), DlTab::Nerd) if !c.nerd_tag.is_empty() => {
            format!("   {row_count} fonts   release {}", c.nerd_tag)
        }
        _ => format!("   {row_count} fonts"),
    }
}

/// The search text: the typed search (with a cursor while typing) or a hint.
fn search_text(app: &App) -> String {
    if app.dl.searching || !app.dl.search.is_empty() {
        format!(
            " / {}{}",
            app.dl.search,
            if app.dl.searching { "▏" } else { "" }
        )
    } else {
        " / search".to_string()
    }
}

/// The search line (" / search" and the number of fonts) on the first row of
/// `area`; the search text is clickable.
fn draw_search_line(f: &mut Frame, app: &mut App, area: Rect, row_count: usize) {
    let count = count_text(app, row_count);
    let search = search_text(app);
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

/// The name, kind and status texts of one list row.
fn row_texts(
    app: &App,
    row: &DlRow,
    listed: &Map<String, Value>,
) -> (String, String, Span<'static>) {
    match row {
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
    }
}

/// One row of the list: name, kind and status, clickable. The cursor row is
/// highlighted when the list has the keyboard.
fn draw_list_row(
    f: &mut Frame,
    app: &mut App,
    rect: Rect,
    (index, row): (usize, &DlRow),
    listed: &Map<String, Value>,
    name_w: usize,
) {
    let (name, kind, status) = row_texts(app, row, listed);
    let line = Line::from(vec![
        Span::raw(format!(" {:<name_w$}", truncate(&name, name_w))),
        Span::styled(format!(" {kind:<12}"), Style::new().dark_gray()),
        status,
    ]);
    let look = RowLook {
        selected: index == app.dl.sel && !app.dl.tabs_focused && !app.layout.page_tabs_focused,
        focused: true,
        target: Some(Target::DlRow(index)),
        underline_skip: 0,
        hover_when_selected: false,
    };
    draw_selectable_row(f, app, rect, line, look);
}

/// The red note about a failed font-list download, three rows from row `y`.
/// Returns the row below the note.
fn draw_load_error(f: &mut Frame, error: &str, area: Rect, y: u16) -> u16 {
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
    y + 3
}

/// The note under an empty list on tab `tab`.
fn empty_list_note(tab: DlTab) -> &'static str {
    if tab == DlTab::Installed {
        " Nothing installed by the studio yet."
    } else {
        " No fonts match."
    }
}

/// Draws the list column: search line, chips, load error, then the rows.
fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = app.dl_rows();
    draw_search_line(f, app, area, rows.len());
    let mut y = area.y + 1;
    // category chips (Google)
    if app.dl.tab == DlTab::Google {
        draw_category_chips(f, app, area, y);
        y += 1;
    }
    if let Some(error) = &app.dl.load_error {
        y = draw_load_error(f, error, area, y);
    }
    let list = Rect {
        y,
        height: area.bottom().saturating_sub(y),
        ..area
    };
    let height = usize::from(list.height);
    app.dl.sel = app.dl.sel.min(rows.len().saturating_sub(1));
    let sel = app.dl.sel;
    follow(&mut app.dl.scroll, sel, height);
    let listed = app.dl_listed();
    let name_w = usize::from(list.width).saturating_sub(30).clamp(16, 40);
    for (index, row) in rows.iter().enumerate().skip(app.dl.scroll).take(height) {
        let rect = row_rect(list, index - app.dl.scroll);
        draw_list_row(f, app, rect, (index, row), &listed, name_w);
    }
    if rows.is_empty() && app.dl.catalogue.is_some() {
        let note = empty_list_note(app.dl.tab);
        f.render_widget(Paragraph::new(note).dark_gray(), Rect { height: 1, ..list });
    }
}
