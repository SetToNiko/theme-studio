//! The box helpers that the overlays share: `centered` and `popup`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};

/// A rectangle of `w` by `h` cells in the middle of `area`, cut to fit.
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

/// Draws a yellow box with `title` and `lines` in `area`, over what is there.
pub fn popup(f: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
    f.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(" {title} "))
        .border_style(Style::new().yellow());
    f.render_widget(Paragraph::new(lines).block(block), area);
}
