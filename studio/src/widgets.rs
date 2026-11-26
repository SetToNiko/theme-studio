//! Small drawing pieces that several screens share.
//!
//! This module depends only on ratatui. Keep it that way: `preview.rs`,
//! `fonts_ui.rs`, and `ui.rs` all use it, and if it named any of them, the
//! modules would depend on each other in a circle (scripts/dev/module-cycles.py).

use ratatui::style::Style;
use ratatui::text::Span;

/// Superscript panel number for a title, btop-style ("¹Palette"); the
/// number key jumps to that panel.
pub fn panel_number(n: u8) -> Span<'static> {
    const SUP: [&str; 10] = ["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
    Span::styled(format!(" {}", SUP[n as usize % 10]), Style::new().yellow())
}
