//! Pane chrome shared by the pages: the border colour and the clickable title.
//!
//! Split from `widgets.rs` because these two helpers need `app` state
//! (keyboard focus, mouse hover, hit areas). `widgets.rs` must stay free of
//! `app`: `desktop_preview.rs` uses it, and `app` uses `ui`, so a `widgets`
//! to `app` edge would be a cycle (scripts/dev/module-cycles.py). Like
//! `widgets.rs`, this file must NOT use `ui/mod.rs` or any page file.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;

use crate::app::App;
use crate::app::state::hit::{Target, hover};

/// A pane's border: yellow when it has the keyboard, the hover colour when
/// the mouse is over it, else grey.
pub fn pane_border(app: &App, area: Rect, focused: bool) -> Style {
    if focused {
        Style::new().yellow()
    } else if app.pane_hovered(area) {
        Style::new().fg(crate::app::state::hit::looks().pane)
    } else {
        Style::new().dark_gray()
    }
}

/// A pane title that focuses the pane when clicked (hover-highlighted).
pub fn pane_title(app: &mut App, area: Rect, title: Line<'static>, pane: u8) -> Line<'static> {
    let r = Rect {
        x: area.x + 1,
        y: area.y,
        width: (title.width() as u16).min(area.width.saturating_sub(2)),
        height: 1,
    };
    if app.hits.add(r, Target::PaneTitle(pane)) {
        title.patch_style(hover(Style::new()))
    } else {
        title
    }
}
