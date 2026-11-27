//! One frame of the window: the entry point of drawing.
//!
//! `draw` picks the arrangement for the window size, then draws the tabs, the
//! page, the status line and the overlays on top, in this order.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

use super::pages::{draw_body_page, draw_main_page, draw_too_small};
use super::tabs::draw_tabs;
use crate::app::App;
use crate::ui::layout::{Arrangement, arrange};
use crate::ui::overlays::{dialogs, notice, pending};
use crate::ui::status::draw::draw_status;

/// Draws the whole window: the tabs, the page for the window size, the status
/// line, then the overlays on top.
// Long function (reason verified, phase 6): the order of the drawing calls is the order on screen: tabs, page, status, then the overlays on top.
pub fn draw(f: &mut Frame, app: &mut App) {
    reset_frame_state(app);
    let full = f.area();
    // tabs on top: their own row; at the bottom they share the status line
    let top = app.settings.layout.tabs_on_top();
    let (tabs_row, area) = if top && full.height > 1 {
        let [tabs, rest] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(full);
        (Some(tabs), rest)
    } else {
        (None, full)
    };
    let arrangement = arrange(
        area,
        app.page,
        app.layout.picker_visible,
        app.layout.sample_visible,
        app.layout.main_preview_visible,
        &app.layout.sizes,
    );
    if let Arrangement::TooSmall { need_w, need_h } = arrangement {
        return draw_too_small(f, full, need_w, need_h + u16::from(top));
    }
    if let Some(tabs) = tabs_row {
        draw_tabs(f, app, tabs);
    }
    match arrangement {
        Arrangement::TooSmall { .. } => {}
        Arrangement::Page { body, status } => {
            draw_body_page(f, app, body);
            draw_status(f, app, status);
        }
        Arrangement::Main(l) => {
            draw_main_page(f, app, &l);
            draw_status(f, app, l.status);
        }
    }
    dialogs::draw_overlay(f, app, full);
    pending::draw_pending(f, app, full);
    notice::draw_notice(f, app, full);
}

/// Clears what the last frame left in `app`: each draw call fills it again.
fn reset_frame_state(app: &mut App) {
    app.hits.clear();
    // images still waiting for a size to settle set this again
    app.redraw_at = None;
    // set again if the Palette pane is drawn (the palette list hangs from it)
    app.layout.palette_pane = None;
    app.messages.hover_note = None;
    crate::app::state::hit::set_looks(app.looks(&app.settings));
}
