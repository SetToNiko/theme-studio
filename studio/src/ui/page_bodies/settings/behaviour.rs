//! The Behaviour tab of the Settings page: grouped settings with values, colour
//! swatches and sample lines.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{follow, row_rect, selected_style};
use crate::app::App;
use crate::app::state::hit::{Looks, Target, hover_line, hover_row};
use crate::app::state::nav::{BhRow, LookId};

/// The Behaviour tab's value column (the longest value fits).
const VALUE_W: usize = 20;

// Long function: scroll set-up, the row loop with its hover and cursor handling, and the note line run in one top-to-bottom pass over shared state.
pub(super) fn draw_behaviour(f: &mut Frame, app: &mut App, area: Rect) {
    let rows =
        crate::settings_page::behaviour::visible_behaviour_rows(&app.settings_page.bh_closed);
    // the cursor never rests on a blank line or a sample
    if !rows[app.settings_page.bh_sel.min(rows.len() - 1)].selectable() {
        app.settings_page.bh_sel = crate::settings_page::behaviour::selectable_ends(&rows).0;
    }
    let sel = app.settings_page.bh_sel.min(rows.len() - 1);
    // the list scrolls; a note stays below it
    let [list, _, note] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    let mut scroll = app.settings_page.bh_scroll;
    follow(&mut scroll, sel, list.height as usize);
    // show a group's title when its first row is at the top
    if scroll > 0
        && matches!(rows.get(scroll - 1), Some(BhRow::Header(_)))
        && sel < scroll + list.height as usize - 1
    {
        scroll -= 1;
    }
    app.settings_page.bh_scroll = scroll;
    // the looks as edited (this is drawn with the draft in place)
    let looks = app.looks(&app.settings);
    let focused = !app.settings_page.tabs_focused && !app.layout.page_tabs_focused;
    for (i, row) in rows
        .iter()
        .enumerate()
        .skip(scroll)
        .take(list.height as usize)
    {
        let r = row_rect(list, i - scroll);
        let line = behaviour_row_line(app, row, &looks);
        let on = i == sel;
        let mut style = selected_style(on, focused);
        let mut line = line;
        if row.selectable() && app.hits.add(r, Target::SettingsRow(i)) && !on {
            // only the name is underlined
            style = hover_row(style);
            line = hover_line(line, usize::from(matches!(row, BhRow::Header(_))));
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
    f.render_widget(
        Paragraph::new(Line::styled(
            " ←/→ or Enter changes a value; click a selected row to change it.",
            Style::new().dark_gray(),
        )),
        note,
    );
}

/// The text of one row of the list: a gap, a group title, a sample line or a setting.
// Long function: one match over all kinds of Behaviour row (gap, title, two samples, button, colour, choice).
fn behaviour_row_line(app: &App, row: &BhRow, looks: &Looks) -> Line<'static> {
    match *row {
        BhRow::Gap => Line::raw(""),
        BhRow::Header(h) => Line::from(vec![
            Span::styled(
                if app.settings_page.bh_closed.contains(h) {
                    " ▸ "
                } else {
                    " ▾ "
                },
                Style::new().yellow(),
            ),
            Span::styled(h, Style::new().bold().cyan()),
        ]),
        BhRow::Sample(LookId::Hover) => Line::from(vec![
            Span::styled("   Sample:  ", Style::new().dark_gray()),
            Span::styled(" a clickable row ", looks.hover.apply(Style::new())),
            Span::styled("   pane ", Style::new().dark_gray()),
            Span::styled("──────", Style::new().fg(looks.pane)),
            Span::styled("   preview window ", Style::new().dark_gray()),
            Span::styled("──────", Style::new().fg(looks.preview)),
        ]),
        BhRow::Sample(LookId::Derived) => {
            let mut spans = vec![Span::styled("   Sample:  ", Style::new().dark_gray())];
            for (key, lit) in [
                ("bg_visual_red", false),
                ("bg0", true),
                ("red", true),
                ("green", false),
            ] {
                let st = if lit {
                    looks.derived.apply(Style::new())
                } else {
                    Style::new()
                };
                spans.push(Span::styled(format!(" {key:<14}"), st));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                " (bg_visual_red selected)",
                Style::new().dark_gray(),
            ));
            Line::from(spans)
        }
        _ => {
            // columns: label | swatch | value | what Enter does
            let label = App::behaviour_label(*row);
            let value = app.behaviour_value(*row);
            let (swatch, text, style, note) = if row.is_button() {
                (None, "[ Enter ]".to_string(), Style::new().cyan(), "")
            } else if row.is_colour() {
                // a colour: « key » in magenta, unlike the ‹ choices ›
                (
                    Some(app.theme_colour(&value)),
                    format!("« {value} »"),
                    Style::new().magenta(),
                    "Enter: choose",
                )
            } else if *row == BhRow::Leader {
                (
                    None,
                    format!("‹ {value} ›"),
                    Style::new().yellow(),
                    "Enter: press the new key",
                )
            } else {
                (None, format!("‹ {value} ›"), Style::new().yellow(), "")
            };
            Line::from(vec![
                Span::raw(format!("   {label:<50} ")),
                match swatch {
                    Some(c) => Span::styled("██ ", Style::new().fg(c)),
                    None => Span::raw("   "),
                },
                Span::styled(format!("{text:<VALUE_W$}"), style),
                Span::styled(format!("  {note}"), Style::new().dark_gray()),
            ])
        }
    }
}
