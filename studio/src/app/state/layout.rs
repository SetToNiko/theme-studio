//! The window layout: pane sizes, draggable borders, and what is shown.
//!
//! [`Layout`](crate::app::state::layout::Layout) knows nothing about the rest of the studio (pages, palette,
//! fonts). It gets the focused pane's number as a parameter.
//! [`App`](crate::app::app_state::App) calls it and then does the parts that need the rest of
//! the studio, such as moving the keyboard to another pane.

use ratatui::layout::Rect;

use crate::app::state::hit::{Border, BorderId};
use crate::config::settings::{Settings, Sizes};

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

#[cfg(test)]
mod tests {
    use crate::app::app_state::App;
    use crate::app::startup::Start;
    use crate::app::test_support::*;
    use crate::keymap::map::Keymap;
    use crate::model::store::Store;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};

    #[test]
    fn borders_drag_and_resize_mode_moves_edges() {
        use crate::app::state::hit::{BorderId as B, Target as T};
        let mut app = test_app(temp_store("resize"), Start::Named("One".into()));
        render(&mut app, 200, 50);
        // drag the border between fonts and colours 10 columns right
        let b = app.hits.rect_of(T::Border(B::FontsColours)).unwrap();
        mouse(
            &mut app,
            MouseEventKind::Down(MouseButton::Left),
            b.x,
            b.y + 5,
        );
        assert_eq!(app.layout.dragging, Some(B::FontsColours));
        mouse(
            &mut app,
            MouseEventKind::Drag(MouseButton::Left),
            b.x + 10,
            b.y + 5,
        );
        mouse(
            &mut app,
            MouseEventKind::Up(MouseButton::Left),
            b.x + 10,
            b.y + 5,
        );
        assert_eq!(app.layout.dragging, None);
        render(&mut app, 200, 50);
        assert_eq!(
            app.hits.rect_of(T::Border(B::FontsColours)).unwrap().x,
            b.x + 10
        );
        // resize mode on the colour list: l moves its right edge, = resets, Esc leaves
        press(&mut app, " 4 r");
        assert!(app.layout.resize_mode);
        let before = app.hits.rect_of(T::Border(B::ColoursPreview)).unwrap().x;
        press(&mut app, "lll");
        render(&mut app, 200, 50);
        assert_eq!(
            app.hits.rect_of(T::Border(B::ColoursPreview)).unwrap().x,
            before + 3
        );
        press(&mut app, "H");
        render(&mut app, 200, 50);
        assert_eq!(
            app.hits.rect_of(T::Border(B::ColoursPreview)).unwrap().x,
            before - 2
        );
        // nvim / shell split from the preview panes isn't reachable from the list; j
        // here moves the list/picker border, which only exists with the picker shown
        press(&mut app, "j");
        press(&mut app, "=");
        render(&mut app, 200, 50);
        assert_eq!(app.layout.sizes, crate::config::settings::Sizes::default());
        press(&mut app, "\x1b");
        assert!(!app.layout.resize_mode);
        // keys work normally again
        press(&mut app, "j");
        assert_eq!(app.row_sel, 2);
    }

    #[test]
    fn extreme_sizes_render_at_every_window_size() {
        let mut app = test_app(temp_store("extreme"), Start::Named("One".into()));
        app.layout.picker_visible = true;
        for share in [0.0, 0.02, 0.5, 0.98, 1.0] {
            app.layout.sizes = crate::config::settings::Sizes {
                fonts_width: share,
                colours_width: share,
                sample_height: share,
                picker_height: share,
                preview_split: share,
                narrow_preview_height: share,
                narrow_fonts_width: share,
            };
            for w in (90..=260).step_by(17) {
                for h in (20..=70).step_by(9) {
                    render(&mut app, w, h);
                }
            }
        }
    }

    #[test]
    fn hints_and_previews_toggle_per_page() {
        let store = temp_store("toggles");
        let dir = store.dir.clone();
        let mut app = test_app(store, Start::Named("One".into()));
        let screen = |app: &mut App| render(app, 160, 45);
        // Main: <leader>tp hides the preview; the columns take its room
        assert!(screen(&mut app).contains("Preview"));
        press(&mut app, " tp");
        assert!(!app.layout.main_preview_visible);
        let s = screen(&mut app);
        assert!(!s.contains("Space 6 / Space 7"), "{s}");
        let pane = app.layout.palette_pane.unwrap();
        assert_eq!(pane.x + pane.width, 160, "colours reach the right edge");
        // jumping to nvim shows it again
        press(&mut app, " 6");
        assert!(app.layout.main_preview_visible);
        // (nvim has the keyboard now: take it back)
        app.preview.focus = None;
        press(&mut app, " tp");
        // the Fonts page's preview is its own
        app.on_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert!(app.layout.fonts_preview_visible && !app.layout.main_preview_visible);
        press(&mut app, " tp");
        assert!(!app.layout.fonts_preview_visible && !app.layout.main_preview_visible);
        // <leader>th hides the key hints anywhere; messages still show
        assert!(screen(&mut app).contains("? keys"));
        press(&mut app, " th");
        let s = screen(&mut app);
        assert!(!s.contains("? keys"), "{s}");
        app.messages.info("Hello");
        assert!(screen(&mut app).contains("Hello"));
        // the settings say how a run starts
        let mut config = crate::config::settings::Settings::default();
        config.layout.key_hints = "hidden".into();
        config.layout.main_preview = "hidden".into();
        let loaded = crate::config::load::Loaded {
            keymap: Keymap::vim(),
            presets: Keymap::builtin(),
            settings: config,
            warnings: Vec::new(),
            dir: dir.join("studio-config"),
        };
        let mut app2 =
            App::with_config(Store::at(dir.clone()), Start::Named("One".into()), loaded).unwrap();
        app2.preview.set_headless();
        assert!(
            !app2.layout.hints_visible
                && !app2.layout.main_preview_visible
                && app2.layout.fonts_preview_visible
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
