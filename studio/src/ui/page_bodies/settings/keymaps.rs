//! The Keymaps tab of the Settings page: the preset row, the key bindings by
//! context, and an info column with the selected row and warnings.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use super::super::rows::{RowLook, draw_selectable_row, follow, row_rect};
use super::truncate;
use crate::app::App;
use crate::app::state::hit::Target;
use crate::keymap::Context;
use crate::keymap::Keymap;
use crate::keymap::action::Action;
use crate::keymap::key::display_seq;
use crate::settings_page::KmRow;
use crate::ui::widgets::to_u16;
/// Splits the tab area into the list and, when the width allows, the info column.
fn split_keymaps_area(area: Rect) -> (Rect, Option<Rect>) {
    if area.width >= 100 {
        let [l, _, i] = Layout::horizontal([
            Constraint::Min(56),
            Constraint::Length(1),
            Constraint::Length(44),
        ])
        .areas(area);
        (l, Some(i))
    } else {
        (area, None)
    }
}

/// The preset browser: the difference on the left, the list of presets on the
/// right (stacked when narrow, where `info` is `None`).
fn draw_keymap_browse(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    (list, info): (Rect, Option<Rect>),
) {
    let (diff, presets) = if let Some(info) = info {
        (list, info)
    } else {
        let h = to_u16(app.presets.len()) + 10;
        let [p, _, d] = Layout::vertical([
            Constraint::Length(h),
            Constraint::Length(1),
            Constraint::Min(3),
        ])
        .areas(area);
        (d, p)
    };
    crate::settings_page::keymap_presets::draw_browse(
        f,
        crate::settings_page::keymap_presets::BrowseParts {
            hits: &mut app.hits,
            presets: &app.presets,
            keymap: &app.keymap,
        },
        &mut app.settings_page,
        diff,
        presets,
    );
}

/// The visible binding rows of the list, scrolled to keep the selected row on screen.
fn draw_keymap_rows(
    f: &mut Frame,
    app: &mut App,
    list: Rect,
    rows: &[KmRow],
    shipped: Option<&Keymap>,
) {
    let sel = app.settings_page.km_sel;
    let mut scroll = app.settings_page.km_scroll;
    follow(&mut scroll, sel, list.height as usize);
    app.settings_page.km_scroll = scroll;
    let w = list.width as usize;
    for (i, row) in rows
        .iter()
        .enumerate()
        .skip(scroll)
        .take(list.height as usize)
    {
        let look = RowLook {
            selected: i == sel,
            focused: true,
            target: (!matches!(row, KmRow::Gap)).then_some(Target::SettingsRow(i)),
            underline_skip: usize::from(matches!(row, KmRow::Header(_))),
            hover_when_selected: false,
        };
        let line = keymaps_row_line(app, row, shipped, w);
        draw_selectable_row(f, app, row_rect(list, i - scroll), line, look);
    }
}

/// The info column (when there is room): the selected row, what to do, and warnings.
fn draw_keymap_info(
    f: &mut Frame,
    app: &App,
    info: Option<Rect>,
    row: Option<&KmRow>,
    shipped: Option<&Keymap>,
) {
    let Some(info) = info else { return };
    f.render_widget(
        Paragraph::new(km_info_lines(app, row, shipped))
            .wrap(Wrap { trim: false })
            .block(
                Block::new()
                    .borders(ratatui::widgets::Borders::LEFT)
                    .border_style(Style::new().dark_gray()),
            ),
        info,
    );
}

/// Draws the Keymaps tab: the browser, or the rows with an info column.
pub(super) fn draw_keymaps(f: &mut Frame, app: &mut App, area: Rect) {
    let (list, info) = split_keymaps_area(area);
    if app.settings_page.browse.is_some() {
        return draw_keymap_browse(f, app, area, (list, info));
    }
    let rows = app.keymap_rows();
    app.settings_page.km_sel = app.settings_page.km_sel.min(rows.len().saturating_sub(1));
    let shipped = Keymap::builtin()
        .into_iter()
        .find(|b| b.name == app.keymap.name);
    draw_keymap_rows(f, app, list, &rows, shipped.as_ref());
    let selected = rows.get(app.settings_page.km_sel);
    draw_keymap_info(f, app, info, selected, shipped.as_ref());
}

/// The text of one row of the list: the preset, a gap, a context title or a binding.
/// `shipped` is the built-in keymap of the same name (to mark changed keys); `w` is the list width.
fn keymaps_row_line(app: &App, row: &KmRow, shipped: Option<&Keymap>, w: usize) -> Line<'static> {
    match *row {
        KmRow::Preset => preset_line(app, shipped),
        KmRow::Gap => Line::raw(""),
        KmRow::Header(ctx) => {
            let open = !app.settings_page.km_closed.contains(&ctx);
            Line::from(vec![
                Span::styled(if open { " ▾ " } else { " ▸ " }, Style::new().yellow()),
                Span::styled(ctx.title(), Style::new().bold().cyan()),
            ])
        }
        KmRow::Binding(ctx, action) => binding_line(app, (ctx, action), shipped, w),
    }
}

/// The preset row: the name, its position in the list, and whether it is shipped or yours.
fn preset_line(app: &App, shipped: Option<&Keymap>) -> Line<'static> {
    let pos = app
        .presets
        .iter()
        .position(|p| p.name == app.keymap.name)
        .map_or(0, |p| p + 1);
    Line::from(vec![
        Span::styled(" Preset  ", Style::new().bold()),
        Span::styled(
            format!("[ {} ▾ ]", app.keymap.name),
            Style::new().yellow().bold(),
        ),
        Span::styled(
            format!(
                "  {pos}/{}  {}",
                app.presets.len(),
                if shipped.is_some() {
                    "shipped"
                } else {
                    "yours"
                }
            ),
            Style::new().dark_gray(),
        ),
    ])
}

/// A binding row: the action, a mark when its keys changed, the keys, and a clash warning.
fn binding_line(
    app: &App,
    (ctx, action): (Context, Action),
    shipped: Option<&Keymap>,
    w: usize,
) -> Line<'static> {
    let leader = app.keymap.leader;
    let keys = app.keymap.keys_of(ctx, action);
    let text: Vec<String> = keys.iter().map(|k| display_seq(k, leader)).collect();
    let changed = shipped
        .as_ref()
        .is_some_and(|s| s.keys_of(ctx, action) != keys);
    let clash = keys
        .iter()
        .any(|k| !app.keymap.conflicts(ctx, action, k).is_empty());
    let label_w = w.saturating_sub(24).clamp(20, 48);
    let mut spans = vec![
        Span::raw(format!(
            "   {:<label_w$}",
            truncate(&action.describe(), label_w)
        )),
        Span::styled(if changed { "• " } else { "  " }, Style::new().cyan()),
    ];
    if text.is_empty() {
        spans.push(Span::styled("no keys", Style::new().dark_gray().italic()));
    } else {
        spans.push(Span::styled(text.join("  "), Style::new().yellow()));
    }
    if clash {
        spans.push(Span::styled("  ! clash", Style::new().red().bold()));
    }
    Line::from(spans)
}

/// The lines of the info column: what the selected row is, what can be done, and warnings.
fn km_info_lines(
    app: &App,
    selected: Option<&KmRow>,
    shipped: Option<&Keymap>,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    match selected {
        Some(KmRow::Preset) => {
            lines.push(Line::styled("Keymap preset", Style::new().bold()));
            lines.push(Line::raw("Enter or a click opens the list of"));
            lines.push(Line::raw("presets: see what each would change,"));
            lines.push(Line::raw("load one, make a new one from it,"));
            lines.push(Line::raw("delete or reset one."));
        }
        Some(KmRow::Binding(ctx, action)) => {
            lines.extend(binding_info_lines(app, (*ctx, *action), shipped));
        }
        _ => {}
    }
    let warnings = app.keymap.warnings();
    if !warnings.is_empty() {
        lines.push(Line::raw(""));
        for w in warnings {
            lines.push(Line::styled(format!("⚠ {w}"), Style::new().red()));
        }
    }
    lines
}

/// The info lines of a binding: its name, shipped keys, clashes and what each key does.
// Long function: the order of the lines is the order on screen: name, shipped keys, clashes, then the key help.
fn binding_info_lines(
    app: &App,
    (ctx, action): (Context, Action),
    shipped: Option<&Keymap>,
) -> Vec<Line<'static>> {
    let leader = app.keymap.leader;
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::styled(action.describe(), Style::new().bold()));
    lines.push(Line::styled(
        format!("in {} · {}", ctx.title(), action.id()),
        Style::new().dark_gray(),
    ));
    lines.push(Line::raw(""));
    if let Some(s) = shipped {
        let k: Vec<String> = s
            .keys_of(ctx, action)
            .iter()
            .map(|k| display_seq(k, leader))
            .collect();
        lines.push(Line::from(vec![
            Span::raw("shipped: "),
            Span::styled(
                if k.is_empty() {
                    "no keys".into()
                } else {
                    k.join("  ")
                },
                Style::new().yellow(),
            ),
        ]));
    }
    for k in app.keymap.keys_of(ctx, action) {
        for c in app.keymap.conflicts(ctx, action, &k) {
            lines.push(Line::styled(format!("! {c}"), Style::new().red()));
        }
    }
    lines.push(Line::raw(""));
    lines.push(Line::raw("Enter  press new keys (replace)"));
    lines.push(Line::raw("a      press keys to add"));
    lines.push(Line::raw("i      type keys (Vim notation)"));
    lines.push(Line::raw("d      unbind"));
    lines.push(Line::raw("n      save the keymap as a preset"));
    lines
}
