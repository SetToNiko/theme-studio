//! Small drawing pieces that several screens share.
//!
//! This is the bottom of the `ui` module tree and depends only on ratatui and
//! the colour type of `model`.
//! Keep it that way: `desktop_preview.rs` uses it and `app` uses `ui`, so if
//! this file named `app` or a page file, the modules would
//! depend on each other in a circle (scripts/dev/module-cycles.py).
use ratatui::style::Style;
use ratatui::text::Span;

use crate::model::color::Rgb;

/// Superscript panel number for a title, btop-style ("¹Palette"); the
/// number key jumps to that panel.
pub fn panel_number(n: u8) -> Span<'static> {
    const SUP: [&str; 10] = ["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
    Span::styled(format!(" {}", SUP[n as usize % 10]), Style::new().yellow())
}

/// A length or count as a terminal size: `n`, or `u16::MAX` when it does not
/// fit.
///
/// Terminal sizes are `u16` and lengths are `usize`, so this is the one place
/// where the two meet. Saturating is safe here: no terminal is that large.
pub fn to_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// An index or count of a short fixed list (tabs, panes, menus) as a `u8`:
/// `n`, or `u8::MAX` when it does not fit.
pub fn to_u8(n: usize) -> u8 {
    u8::try_from(n).unwrap_or(u8::MAX)
}

/// A rounded-down, non-negative screen position from a calculation in
/// `f64`: negative values give 0 and values that are too large give
/// `u16::MAX`.
pub fn f64_to_u16(x: f64) -> u16 {
    // `as` from f64 saturates at both ends and gives 0 for NaN, which is the
    // behaviour that the doc comment states.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "f64 to u16 `as` saturates (negative and NaN give 0), as documented above"
    )]
    let v = x as u16;
    v
}

/// A swatch: `width` cells of background colour.
pub fn swatch(c: Rgb, width: usize) -> Span<'static> {
    Span::styled(" ".repeat(width), Style::new().bg(c.color()))
}

/// Cuts `s` to `n` characters, ending with `…` when it was longer.
pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
    }
}

/// Key text for hints: `<CR>` → `Enter`, `<leader>sp` → `Space s p`,
/// `<C-r>` → `C-r`; plain characters stay as they are.
pub fn friendly(seq: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut rest = seq;
    while !rest.is_empty() {
        if rest.starts_with('<')
            && let Some(end) = rest.find('>')
            && end > 1
        {
            let name = &rest[1..end];
            parts.push(
                match name {
                    "CR" => "Enter",
                    "leader" => "Space",
                    "Up" => "↑",
                    "Down" => "↓",
                    "Left" => "←",
                    "Right" => "→",
                    other => other,
                }
                .to_string(),
            );
            rest = &rest[end + 1..];
        } else {
            let c = rest.chars().next().unwrap();
            parts.push(c.to_string());
            rest = &rest[c.len_utf8()..];
        }
    }
    parts.join(if parts.len() > 1 { " " } else { "" })
}

#[cfg(test)]
mod tests {
    use super::{f64_to_u16, friendly, swatch, to_u8, to_u16, truncate};
    use crate::model::color::Rgb;

    #[test]
    fn to_u8_saturates() {
        assert_eq!(to_u8(7), 7);
        assert_eq!(to_u8(300), u8::MAX);
    }

    #[test]
    fn to_u16_saturates() {
        assert_eq!(to_u16(7), 7);
        assert_eq!(to_u16(70_000), u16::MAX);
    }

    #[test]
    fn f64_to_u16_clamps() {
        assert_eq!(f64_to_u16(3.9), 3);
        assert_eq!(f64_to_u16(-2.0), 0);
        assert_eq!(f64_to_u16(1e9), u16::MAX);
    }

    #[test]
    fn truncate_cuts_and_marks() {
        assert_eq!(truncate("abc", 3), "abc");
        assert_eq!(truncate("abcd", 3), "ab…");
        assert_eq!(truncate("abcd", 1), "…");
        assert_eq!(truncate("abcd", 0), "…");
        assert_eq!(truncate("äöüx", 3), "äö…");
    }

    #[test]
    fn swatch_fills_width_with_colour() {
        let s = swatch(Rgb(1, 2, 3), 3);
        assert_eq!(s.content, "   ");
        assert_eq!(s.style.bg, Some(Rgb(1, 2, 3).color()));
    }

    #[test]
    fn friendly_key_names() {
        assert_eq!(friendly("<CR>"), "Enter");
        assert_eq!(friendly("<leader>sp"), "Space s p");
        assert_eq!(friendly("<C-r>"), "C-r");
        assert_eq!(friendly("j"), "j");
        assert_eq!(friendly("<lt>"), "lt");
    }
}
