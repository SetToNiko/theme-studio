//! Editor state and input handling.

mod actions;
mod layout;
mod messages;
mod mouse;
mod overlay;
mod palette_editor;

use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::Result;
/// The terminal the studio draws on.
pub type Term = ratatui::Terminal<ratatui::backend::CrosstermBackend<crate::out::Out>>;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::Rect;

use crate::color::Rgb;
use crate::fonts::FontsTab;
use crate::keymap::{Action, Context, Key, Keymap};
use crate::palette::{self, GROUPS, Palette};
use crate::preview::{Mode as PreviewMode, PaneId, Preview};
use crate::store::{Store, TerminalMap};
use crate::theme::ThemeStore;
use layout::Layout;
use messages::Messages;
use palette_editor::PaletteEditor;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

/// Which control the keyboard drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    List,
    Square,
    Hue,
    SliderH,
    SliderS,
    SliderL,
    Hex,
}

const FOCUS_ORDER: [Focus; 7] = [
    Focus::List,
    Focus::Square,
    Focus::Hue,
    Focus::SliderH,
    Focus::SliderS,
    Focus::SliderL,
    Focus::Hex,
];

/// Something that would throw away unsaved changes, waiting for confirmation.
#[derive(Clone, Debug)]
pub enum Pending {
    Quit,
    New,
    Open(String),
    NewFonts,
    OpenFonts(String),
    NewTheme,
    OpenTheme(String),
}

/// What the Open / Save as pop-up is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum For {
    Palette,
    Fonts,
    Theme,
}

impl Pending {
    fn is_fonts(&self) -> bool {
        matches!(self, Pending::NewFonts | Pending::OpenFonts(_))
    }
}

/// Which column has the keyboard (both are always on screen).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Colours,
    Fonts,
}

/// F1 / F2 / F3.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Main,
    Downloads,
    Settings,
}

/// A row of the colour list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColourRow {
    Group { index: usize, expanded: bool },
    Key(usize),
}

pub enum Overlay {
    None,
    Open {
        filter: String,
        sel: usize,
        entries: Vec<(String, Vec<Rgb>)>,
    },
    SaveAs {
        input: String,
        overwrite: bool,
        then_apply: bool,
    },
    Confirm(Pending),
    /// Picking the key whose colour gets copied into `target`.
    CopyFrom {
        target: &'static str,
    },
    /// The keys, in columns; `page`: the first column shown when they don't
    /// all fit.
    Help {
        page: usize,
    },
    /// Settings page: recording keys (a binding or the leader).
    Capture {
        what: crate::settings::Capture,
        keys: Vec<Key>,
    },
    /// Settings page: typing a name or keys.
    Prompt {
        title: String,
        input: String,
        purpose: crate::settings::Naming,
    },
    /// Settings page: choosing a palette key for a hover colour.
    ChooseKey {
        row: crate::behaviour::BhRow,
        sel: usize,
        /// The key before the list opened (Esc puts it back).
        original: String,
    },
    /// Settings page: a yes / no question.
    Ask {
        question: String,
        then: crate::settings::Deferred,
    },
    /// A dropdown list hanging under `at`: the preview's text kind (0) or
    /// language (1).
    Menu {
        menu: u8,
        sel: usize,
        at: Rect,
    },
}

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
    pub hits: crate::hit::HitMap,
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
    pub sample_settle: crate::settle::Settle,
    /// Previews fetched ahead of time (settings: fonts-page.prefetch).
    pub prefetch: crate::prefetch::Prefetch,
    /// When an image's size settles and the screen should be drawn again.
    pub redraw_at: Option<Instant>,
    pub sample_error: Option<String>,
    pub keymap: Keymap,
    /// Where keymaps.json and settings.json are (for saving).
    pub config_dir: std::path::PathBuf,
    /// Every keymap preset (listed on the Settings page).
    pub presets: Vec<Keymap>,
    pub settings: crate::config::Settings,
    /// The Download fonts page.
    pub dl: crate::fonts_dl::DlPage,
    /// The Fonts page's Tournament tab (made on first use).
    pub tour_page: Option<crate::tour_page::TourPage>,
    /// The Settings page's tabs and selections.
    pub settings_page: crate::settings::SettingsPage,
    /// Keys of a sequence typed so far (e.g. the leader), waiting for the rest.
    pub pending_keys: Vec<Key>,
    /// What the Open / Save as pop-up is for.
    pub overlay_for: For,
    /// The palette list (For::Palette): each entry's palette, for previewing
    /// it while it's highlighted, and the palette from before the list opened.
    pub open_palettes: Vec<Option<Palette>>,
    pub open_original: Option<Palette>,
    /// The theme: a name for the palette plus the font preset.
    pub themes: ThemeStore,
    pub theme: String,
    /// Not in themes.json yet.
    pub theme_new: bool,
    /// The theme's parts as saved (None: never saved).
    pub theme_saved: Option<crate::theme::Theme>,
    quit: bool,
}

pub enum Start {
    Active,
    Named(String),
    New,
}

impl App {
    /// Starts with the settings in ~/.config/theme-studio (creating or
    /// topping up the files).
    pub fn new(store: Store, start: Start) -> Result<App> {
        App::with_config(
            store,
            start,
            crate::config::load(&crate::config::dir(), true),
        )
    }

    pub fn with_config(store: Store, start: Start, config: crate::config::Loaded) -> Result<App> {
        let active = store.active_name();
        let system_palette = active.as_ref().and_then(|n| store.load(n).ok());
        // a name on the command line: a theme, or a palette
        let named = match &start {
            Start::Named(n) => Some(n.clone()),
            _ => None,
        };
        let start_new = matches!(start, Start::New);
        let (name, palette, is_new) = match start {
            Start::Named(n) => match store.load(&n) {
                Ok(p) => (n, p, false),
                Err(e)
                    if !ThemeStore {
                        dir: store.dir.clone(),
                    }
                    .load_all()
                    .iter()
                    .any(|(t, _)| *t == n) =>
                {
                    return Err(e);
                }
                Err(_) => match active.as_deref().map(|n| (n, store.load(n))) {
                    Some((n, Ok(p))) => (n.to_string(), p, false),
                    _ => ("untitled".to_string(), Palette::blank(), true),
                },
            },
            Start::Active => match active.as_deref().map(|n| (n, store.load(n))) {
                Some((n, Ok(p))) => (n.to_string(), p, false),
                _ => ("untitled".to_string(), Palette::blank(), true),
            },
            Start::New => ("untitled".to_string(), Palette::blank(), true),
        };
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
            sample_settle: Default::default(),
            prefetch: Default::default(),
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
            settings_page: Default::default(),
            dl: Default::default(),
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
            hits: Default::default(),
            pointer_hand: false,
            quit: false,
        };
        app.load_picker();
        app.fonts.filters = app.settings.font_list.filters();
        app.fonts.unfold_current = app.settings.fonts.list_opens_to_current_font;
        app.fonts.refresh_installed();
        // palettes.json / fonts.json: version 1 files become version 2 (once;
        // the old file goes to the history)
        for (what, result) in [
            ("palettes.json", app.store.migrate()),
            // before fonts.json: it takes the sizes from the presets
            ("themes.json", app.themes.migrate(&app.fonts.store)),
            ("fonts.json", app.fonts.store.migrate()),
        ] {
            match result {
                Ok(Some(v)) => app.messages.info(format!(
                    "{what} updated to file format version {v} (the old file is in the history)"
                )),
                Ok(None) => {}
                Err(e) => app.messages.error(format!("{what}: {e:#}")),
            }
        }
        if start_new {
            app.new_theme();
            app.messages.message = None;
        } else {
            app.init_theme(named.as_deref());
        }
        app.sync_row();
        if !config.warnings.is_empty() {
            app.messages
                .error(format!("Settings: {}", config.warnings.join("; ")));
        }
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut Term) -> Result<()> {
        let mut redraw = true;
        while !self.quit {
            // nvim in the preview re-reads this whenever it changes
            self.preview.sync_draft(&self.editor.palette);
            if self.page == Page::Main && self.layout.sample_visible {
                self.update_sample();
            }
            // downloads: the catalogue, progress, finished installs
            if self.poll_downloads() {
                redraw = true;
            }
            if self.poll_prefetch() {
                redraw = true;
            }
            if self.poll_tournament() {
                redraw = true;
            }
            if self.messages.expire(self.settings.messages.seconds) {
                redraw = true;
            }
            // a role's size, once h / l have been still for a moment: saved
            // to the theme and applied (one i3 restart, not one per step)
            if self
                .fonts
                .sized_at
                .is_some_and(|t| t.elapsed() >= Duration::from_millis(400))
            {
                self.apply_sizes();
                redraw = true;
            }
            // an image waiting for its pane's size to settle
            if self.redraw_at.is_some_and(|t| Instant::now() >= t) {
                redraw = true;
            }
            if self.preview.take_dirty() || redraw {
                terminal.draw(|f| crate::ui::draw(f, self))?;
                self.update_pointer();
                redraw = false;
            }
            // Short poll: the preview's programs can print at any time.
            if event::poll(Duration::from_millis(25))? {
                match event::read()? {
                    Event::Key(k) if k.kind == KeyEventKind::Press => self.on_key(k),
                    Event::Mouse(m) => self.on_mouse(m),
                    Event::Paste(s) => {
                        self.on_paste(&s);
                        self.sync_palette_preview();
                    }
                    _ => {}
                }
                redraw = true;
            }
        }
        Ok(())
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
        let markup = crate::fonts::sample_markup(&fonts, sizes, &self.editor.palette, &self.name);
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

    /// Starts with the font roles focused (`--fonts`).
    pub fn open_fonts_tab(&mut self) {
        self.focus_pane(1);
    }

    // ----- font presets ----------------------------------------------------

    fn fonts_save(&mut self, then_apply: bool) {
        if self.fonts.is_new {
            self.overlay_for = For::Fonts;
            self.overlay = Overlay::SaveAs {
                input: String::new(),
                overwrite: false,
                then_apply,
            };
            return;
        }
        let name = self.fonts.name.clone();
        self.fonts_save_as(&name, then_apply);
    }

    fn fonts_save_as(&mut self, name: &str, then_apply: bool) {
        match self.fonts.save_as(name) {
            Ok(()) if then_apply => match self.fonts.store.apply(name, self.fonts.preset.sizes()) {
                Ok(()) => {
                    self.fonts.active = Some(name.to_string());
                    self.notify_saved(format!("Saved and applied \"{name}\""));
                }
                Err(e) => self.messages.error(format!("Apply failed: {e:#}")),
            },
            Ok(()) => self.notify_saved(format!("Saved \"{name}\"")),
            Err(e) => self.messages.error(format!("Save failed: {e:#}")),
        }
    }

    fn fonts_open_overlay(&mut self) {
        match self.fonts.store.load_all() {
            Ok(all) => {
                let entries: Vec<(String, Vec<Rgb>)> =
                    all.keys().map(|n| (n.clone(), Vec::new())).collect();
                let sel = entries
                    .iter()
                    .position(|(n, _)| *n == self.fonts.name)
                    .unwrap_or(0);
                self.overlay_for = For::Fonts;
                self.overlay = Overlay::Open {
                    filter: String::new(),
                    sel,
                    entries,
                };
            }
            Err(e) => self.messages.error(format!("{e:#}")),
        }
    }

    /// A save confirmation: in the status line like any message, or in a
    /// box (settings.messages.saved).
    pub fn notify_saved(&mut self, s: impl Into<String>) {
        if self.settings.messages.saved_in_popup() {
            self.messages.popup(s);
        } else {
            self.messages.info(s);
        }
    }

    // ----- editing ---------------------------------------------------------

    /// Points the picker at the selected key's colour.
    fn load_picker(&mut self) {
        self.editor.load_picker(self.key());
    }

    /// Selects a colour without touching the list cursor.
    fn select_key(&mut self, i: usize) {
        if i < self.keys.len() && i != self.sel {
            self.sel = i;
            self.editor.end_edit_run();
            self.load_picker();
        }
    }

    /// Selects a colour, opening its group and moving the list cursor to it.
    fn select(&mut self, i: usize) {
        if i < self.keys.len() {
            self.select_key(i);
            let g = self.group_of(i);
            self.expanded_groups.insert(g);
            self.sync_row();
        }
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
        let seq = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
        let mut out = std::io::stdout();
        let _ = out.write_all(seq.as_bytes()).and_then(|_| out.flush());
        self.messages.info(format!("Copied {text}"));
    }

    // ----- files -----------------------------------------------------------

    fn save(&mut self, then_apply: bool) {
        self.overlay_for = For::Palette;
        if self.is_new {
            self.overlay = Overlay::SaveAs {
                input: String::new(),
                overwrite: false,
                then_apply,
            };
            return;
        }
        let name = self.name.clone();
        self.save_as(&name, then_apply);
    }

    pub(crate) fn save_as(&mut self, name: &str, then_apply: bool) {
        match self.store.save(name, &self.editor.palette) {
            Ok(()) => {
                self.name = name.to_string();
                self.is_new = false;
                self.editor.mark_saved();
                if then_apply {
                    self.apply_saved();
                } else {
                    self.notify_saved(format!("Saved \"{name}\""));
                }
            }
            Err(e) => self.messages.error(format!("Save failed: {e:#}")),
        }
    }

    /// Saves if needed, then makes this palette the system theme.
    fn apply(&mut self) {
        if self.dirty() {
            self.save(true);
        } else {
            self.apply_saved();
        }
    }

    fn apply_saved(&mut self) {
        let name = self.name.clone();
        match self.store.apply(&name) {
            Ok(()) => {
                self.active = Some(name.clone());
                self.system_palette = Some(self.editor.palette.clone());
                self.notify_saved(format!("Saved and applied \"{name}\""));
            }
            Err(e) => self.messages.error(format!("Apply failed: {e:#}")),
        }
    }

    fn open(&mut self, name: &str) {
        match self.store.load(name) {
            Ok(p) => {
                self.name = name.to_string();
                self.is_new = false;
                self.editor.load(p);
                self.reset_history();
                self.messages.info(format!("Opened \"{name}\""));
            }
            Err(e) => self.messages.error(format!("{e:#}")),
        }
    }

    fn new_palette(&mut self) {
        self.name = "untitled".to_string();
        self.is_new = true;
        self.editor.start_blank();
        self.reset_history();
        self.messages
            .info("New palette: edit the 8 base colours first, the rest follow");
    }

    pub(crate) fn reset_history(&mut self) {
        self.editor.clear_history();
        self.sel = 0;
        self.list_scroll = 0;
        self.focus = Focus::List;
        self.load_picker();
    }

    /// Runs `p` now, or asks first when there are unsaved changes.
    fn guard(&mut self, p: Pending) {
        let dirty = match p {
            Pending::NewTheme | Pending::OpenTheme(_) => self.theme_dirty(),
            Pending::Quit => self.theme_dirty() || self.settings_page.draft.is_some(),
            _ if p.is_fonts() => self.fonts.dirty(),
            _ => self.dirty(),
        };
        if dirty {
            self.overlay = Overlay::Confirm(p);
        } else {
            self.run_pending(p);
        }
    }

    fn run_pending(&mut self, p: Pending) {
        self.overlay = Overlay::None;
        match p {
            Pending::Quit => self.quit = true,
            Pending::New => self.new_palette(),
            Pending::Open(n) => self.open(&n),
            Pending::NewTheme => self.new_theme(),
            Pending::OpenTheme(n) => self.open_theme(&n),
            Pending::NewFonts => {
                self.fonts.new_preset();
                self.messages
                    .info("New font preset (a copy of the current fonts); s asks for a name");
            }
            Pending::OpenFonts(n) => match self.fonts.open(&n) {
                Ok(()) => self.messages.info(format!("Opened font preset \"{n}\"")),
                Err(e) => self.messages.error(format!("{e:#}")),
            },
        }
    }

    /// "Save first" from the unsaved-changes prompt: saves whatever `p`
    /// would throw away (both for quit), if it already has a name.
    fn save_before(&mut self, p: &Pending) {
        // themes (and quitting): the whole theme, parts included
        if matches!(p, Pending::Quit | Pending::NewTheme | Pending::OpenTheme(_)) && !self.theme_new
        {
            if self.theme_dirty() {
                self.theme_save(false);
            }
            if matches!(p, Pending::Quit) && self.settings_page.draft.is_some() {
                self.settings_save();
            }
            return;
        }
        let palette = matches!(p, Pending::Quit) || !p.is_fonts();
        let fonts = matches!(p, Pending::Quit) || p.is_fonts();
        if palette && self.dirty() && !self.is_new {
            let name = self.name.clone();
            self.save_as(&name, false);
        }
        if fonts && self.fonts.dirty() && !self.fonts.is_new {
            let name = self.fonts.name.clone();
            self.fonts_save_as(&name, false);
        }
        if matches!(p, Pending::Quit) && self.settings_page.draft.is_some() {
            self.settings_save();
        }
    }

    /// The palette list shows the highlighted palette everywhere (the
    /// colour list, the preview windows, the sample); closing it any way
    /// but choosing puts back the palette from before.
    fn sync_palette_preview(&mut self) {
        let Some(original) = &self.open_original else {
            return;
        };
        match &self.overlay {
            Overlay::Open {
                filter,
                sel,
                entries,
            } if self.overlay_for == For::Palette => {
                let shown = open_matches(entries, filter)
                    .get(*sel)
                    .and_then(|&i| self.open_palettes.get(i).cloned().flatten());
                self.editor.palette = shown.unwrap_or_else(|| original.clone());
            }
            _ => self.end_palette_preview(),
        }
    }

    /// Back to the palette from before the palette list opened.
    fn end_palette_preview(&mut self) {
        if let Some(p) = self.open_original.take() {
            self.editor.palette = p;
        }
        self.open_palettes.clear();
    }

    // ----- input -----------------------------------------------------------

    /// Number keys jump between the numbered panes: ¹ font roles, ² font
    /// list, ³ font sample, ⁴ colour list, ⁵ colour picker, ⁶ nvim, ⁷ shell
    /// (or yazi). 6 and 7 give that preview window the keyboard; F12 hands it
    /// back. A hidden picker or sample is shown.
    /// Moving off the top (`up`) or bottom of the page: the keyboard goes to
    /// the page tabs if they're that way (settings: layout.tabs). True if so.
    pub fn focus_page_tabs(&mut self, up: bool) -> bool {
        if up == self.settings.layout.tabs_on_top() {
            self.layout.page_tabs_focused = true;
        }
        self.layout.page_tabs_focused
    }

    /// Keys with the keyboard on the page tabs: left / right change pages
    /// (the keyboard stays on the tabs); back towards the page, Enter or Esc
    /// return to it. True if handled here.
    fn page_tabs_action(&mut self, a: Action) -> bool {
        const PAGES: [Page; 3] = [Page::Main, Page::Downloads, Page::Settings];
        let top = self.settings.layout.tabs_on_top();
        let i = PAGES.iter().position(|p| *p == self.page).unwrap_or(0);
        match a {
            Action::Left | Action::TabPrev => self.page = PAGES[(i + 2) % 3],
            Action::Right | Action::TabNext => self.page = PAGES[(i + 1) % 3],
            Action::Down if top => self.layout.page_tabs_focused = false,
            Action::Up if !top => self.layout.page_tabs_focused = false,
            Action::Confirm | Action::Back => self.layout.page_tabs_focused = false,
            // page keys, quitting and help work as anywhere
            Action::Quit
            | Action::Help
            | Action::PageMain
            | Action::PageDownloads
            | Action::PageSettings
            | Action::ToggleHints
            | Action::TogglePreview => return false,
            _ => {}
        }
        true
    }

    pub fn focus_pane(&mut self, n: u8) {
        use crate::fonts::FontFocus;
        self.page = Page::Main;
        match n {
            1..=3 => {
                self.preview.focus = None;
                self.side = Side::Fonts;
                match n {
                    1 => self.fonts.focus = FontFocus::Slots,
                    2 => self.fonts.open_list(),
                    _ if !self.layout.wide => {} // not drawn at this width
                    _ => {
                        self.layout.sample_visible = true;
                        self.fonts.focus = FontFocus::Sample;
                    }
                }
            }
            4 => {
                self.preview.focus = None;
                self.side = Side::Colours;
                if self.focus != Focus::List {
                    self.leave_picker(true); // only Enter keeps a change
                }
            }
            5 => {
                self.preview.focus = None;
                self.open_picker();
            }
            6 | 7 => {
                // jumping to a hidden preview shows it again
                if !self.layout.main_preview_visible {
                    self.layout.main_preview_visible = true;
                    self.preview.rects.clear();
                }
                self.preview.mode = PreviewMode::Desktop;
                let id = match (n, self.preview.show_yazi) {
                    (6, _) => PaneId::Nvim,
                    (_, true) => PaneId::Yazi,
                    _ => PaneId::Shell,
                };
                // the lower slot counts whether it last showed the shell or yazi
                let shown = self
                    .preview
                    .rects
                    .iter()
                    .any(|(p, _)| *p == id || (n == 7 && *p != PaneId::Nvim));
                if !shown && !self.preview.rects.is_empty() {
                    self.messages
                        .info("That preview window isn't shown at this window size");
                    return;
                }
                self.preview.focus = Some(id);
            }
            _ => {}
        }
    }

    /// The number of the pane with the keyboard.
    pub fn current_pane(&self) -> u8 {
        use crate::fonts::FontFocus;
        match (self.preview.focus, self.side) {
            (Some(PaneId::Nvim), _) => 6,
            (Some(_), _) => 7,
            (None, Side::Fonts) => match self.fonts.focus {
                FontFocus::Slots => 1,
                FontFocus::List | FontFocus::Search => 2,
                FontFocus::Sample => 3,
            },
            (None, Side::Colours) if self.focus == Focus::List => 4,
            (None, Side::Colours) => 5,
        }
    }

    /// `<C-w>h/j/k/l`: the neighbouring pane in the fonts and colour columns
    /// (the preview windows take the keyboard, so they're only entered on
    /// purpose: 6, 7, F12 or a click). Hidden panes are skipped.
    fn move_pane(&mut self, dx: i32, dy: i32) {
        let columns = self.layout.pane_columns();
        let cur = self.current_pane();
        let Some((c, r)) = columns
            .iter()
            .enumerate()
            .find_map(|(c, col)| col.iter().position(|&p| p == cur).map(|r| (c, r)))
        else {
            return;
        };
        let c2 = (c as i32 + dx).clamp(0, 1) as usize;
        let col = &columns[c2];
        let r2 = if dx != 0 {
            r.min(col.len() - 1)
        } else {
            (r as i32 + dy).clamp(0, col.len() as i32 - 1) as usize
        };
        let target = col[r2];
        if target != cur {
            self.focus_pane(target);
        }
    }

    /// Shows the picker and gives it the keyboard (editing a colour).
    fn open_picker(&mut self) {
        self.side = Side::Colours;
        self.layout.picker_visible = true;
        if self.focus == Focus::List {
            self.focus = Focus::Square;
        }
    }

    /// Back to the colour list from the picker: Esc (`discard`) puts back what
    /// the colours were when you started changing them, Enter keeps them. The
    /// picker hides again
    /// unless Settings says it stays open.
    fn leave_picker(&mut self, discard: bool) {
        if self.editor.end_session(self.key(), discard) {
            self.messages
                .info(format!("Discarded the change to {}", self.key()));
        }
        self.focus = Focus::List;
        if self.settings.colours.picker_hides_on_esc() {
            self.layout.picker_visible = false;
        }
    }

    /// The mouse is over this pane (and no pop-up or drag is in the way).
    pub fn pane_hovered(&self, area: Rect) -> bool {
        self.hits.over(area)
            && self.layout.dragging.is_none()
            && matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. })
    }

    /// The keymap contexts for the pane with the keyboard, most specific first.
    pub fn contexts(&self) -> [Context; 3] {
        use crate::fonts::FontFocus;
        if self.layout.resize_mode {
            return [Context::Resize; 3];
        }
        match self.page {
            Page::Main => {}
            Page::Settings => return [Context::Settings, Context::Global, Context::Global],
            Page::Downloads => return [Context::Downloads, Context::Global, Context::Global],
        }
        match self.side {
            Side::Fonts => match self.fonts.focus {
                FontFocus::Slots => [Context::FontRoles, Context::Fonts, Context::Global],
                FontFocus::List | FontFocus::Search => {
                    [Context::FontList, Context::Fonts, Context::Global]
                }
                FontFocus::Sample => [Context::Fonts, Context::Global, Context::Global],
            },
            Side::Colours if self.focus == Focus::List => {
                [Context::ColourList, Context::Colours, Context::Global]
            }
            Side::Colours => [Context::Picker, Context::Colours, Context::Global],
        }
    }

    // ----- resizing --------------------------------------------------------

    /// Resize-mode keys: h/l move the focused pane's side edge, j/k its lower
    /// edge, by one cell (capitals: five); = resets; Esc / Enter leave.
    fn run_resize(&mut self, a: Action) {
        let (dx, dy, n): (i32, i32, i32) = match a {
            Action::Left => (-1, 0, 1),
            Action::Right => (1, 0, 1),
            Action::Up => (0, -1, 1),
            Action::Down => (0, 1, 1),
            Action::LeftBig => (-1, 0, 5),
            Action::RightBig => (1, 0, 5),
            Action::UpBig => (0, -1, 5),
            Action::DownBig => (0, 1, 5),
            Action::ResetSizes => {
                self.layout.sizes = self.settings.layout.sizes.clone();
                return;
            }
            Action::Back | Action::Confirm | Action::ResizeMode => {
                self.layout.resize_mode = false;
                return;
            }
            _ => return,
        };
        let vertical = dx != 0;
        let Some(id) = self.layout.border_for_pane(self.current_pane(), vertical) else {
            return;
        };
        let b = self
            .layout
            .borders
            .iter()
            .find(|b| b.id == id)
            .copied()
            .unwrap();
        let pos = (b.pos() as i32 + (dx + dy) * n).max(0) as u16;
        self.layout.move_border(id, pos);
    }

    // ----- colour list rows -----------------------------------------------

    /// The colour list as shown: a row per group, and the colours of open groups.
    pub fn colour_rows(&self) -> Vec<ColourRow> {
        let mut out = Vec::new();
        let mut key = 0;
        for (index, (_, keys)) in GROUPS.iter().enumerate() {
            let expanded = self.expanded_groups.contains(&index);
            out.push(ColourRow::Group { index, expanded });
            for _ in keys.iter() {
                if expanded {
                    out.push(ColourRow::Key(key));
                }
                key += 1;
            }
        }
        out
    }

    fn group_of(&self, key: usize) -> usize {
        let mut start = 0;
        for (gi, (_, keys)) in GROUPS.iter().enumerate() {
            if key < start + keys.len() {
                return gi;
            }
            start += keys.len();
        }
        0
    }

    /// Moves the list cursor; landing on a colour selects it.
    fn move_row(&mut self, d: i32) {
        let rows = self.colour_rows();
        self.row_sel = (self.row_sel as i32 + d).clamp(0, rows.len() as i32 - 1) as usize;
        if let ColourRow::Key(k) = rows[self.row_sel] {
            self.select_key(k);
        }
    }

    /// Puts the list cursor on the selected colour if its group is open, else
    /// on its group.
    fn sync_row(&mut self) {
        let rows = self.colour_rows();
        let g = self.group_of(self.sel);
        self.row_sel = rows
            .iter()
            .position(|r| *r == ColourRow::Key(self.sel))
            .or_else(|| {
                rows.iter()
                    .position(|r| matches!(r, ColourRow::Group { index, .. } if *index == g))
            })
            .unwrap_or(0);
    }

    fn set_group_open(&mut self, g: usize, open: bool) {
        if open {
            self.expanded_groups.insert(g);
        } else {
            self.expanded_groups.remove(&g);
        }
    }

    /// Colour-list keys: open/close groups, move, start editing.
    fn list_action(&mut self, a: Action) {
        let rows = self.colour_rows();
        let last = rows.len() as i32 - 1;
        let copying = matches!(self.overlay, Overlay::CopyFrom { .. });
        if self.layout.palette_menu_focused && !copying {
            match a {
                Action::Down | Action::Back => self.layout.palette_menu_focused = false,
                Action::Confirm | Action::Right => self.open_overlay(),
                Action::Up => {
                    self.focus_page_tabs(true);
                }
                _ => {}
            }
            return;
        }
        if !copying {
            match a {
                Action::Up if self.row_sel == 0 => {
                    self.layout.palette_menu_focused = true;
                    return;
                }
                Action::Down
                    if self.row_sel as i32 >= last
                        && !self.layout.picker_visible
                        && self.focus_page_tabs(false) =>
                {
                    return;
                }
                _ => {}
            }
        }
        match (a, rows.get(self.row_sel).copied()) {
            (Action::Up, _) => self.move_row(-1),
            (Action::Down, _) => self.move_row(1),
            (Action::Top, _) => self.move_row(-(self.row_sel as i32)),
            (Action::Bottom, _) => self.move_row(last - self.row_sel as i32),
            (Action::PageDown, _) => self.move_row(10),
            (Action::PageUp, _) => self.move_row(-10),
            (Action::Confirm, Some(ColourRow::Group { index, expanded })) => {
                self.set_group_open(index, !expanded)
            }
            (
                Action::Right,
                Some(ColourRow::Group {
                    index,
                    expanded: false,
                }),
            ) => self.set_group_open(index, true),
            (Action::Right, Some(ColourRow::Group { expanded: true, .. })) => self.move_row(1),
            (
                Action::Left,
                Some(ColourRow::Group {
                    index,
                    expanded: true,
                }),
            ) => self.set_group_open(index, false),
            (Action::Left, Some(ColourRow::Key(k))) => {
                let g = self.group_of(k);
                self.row_sel = rows
                    .iter()
                    .position(|r| matches!(r, ColourRow::Group { index, .. } if *index == g))
                    .unwrap_or(0);
            }
            (Action::Right | Action::Confirm, Some(ColourRow::Key(_))) => self.open_picker(),
            _ => {}
        }
    }

    fn on_hex_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Enter => match Rgb::parse(&self.editor.hex_input) {
                Some(c) => {
                    self.editor.end_edit_run();
                    self.edit(c);
                    self.load_picker();
                    self.leave_picker(false);
                }
                None => self.messages.error(format!(
                    "Not a colour: {} (use #rrggbb)",
                    self.editor.hex_input
                )),
            },
            KeyCode::Esc => {
                self.load_picker();
                self.leave_picker(true);
            }
            KeyCode::Tab => self.cycle_focus(1),
            KeyCode::BackTab => self.cycle_focus(-1),
            KeyCode::Backspace => {
                self.editor.hex_input.pop();
            }
            KeyCode::Char(c) if c.is_ascii_hexdigit() || c == '#' => {
                if c == '#' {
                    self.editor.hex_input.clear();
                }
                if self.editor.hex_input.len() < 7 {
                    if self.editor.hex_input.is_empty() && c != '#' {
                        self.editor.hex_input.push('#');
                    }
                    self.editor.hex_input.push(c.to_ascii_lowercase());
                }
            }
            _ => {}
        }
    }

    /// Bracketed paste (Ctrl+Shift+V): a hex colour sets the selected key.
    fn on_paste(&mut self, s: &str) {
        if self.side == Side::Fonts && matches!(self.overlay, Overlay::None) {
            if self.fonts.focus == crate::fonts::FontFocus::Search {
                self.fonts.search.push_str(s.trim());
                self.fonts.list_sel = 0;
            }
            return;
        }
        if let Some(id) = self.preview.focus {
            return self.preview.send_paste(id, s);
        }
        match (&mut self.overlay, Rgb::parse(s)) {
            (Overlay::SaveAs { input, .. }, _) => input.push_str(s.trim()),
            (Overlay::Open { filter, sel, .. }, _) => {
                filter.push_str(s.trim());
                *sel = 0;
            }
            (Overlay::None, Some(c)) => {
                let key = self.key();
                self.set_key(key, c);
                self.messages.info(format!("{key} = {}", c.hex()));
            }
            (Overlay::None, None) => self
                .messages
                .error(format!("Pasted text isn't a colour: {}", s.trim())),
            _ => {}
        }
    }

    fn cycle_focus(&mut self, d: i32) {
        let i = FOCUS_ORDER
            .iter()
            .position(|f| *f == self.focus)
            .unwrap_or(0) as i32;
        let n = FOCUS_ORDER.len() as i32;
        self.focus = FOCUS_ORDER[(i + d).rem_euclid(n) as usize];
        if self.focus != Focus::List {
            self.layout.picker_visible = true;
        }
        if self.focus == Focus::Hex {
            self.editor.hex_input = "#".into(); // typing replaces the colour; Esc restores it
        }
    }
}

/// Sets the mouse pointer shape (OSC 22): "pointer" (hand) or "default".
pub fn set_pointer(shape: &str) {
    let mut out = std::io::stdout();
    let _ = write!(out, "\x1b]22;{shape}\x1b\\").and_then(|_| out.flush());
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

pub(crate) fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

#[cfg(test)]
mod tests;
