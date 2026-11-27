//! The save notice: a confirmation in a box (settings.messages.saved = "popup").

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};

use super::boxes::centered;
use crate::app::App;
use crate::ui::widgets::to_u16;

/// Draws the save notice in a box, when there is one.
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
