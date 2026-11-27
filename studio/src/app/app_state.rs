//! The `App` struct: the whole state of the studio.
//!
//! This file holds only the data. The methods of `App` are in the other files of this folder
//! (`startup.rs`, `methods/queries.rs`, `methods/editing.rs`, and so on). Each of these files has its own
//! `impl App` block.

use std::time::Instant;

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use super::state::layout::Layout;
use super::state::messages::Messages;
use super::state::nav::{Focus, For, Overlay, Page, Side};
use super::state::palette_editor::PaletteEditor;
use crate::fonts::tab::state::FontsTab;
use crate::keymap::key::Key;
use crate::keymap::map::Keymap;
use crate::model::palette::Palette;
use crate::model::store::{Store, TerminalMap};
use crate::model::theme::ThemeStore;
use crate::preview_term::preview::Preview;

/// The whole state of the studio: the palette being edited, the page and
/// focus, the open pop-up, the settings and the keymap. Key and mouse input
/// change it, and `ui::draw` reads it to draw a frame.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent yes/no state (new palette, new theme, hand pointer, quit); an enum would not make it clearer"
)]
pub struct App {
    pub store: Store,
    pub name: String,
    /// Not in palettes.json yet.
    pub is_new: bool,
    /// The palette being edited: colours, undo history, picker numbers.
    pub editor: PaletteEditor,
    /// Pane sizes, borders, resize and drag state, and what is shown.
    pub layout: Layout,
    /// Keys in display order (GROUPS flattened).
    pub keys: Vec<&'static str>,
    pub sel: usize,
    pub list_scroll: usize,
    pub focus: Focus,
    pub active: Option<String>,
    /// The active system palette (hover colours are its keys).
    pub system_palette: Option<Palette>,
    pub terminal: Option<TerminalMap>,
    /// The status-line message, the save confirmation box, and the hover note.
    pub messages: Messages,
    pub overlay: Overlay,
    /// What's clickable where (from the last draw), and where the mouse is.
    pub hits: crate::app::state::hit::HitMap,
    /// The pointer is currently the hand (OSC 22).
    pub(super) pointer_hand: bool,
    pub preview: Preview,
    pub side: Side,
    pub page: Page,
    /// Cursor in the colour list (group and colour rows); `sel` is the colour.
    pub row_sel: usize,
    /// Open colour groups (indexes into GROUPS).
    pub expanded_groups: std::collections::HashSet<usize>,
    pub fonts: FontsTab,
    /// Draws images in the terminal (kitty graphics); None in tests.
    pub picker: Option<Picker>,
    pub sample_image: Option<StatefulProtocol>,
    /// The sample's size while resizing (re-encoded once it holds still).
    pub sample_settle: crate::model::settle::Settle,
    /// Previews fetched ahead of time (settings: fonts-page.prefetch).
    pub prefetch: crate::system::prefetch::Prefetch,
    /// When an image's size settles and the screen should be drawn again.
    pub redraw_at: Option<Instant>,
    pub sample_error: Option<String>,
    pub keymap: Keymap,
    /// Where keymaps.json and settings.json are (for saving).
    pub config_dir: std::path::PathBuf,
    /// Every keymap preset (listed on the Settings page).
    pub presets: Vec<Keymap>,
    pub settings: crate::config::settings::Settings,
    /// The Download fonts page.
    pub dl: crate::fonts::pages::download::DlPage,
    /// The Fonts page's Tournament tab (made on first use).
    pub tour_page: Option<crate::fonts::pages::tournament::TourPage>,
    /// The Settings page's tabs and selections.
    pub settings_page: crate::settings_page::state::SettingsPage,
    /// Keys of a sequence typed so far (e.g. the leader), waiting for the rest.
    pub pending_keys: Vec<Key>,
    /// What the Open / Save as pop-up is for.
    pub overlay_for: For,
    /// The palette list (`For::Palette`): each entry's palette, for previewing
    /// it while it's highlighted, and the palette from before the list opened.
    pub open_palettes: Vec<Option<Palette>>,
    pub open_original: Option<Palette>,
    /// The theme: a name for the palette plus the font preset.
    pub themes: ThemeStore,
    pub theme: String,
    /// Not in themes.json yet.
    pub theme_new: bool,
    /// The theme's parts as saved (None: never saved).
    pub theme_saved: Option<crate::model::theme::Theme>,
    pub(super) quit: bool,
}
