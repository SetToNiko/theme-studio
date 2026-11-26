//! The Palette pane: the palette dropdown row and the list of colour keys.
//!
//! The list shows groups (open or closed) and keys with their swatch, hex
//! value, derivation source and contrast. Each row is clickable.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::app::state::hit::{Target, hover};
use crate::app::state::nav::{ColourRow, Focus, Overlay, Side};
use crate::model::color::{self, Rgb};
use crate::model::palette::{self, GROUPS, TEXT_KEYS};
use crate::ui::pane_chrome::pane_title;
use crate::ui::widgets::{panel_number, to_u16};
use crate::ui::{focused_block, swatch};

/// Draws the Palette pane: frame, dropdown row, then the scrolled colour rows.
pub fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    app.layout.palette_pane = Some(area);
    // the title names the palette; the dropdown is the pane's first row
    let title = pane_title(app, area, list_title(app), 4);
    let focused = app.side == Side::Colours
        && app.focus == Focus::List
        && app.preview.focus.is_none()
        && !app.layout.page_tabs_focused;
    let block = focused_block(app, area, title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let [menu_row, inner] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    draw_palette_menu(f, app, menu_row, focused && app.layout.palette_menu_focused);
    // the colour rows have the cursor unless the dropdown row does
    let focused = focused && !app.layout.palette_menu_focused;

    let rows = app.colour_rows();
    let lines = row_lines(app, &rows, focused);
    scroll_to_cursor(app, inner.height as usize, rows.len());
    let shown: Vec<Line> = lines
        .into_iter()
        .skip(app.list_scroll)
        .take(inner.height as usize)
        .collect();
    let shown = add_row_hits(app, shown, &rows, inner);
    f.render_widget(Paragraph::new(shown), inner);
}

/// The pane title: the palette's name, with a dot when it has unsaved changes.
fn list_title(app: &App) -> Line<'static> {
    let dot = if app.dirty() {
        Span::styled(" ●", Style::new().yellow())
    } else {
        "".into()
    };
    Line::from(vec![
        panel_number(4),
        "Palette: ".into(),
        app.name.clone().into(),
        dot,
        " ".into(),
    ])
}

/// The selected colour's relatives by derivation (pinned keys are set by
/// hand, so they're left out). They get the derived-colour highlight.
fn related_keys(app: &App) -> Vec<&'static str> {
    let selected = app.keys[app.sel];
    if app.editor.palette.is_pinned(selected) {
        Vec::new()
    } else {
        palette::related(selected)
            .into_iter()
            .filter(|k| palette::is_base(k) || !app.editor.palette.is_pinned(k))
            .collect()
    }
}

/// One line per row (all rows, before scrolling).
fn row_lines(app: &App, rows: &[ColourRow], focused: bool) -> Vec<Line<'static>> {
    let copy_target = match app.overlay {
        Overlay::CopyFrom { target } => Some(target),
        _ => None,
    };
    let related = related_keys(app);
    let mut lines = Vec::with_capacity(rows.len());
    for (r, row) in rows.iter().enumerate() {
        let cursor = r == app.row_sel && (focused || copy_target.is_some());
        let cursor_style = if cursor {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new()
        };
        lines.push(match *row {
            ColourRow::Group { index, expanded } => {
                group_line(index, expanded, cursor_style, &related)
            }
            ColourRow::Key(i) => key_line(app, i, cursor, copy_target, &related),
        });
    }
    lines
}

/// A group header row: its name and key count.
fn group_line(
    index: usize,
    expanded: bool,
    cursor_style: Style,
    related: &[&str],
) -> Line<'static> {
    let (name, keys) = GROUPS[index];
    let line = Line::from(vec![
        Span::styled(if expanded { " ▾ " } else { " ▸ " }, Style::new().yellow()),
        Span::styled(name, cursor_style.bold()),
        Span::styled(format!("  {}", keys.len()), Style::new().dark_gray()),
    ]);
    // a closed group stands in for its related keys
    if !expanded && keys.iter().any(|k| related.contains(k)) {
        line.patch_style(crate::app::state::hit::derived(Style::new()))
    } else {
        line
    }
}

/// The style of a key name: reversed under the cursor, underlined when selected.
fn name_style(cursor: bool, is_selected: bool) -> Style {
    if cursor {
        Style::new().add_modifier(Modifier::REVERSED)
    } else if is_selected {
        Style::new().bold().underlined()
    } else {
        Style::new()
    }
}

/// The marker before a key: an arrow for the copy target, a pin for a pinned key.
fn row_marker(copy_target: Option<&str>, key: &str, pinned: bool) -> Span<'static> {
    if copy_target == Some(key) {
        Span::styled("→", Style::new().yellow())
    } else if pinned {
        Span::styled("", Style::new().yellow())
    } else {
        " ".into()
    }
}

/// The derivation source of a key ("⟵ bg0"), or nothing for a base or pinned key.
fn derivation_text(key: &str, pinned: bool) -> String {
    if palette::is_base(key) || pinned {
        String::new()
    } else {
        format!("⟵ {}", palette::sources(key))
    }
}

/// A key row: marker, name, swatch, hex, derivation source and (for text
/// keys) the contrast against the background.
fn key_line(
    app: &App,
    i: usize,
    cursor: bool,
    copy_target: Option<&str>,
    related: &[&str],
) -> Line<'static> {
    let key = app.keys[i];
    let c = app.editor.palette.get(key);
    let pinned = app.editor.palette.is_pinned(key);
    let name_look = name_style(cursor, i == app.sel);
    let marker = row_marker(copy_target, key, pinned);
    let mut spans = vec!["  ".into(), marker, " ".into()];
    // the cursor on the name, not the padding before the swatch
    spans.extend(crate::app::state::hit::cursor_spans(
        format!("{key:<16}"),
        Style::new(),
        name_look,
    ));
    spans.push(swatch(c, 4));
    spans.push(Span::raw(format!(" {} ", c.hex())));
    let info = derivation_text(key, pinned);
    spans.push(Span::styled(
        format!("{info:<11}"),
        Style::new().dark_gray(),
    ));
    if TEXT_KEYS.contains(&key) {
        spans.push(contrast_span(c, app.editor.palette.get("bg0")));
    }
    if related.contains(&key) {
        Line::from(spans).patch_style(crate::app::state::hit::derived(Style::new()))
    } else {
        Line::from(spans)
    }
}

/// The contrast ratio against the background, red when it is poor.
fn contrast_span(c: Rgb, bg0: Rgb) -> Span<'static> {
    let ratio = color::contrast(c, bg0);
    let style = if ratio < 3.0 {
        Style::new().red().bold()
    } else if ratio < 4.5 {
        Style::new().yellow()
    } else {
        Style::new().dark_gray()
    };
    Span::styled(format!("{ratio:>4.1}:1"), style)
}

/// Scrolls so the cursor stays visible in `h` rows.
fn scroll_to_cursor(app: &mut App, h: usize, row_count: usize) {
    if app.row_sel < app.list_scroll {
        app.list_scroll = app.row_sel;
    } else if h > 0 && app.row_sel >= app.list_scroll + h {
        app.list_scroll = app.row_sel + 1 - h;
    }
    app.list_scroll = app.list_scroll.min(row_count.saturating_sub(h.max(1)));
}

/// Makes each shown row clickable; the one under the mouse is highlighted.
fn add_row_hits<'a>(
    app: &mut App,
    shown: Vec<Line<'a>>,
    rows: &[ColourRow],
    inner: Rect,
) -> Vec<Line<'a>> {
    shown
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let r = Rect {
                y: inner.y + to_u16(i),
                height: 1,
                ..inner
            };
            if app.hits.add(r, Target::ColourRow(app.list_scroll + i)) {
                // only the group's or key's name is underlined
                let name = match rows.get(app.list_scroll + i) {
                    Some(ColourRow::Group { .. }) => 1,
                    _ => 3,
                };
                crate::app::state::hit::hover_line(line, name)
            } else {
                line
            }
        })
        .collect()
}

/// The Palette pane's dropdown row: " CRT Amber ▾ " and the palette's base
/// colours. Enter (with the keyboard on it) or a click opens the list.
fn draw_palette_menu(f: &mut Frame, app: &mut App, area: Rect, cursor: bool) {
    let text = format!(" {} ▾ ", app.name);
    let w = to_u16(text.chars().count()).min(area.width);
    let hit = Rect { width: w, ..area };
    let base = Style::new().yellow().bold();
    let mut spans = if cursor {
        crate::app::state::hit::cursor_spans(text, base, Style::new().reversed())
    } else {
        vec![Span::styled(text, base)]
    };
    if app.hits.add(hit, Target::PaletteMenu) && !cursor {
        spans = vec![Span::styled(spans[0].content.to_string(), hover(base))];
    }
    spans.push(" ".into());
    for k in [
        "bg0", "fg0", "red", "orange", "yellow", "green", "aqua", "blue", "purple",
    ] {
        spans.push(swatch(app.editor.palette.get(k), 2));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
