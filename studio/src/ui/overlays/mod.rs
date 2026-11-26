//! Overlays drawn on top of a page: the dialogs, the key-sequence hint and the save notice.
//!
//! `popup` and `centered` are the shared box helpers; only overlays use them.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::app::App;
use crate::ui::widgets::to_u16;

pub mod dialogs;
mod help;

/// A save confirmation in a box (settings.messages.saved = "popup").
pub fn draw_notice(f: &mut Frame, app: &App, area: Rect) {
    let Some(text) = &app.messages.notice else {
        return;
    };
    let hint = "press any key to dismiss";
    let w = to_u16(text.chars().count().max(hint.len())) + 4;
    let lines = vec![
        Line::styled(format!(" {text}"), Style::new().green().bold()),
        Line::raw(""),
        Line::styled(format!(" {hint}"), Style::new().dark_gray()),
    ];
    let r = centered(area, w, 5);
    f.render_widget(Clear, r);
    let block = Block::bordered().border_style(Style::new().green());
    f.render_widget(Paragraph::new(lines).block(block), r);
}

pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

pub fn popup(f: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
    f.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(" {title} "))
        .border_style(Style::new().yellow());
    f.render_widget(Paragraph::new(lines).block(block), area);
}

/// While a sequence is half typed (e.g. after the leader): what can follow.
pub fn draw_pending(f: &mut Frame, app: &App, area: Rect) {
    if app.pending_keys.is_empty() {
        return;
    }
    let ctx = app.contexts();
    let next = app.keymap.continuations(&ctx, &app.pending_keys);
    let typed = crate::keymap::key::display_seq(&app.pending_keys, app.keymap.leader);
    let mut lines: Vec<Line> = next
        .iter()
        .map(|(k, what)| {
            Line::from(vec![
                Span::styled(format!(" {k:<6}"), Style::new().yellow()),
                Span::raw(what.clone()),
            ])
        })
        .collect();
    lines.push(Line::styled(" Esc cancels", Style::new().dark_gray()));
    let w = to_u16(lines.iter().map(Line::width).max().unwrap_or(10)) + 4;
    let h = to_u16(lines.len()) + 2;
    let r = Rect {
        x: area.x + area.width.saturating_sub(w + 1),
        y: area.y + area.height.saturating_sub(h + 1),
        width: w.min(area.width),
        height: h.min(area.height),
    };
    f.render_widget(Clear, r);
    let block = Block::bordered()
        .title(format!(" {typed} "))
        .border_style(Style::new().yellow());
    f.render_widget(Paragraph::new(lines).block(block), r);
}
