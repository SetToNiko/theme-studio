//! Small drawing pieces that several screens share.
//!
//! This is the bottom of the `ui` module tree and depends only on ratatui.
//! Keep it that way: `preview.rs` uses it and `app` uses `preview.rs`, so if
//! this file named `app`, `ui/mod.rs`, or a page file, the modules would
//! depend on each other in a circle (scripts/dev/module-cycles.py).
use ratatui::style::Style;
use ratatui::text::Span;

/// Superscript panel number for a title, btop-style ("¹Palette"); the
/// number key jumps to that panel.
pub fn panel_number(n: u8) -> Span<'static> {
    const SUP: [&str; 10] = ["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
    Span::styled(format!(" {}", SUP[n as usize % 10]), Style::new().yellow())
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
    use super::friendly;

    #[test]
    fn friendly_key_names() {
        assert_eq!(friendly("<CR>"), "Enter");
        assert_eq!(friendly("<leader>sp"), "Space s p");
        assert_eq!(friendly("<C-r>"), "C-r");
        assert_eq!(friendly("j"), "j");
        assert_eq!(friendly("<lt>"), "lt");
    }
}
