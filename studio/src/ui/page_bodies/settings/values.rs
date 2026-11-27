//! A value as coloured text: shared by the History tab and the Keymaps tab.

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::model::color::Rgb;

/// A piece of `text` in the colour `c`.
pub(super) fn colour_span(c: Rgb, text: &str) -> Span<'static> {
    Span::styled(text.to_string(), Style::new().fg(Color::Rgb(c.0, c.1, c.2)))
}

/// A value as text, with a swatch in front when it's a colour.
pub(super) fn value_spans(v: &str, style: Style) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    if v.len() == 7
        && let Some(c) = Rgb::parse(v)
    {
        out.push(colour_span(c, "██ "));
    }
    out.push(Span::styled(v.to_string(), style));
    out
}
