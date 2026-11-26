//! Drawing the Main page's fonts column: ¹ the three roles, ² the installed
//! fonts (only those that fit while picking for a role), ³ the sample.
//!
//! The roles box is in `fonts_roles.rs`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui_image::{Resize, StatefulImage};

use super::fonts_roles::draw_roles;
use crate::app::App;
use crate::app::state::hit::{Target, hover};
use crate::app::state::nav::Side;
use crate::fonts::types::{CHIPS, FontFocus, ROLE_NAMES, ROLE_NEEDS, Row};
use crate::ui::pane_chrome::pane_title;
use crate::ui::truncate;
use crate::ui::widgets::{panel_number, to_u16};

/// A bordered box with a title, in the pane border colour.
pub(super) fn block(app: &App, area: Rect, title: Line<'static>, focused: bool) -> Block<'static> {
    Block::bordered()
        .title(title)
        .border_style(crate::ui::pane_chrome::pane_border(app, area, focused))
}

/// ¹ the font roles and ² the installed fonts (the fonts column; the sample
/// is drawn by `draw_sample`). Roles, the search line, chips and rows are
/// clickable and highlighted under the mouse.
// Long function: the order of the drawing calls is the order on screen, and the two titles must be registered as click areas before the boxes that they belong to are drawn.
pub fn draw_fonts(f: &mut Frame, app: &mut App, slots_area: Rect, list_area: Rect) {
    // the fonts column has the keyboard (and not a preview window)
    let active =
        app.side == Side::Fonts && app.preview.focus.is_none() && !app.layout.page_tabs_focused;

    // titles first (they're clickable too)
    let roles_title = Line::from(vec![
        panel_number(1),
        // the preset's name is part of the theme now (in the tab row)
        "Fonts".into(),
        if app.fonts.dirty() {
            Span::styled(" ●", Style::new().yellow())
        } else {
            "".into()
        },
        " ".into(),
    ]);
    let roles_title = pane_title(app, slots_area, roles_title, 1);
    let list_title = Line::from(vec![
        panel_number(2),
        "Installed fonts".into(),
        // picking for a role: which (only fonts that fit it are listed)
        match app.fonts.picking {
            Some(role) => Span::styled(
                format!(" picking {} ", ROLE_NAMES[role]),
                Style::new().yellow(),
            ),
            None => Span::styled(
                format!(" for {} ", ROLE_NAMES[app.fonts.slot]),
                Style::new().dark_gray(),
            ),
        },
    ]);
    let list_title = pane_title(app, list_area, list_title, 2);

    // ¹ roles: three lines each
    draw_roles(
        f,
        app,
        slots_area,
        roles_title,
        active && app.fonts.focus == FontFocus::Slots,
    );
    // ² installed fonts: search, filter chips, then the grouped list
    let browsing = active && matches!(app.fonts.focus, FontFocus::List | FontFocus::Search);
    draw_installed(f, app, list_area, list_title, browsing);
}

/// The search line: " / search   N fonts"; clickable.
// Long function: the count, the text and the click area of the search line come from the same row list.
fn search_line(app: &mut App, inner: Rect, rows: &[Row]) -> Line<'static> {
    let t = &app.fonts;
    let fonts_shown: usize = rows
        .iter()
        .map(|row| match row {
            Row::Group {
                depth: 0, count, ..
            } => *count,
            Row::Font { depth: 0, .. } => 1,
            _ => 0,
        })
        .sum();
    let searching = t.focus == FontFocus::Search;
    let mut style = if searching {
        Style::new().yellow()
    } else {
        Style::new().dark_gray()
    };
    let text = if searching {
        format!("{}▏", t.search)
    } else if t.search.is_empty() {
        "search".to_string()
    } else {
        t.search.clone()
    };
    if app
        .hits
        .add(Rect { height: 1, ..inner }, Target::FontSearch)
    {
        style = hover(style);
    }
    Line::from(vec![
        Span::styled(" / ", style),
        Span::styled(text, style),
        Span::styled(format!("   {fonts_shown} fonts"), Style::new().dark_gray()),
    ])
}

/// The filter chips, wrapped to the panel width; each is clickable. Picking
/// for a role: its requirement comes first, locked, and the chips it decides
/// are hidden.
// Long function: chips wrap to the panel width with one running column `x`; each chip's click area depends on where the wrap put it.
fn chip_lines(app: &mut App, inner: Rect) -> Vec<Line<'static>> {
    let t = &app.fonts;
    let mut chip_lines: Vec<Vec<Span>> = vec![vec![" ".into()]];
    let mut x = 1u16;
    if let Some(role) = t.filters.need {
        let need = ROLE_NEEDS[role];
        let need = need.strip_prefix("a ").unwrap_or(need);
        let text = format!("[🔒 {need}]");
        x += to_u16(Span::raw(text.as_str()).width()) + 1;
        let line = chip_lines.last_mut().unwrap();
        line.push(Span::styled(text, Style::new().black().on_yellow()));
        line.push(" ".into());
    }
    for (chip, label) in CHIPS.into_iter().filter(|(c, _)| t.filters.chip_free(*c)) {
        let text = format!("[{chip} {label}]");
        let width = to_u16(text.chars().count());
        if x + width > inner.width && x > 1 {
            chip_lines.push(vec![" ".into()]);
            x = 1;
        }
        let row = to_u16(chip_lines.len()) - 1;
        let area = Rect {
            x: inner.x + x,
            y: inner.y + 1 + row,
            width,
            height: 1,
        };
        let mut style = if t.filters.is_on(chip) {
            Style::new().black().on_yellow()
        } else {
            Style::new().dark_gray()
        };
        if app.hits.add(area, Target::Chip(chip)) {
            style = hover(style);
        }
        let line = chip_lines.last_mut().unwrap();
        line.push(Span::styled(text, style));
        line.push(" ".into());
        x += width + 1;
    }
    chip_lines.into_iter().map(Line::from).collect()
}

/// One row of the font list: a group heading or a font with its tags.
/// `current` is the lower-case family of the selected role's font.
fn font_row_line(app: &App, row: &Row, selected: bool, current: &str) -> Line<'static> {
    let sel_style = if selected {
        Style::new().add_modifier(Modifier::REVERSED)
    } else {
        Style::new()
    };
    let indent = "  ".repeat(row.depth());
    match row {
        Row::Group {
            name,
            expanded,
            count,
            ..
        } => group_line(&indent, name, *expanded, *count, sel_style),
        Row::Font { index, .. } => font_line(app, *index, &indent, current, sel_style),
    }
}

/// A group heading: open or closed arrow, the name, and the number of fonts in it.
fn group_line(
    indent: &str,
    name: &str,
    expanded: bool,
    count: usize,
    sel_style: Style,
) -> Line<'static> {
    Line::from(vec![
        Span::raw(format!(" {indent}")),
        Span::styled(if expanded { "▾ " } else { "▸ " }, Style::new().yellow()),
        Span::styled(name.to_string(), sel_style.bold()),
        Span::styled(format!("  {count}"), Style::new().dark_gray()),
    ])
}

/// A font row: a dot when it is the role's font, the name, and its tags (mono, NF, bitmap).
fn font_line(
    app: &App,
    index: usize,
    indent: &str,
    current: &str,
    sel_style: Style,
) -> Line<'static> {
    let font = &app.fonts.installed[index];
    let width = 34usize.saturating_sub(indent.len());
    let mut spans = vec![
        Span::styled(
            if font.family.to_lowercase() == current {
                " ●"
            } else {
                "  "
            },
            Style::new().yellow(),
        ),
        Span::raw(indent.to_string()),
    ];
    // the cursor on the name, not the padding after it
    spans.extend(crate::app::state::hit::cursor_spans(
        format!("{:<width$}", truncate(&font.family, width)),
        Style::new(),
        sel_style,
    ));
    if font.mono {
        spans.push(Span::styled(" mono", Style::new().dark_gray()));
    }
    if font.nerd {
        spans.push(Span::styled(" NF", Style::new().green()));
    }
    if font.bitmap {
        spans.push(Span::styled(" bitmap", Style::new().red()));
    }
    Line::from(spans)
}

/// The note under an empty font list: no fonts at all, none fit the role, or
/// none match the search.
fn empty_list_notes(app: &App, rows: &[Row]) -> Vec<Line<'static>> {
    let t = &app.fonts;
    if t.installed.is_empty() {
        vec![Line::styled(
            "   (no fonts from fc-list)",
            Style::new().red(),
        )]
    } else if rows.is_empty()
        && t.search.is_empty()
        && let Some(role) = t.filters.need
    {
        // picking, and nothing installed fits the role
        need_notes(role)
    } else if rows.is_empty() {
        vec![Line::styled(
            "   no fonts match the search and filters",
            Style::new().dark_gray(),
        )]
    } else {
        Vec::new()
    }
}

/// The two lines when no installed font fits the role `role` (1: Google Fonts page, else Nerd Fonts).
fn need_notes(role: usize) -> Vec<Line<'static>> {
    let page = if role == 1 {
        "Google Fonts"
    } else {
        "Nerd Fonts"
    };
    vec![
        Line::styled(
            format!("   no installed font is {}", ROLE_NEEDS[role]),
            Style::new().yellow(),
        ),
        Line::styled(
            format!("   Enter: get one ({page}, on the Fonts page)"),
            Style::new().dark_gray(),
        ),
    ]
}

/// ² the installed fonts: search line, filter chips, then the grouped list
/// scrolled so that the selection shows. `browsing`: the list has the keyboard.
// Long function: search line, chips and font rows are stacked from one running line count, and the scroll window depends on the height that is left.
fn draw_installed(f: &mut Frame, app: &mut App, area: Rect, title: Line<'static>, browsing: bool) {
    let block = block(app, area, title, browsing);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 4 {
        return;
    }
    let rows = app.fonts.rows();
    let mut lines = vec![search_line(app, inner, &rows)];
    lines.extend(chip_lines(app, inner));

    let list_top = to_u16(lines.len());
    let height = usize::from(inner.height - list_top.min(inner.height));
    let sel = app.fonts.list_sel.min(rows.len().saturating_sub(1));
    let current = app.fonts.preset.fonts[app.fonts.slot].family.to_lowercase();
    // keep the selection visible
    let mut scroll = app.fonts.list_scroll;
    if sel < scroll {
        scroll = sel;
    } else if height > 0 && sel >= scroll + height {
        scroll = sel + 1 - height;
    }
    for (index, row) in rows.iter().enumerate().skip(scroll).take(height) {
        let line = font_row_line(app, row, browsing && index == sel, &current);
        let row_area = Rect {
            y: inner.y + to_u16(lines.len()),
            height: 1,
            ..inner
        };
        lines.push(if app.hits.add(row_area, Target::FontRow(index)) {
            // only the group's or font's name is underlined
            crate::app::state::hit::hover_line(line, 2)
        } else {
            line
        });
    }
    lines.extend(empty_list_notes(app, &rows));
    f.render_widget(Paragraph::new(lines), inner);
    app.fonts.list_scroll = scroll;
}

/// The sample image: the fonts drawn by Pango at this machine's sizes and
/// DPI, in the palette being edited.
// Long function: the frame, the background, the error text and the image are drawn in this order, each over the previous one.
pub fn draw_sample(f: &mut Frame, app: &mut App, area: Rect) {
    let title = Line::from(vec![panel_number(3), "Sample ".into()]);
    let title = pane_title(app, area, title, 3);
    let focused = app.side == Side::Fonts
        && app.preview.focus.is_none()
        && app.fonts.focus == FontFocus::Sample;
    let b = Block::bordered()
        .title(title)
        .border_style(crate::ui::pane_chrome::pane_border(app, area, focused));
    let inner = b.inner(area);
    app.hits.add(inner, Target::PaneBody(3));
    f.render_widget(b, area);
    f.render_widget(
        Block::new().style(Style::new().bg(app.editor.palette.get("bg0").color())),
        inner,
    );
    let inner = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: inner.height.saturating_sub(1),
    };
    if let Some(e) = &app.sample_error {
        f.render_widget(Paragraph::new(format!("Sample failed: {e}")).red(), inner);
    } else if let Some(img) = app.sample_image.as_mut() {
        // while resizing: the size it had, if that fits (else just the
        // background) until the size holds still
        let place = app.sample_settle.place(inner);
        app.redraw_at = crate::model::settle::sooner(app.redraw_at, app.sample_settle.due());
        if let Some(r) = place {
            // shrink to fit if needed; never stretch past real size
            f.render_stateful_widget(StatefulImage::default().resize(Resize::Fit(None)), r, img);
        }
    } else {
        f.render_widget(Paragraph::new("rendering sample…").dark_gray(), inner);
    }
}
