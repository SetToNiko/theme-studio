//! The window layout: pane sizes, draggable borders, and what is shown.
//!
//! [`Layout`] knows nothing about the rest of the studio (pages, palette,
//! fonts). It gets the focused pane's number as a parameter.
//! [`App`](crate::app::App) calls it and then does the parts that need the rest of
//! the studio, such as moving the keyboard to another pane.

use ratatui::layout::Rect;

use crate::config::settings::{Settings, Sizes};
use crate::hit::{Border, BorderId};

/// Pane sizes, resize and drag state, visibility flags, and where the keyboard is on the page tabs.
#[expect(
    clippy::struct_excessive_bools,
    reason = "show/hide flags toggled one by one by keys and settings, plus `wide` (set by each draw), `resize_mode`, and two keyboard-focus flags; they do not form a state machine"
)]
pub struct Layout {
    /// Pane sizes (shares), from settings.json; resizing changes them.
    pub sizes: Sizes,
    /// The draggable borders from the last draw.
    pub borders: Vec<Border>,
    /// The border being dragged with the mouse.
    pub dragging: Option<BorderId>,
    /// Resize mode (`<leader>r`): keys move the focused pane's edges.
    pub resize_mode: bool,
    /// Three columns (false: the preview is a row underneath; no font sample).
    pub wide: bool,
    /// The colour picker (pane 5) is on screen.
    pub picker_visible: bool,
    /// The font sample (pane 3) is on screen.
    pub sample_visible: bool,
    /// The key hints in the status line, and the previews (Main: nvim and
    /// the shell; Fonts page: the preview pane), for this session.
    pub hints_visible: bool,
    pub main_preview_visible: bool,
    pub fonts_preview_visible: bool,
    /// The keyboard is on the Palette pane's dropdown row, or on the page
    /// tabs (F1 / F2 / F3).
    pub palette_menu_focused: bool,
    pub page_tabs_focused: bool,
    /// Where the Palette pane was last drawn (the palette list hangs from it).
    pub palette_pane: Option<Rect>,
}

impl Layout {
    /// Starts with the sizes and the shown or hidden parts from the settings.
    pub fn new(settings: &Settings) -> Layout {
        Layout {
            sizes: settings.layout.sizes.clone(),
            borders: Vec::new(),
            dragging: None,
            resize_mode: false,
            wide: true,
            picker_visible: settings.colours.picker_shown(),
            sample_visible: settings.fonts.sample_at_start != "hidden",
            hints_visible: settings.layout.key_hints != "hidden",
            main_preview_visible: settings.layout.main_preview != "hidden",
            fonts_preview_visible: settings.layout.fonts_preview != "hidden",
            palette_menu_focused: false,
            page_tabs_focused: false,
            palette_pane: None,
        }
    }

    /// The panes of the fonts column and of the colours column that are on
    /// screen, top to bottom (hidden panes are left out).
    pub fn pane_columns(&self) -> [Vec<u8>; 2] {
        [
            [1, 2, 3]
                .into_iter()
                .filter(|&p| p != 3 || (self.sample_visible && self.wide))
                .collect(),
            [4, 5]
                .into_iter()
                .filter(|&p| p != 5 || self.picker_visible)
                .collect(),
        ]
    }

    /// Sets the size share a border controls.
    pub fn set_share(&mut self, id: BorderId, share: f64) {
        use BorderId as B;
        let s = &mut self.sizes;
        let field = match id {
            B::FontsColours => &mut s.fonts_width,
            B::ColoursPreview => &mut s.colours_width,
            B::ListSample => &mut s.sample_height,
            B::ListPicker => &mut s.picker_height,
            B::PreviewSplit => &mut s.preview_split,
            B::TopPreview => &mut s.narrow_preview_height,
            B::NarrowFontsColours => &mut s.narrow_fonts_width,
        };
        // keep a little away from the ends; the layout's minimums do the rest
        *field = share.clamp(0.02, 0.98);
    }

    /// Moves a border to `pos` (a column for vertical borders, a row otherwise).
    pub fn move_border(&mut self, id: BorderId, pos: u16) {
        if let Some(b) = self.borders.iter_mut().find(|b| b.id == id) {
            let share = b.share_at(pos);
            // remember where it went, so quick repeats add up before the next draw
            if b.vertical {
                b.hit.x = pos;
            } else {
                b.hit.y = pos;
            }
            self.set_share(id, share);
        }
    }

    /// The border resize mode moves for the pane `pane` (the one with the
    /// keyboard): its side edge (`vertical`) or its lower edge.
    pub fn border_for_pane(&self, pane: u8, vertical: bool) -> Option<BorderId> {
        use BorderId as B;
        let id = match (self.wide, vertical, pane) {
            (true, true, 1..=3) => B::FontsColours,
            (true, true, _) => B::ColoursPreview,
            (true, false, 1..=3) => B::ListSample,
            (true, false, 4 | 5) => B::ListPicker,
            (true, false, _) | (false, true, 6 | 7) => B::PreviewSplit,
            (false, true, _) => B::NarrowFontsColours,
            (false, false, 4 | 5) if self.picker_visible => B::ListPicker,
            (false, false, _) => B::TopPreview,
        };
        self.borders.iter().any(|b| b.id == id).then_some(id)
    }
}
