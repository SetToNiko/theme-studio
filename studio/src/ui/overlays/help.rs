//! The key-list overlay (help): every key of the active keymap, in columns.
//!
//! [`draw_help`] is called by `dialogs::draw_overlay` while `Overlay::Help` is open.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::centered;
use crate::app::App;
use crate::keymap::Context;
use crate::keymap::action::Action;
use crate::ui::widgets::to_u16;

/// Width of one column of the key list: wide enough to read.
const COL_W: u16 = 52;

/// The key list, in columns, generated from the keymap (`!90`/`!92` page through them).
pub(super) fn draw_help(f: &mut Frame, app: &App, area: Rect, page: usize) {
    let lines = help_lines(app);
    // when the columns don't all fit, !90/!92 page
    let r = centered(
        area,
        area.width.saturating_sub(4).min(4 * COL_W + 2),
        area.height.saturating_sub(2),
    );
    f.render_widget(Clear, r);
    let inner_h = r.height.saturating_sub(2) as usize;
    let per_col = inner_h.max(1);
    let cols: Vec<&[Line]> = lines.chunks(per_col).collect();
    let fit = (r.width.saturating_sub(2) / COL_W).max(1) as usize;
    let page = page.min(cols.len().saturating_sub(fit));
    let more = more_text(page, fit, cols.len());
    let block = Block::bordered()
        .title(format!(" Keys: {} preset ", app.keymap.name))
        .title_bottom(Line::styled(more, Style::new().yellow()))
        .border_style(Style::new().yellow());
    let inner = block.inner(r);
    f.render_widget(block, r);
    for (i, chunk) in cols.iter().skip(page).take(fit).enumerate() {
        let left = to_u16(i) * COL_W;
        let col = Rect {
            x: inner.x + left,
            width: COL_W.min(inner.right().saturating_sub(inner.x + left)),
            ..inner
        };
        f.render_widget(Paragraph::new(chunk.to_vec()), col);
    }
}

/// The note under the box when the columns do not all fit: which ones show.
fn more_text(page: usize, fit: usize, total: usize) -> String {
    if total > fit {
        format!(
            " columns {} 13{} of {} (!90/!92 for more) ",
            page + 1,
            (page + fit).min(total),
            total
        )
    } else {
        String::new()
    }
}

/// All lines of the key list: one block per context, then the footer.
///
/// Generated from the keymap, so it shows what the keys really are.
// Long function: one pass over `Context::ALL` in the order the keys are shown, then the role-needs rows and the footer.
fn help_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    for ctx in Context::ALL {
        let rows = context_rows(app, ctx);
        if rows.is_empty() {
            continue;
        }
        lines.push(Line::styled(
            format!(" {}", ctx.title()),
            Style::new().bold(),
        ));
        for (action, keys) in rows {
            lines.push(Line::from(vec![
                Span::styled(format!("   {:<20}", keys.join(" ")), Style::new().yellow()),
                Span::raw(action.describe()),
            ]));
        }
        // what each role's font has to be (the list shows only those)
        if ctx == Context::FontRoles {
            for (name, need) in crate::fonts::types::ROLE_NAMES
                .iter()
                .zip(crate::fonts::types::ROLE_NEEDS)
            {
                lines.push(Line::from(vec![
                    Span::styled(format!("   {name:<20}"), Style::new().dark_gray()),
                    Span::raw(format!("needs {need}")),
                ]));
            }
        }
    }
    lines.push(Line::styled(
        format!(
            " Leader: {}   Text fields take keys as typing.   ←/→ more columns; any other key closes.",
            crate::keymap::key::display_seq(&[app.keymap.leader], crate::keymap::key::Key::new(ratatui::crossterm::event::KeyCode::Null, ratatui::crossterm::event::KeyModifiers::NONE))
        ),
        Style::new().dark_gray(),
    ));
    lines
}

/// The rows of one context: one row per action, with all its keys together.
fn context_rows(app: &App, ctx: Context) -> Vec<(Action, Vec<String>)> {
    let mut rows: Vec<(Action, Vec<String>)> = Vec::new();
    for b in app.keymap.bindings.iter().filter(|b| b.context == ctx) {
        let keys = crate::keymap::key::display_seq(&b.keys, app.keymap.leader);
        match rows.iter_mut().find(|(action, _)| *action == b.action) {
            Some((_, all_keys)) => all_keys.push(keys),
            None => rows.push((b.action, vec![keys])),
        }
    }
    rows
}
