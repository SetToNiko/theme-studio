//! The hint for a half-typed key sequence.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::app::app_state::App;
use crate::ui::widgets::to_u16;

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
