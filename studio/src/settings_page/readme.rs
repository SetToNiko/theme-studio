//! The Settings page's README tab (`settings_page/`): docs/theme-studio.md (found by
//! model/paths.rs `guide`, read when the tab opens), rendered from Markdown into
//! styled lines wrapped to the pane: headings, paragraphs, lists, block
//! quotes, code, tables and rules. Headings are remembered so `}` / `{` can
//! jump between them.

use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// The document, as rendered for one width.
#[derive(Debug, Default)]
pub struct Doc {
    pub lines: Vec<Line<'static>>,
    /// The line each heading starts on, in order.
    pub headings: Vec<usize>,
}

/// A piece of text in one style.
type Piece = (String, Style);

fn heading_style(level: HeadingLevel) -> Style {
    match level {
        HeadingLevel::H1 | HeadingLevel::H2 => Style::new().yellow().bold(),
        _ => Style::new().bold(),
    }
}

fn code_style() -> Style {
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
fn wrap(
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
// Long function: one loop that keeps the line, its used width and its start column together; splitting it would pass five shared values.
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

/// One list level: its next number (None: bullets).
struct List {
    next: Option<u64>,
}

/// A table being read: rows of cells of styled text.
#[derive(Default)]
struct Table {
    align: Vec<Alignment>,
    rows: Vec<Vec<Vec<Piece>>>,
    head_rows: usize,
}

/// Lays a table out in columns: as wide as their text, narrowed (widest
/// first) to fit `width`, cells wrapped inside their column.
fn render_table(t: &Table, width: usize, indent: &str) -> Vec<Line<'static>> {
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

/// Renders Markdown into lines `width` columns wide.
pub fn render(md: &str, width: u16) -> Doc {
    let mut renderer = Renderer::new(usize::from(width).max(20));
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    for ev in Parser::new_ext(md, opts) {
        renderer.event(ev);
    }
    renderer.finish()
}

/// The state while Markdown events turn into lines. [`render`] feeds the
/// events in; each event changes this state or adds lines to `doc`.
struct Renderer {
    width: usize,
    doc: Doc,
    /// The text of the current block, not yet wrapped into lines.
    pieces: Vec<Piece>,
    /// The inline style: bold, italic, strikethrough, links.
    style: Style,
    /// The styles to return to when an inline style ends.
    styles: Vec<Style>,
    heading: Option<HeadingLevel>,
    lists: Vec<List>,
    /// The bullet waiting for the item's first line.
    bullet: Option<String>,
    quote: usize,
    code: Option<Vec<String>>,
    table: Option<Table>,
}

impl Renderer {
    fn new(width: usize) -> Renderer {
        Renderer {
            width,
            doc: Doc::default(),
            pieces: Vec::new(),
            style: Style::new(),
            styles: Vec::new(),
            heading: None,
            lists: Vec::new(),
            bullet: None,
            quote: 0,
            code: None,
            table: None,
        }
    }

    /// Ends the document: the last text, and no blank lines at the end.
    fn finish(mut self) -> Doc {
        self.flush();
        while self.doc.lines.last().is_some_and(|l| l.width() == 0) {
            self.doc.lines.pop();
        }
        self.doc
    }

    /// The indent of the current block (lists, quotes): the spaces for the
    /// list level, and the quote bars.
    fn indent(&self) -> (String, Vec<Span<'static>>) {
        let mut q = Vec::new();
        for _ in 0..self.quote {
            q.push(Span::styled("│ ", Style::new().dark_gray()));
        }
        (" ".repeat(self.lists.len().saturating_sub(1) * 3), q)
    }

    /// A blank line between blocks (not at the start, not twice).
    fn gap(&mut self) {
        if self.doc.lines.last().is_some_and(|l| l.width() > 0) {
            self.doc.lines.push(Line::raw(""));
        }
    }

    /// The text so far, as wrapped lines.
    fn flush(&mut self) {
        if self.pieces.is_empty() && self.bullet.is_none() {
            return;
        }
        let (ind, q) = self.indent();
        let mut first = q.clone();
        let mut rest = q;
        first.push(Span::raw(ind.clone()));
        rest.push(Span::raw(ind));
        if let Some(b) = self.bullet.take() {
            rest.push(Span::raw(" ".repeat(b.width())));
            first.push(Span::styled(b, Style::new().yellow()));
        } else if !self.lists.is_empty() {
            // a later paragraph in an item: under its text (after "• ")
            first.push(Span::raw("  "));
            rest.push(Span::raw("  "));
        }
        self.doc
            .lines
            .extend(wrap(&self.pieces, self.width, first, &rest));
        self.pieces.clear();
    }

    /// Starts an inline style that `change` makes from the current one.
    fn push_style(&mut self, change: impl FnOnce(Style) -> Style) {
        self.styles.push(self.style);
        self.style = change(self.style);
    }

    /// Returns to the style from before the last `push_style`.
    fn pop_style(&mut self) {
        self.style = self.styles.pop().unwrap_or_default();
    }

    /// Handles a bold, italic, link (and, outside tables, strikethrough)
    /// start or end. Returns false for any other event.
    fn inline_style_event(&mut self, ev: &Event, with_strikethrough: bool) -> bool {
        match ev {
            Event::Start(Tag::Strong) => self.push_style(Style::bold),
            Event::Start(Tag::Emphasis) => self.push_style(Style::italic),
            Event::Start(Tag::Strikethrough) if with_strikethrough => {
                self.push_style(Style::crossed_out);
            }
            Event::Start(Tag::Link { .. }) => self.push_style(|s| s.blue().underlined()),
            Event::End(TagEnd::Strong | TagEnd::Emphasis | TagEnd::Link) => self.pop_style(),
            Event::End(TagEnd::Strikethrough) if with_strikethrough => self.pop_style(),
            _ => return false,
        }
        true
    }

    /// Handles one Markdown event.
    fn event(&mut self, ev: Event) {
        if self.table.is_some() {
            self.table_event(ev);
        } else if !self.inline_style_event(&ev, true) {
            match ev {
                Event::Text(_) | Event::Code(_) | Event::SoftBreak | Event::HardBreak => {
                    self.text_event(ev);
                }
                Event::Start(tag) => self.block_start(tag),
                Event::End(end) => self.block_end(end),
                Event::Rule => self.rule(),
                _ => {}
            }
        }
    }

    /// Handles an event inside a table: the cells collect the text.
    fn table_event(&mut self, ev: Event) {
        if self.inline_style_event(&ev, false) {
            return;
        }
        let style = self.style;
        let Some(t) = self.table.as_mut() else { return };
        match ev {
            Event::Start(Tag::TableHead) => {
                t.rows.push(Vec::new());
                t.head_rows += 1;
            }
            Event::Start(Tag::TableRow) => t.rows.push(Vec::new()),
            Event::Start(Tag::TableCell) => {
                if let Some(r) = t.rows.last_mut() {
                    r.push(Vec::new());
                }
            }
            Event::Text(s) => {
                if let Some(c) = t.rows.last_mut().and_then(|r| r.last_mut()) {
                    c.push((s.to_string(), style));
                }
            }
            Event::Code(s) => {
                if let Some(c) = t.rows.last_mut().and_then(|r| r.last_mut()) {
                    c.push((s.to_string(), code_style()));
                }
            }
            Event::End(TagEnd::Table) => self.end_table(),
            _ => {}
        }
    }

    /// The table is complete: lays it out under the item's text, if any.
    fn end_table(&mut self) {
        let Some(t) = self.table.take() else { return };
        let (mut ind, _) = self.indent();
        if !self.lists.is_empty() {
            ind += "  ";
        }
        self.gap();
        self.doc.lines.extend(render_table(&t, self.width, &ind));
    }

    /// Text, inline code and line breaks.
    fn text_event(&mut self, ev: Event) {
        match ev {
            Event::Text(s) => match self.code.as_mut() {
                Some(c) => c.extend(s.lines().map(str::to_string)),
                None => self.pieces.push((s.to_string(), self.style)),
            },
            Event::Code(s) => {
                let s_style = if self.heading.is_some() {
                    self.style
                } else {
                    code_style()
                };
                self.pieces.push((s.to_string(), s_style));
            }
            Event::SoftBreak => self.pieces.push((" ".into(), self.style)),
            Event::HardBreak => self.flush(),
            _ => {}
        }
    }

    /// A horizontal rule.
    fn rule(&mut self) {
        self.flush();
        self.gap();
        self.doc.lines.push(Line::styled(
            "─".repeat(self.width),
            Style::new().dark_gray(),
        ));
    }

    /// The start of a block: heading, paragraph, list, item, quote, code, table.
    fn block_start(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.gap();
                self.heading = Some(level);
                self.push_style(|_| heading_style(level));
            }
            // a paragraph opening a (loose) list item goes on its bullet
            Tag::Paragraph if self.bullet.is_none() => self.gap(),
            Tag::List(start) => self.start_list(start),
            Tag::Item => self.start_item(),
            Tag::BlockQuote(_) => self.start_quote(),
            Tag::CodeBlock(_) => {
                self.flush();
                self.gap();
                self.code = Some(Vec::new());
            }
            Tag::Table(align) => {
                self.flush();
                self.table = Some(Table {
                    align,
                    ..Default::default()
                });
            }
            _ => {}
        }
    }

    /// A list: after the text of the item it is in, if any.
    fn start_list(&mut self, start: Option<u64>) {
        self.flush();
        if self.lists.is_empty() {
            self.gap();
        }
        self.lists.push(List { next: start });
    }

    /// A block quote: one more bar, and grey text.
    fn start_quote(&mut self) {
        self.flush();
        self.gap();
        self.quote += 1;
        self.push_style(Style::dark_gray);
    }

    /// A list item: its bullet (or number) waits for the first line.
    fn start_item(&mut self) {
        self.flush();
        self.bullet = Some(match self.lists.last_mut().and_then(|l| l.next.as_mut()) {
            Some(n) => {
                *n += 1;
                format!("{}. ", *n - 1)
            }
            None => "• ".to_string(),
        });
    }

    /// The end of a block.
    fn block_end(&mut self, end: TagEnd) {
        match end {
            TagEnd::Heading(level) => {
                self.doc.headings.push(self.doc.lines.len());
                self.flush();
                if level == HeadingLevel::H1 {
                    let w = self.doc.lines.last().map_or(0, Line::width);
                    self.doc
                        .lines
                        .push(Line::styled("─".repeat(w), Style::new().yellow()));
                }
                self.heading = None;
                self.pop_style();
            }
            TagEnd::Paragraph | TagEnd::Item => self.flush(),
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.quote -= 1;
                self.pop_style();
            }
            TagEnd::CodeBlock => self.end_code_block(),
            _ => {}
        }
    }

    /// The code lines, indented and in the code style, then a blank line.
    fn end_code_block(&mut self) {
        let (ind, q) = self.indent();
        let extra = if self.lists.is_empty() { "" } else { "  " };
        for l in self.code.take().unwrap_or_default() {
            let mut spans = q.clone();
            spans.push(Span::raw(format!("{ind}{extra}  ")));
            spans.push(Span::styled(l, code_style()));
            self.doc.lines.push(Line::from(spans));
        }
        self.doc.lines.push(Line::raw(""));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(d: &Doc) -> Vec<String> {
        d.lines
            .iter()
            .map(|l| l.to_string().trim_end().to_string())
            .collect()
    }

    #[test]
    fn headings_paragraphs_lists_and_code() {
        let d = render(
            "# Title\n\nSome *words* that wrap around here.\n\n## Part\n\n- one\n- two `code`\n  - nested\n\n1. first\n2. second\n\n```sh\nnrs\n```\n",
            24,
        );
        assert_eq!(
            text(&d),
            [
                "Title",
                "─────",
                "",
                "Some words that wrap",
                "around here.",
                "",
                "Part",
                "",
                "• one",
                "• two code",
                "   • nested",
                "",
                "1. first",
                "2. second",
                "",
                "  nrs",
            ]
        );
        assert_eq!(d.headings, [0, 6]);
    }

    #[test]
    fn tables_fit_the_width() {
        let d = render(
            "| Key | What |\n| --- | --- |\n| `s` | save the theme and everything in it |\n",
            30,
        );
        let t = text(&d);
        assert_eq!(t[0], "Key │ What");
        assert!(t[1].starts_with("────┼─"));
        assert!(t.iter().all(|l| l.width() <= 30), "{t:?}");
        assert!(t.len() > 3, "the long cell wraps: {t:?}");
    }

    #[test]
    fn the_real_guide_renders_at_any_width() {
        let p = crate::model::paths::guide(&crate::model::store::Store::locate().share);
        let Ok(md) = std::fs::read_to_string(&p) else {
            eprintln!("skipped: no {}", p.display());
            return;
        };
        // (code blocks aren't wrapped; the pane cuts them off)
        for w in [40u16, 60, 80, 100] {
            let d = render(&md, w);
            assert!(d.headings.len() > 5);
            let over: Vec<String> = d
                .lines
                .iter()
                .filter(|l| l.width() > w as usize)
                .map(ToString::to_string)
                .filter(|l| !l.starts_with("  "))
                .collect();
            assert!(over.is_empty(), "width {w}: {over:#?}");
        }
    }
}

/// `cargo test show_the_guide -- --ignored --nocapture`: the real guide as
/// the README tab draws it (without colours), 90 columns wide.
#[cfg(test)]
#[test]
#[ignore = "prints the real guide; run by hand"]
fn show_the_guide() {
    let p = crate::model::paths::guide(&crate::model::store::Store::locate().share);
    let md = std::fs::read_to_string(&p).unwrap();
    for l in render(&md, 90).lines {
        println!("{l}");
    }
}
