//! The History tab of the Settings page: the files, the saved versions of the
//! selected file, and a preview of what restoring the selected version changes.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::{colour_span, follow, row_rect, selected_style, truncate, value_spans};
use crate::app::App;
use crate::app::state::hit::{Target, hover_line, hover_row};
use crate::model::color::Rgb;
use crate::model::history::SnapshotInfo;
use crate::settings_page::{DiffLine, DiffView, FILES, HistFocus};

pub(super) fn draw_history(f: &mut Frame, app: &mut App, area: Rect) {
    let narrow = area.width < 110;
    let (files, versions, preview) = if narrow {
        let [top, prev] =
            Layout::vertical([Constraint::Percentage(40), Constraint::Min(4)]).areas(area);
        let [fl, vl] = Layout::horizontal([Constraint::Length(18), Constraint::Min(0)]).areas(top);
        (fl, vl, prev)
    } else {
        let [fl, vl, prev] = Layout::horizontal([
            Constraint::Length(18),
            Constraint::Length(52),
            Constraint::Min(0),
        ])
        .areas(area);
        (fl, vl, prev)
    };
    let focus = app.settings_page.hist_focus;
    draw_files(f, app, files, focus);
    let list = app.versions();
    draw_versions(f, app, versions, focus, &list);
    draw_preview(f, app, preview, &list);
}

/// The Files box: one row per file that has a history.
// Long function: the row loop needs the block's inner rectangle, and each row's style depends on the click-area result of the same row.
fn draw_files(f: &mut Frame, app: &mut App, files: Rect, focus: HistFocus) {
    let block = Block::bordered()
        .title(" Files ")
        .border_style(if focus == HistFocus::Files {
            Style::new().yellow()
        } else {
            Style::new().dark_gray()
        });
    let inner = block.inner(files);
    f.render_widget(block, files);
    for (i, name) in FILES.iter().enumerate().take(inner.height as usize) {
        let r = row_rect(inner, i);
        let on = i == app.settings_page.hist_file;
        let mut style = selected_style(on, focus == HistFocus::Files);
        let mut line = Line::raw(format!(" {name}"));
        if app.hits.add(r, Target::HistFile(i)) && !(on && focus == HistFocus::Files) {
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
        f.render_widget(Paragraph::new(line).style(style), r);
    }
}

/// The Versions box: one row per saved version of the selected file.
// Long function: the scroll offset follows the selection and the box height, and the row loop then uses that offset and registers each row's click area.
fn draw_versions(
    f: &mut Frame,
    app: &mut App,
    versions: Rect,
    focus: HistFocus,
    list: &[(std::path::PathBuf, SnapshotInfo)],
) {
    let n = list.len();
    app.settings_page.hist_sel = app.settings_page.hist_sel.min(n.saturating_sub(1));
    let sel = app.settings_page.hist_sel;
    let file = FILES[app.settings_page.hist_file];
    let block = Block::bordered()
        .title(format!(" Versions of {file} ({n}) "))
        .title_bottom(" n name the current file ")
        .border_style(if focus == HistFocus::Versions {
            Style::new().yellow()
        } else {
            Style::new().dark_gray()
        });
    let inner = block.inner(versions);
    f.render_widget(block, versions);
    if list.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::raw(" No earlier versions yet."),
                Line::styled(
                    " Every save keeps the file as it was.",
                    Style::new().dark_gray(),
                ),
            ]),
            inner,
        );
    }
    let mut scroll = app.settings_page.hist_scroll;
    follow(&mut scroll, sel, inner.height as usize);
    app.settings_page.hist_scroll = scroll;
    let w = inner.width as usize;
    for (i, (_, s)) in list
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner.height as usize)
    {
        let r = row_rect(inner, i - scroll);
        let when = s.when.get(5..16).unwrap_or(&s.when).to_string();
        let (text, st) = match &s.name {
            Some(name) => (format!("★ {name}"), Style::new().yellow().bold()),
            None => (s.summary.clone(), Style::new()),
        };
        let line = Line::from(vec![
            Span::styled(format!(" {when}  "), Style::new().dark_gray()),
            Span::styled(truncate(&text, w.saturating_sub(15)), st),
        ]);
        let on = i == sel;
        let mut style = selected_style(on, focus == HistFocus::Versions);
        let mut line = line;
        if app.hits.add(r, Target::SettingsRow(i)) && !(on && focus == HistFocus::Versions) {
            style = hover_row(style);
            line = hover_line(line, 1);
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
        f.render_widget(Paragraph::new(line).style(style), r);
    }
}

/// The preview box: the selected version in place of what's there now.
// Long function: the scroll limit depends on the head lines built first and on the box height, and the diff area is registered before the lines are drawn.
fn draw_preview(
    f: &mut Frame,
    app: &mut App,
    preview: Rect,
    list: &[(std::path::PathBuf, SnapshotInfo)],
) {
    let sel = app.settings_page.hist_sel;
    let all = app.settings_page.show_all;
    let title = match list.get(sel) {
        Some((_, s)) => format!(" Restore preview: now → {} ", s.when),
        None => " Restore preview ".into(),
    };
    let block = Block::bordered()
        .title(title)
        .title_bottom(format!(
            " r restore   A {}   PgUp/PgDn scroll ",
            if all {
                "only differences"
            } else {
                "everything"
            }
        ))
        .border_style(Style::new().dark_gray());
    let inner = block.inner(preview);
    f.render_widget(block, preview);
    app.hits.add(inner, Target::SettingsDiff);
    if list.is_empty() {
        return;
    }
    let view = app.diff_view();
    let mut lines = preview_head(app, &view);
    let head = lines.len();
    let body_h = (inner.height as usize).saturating_sub(head);
    let max_scroll = view.lines.len().saturating_sub(body_h);
    app.settings_page.diff_scroll = app.settings_page.diff_scroll.min(max_scroll);
    let scroll = app.settings_page.diff_scroll;
    for l in view.lines.iter().skip(scroll).take(body_h) {
        lines.push(diff_line(l));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

/// The lines above the differences: the palette strips and the legend, or "Same as now."
fn preview_head(app: &App, view: &DiffView) -> Vec<Line<'static>> {
    let mut lines = palette_lines(app, view);
    if view.lines.is_empty() {
        lines.push(Line::styled(" Same as now.", Style::new().dark_gray()));
    } else {
        lines.push(Line::from(vec![
            Span::styled(" changed ", Style::new().black().on_yellow()),
            Span::styled(" now → selected   ", Style::new().dark_gray()),
            Span::styled("+ ", Style::new().green()),
            Span::styled("restoring adds   ", Style::new().dark_gray()),
            Span::styled("- ", Style::new().red()),
            Span::styled("restoring removes", Style::new().dark_gray()),
        ]));
    }
    lines
}

/// The palette strips of "now" and "selected" (nothing when both are empty).
fn palette_lines(app: &App, view: &DiffView) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    if let Some((now, then)) = &view.palette {
        let strip = |label: &str, cs: &[(String, Rgb)]| {
            let mut spans = vec![Span::styled(
                format!(" {label:<9}"),
                Style::new().dark_gray(),
            )];
            spans.extend(cs.iter().map(|(_, c)| colour_span(*c, "██")));
            Line::from(spans)
        };
        if !then.is_empty() || !now.is_empty() {
            lines.push(Line::styled(
                format!(" \"{}\" ", app.name),
                Style::new().bold(),
            ));
            lines.push(strip("now", now));
            lines.push(strip("selected", then));
            lines.push(Line::raw(""));
        }
    }
    lines
}

/// One line of the difference list, coloured by the kind of change.
fn diff_line(l: &DiffLine) -> Line<'static> {
    let pad = |d: usize| "  ".repeat(d + 1);
    match l {
        DiffLine::Head { depth, key } => {
            Line::styled(format!("{}{key}", pad(*depth)), Style::new().bold())
        }
        DiffLine::Same { depth, key, value } => same_line(&pad(*depth), key, value),
        DiffLine::Changed {
            depth,
            key,
            now,
            then,
        } => changed_line(&pad(*depth), key, now, then),
        DiffLine::Added { depth, key, value } => marked_line(
            &pad(*depth),
            '+',
            key,
            value,
            Style::new().green(),
            Style::new().green(),
        ),
        DiffLine::Removed { depth, key, value } => marked_line(
            &pad(*depth),
            '-',
            key,
            value,
            Style::new().red(),
            Style::new().red().add_modifier(Modifier::CROSSED_OUT),
        ),
    }
}

/// A changed line: the key on yellow, the old value crossed out, then the new value.
fn changed_line(pad: &str, key: &str, now: &str, then: &str) -> Line<'static> {
    let mut s = vec![Span::styled(
        format!("{pad}{key}: "),
        Style::new().black().on_yellow(),
    )];
    s.extend(value_spans(
        now,
        Style::new().dark_gray().add_modifier(Modifier::CROSSED_OUT),
    ));
    s.push(Span::raw(" → "));
    s.extend(value_spans(then, Style::new().yellow().bold()));
    Line::from(s)
}

/// An added (`+`) or removed (`-`) line: the marker and key in `key_style`, the value in `value_style`.
fn marked_line(
    pad: &str,
    marker: char,
    key: &str,
    value: &str,
    key_style: Style,
    value_style: Style,
) -> Line<'static> {
    let mut s = vec![Span::styled(format!("{pad}{marker} {key}: "), key_style)];
    s.extend(value_spans(value, value_style));
    Line::from(s)
}

/// An unchanged line: the key and its value in the normal style.
fn same_line(pad: &str, key: &str, value: &str) -> Line<'static> {
    let mut s = vec![Span::raw(format!("{pad}{key}: "))];
    s.extend(value_spans(value, Style::new()));
    Line::from(s)
}
