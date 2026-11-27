//! Questions about the state of `App`: the selected key, unsaved edits, and the hovered pane.

use ratatui::layout::Rect;

use crate::app::app_state::App;
use crate::app::state::nav::Overlay;

impl App {
    pub fn key(&self) -> &'static str {
        self.keys[self.sel]
    }

    /// Unsaved palette edits (not counting a palette previewed from the
    /// palette list).
    pub fn dirty(&self) -> bool {
        self.is_new
            || self.open_original.as_ref().unwrap_or(&self.editor.palette) != &self.editor.saved
    }

    /// The mouse is over this pane (and no pop-up or drag is in the way).
    pub fn pane_hovered(&self, area: Rect) -> bool {
        self.hits.over(area)
            && self.layout.dragging.is_none()
            && matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. })
    }
}
