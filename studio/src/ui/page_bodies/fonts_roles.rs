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

use crate::app::app_state::App;
use crate::app::state::hit::Target;
use crate::fonts::presets::ROLE_USE;
use crate::fonts::types::{ROLE_NAMES, ROLE_NEEDS};
use crate::ui::widgets::truncate;
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
                &[crate::keymap::context::Context::FontRoles],
                crate::keymap::action::Action::FixRoleFont,
            )
            .map(|k| friendly(&k)),
    };
    for role in 0..app.fonts.preset.fonts.len() {
        lines.extend(role_lines(app, role, &boxed));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

/// The size of a role as text: "12", or "12 → 10 here" when the layout shrinks it.
fn role_size_text(app: &App, role: usize) -> String {
    let t = &app.fonts;
    let size = t.preset.fonts[role].size;
    let effective = t.effective_size(role, size);
    if effective == size {
        size.to_string()
    } else {
        format!("{size} → {effective} here")
    }
}

/// The start of a role's first line: the marker, the name of the role, and
/// `family`, the font's name (only that is under the cursor).
fn role_start_spans(app: &App, role: usize, name_w: usize, family: String) -> Vec<Span<'static>> {
    let selected = role == app.fonts.slot;
    let name_style = if selected {
        Style::new().add_modifier(Modifier::REVERSED)
    } else {
        Style::new()
    };
    vec![
        Span::styled(if selected { " ▸ " } else { "   " }, Style::new().yellow()),
        Span::styled(
            format!("{:<name_w$}", ROLE_NAMES[role]),
            Style::new().bold(),
        ),
        // the cursor on the name only, not the space before it
        " ".into(),
        Span::styled(family, name_style),
    ]
}

/// The two lines of a role in the wide layout: the size beside the font.
fn wide_role_lines(app: &App, role: usize, boxed: &RolesBox) -> (Line<'static>, Line<'static>) {
    let font = &app.fonts.preset.fonts[role];
    let size = role_size_text(app, role);
    let room = usize::from(boxed.inner.width)
        .saturating_sub(3 + boxed.name_w + 1 + 2 + size.chars().count());
    let family = truncate(&font.family, room.max(4));
    let mut first = role_start_spans(app, role, boxed.name_w, family);
    first.push(Span::raw(format!("  {size}")));
    (
        Line::from(first),
        Line::from(vec![
            Span::raw("   "),
            Span::styled(ROLE_USE[role], Style::new().dark_gray()),
        ]),
    )
}

/// The two lines of a role in the narrow layout: the size below the font.
fn narrow_role_lines(app: &App, role: usize, name_w: usize) -> (Line<'static>, Line<'static>) {
    let family = app.fonts.preset.fonts[role].family.clone();
    let size = role_size_text(app, role);
    (
        Line::from(role_start_spans(app, role, name_w, family)),
        Line::from(vec![
            Span::raw(format!("   {size:<12}")),
            Span::styled(ROLE_USE[role], Style::new().dark_gray()),
        ]),
    )
}

/// The two text lines of a role: the name with its font, and the use with the
/// size. Wide layouts put the size beside the name, narrow ones below it.
fn role_text_lines(app: &App, role: usize, boxed: &RolesBox) -> (Line<'static>, Line<'static>) {
    if app.layout.wide {
        wide_role_lines(app, role, boxed)
    } else {
        narrow_role_lines(app, role, boxed.name_w)
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
// Long function (reason verified, phase 6): the three lines, the name hover note and the click area of one role use the same rectangle.
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
