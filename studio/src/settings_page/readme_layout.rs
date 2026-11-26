//! Text layout for the README tab: word wrapping of styled text and table
//! layout in columns. `readme.rs` calls these while it turns Markdown into lines.

use pulldown_cmark::{Alignment, HeadingLevel};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// A piece of text in one style.
pub(super) type Piece = (String, Style);

pub(super) fn heading_style(level: HeadingLevel) -> Style {
    match level {
        HeadingLevel::H1 | HeadingLevel::H2 => Style::new().yellow().bold(),
        _ => Style::new().bold(),
    }
}

pub(super) fn code_style() -> Style {
    Style::new().cyan()
}

/// The words of styled text: runs of non-space characters, possibly in
/// several styles (`**bold**text` is one word).
fn words_of(pieces: &[Piece]) -> Vec<Vec<Piece>> {
    let mut words: Vec<Vec<Piece>> = Vec::new();
    let mut gap = true;
    for (text, style) in pieces {
        for (i, part) in text.split(' ').enumerate() {
            if i > 0 {
                gap = true;
            }
            if part.is_empty() {
                continue;
            }
            if gap || words.is_empty() {
                words.push(Vec::new());
                gap = false;
            }
            words.last_mut().unwrap().push((part.to_string(), *style));
        }
    }
    words
}

/// The width of spans in columns.
fn spans_width(p: &[Span]) -> usize {
    p.iter().map(Span::width).sum()
}

/// Splits styled text into words (with the spaces between them dropped) and
/// fills lines up to `width`. The first line starts with `first`, the rest
/// with `rest` (indents, bullets).
pub(super) fn wrap(
    pieces: &[Piece],
    width: usize,
    first: Vec<Span<'static>>,
    rest: &[Span<'static>],
) -> Vec<Line<'static>> {
    // a word wider than a whole line is cut into line-sized pieces
    let max = width.saturating_sub(spans_width(rest)).max(8);
    let words: Vec<Vec<Piece>> = words_of(pieces)
        .into_iter()
        .flat_map(|w| split_word(w, max))
        .collect();
    fill_lines(&words, width, first, rest)
}

/// Puts the words on lines, as many as fit in `width` columns on each.
fn fill_lines(
    words: &[Vec<Piece>],
    width: usize,
    first: Vec<Span<'static>>,
    rest: &[Span<'static>],
) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut line: Vec<Span<'static>> = first;
    let mut used = spans_width(&line);
    let mut start = used;
    let room = |p: usize| width.saturating_sub(p).max(8);
    let mut empty = true;
    for w in words {
        let ww: usize = w.iter().map(|(t, _)| t.width()).sum();
        if !empty && used + 1 + ww > start + room(start) {
            out.push(Line::from(std::mem::take(&mut line)));
            line = rest.to_vec();
            used = spans_width(&line);
            start = used;
            empty = true;
        }
        if !empty {
            line.push(Span::raw(" "));
            used += 1;
        }
        for (t, s) in w {
            line.push(Span::styled(t.clone(), *s));
        }
        used += ww;
        empty = false;
    }
    if !empty || out.is_empty() {
        out.push(Line::from(line));
    }
    out
}

/// A word in pieces no wider than `max` columns each.
fn split_word(word: Vec<Piece>, max: usize) -> Vec<Vec<Piece>> {
    let total: usize = word.iter().map(|(t, _)| t.width()).sum();
    if total <= max {
        return vec![word];
    }
    let mut out: Vec<Vec<Piece>> = vec![Vec::new()];
    let mut used = 0;
    for (text, style) in word {
        for c in text.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if used + cw > max && used > 0 {
                out.push(Vec::new());
                used = 0;
            }
            let part = out.last_mut().unwrap();
            match part.last_mut() {
                Some((t, s)) if *s == style => t.push(c),
                _ => part.push((c.to_string(), style)),
            }
            used += cw;
        }
    }
    out
}

/// A table being read: rows of cells of styled text.
#[derive(Default)]
pub(super) struct Table {
    pub(super) align: Vec<Alignment>,
    pub(super) rows: Vec<Vec<Vec<Piece>>>,
    pub(super) head_rows: usize,
}

/// Lays a table out in columns: as wide as their text, narrowed (widest
/// first) to fit `width`, cells wrapped inside their column.
pub(super) fn render_table(t: &Table, width: usize, indent: &str) -> Vec<Line<'static>> {
    let cols = t.rows.iter().map(Vec::len).max().unwrap_or(0);
    if cols == 0 {
        return Vec::new();
    }
    let w = column_widths(t, cols, width.saturating_sub(indent.width()));
    let sep = Style::new().dark_gray();
    let mut out = Vec::new();
    for (ri, row) in t.rows.iter().enumerate() {
        let head = ri < t.head_rows;
        out.extend(row_lines(t, row, head, &w, indent));
        if head && ri + 1 == t.head_rows {
            let rule: Vec<String> = w.iter().map(|n| "─".repeat(*n)).collect();
            out.push(Line::styled(format!("{indent}{}", rule.join("─┼─")), sep));
        }
    }
    out
}

/// The width of each column: as wide as its widest cell, narrowed (the
/// widest first, down to 6) until the table fits `room` columns.
fn column_widths(t: &Table, cols: usize, room: usize) -> Vec<usize> {
    let text_w = |c: &[Piece]| c.iter().map(|(s, _)| s.width()).sum::<usize>();
    let mut w: Vec<usize> = (0..cols)
        .map(|i| {
            t.rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|c| text_w(c))
                .max()
                .unwrap_or(0)
                .max(1)
        })
        .collect();
    // " │ " between columns
    let avail = room.saturating_sub(3 * (cols - 1));
    while w.iter().sum::<usize>() > avail {
        let (i, &max) = w.iter().enumerate().max_by_key(|(_, v)| **v).unwrap();
        if max <= 6 {
            break;
        }
        w[i] = max - 1;
    }
    w
}

/// The lines of one cell wrapped to `col_w` columns (`head` cells in bold).
fn cell_lines(cell: Option<&Vec<Piece>>, col_w: usize, head: bool) -> Vec<Line<'static>> {
    let mut c = cell.cloned().unwrap_or_default();
    if head {
        for p in &mut c {
            p.1 = p.1.add_modifier(Modifier::BOLD);
        }
    }
    wrap(&c, col_w, Vec::new(), &[])
}

/// Adds the spans of `line` to `spans`, padded with spaces to `width`
/// columns (on the left for `right`-aligned columns, else on the right).
fn push_padded(spans: &mut Vec<Span<'static>>, line: Line<'static>, width: usize, right: bool) {
    let pad = Span::raw(" ".repeat(width.saturating_sub(line.width())));
    if right {
        spans.push(pad.clone());
    }
    spans.extend(line.spans);
    if !right {
        spans.push(pad);
    }
}

/// The lines of one table row: the cells wrapped inside their columns
/// (`head` rows in bold), side by side.
fn row_lines(
    t: &Table,
    row: &[Vec<Piece>],
    head: bool,
    w: &[usize],
    indent: &str,
) -> Vec<Line<'static>> {
    let sep = Style::new().dark_gray();
    let cells: Vec<Vec<Line>> = w
        .iter()
        .enumerate()
        .map(|(i, &col_w)| cell_lines(row.get(i), col_w, head))
        .collect();
    let height = cells.iter().map(Vec::len).max().unwrap_or(1);
    let mut out = Vec::new();
    for li in 0..height {
        let mut spans = vec![Span::raw(indent.to_string())];
        for (i, cell) in cells.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", sep));
            }
            let line = cell.get(li).cloned().unwrap_or_default();
            let right = t.align.get(i) == Some(&Alignment::Right);
            push_padded(&mut spans, line, w[i], right);
        }
        out.push(Line::from(spans));
    }
    out
}
