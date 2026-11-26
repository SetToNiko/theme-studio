//! The Keymaps tab of the Settings page: the preset row, the key bindings by
//! context, and an info column with the selected row and warnings.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use super::{follow, row_rect, selected_style, truncate};
use crate::app::App;
use crate::app::state::hit::{Target, hover_line, hover_row};
use crate::keymap::Keymap;
use crate::keymap::key::display_seq;
use crate::settings_page::KmRow;

// Long function: layout, the preset browser, the row loop with hover and cursor handling, and the info column run in one pass over shared state.
pub(super) fn draw_keymaps(f: &mut Frame, app: &mut App, area: Rect) {
    let side = area.width >= 100;
    let (list, info) = if side {
        let [l, _, i] = Layout::horizontal([
            Constraint::Min(56),
            Constraint::Length(1),
            Constraint::Length(44),
        ])
        .areas(area);
        (l, Some(i))
    } else {
        (area, None)
    };
    // the preset list: the difference on the left, the list on the right
    // (stacked when narrow)
    if app.settings_page.browse.is_some() {
        let (diff, presets) = match info {
            Some(i) => (list, i),
            None => {
                let h = app.presets.len() as u16 + 10;
                let [p, _, d] = Layout::vertical([
                    Constraint::Length(h),
                    Constraint::Length(1),
                    Constraint::Min(3),
                ])
                .areas(area);
                (d, p)
            }
        };
        return crate::settings_page::keymap_presets::draw_browse(
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
    let rows = app.keymap_rows();
    let n = rows.len();
    app.settings_page.km_sel = app.settings_page.km_sel.min(n.saturating_sub(1));
    let sel = app.settings_page.km_sel;
    let mut scroll = app.settings_page.km_scroll;
    follow(&mut scroll, sel, list.height as usize);
    app.settings_page.km_scroll = scroll;
    let shipped = Keymap::builtin()
        .into_iter()
        .find(|b| b.name == app.keymap.name);
    let w = list.width as usize;
    for (i, row) in rows
        .iter()
        .enumerate()
        .skip(scroll)
        .take(list.height as usize)
    {
        let r = row_rect(list, i - scroll);
        let on = i == sel;
        let line = keymaps_row_line(app, row, shipped.as_ref(), w);
        let clickable = !matches!(row, KmRow::Gap);
        let mut style = selected_style(on, true);
        let mut line = line;
        if clickable && app.hits.add(r, Target::SettingsRow(i)) && !on {
            // only the name is underlined
            style = hover_row(style);
            line = hover_line(
                line,
                if matches!(row, KmRow::Header(_)) {
                    1
                } else {
                    0
                },
            );
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
    let lines = km_info_lines(app, rows.get(sel), shipped.as_ref());
    if let Some(info) = info {
        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                Block::new()
                    .borders(ratatui::widgets::Borders::LEFT)
                    .border_style(Style::new().dark_gray()),
            ),
            info,
        );
    }
}

/// The text of one row of the list: the preset, a gap, a context title or a binding.
/// `shipped` is the built-in keymap of the same name (to mark changed keys); `w` is the list width.
// Long function: one match over the three row kinds (preset, gap, context title, binding).
fn keymaps_row_line(app: &App, row: &KmRow, shipped: Option<&Keymap>, w: usize) -> Line<'static> {
    let leader = app.keymap.leader;
    match *row {
        KmRow::Preset => {
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
        KmRow::Gap => Line::raw(""),
        KmRow::Header(ctx) => {
            let open = !app.settings_page.km_closed.contains(&ctx);
            Line::from(vec![
                Span::styled(if open { " ▾ " } else { " ▸ " }, Style::new().yellow()),
                Span::styled(ctx.title(), Style::new().bold().cyan()),
            ])
        }
        KmRow::Binding(ctx, action) => {
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
    }
}

/// The lines of the info column: what the selected row is, what can be done, and warnings.
// Long function: one match over the selected row kind, then the keymap warnings.
fn km_info_lines(
    app: &App,
    selected: Option<&KmRow>,
    shipped: Option<&Keymap>,
) -> Vec<Line<'static>> {
    let leader = app.keymap.leader;
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
            lines.push(Line::styled(action.describe(), Style::new().bold()));
            lines.push(Line::styled(
                format!("in {} · {}", ctx.title(), action.id()),
                Style::new().dark_gray(),
            ));
            lines.push(Line::raw(""));
            if let Some(s) = shipped {
                let k: Vec<String> = s
                    .keys_of(*ctx, *action)
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
            for k in app.keymap.keys_of(*ctx, *action) {
                for c in app.keymap.conflicts(*ctx, *action, &k) {
                    lines.push(Line::styled(format!("! {c}"), Style::new().red()));
                }
            }
            lines.push(Line::raw(""));
            lines.push(Line::raw("Enter  press new keys (replace)"));
            lines.push(Line::raw("a      press keys to add"));
            lines.push(Line::raw("i      type keys (Vim notation)"));
            lines.push(Line::raw("d      unbind"));
            lines.push(Line::raw("n      save the keymap as a preset"));
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
