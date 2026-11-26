//! Editor state and input handling: the `App` struct and its methods.
//!
//! `App` is one struct. Its methods are split by topic over several files
//! (Rust allows more than one `impl App` block, in any file of the crate).
//! Each file's own `//!` says what stays outside it. A key press starts at
//! `input/actions.rs` (`run_action`). Where to look:
//! - `state/`: the data behind `App`: `nav.rs` (where the user is: `Page`, `Focus`, `Overlay`, ...), `palette_editor.rs`, `layout.rs`, `messages.rs` (own structs, not `impl App`)
//! - `input/`: `actions.rs` (`run_action`), `focus.rs`, `mouse.rs`, `overlay.rs` (pop-ups), `colours.rs`
//! - `pages/settings/`: `settings_actions.rs` (the page as a whole), `settings_keymaps_actions.rs`, `settings_history_actions.rs`, `draft_actions.rs` (Settings drafts), `behaviour_actions.rs`, `behaviour_colours_actions.rs`, `behaviour_labels_actions.rs` (Behaviour tab)
//! - `pages/fonts/`: `fonts_dl_actions.rs` (Download fonts), `fonts_preview_actions.rs`, `prefetch_actions.rs`, `tour_actions.rs` and `tour_rounds.rs` (Tournament)
//! - `pages/themes/`: `theme_actions.rs`, `presets_actions.rs`
//! - this folder: this file (`App` and its core methods), `run.rs` (the main loop), `persist.rs` (save, open, unsaved-changes prompt), `tests/` (tests of the above, one file per area)
//!
//! The run loop (`App::run`) does not draw by itself: `main.rs` passes in `ui::draw`.
//!
//! All `impl App` blocks are in this folder: no other module of the crate adds
//! methods to `App`.

mod input;
mod pages;
mod persist;
mod run;
pub mod state;

use std::io::Write;
use std::time::Instant;

use anyhow::Result;
/// The terminal the studio draws on.
pub type Term = ratatui::Terminal<ratatui::backend::CrosstermBackend<crate::system::out::Out>>;
use ratatui::layout::Rect;

use crate::fonts::FontsTab;
use crate::keymap::Keymap;
use crate::keymap::key::Key;
use crate::model::color::Rgb;
use crate::model::palette::{self, GROUPS, Palette};
use crate::model::store::{Store, TerminalMap};
use crate::model::theme::ThemeStore;
use crate::preview_term::{Mode as PreviewMode, Preview};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use state::layout::Layout;
use state::messages::Messages;
use state::nav::{Focus, For, Overlay, Page, Side};
use state::palette_editor::PaletteEditor;

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
    pointer_hand: bool,
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
    pub dl: crate::fonts::download_page::DlPage,
    /// The Fonts page's Tournament tab (made on first use).
    pub tour_page: Option<crate::fonts::tournament_page::TourPage>,
    /// The Settings page's tabs and selections.
    pub settings_page: crate::settings_page::SettingsPage,
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
    quit: bool,
}

/// What to open at start (from the command line): the active theme, a theme
/// or palette by name, or a new one.
pub enum Start {
    Active,
    Named(String),
    New,
}

impl App {
    /// Starts with the settings in ~/.config/theme-studio (creating or
    /// topping up the files).
    ///
    /// # Errors
    /// Fails when a palette named on the command line cannot be loaded and no theme has that name
    /// (see [`App::with_config`]).
    pub fn new(store: Store, start: Start) -> Result<App> {
        App::with_config(
            store,
            start,
            crate::config::load(&crate::model::paths::studio_config_dir(), true),
        )
    }

    /// Starts with a given config: loads the first palette, builds the state,
    /// then restores the session (migrations, theme, warnings).
    ///
    /// # Errors
    /// Fails when a palette named on the command line cannot be loaded and no
    /// theme has that name either.
    pub fn with_config(store: Store, start: Start, config: crate::config::Loaded) -> Result<App> {
        let active = Store::active_name();
        let system_palette = active.as_ref().and_then(|n| store.load(n).ok());
        // a name on the command line: a theme, or a palette
        let named = match &start {
            Start::Named(n) => Some(n.clone()),
            _ => None,
        };
        let start_new = matches!(start, Start::New);
        let first = first_palette(&store, start, active.as_deref())?;
        let warnings = config.warnings.clone();
        let mut app = App::build(store, first, active, system_palette, config);
        app.restore_session(start_new, named.as_deref(), &warnings);
        Ok(app)
    }

    /// Builds the state: each field gets its start value. Only the fonts tab
    /// reads the disk.
    // Long function (reason verified, phase 6): one struct literal that gives every `App` field its start value.
    fn build(
        store: Store,
        (name, palette, is_new): (String, Palette, bool),
        active: Option<String>,
        system_palette: Option<Palette>,
        config: crate::config::Loaded,
    ) -> App {
        let terminal = store.terminal_map().ok();
        let preview = Preview::new(&store.desktop);
        let fonts = FontsTab::new(
            &store.dir,
            &store.share,
            preview.draft_dir(),
            preview.dirty_flag(),
        );
        let mut app = App {
            preview,
            side: Side::Colours,
            page: Page::Main,
            row_sel: 0,
            expanded_groups: GROUPS
                .iter()
                .enumerate()
                .filter(|(_, (g, _))| {
                    config
                        .settings
                        .colours
                        .open_groups
                        .iter()
                        .any(|o| o.eq_ignore_ascii_case(g))
                })
                .map(|(i, _)| i)
                .collect(),
            fonts,
            picker: None,
            sample_image: None,
            sample_settle: crate::model::settle::Settle::default(),
            prefetch: crate::system::prefetch::Prefetch::default(),
            redraw_at: None,
            sample_error: None,
            overlay_for: For::Palette,
            open_palettes: Vec::new(),
            open_original: None,
            themes: ThemeStore {
                dir: store.dir.clone(),
            },
            theme: String::new(),
            theme_new: true,
            theme_saved: None,
            keymap: config.keymap,
            presets: config.presets,
            config_dir: config.dir.clone(),
            settings: config.settings.clone(),
            settings_page: crate::settings_page::SettingsPage::default(),
            dl: crate::fonts::download_page::DlPage::default(),
            tour_page: None,
            pending_keys: Vec::new(),
            store,
            name,
            is_new,
            editor: PaletteEditor::new(palette),
            layout: Layout::new(&config.settings),
            keys: GROUPS
                .iter()
                .flat_map(|(_, ks)| ks.iter().copied())
                .collect(),
            sel: 0,
            list_scroll: 0,
            focus: Focus::List,
            active,
            system_palette,
            terminal,
            messages: Messages::new(),
            overlay: Overlay::None,
            hits: crate::app::state::hit::HitMap::default(),
            pointer_hand: false,
            quit: false,
        };
        app.load_picker();
        app.fonts.filters = app.settings.font_list.filters();
        app.fonts.unfold_current = app.settings.fonts.list_opens_to_current_font;
        app.fonts.refresh_installed();
        app
    }

    /// Restores the session: file migrations, the theme, the selected row,
    /// and the warnings that the config loading found.
    fn restore_session(&mut self, start_new: bool, named: Option<&str>, warnings: &[String]) {
        self.migrate_files();
        if start_new {
            self.new_theme();
            self.messages.message = None;
        } else {
            self.init_theme(named);
        }
        self.sync_row();
        if !warnings.is_empty() {
            self.messages
                .error(format!("Settings: {}", warnings.join("; ")));
        }
    }

    /// palettes.json / fonts.json: version 1 files become version 2 (once;
    /// the old file goes to the history).
    fn migrate_files(&mut self) {
        for (what, result) in [
            ("palettes.json", self.store.migrate()),
            // before fonts.json: it takes the sizes from the presets
            ("themes.json", self.themes.migrate(&self.fonts.store)),
            ("fonts.json", self.fonts.store.migrate()),
        ] {
            match result {
                Ok(Some(v)) => self.messages.info(format!(
                    "{what} updated to file format version {v} (the old file is in the history)"
                )),
                Ok(None) => {}
                Err(e) => self.messages.error(format!("{what}: {e:#}")),
            }
        }
    }

    /// The mouse pointer is a hand over anything clickable (OSC 22; Ghostty
    /// supports it, other terminals ignore it).
    fn update_pointer(&mut self) {
        let hand = self.hits.hovered().is_some();
        if hand != self.pointer_hand {
            self.pointer_hand = hand;
            set_pointer(if hand { "pointer" } else { "default" });
        }
    }

    pub fn key(&self) -> &'static str {
        self.keys[self.sel]
    }

    /// Unsaved palette edits (not counting a palette previewed from the
    /// palette list).
    pub fn dirty(&self) -> bool {
        self.is_new
            || self.open_original.as_ref().unwrap_or(&self.editor.palette) != &self.editor.saved
    }

    /// Asks for a new font sample when the fonts or colours changed, and
    /// shows a finished one.
    fn update_sample(&mut self) {
        let fonts = self.fonts.shown_fonts();
        let sizes = [0, 1, 2].map(|r| self.fonts.effective_size(r, fonts[r].size));
        let markup =
            crate::fonts::render::sample_markup(&fonts, sizes, &self.editor.palette, &self.name);
        let dpi = self.fonts.dpi;
        self.fonts
            .sample
            .request(markup, self.editor.palette.get("bg0"), dpi);
        match self.fonts.sample.take() {
            Some(Ok(img)) => {
                self.sample_image = self.picker.as_ref().map(|p| p.new_resize_protocol(img));
                self.sample_error = None;
            }
            Some(Err(e)) => self.sample_error = Some(e),
            None => {}
        }
    }

    // ----- editing ---------------------------------------------------------

    /// Points the picker at the selected key's colour.
    fn load_picker(&mut self) {
        self.editor.load_picker(self.key());
    }

    /// Sets the selected key from the picker. Consecutive changes to one key
    /// are a single undo step.
    fn edit(&mut self, c: Rgb) {
        let in_picker = self.focus != Focus::List;
        self.editor.edit(self.key(), c, in_picker);
    }

    /// Sets `key` to `c` as its own undo step (paste, copy from another key).
    fn set_key(&mut self, key: &'static str, c: Rgb) {
        self.editor.set_key(key, c);
        if key == self.key() {
            self.load_picker();
        }
    }

    fn set_hsv(&mut self, h: f64, s: f64, v: f64) {
        let in_picker = self.focus != Focus::List;
        self.editor.set_hsv(self.key(), h, s, v, in_picker);
    }

    fn set_hsl(&mut self, s: f64, l: f64) {
        let in_picker = self.focus != Focus::List;
        self.editor.set_hsl(self.key(), s, l, in_picker);
    }

    fn toggle_pin(&mut self) {
        let key = self.key();
        if palette::is_base(key) {
            self.messages
                .info(format!("{key} is a base colour; it's always set by hand"));
            return;
        }
        let message = self.editor.toggle_pin(key, palette::sources(key));
        self.messages.info(message);
    }

    fn undo(&mut self) {
        if !self.editor.undo(self.key()) {
            self.messages.info("Nothing to undo");
        }
    }

    fn redo(&mut self) {
        if !self.editor.redo(self.key()) {
            self.messages.info("Nothing to redo");
        }
    }

    /// Copies text to the system clipboard with OSC 52 (Ghostty supports it).
    fn copy(&mut self, text: &str) {
        let seq = format!(
            "\x1b]52;c;{}\x07",
            crate::model::encoding::base64(text.as_bytes())
        );
        let mut out = std::io::stdout();
        let _ = out.write_all(seq.as_bytes()).and_then(|()| out.flush());
        self.messages.info(format!("Copied {text}"));
    }

    // ----- files -----------------------------------------------------------

    // ----- input -----------------------------------------------------------

    /// The mouse is over this pane (and no pop-up or drag is in the way).
    pub fn pane_hovered(&self, area: Rect) -> bool {
        self.hits.over(area)
            && self.layout.dragging.is_none()
            && matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. })
    }
}

/// Picks the palette to start with: the name, the active palette, or a blank
/// one. Returns its name, the palette, and whether it is new (not saved yet).
///
/// # Errors
/// Fails when `start` names a palette that cannot be loaded and no theme has that name.
fn first_palette(
    store: &Store,
    start: Start,
    active: Option<&str>,
) -> Result<(String, Palette, bool)> {
    // The active palette, or a blank one when there is none or it fails to load.
    let active_or_blank = || match active.map(|n| (n, store.load(n))) {
        Some((n, Ok(p))) => (n.to_string(), p, false),
        _ => ("untitled".to_string(), Palette::blank(), true),
    };
    Ok(match start {
        Start::Named(n) => match store.load(&n) {
            Ok(p) => (n, p, false),
            // A name that is a theme (not a palette) is fine: the theme sets the palette.
            Err(e) => {
                let themes = ThemeStore {
                    dir: store.dir.clone(),
                };
                if !themes.load_all().iter().any(|(t, _)| *t == n) {
                    return Err(e);
                }
                active_or_blank()
            }
        },
        Start::Active => active_or_blank(),
        Start::New => ("untitled".to_string(), Palette::blank(), true),
    })
}

/// Sets the mouse pointer shape (OSC 22): "pointer" (hand) or "default".
pub fn set_pointer(shape: &str) {
    let mut out = std::io::stdout();
    let _ = write!(out, "\x1b]22;{shape}\x1b\\").and_then(|()| out.flush());
}

/// Indices of entries whose name contains `filter` (case-insensitive).
pub fn open_matches(entries: &[(String, Vec<Rgb>)], filter: &str) -> Vec<usize> {
    let f = filter.to_lowercase();
    entries
        .iter()
        .enumerate()
        .filter(|(_, (n, _))| n.to_lowercase().contains(&f))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests;
