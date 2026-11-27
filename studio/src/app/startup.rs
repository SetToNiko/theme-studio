//! Starting `App`: load the config, pick the first palette, build the state, restore the session.
//!
//! [`App::new`](crate::app::app_state::App::new) and [`App::with_config`](crate::app::app_state::App::with_config) are the two ways to make an `App`.

use anyhow::Result;

use super::app_state::App;
use super::state::layout::Layout;
use super::state::messages::Messages;
use super::state::nav::{Focus, For, Overlay, Page, Side};
use super::state::palette_editor::PaletteEditor;
use crate::fonts::tab::state::FontsTab;
use crate::model::palette::{GROUPS, Palette};
use crate::model::store::Store;
use crate::model::theme::ThemeStore;
use crate::preview_term::preview::Preview;

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
    /// (see [`App::with_config`](crate::app::app_state::App::with_config)).
    pub fn new(store: Store, start: Start) -> Result<App> {
        App::with_config(
            store,
            start,
            crate::config::load::load(&crate::model::paths::studio_config_dir(), true),
        )
    }

    /// Starts with a given config: loads the first palette, builds the state,
    /// then restores the session (migrations, theme, warnings).
    ///
    /// # Errors
    /// Fails when a palette named on the command line cannot be loaded and no
    /// theme has that name either.
    pub fn with_config(
        store: Store,
        start: Start,
        config: crate::config::load::Loaded,
    ) -> Result<App> {
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
        config: crate::config::load::Loaded,
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
            settings_page: crate::settings_page::state::SettingsPage::default(),
            dl: crate::fonts::pages::download::DlPage::default(),
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
