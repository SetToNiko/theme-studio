//! The role box of the fonts column (panel 1): the three font roles.
//!
//! Each role takes three lines: the name and font, its use and size, and a
//! warning when the font does not fit the role. The box is clickable, and the
//! row under the mouse is highlighted.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::app::state::hit::Target;
use crate::fonts::presets::ROLE_USE;
use crate::fonts::types::{ROLE_NAMES, ROLE_NEEDS};
use crate::ui::truncate;
use crate::ui::widgets::{friendly, to_u16};

/// What `role_lines` needs to know about the box, so that it has few arguments.
struct RolesBox {
    /// The inside of the box (without the border).
    inner: Rect,
    /// The row of the first role (after an error line, if any).
    first_line: u16,
    /// Width of the role-name column: as wide as the longest name.
    name_w: usize,
    /// The key that swaps in a fitting face, for fonts that do not fit.
    fix_key: Option<String>,
}

/// Draws the role box ` area ` with its `title`; `focused` when it has the keyboard.
pub(super) fn draw_roles(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    title: Line<'static>,
    focused: bool,
) {
    let block = super::fonts_column::block(app, area, title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let mut lines = Vec::new();
    if let Some(error) = &app.fonts.load_error {
        lines.push(Line::styled(format!(" {error}"), Style::new().red()));
    }
    let boxed = RolesBox {
        inner,
        first_line: to_u16(lines.len()),
        name_w: ROLE_NAMES.iter().map(|n| n.len()).max().unwrap_or(0),
        fix_key: app
            .keymap
            .key_for(
                &[crate::keymap::Context::FontRoles],
                crate::keymap::action::Action::FixRoleFont,
            )
            .map(|k| friendly(&k)),
    };
    for role in 0..app.fonts.preset.fonts.len() {
        lines.extend(role_lines(app, role, &boxed));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

/// The two text lines of a role: the name with its font, and the use with the
/// size. Wide layouts put the size beside the name, narrow ones below it.
// Long function: the wide and the narrow layout build the same pieces in a different order and width.
fn role_text_lines(app: &App, role: usize, boxed: &RolesBox) -> (Line<'static>, Line<'static>) {
    let t = &app.fonts;
    let font = &t.preset.fonts[role];
    let selected = role == t.slot;
    let name_style = if selected {
        Style::new().add_modifier(Modifier::REVERSED)
    } else {
        Style::new()
    };
    let effective = t.effective_size(role, font.size);
    let size = if effective == font.size {
        font.size.to_string()
    } else {
        format!("{} → {effective} here", font.size)
    };
    let name_w = boxed.name_w;
    let start = vec![
        Span::styled(if selected { " ▸ " } else { "   " }, Style::new().yellow()),
        Span::styled(
            format!("{:<name_w$}", ROLE_NAMES[role]),
            Style::new().bold(),
        ),
    ];
    if app.layout.wide {
        let room = usize::from(boxed.inner.width)
            .saturating_sub(3 + name_w + 1 + 2 + size.chars().count());
        let mut first = start;
        // the cursor on the name only, not the space before it
        first.push(" ".into());
        first.push(Span::styled(
            truncate(&font.family, room.max(4)),
            name_style,
        ));
        first.push(Span::raw(format!("  {size}")));
        (
            Line::from(first),
            Line::from(vec![
                Span::raw("   "),
                Span::styled(ROLE_USE[role], Style::new().dark_gray()),
            ]),
        )
    } else {
        let mut first = start;
        first.push(" ".into());
        first.push(Span::styled(font.family.clone(), name_style));
        (
            Line::from(first),
            Line::from(vec![
                Span::raw(format!("   {size:<12}")),
                Span::styled(ROLE_USE[role], Style::new().dark_gray()),
            ]),
        )
    }
}

/// The third line of a role: the first warning about its font, with the fix
/// key for a misfit, or an empty line.
fn role_warning_line(app: &App, role: usize, fix_key: Option<&str>) -> Line<'static> {
    let t = &app.fonts;
    match t.warnings(role).first() {
        // a misfit: the key that swaps in a fitting face
        Some(warning) => {
            let fix = match (fix_key, t.misfit(role)) {
                (Some(key), true) => format!(" · {key} fixes"),
                _ => String::new(),
            };
            Line::styled(format!("   ⚠ {warning}{fix}"), Style::new().yellow())
        }
        None => Line::raw(""),
    }
}

/// The three lines of one role, with its click area registered. Hovering the
/// name shows what the font has to be in the status line.
// Long function: the three lines, the name hover note and the click area of one role use the same rectangle.
fn role_lines(app: &mut App, role: usize, boxed: &RolesBox) -> Vec<Line<'static>> {
    let inner = boxed.inner;
    let (first, second) = role_text_lines(app, role, boxed);
    let third = role_warning_line(app, role, boxed.fix_key.as_deref());
    let mut lines = vec![first, second, third];
    let area = Rect {
        y: inner.y + boxed.first_line + 3 * to_u16(role),
        height: 3,
        ..inner
    }
    .intersection(inner);
    // over the role's name: what its font has to be, in the status line
    let name = Rect {
        x: inner.x + 3,
        width: to_u16(boxed.name_w),
        height: 1,
        ..area
    }
    .intersection(inner);
    if app.hits.over(name) {
        app.messages.hover_note = Some(format!("{}: needs {}", ROLE_NAMES[role], ROLE_NEEDS[role]));
    }
    if app.hits.add(area, Target::FontRole(role)) {
        // the row look on all three lines; the underline only under the
        // font's name
        lines = lines
            .into_iter()
            .enumerate()
            .map(|(n, line)| {
                crate::app::state::hit::hover_line(line, if n == 0 { 2 } else { usize::MAX })
            })
            .collect();
    }
    lines
}
