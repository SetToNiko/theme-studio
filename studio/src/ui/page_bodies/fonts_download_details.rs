//! The details box of the Download fonts page: the highlighted font's facts
//! and its Install / Remove button.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use serde_json::{Map, Value};

use super::fonts_download::status_span;
use crate::app::app_state::App;
use crate::app::state::hit::{Target, hover};
use crate::fonts::pages::download::{DlRow, DlStatus, mb};
use crate::system::download::types::{Entry, Source};
use crate::ui::widgets::to_u16;

/// The weights of a font as text: "variable, 100–900", "400–700 (3 weights)" or one weight.
fn weights_text(e: &Entry) -> String {
    match (e.variable, e.weights.first(), e.weights.last()) {
        (true, Some(first), Some(last)) => format!("variable, {first}–{last}"),
        (false, Some(first), Some(last)) if first != last => {
            format!("{first}–{last} ({} weights)", e.weights.len())
        }
        (_, Some(first), _) => first.to_string(),
        _ => "–".into(),
    }
}

/// The label and value rows of a catalogue font (Nerd fonts show no weights or italic).
fn detail_rows(e: &Entry) -> Vec<Line<'static>> {
    let mut rows: Vec<Line> = Vec::new();
    let source = match &e.source {
        Source::Google => "Google Fonts (the google/fonts repo)".to_string(),
        Source::Fontsource => "Fontsource (latin + latin-ext)".to_string(),
        Source::Nerd { asset, size, .. } => {
            format!("Nerd Fonts {asset}.tar.xz, {}", mb(*size))
        }
    };
    for (k, v) in [
        ("category", e.category.clone()),
        (
            "license",
            if e.license.is_empty() {
                "–".into()
            } else {
                e.license.clone()
            },
        ),
        ("weights", weights_text(e)),
        ("italic", if e.italic { "yes".into() } else { "no".into() }),
        ("from", source),
    ] {
        if !(k == "weights" || k == "italic") || !matches!(e.source, Source::Nerd { .. }) {
            rows.push(Line::from(vec![
                Span::styled(format!("{k:<9}"), Style::new().dark_gray()),
                Span::raw(v),
            ]));
        }
    }
    rows
}

/// The lines for a font of a catalogue, and the button that fits its status.
fn entry_lines(
    app: &App,
    e: &Entry,
    listed: &Map<String, Value>,
) -> (Vec<Line<'static>>, Option<(&'static str, char)>) {
    let status = app.dl_status(e, listed);
    let mut lines = vec![
        Line::styled(e.family.clone(), Style::new().bold()),
        Line::raw(""),
    ];
    lines.extend(detail_rows(e));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![status_span(&status)]));
    let button = match status {
        DlStatus::Listed => Some((" Remove ", 'r')),
        DlStatus::Downloading(..) => None,
        _ => Some((" Install ", 'i')),
    };
    (lines, button)
}

/// The lines for a font of the installed list (it has no catalogue entry).
fn listed_lines(name: &str, listed: &Map<String, Value>) -> Vec<Line<'static>> {
    let v = listed.get(name).cloned().unwrap_or(Value::Null);
    let field = |key: &str| {
        v.get(key)
            .and_then(Value::as_str)
            .unwrap_or("–")
            .to_string()
    };
    let here = crate::model::paths::installed_fonts_dir()
        .join(field("id"))
        .exists();
    vec![
        Line::styled(name.to_string(), Style::new().bold()),
        Line::raw(""),
        Line::raw(format!(" from     {}", field("source"))),
        Line::raw(if here {
            "files    downloaded here (a rebuild takes over)"
        } else {
            "files    provided by Nix"
        }),
    ]
}

/// The Install / Remove button at row `y`, clickable and hover-highlighted.
fn draw_button(f: &mut Frame, app: &mut App, inner: Rect, y: u16, (text, id): (&str, char)) {
    let area = Rect {
        x: inner.x + 1,
        y,
        width: to_u16(text.chars().count()),
        height: 1,
    };
    let base = if id == 'i' {
        Style::new().black().on_green().bold()
    } else {
        Style::new().black().on_red().bold()
    };
    let style = if app.hits.add(area, Target::DlButton(id)) {
        hover(base)
    } else {
        base
    };
    f.render_widget(Paragraph::new(Span::styled(text.to_string(), style)), area);
}

/// The detail lines and the button for the highlighted row (none: no lines, no button).
fn highlighted_lines(app: &mut App) -> (Vec<Line<'static>>, Option<(&'static str, char)>) {
    let listed = app.dl_listed();
    match app.dl_rows().get(app.dl.sel).cloned() {
        Some(DlRow::Entry(index)) => {
            let entry = app.dl_entries()[index].clone();
            entry_lines(app, &entry, &listed)
        }
        Some(DlRow::Listed(name)) => (listed_lines(&name, &listed), Some((" Remove ", 'r'))),
        None => (Vec::new(), None),
    }
}

/// The two grey lines under the details: where the files go and the list for Nix.
fn files_note(app: &App) -> [Line<'static>; 2] {
    [
        Line::styled(
            format!(
                " Files go to {}; the list for Nix is",
                crate::model::paths::installed_fonts_dir().display()
            ),
            Style::new().dark_gray(),
        ),
        Line::styled(
            app.installed_list_path().display().to_string(),
            Style::new().dark_gray(),
        ),
    ]
}

/// The details of the highlighted font, then where the files go.
pub(super) fn draw_details(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::new()
        .borders(Borders::LEFT | Borders::TOP)
        .border_style(Style::new().dark_gray());
    let inner = block.inner(area);
    // a column of padding, so wrapped lines keep the margin
    let inner = Rect {
        x: inner.x + 1,
        width: inner.width.saturating_sub(1),
        ..inner
    };
    f.render_widget(block, area);
    let (mut lines, button) = highlighted_lines(app);
    lines.push(Line::raw(""));
    let button_y = inner.y + to_u16(lines.len());
    lines.push(Line::raw(""));
    lines.extend(files_note(app));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    if let Some(button) = button
        && button_y < inner.bottom()
    {
        draw_button(f, app, inner, button_y, button);
    }
}
