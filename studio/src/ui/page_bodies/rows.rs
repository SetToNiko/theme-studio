//! Shared helpers for list pages: scrolling, one-row rectangles and the
//! selectable row that every list on the pages draws the same way.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::app::app_state::App;
use crate::app::state::hit::{Target, cursor_line, hover_line, hover_row};
use crate::ui::widgets::to_u16;

/// Keeps `sel` on screen in a list `height` rows tall.
pub(super) fn follow(scroll: &mut usize, sel: usize, height: usize) {
    if height == 0 {
        return;
    }
    if sel < *scroll {
        *scroll = sel;
    } else if sel >= *scroll + height {
        *scroll = sel + 1 - height;
    }
}

/// The one-row rectangle at row `i` of `area`.
pub(super) fn row_rect(area: Rect, i: usize) -> Rect {
    Rect {
        y: area.y + to_u16(i),
        height: 1,
        ..area
    }
}

/// The style of a row: reversed when selected and focused, bold and
/// underlined when selected but not focused.
pub(super) fn selected_style(on: bool, focused: bool) -> Style {
    match (on, focused) {
        (true, true) => Style::new().reversed(),
        (true, false) => Style::new().bold().underlined(),
        _ => Style::new(),
    }
}

/// How one selectable row looks and what a click on it does.
#[derive(Clone, Copy)]
pub(super) struct RowLook {
    /// This row is the selected row of its list.
    pub selected: bool,
    /// The list has the keyboard (the cursor is then drawn reversed).
    pub focused: bool,
    /// The click target of the row; `None` for a row that is not clickable.
    pub target: Option<Target>,
    /// Characters at the start of the row that the hover underline skips.
    pub underline_skip: usize,
    /// The hover look also shows on the selected row.
    pub hover_when_selected: bool,
}

/// Draws one selectable row of a list: registers its click area, adds the
/// hover look, and draws the cursor on the selected row.
pub(super) fn draw_selectable_row(
    f: &mut Frame,
    app: &mut App,
    rect: Rect,
    line: Line<'static>,
    look: RowLook,
) {
    let mut style = selected_style(look.selected, look.focused);
    let mut line = line;
    let hovered = look.target.is_some_and(|t| app.hits.add(rect, t));
    if hovered && (!look.selected || look.hover_when_selected) {
        // only the name is underlined
        style = hover_row(style);
        line = hover_line(line, look.underline_skip);
    }
    // the cursor covers the text, not padding or swatches
    let (line, style) = if look.selected {
        (cursor_line(line, style), Style::new())
    } else {
        (line, style)
    };
    f.render_widget(Paragraph::new(line).style(style), rect);
}
