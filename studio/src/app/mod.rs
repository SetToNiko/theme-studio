//! Editor state and input handling.

use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::Result;
/// The terminal the studio draws on.
pub type Term = ratatui::Terminal<ratatui::backend::CrosstermBackend<crate::out::Out>>;
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::layout::Rect;

use crate::color::{self, Rgb};
use crate::fonts::FontsTab;
use crate::keymap::{Action, Context, Key, Keymap, Lookup};
use crate::palette::{self, GROUPS, Palette};
use crate::preview::{Mode as PreviewMode, PaneId, Preview};
use crate::store::{Store, TerminalMap};
use crate::theme::ThemeStore;
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
    pub palette: Palette,
    /// The palette as last loaded or saved, for "unsaved" and the old swatch.
    pub saved: Palette,
    undo: Vec<Palette>,
    redo: Vec<Palette>,
    /// Key being edited by consecutive picker changes (one undo step for all).
    last_edit: Option<&'static str>,
    /// Keys in display order (GROUPS flattened).
    pub keys: Vec<&'static str>,
    pub sel: usize,
    pub list_scroll: usize,
    pub focus: Focus,
    /// Picker state: hue (degrees), saturation and value (0..1). Kept as
    /// floats so the hue survives greys and small steps don't get rounded away.
    pub hsv: (f64, f64, f64),
    pub hex_input: String,
    pub active: Option<String>,
    /// The active system palette (hover colours are its keys).
    pub system_palette: Option<Palette>,
    pub terminal: Option<TerminalMap>,
    /// Status-line message and whether it's an error.
    pub message: Option<(String, bool)>,
    /// When an info message was shown (it goes after settings.messages.seconds).
    pub message_at: Option<Instant>,
    /// A save confirmation in a box (settings.messages.saved = "popup");
    /// stays until a key or a click.
    pub notice: Option<String>,
    pub overlay: Overlay,
    /// What's clickable where (from the last draw), and where the mouse is.
    pub hits: crate::hit::HitMap,
    /// Pane sizes (shares), from settings.json; resizing changes them.
    pub sizes: crate::config::Sizes,
    /// The draggable borders from the last draw.
    pub borders: Vec<crate::hit::Border>,
    /// The border being dragged with the mouse.
    pub dragging: Option<crate::hit::BorderId>,
    /// Resize mode (`<leader>r`): keys move the focused pane's edges.
    pub resize_mode: bool,
    /// The pointer is currently the hand (OSC 22).
    pointer_hand: bool,
    pub preview: Preview,
    pub side: Side,
    pub page: Page,
    /// The palette (and undo depth) before the picker's first change, for
    /// Esc to put back. Dropped once the keyboard is back in the list.
    edit_session: Option<(Palette, usize)>,
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
    /// Where the Palette pane was last drawn (the palette list hangs from it).
    pub palette_pane: Option<Rect>,
    /// A note for the status line about what the mouse is over (set while
    /// drawing, e.g. a font role.s requirement).
    pub hover_note: Option<String>,
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
            picker_visible: config.settings.colours.picker_shown(),
            sample_visible: config.settings.fonts.sample_at_start != "hidden",
            hints_visible: config.settings.layout.key_hints != "hidden",
            main_preview_visible: config.settings.layout.main_preview != "hidden",
            fonts_preview_visible: config.settings.layout.fonts_preview != "hidden",
            palette_menu_focused: false,
            page_tabs_focused: false,
            row_sel: 0,
            edit_session: None,
            wide: true,
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
            palette_pane: None,
            hover_note: None,
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
            saved: palette.clone(),
            palette,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: None,
            keys: GROUPS
                .iter()
                .flat_map(|(_, ks)| ks.iter().copied())
                .collect(),
            sel: 0,
            list_scroll: 0,
            focus: Focus::List,
            hsv: (0.0, 0.0, 0.0),
            hex_input: String::new(),
            active,
            system_palette,
            terminal,
            message: None,
            message_at: None,
            notice: None,
            overlay: Overlay::None,
            hits: Default::default(),
            pointer_hand: false,
            sizes: config.settings.layout.sizes.clone(),
            borders: Vec::new(),
            dragging: None,
            resize_mode: false,
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
                Ok(Some(v)) => app.info(format!(
                    "{what} updated to file format version {v} (the old file is in the history)"
                )),
                Ok(None) => {}
                Err(e) => app.error(format!("{what}: {e:#}")),
            }
        }
        if start_new {
            app.new_theme();
            app.message = None;
        } else {
            app.init_theme(named.as_deref());
        }
        app.sync_row();
        if !config.warnings.is_empty() {
            app.error(format!("Settings: {}", config.warnings.join("; ")));
        }
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut Term) -> Result<()> {
        let mut redraw = true;
        while !self.quit {
            // nvim in the preview re-reads this whenever it changes
            self.preview.sync_draft(&self.palette);
            if self.page == Page::Main && self.sample_visible {
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
            if self.expire_message() {
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
        self.is_new || self.open_original.as_ref().unwrap_or(&self.palette) != &self.saved
    }

    /// Asks for a new font sample when the fonts or colours changed, and
    /// shows a finished one.
    fn update_sample(&mut self) {
        let fonts = self.fonts.shown_fonts();
        let sizes = [0, 1, 2].map(|r| self.fonts.effective_size(r, fonts[r].size));
        let markup = crate::fonts::sample_markup(&fonts, sizes, &self.palette, &self.name);
        let dpi = self.fonts.dpi;
        self.fonts
            .sample
            .request(markup, self.palette.get("bg0"), dpi);
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
                Err(e) => self.error(format!("Apply failed: {e:#}")),
            },
            Ok(()) => self.notify_saved(format!("Saved \"{name}\"")),
            Err(e) => self.error(format!("Save failed: {e:#}")),
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
            Err(e) => self.error(format!("{e:#}")),
        }
    }

    pub fn info(&mut self, s: impl Into<String>) {
        self.message = Some((s.into(), false));
        self.message_at = Some(Instant::now());
    }

    pub fn error(&mut self, s: impl Into<String>) {
        self.message = Some((s.into(), true));
        self.message_at = None;
    }

    /// A save confirmation: in the status line like any message, or in a
    /// box (settings.messages.saved).
    pub fn notify_saved(&mut self, s: impl Into<String>) {
        if self.settings.messages.saved_in_popup() {
            self.message = None;
            self.notice = Some(s.into());
        } else {
            self.info(s);
        }
    }

    /// Takes an info message away once it has been shown long enough.
    /// Returns whether it went.
    fn expire_message(&mut self) -> bool {
        let secs = self.settings.messages.seconds;
        match self.message_at {
            Some(at) if secs > 0 && at.elapsed() >= Duration::from_secs(secs.into()) => {
                self.message = None;
                self.message_at = None;
                true
            }
            _ => false,
        }
    }

    // ----- editing ---------------------------------------------------------

    /// Points the picker at the selected key's colour.
    fn load_picker(&mut self) {
        let c = self.palette.get(self.key());
        let (h, s, v) = color::rgb_to_hsv(c);
        self.hsv = (h.unwrap_or(self.hsv.0), s, v);
        self.hex_input = c.hex();
    }

    /// Selects a colour without touching the list cursor.
    fn select_key(&mut self, i: usize) {
        if i < self.keys.len() && i != self.sel {
            self.sel = i;
            self.last_edit = None;
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

    fn push_undo(&mut self) {
        self.undo.push(self.palette.clone());
        if self.undo.len() > 500 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Sets the selected key from the picker. Consecutive changes to one key
    /// are a single undo step.
    fn edit(&mut self, c: Rgb) {
        let key = self.key();
        if self.palette.get(key) == c {
            return;
        }
        // the first change while in the picker: remember the palette, so
        // Esc can put it back
        if self.edit_session.is_none() && self.focus != Focus::List {
            self.edit_session = Some((self.palette.clone(), self.undo.len()));
        }
        if self.last_edit != Some(key) {
            self.push_undo();
            self.last_edit = Some(key);
        }
        self.palette.set(key, c);
        self.hex_input = c.hex();
    }

    /// Sets `key` to `c` as its own undo step (paste, copy from another key).
    fn set_key(&mut self, key: &'static str, c: Rgb) {
        self.push_undo();
        self.last_edit = None;
        self.palette.set(key, c);
        if key == self.key() {
            self.load_picker();
        }
    }

    fn set_hsv(&mut self, h: f64, s: f64, v: f64) {
        self.hsv = (h.clamp(0.0, 360.0), s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
        let (h, s, v) = self.hsv;
        self.edit(color::hsv_to_rgb(h, s, v));
    }

    /// Current saturation and lightness (HSL) of the selected colour.
    pub fn hsl(&self) -> (f64, f64) {
        let (_, s, l) = color::rgb_to_hsl(self.palette.get(self.key()));
        (s, l)
    }

    fn set_hsl(&mut self, s: f64, l: f64) {
        let c = color::hsl_to_rgb(self.hsv.0, s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
        let (_, sv, v) = color::rgb_to_hsv(c);
        self.hsv.1 = sv;
        self.hsv.2 = v;
        self.edit(c);
    }

    fn toggle_pin(&mut self) {
        let key = self.key();
        if palette::is_base(key) {
            self.info(format!("{key} is a base colour; it's always set by hand"));
            return;
        }
        self.push_undo();
        self.last_edit = None;
        if self.palette.is_pinned(key) {
            self.palette.unpin(key);
            self.info(format!("{key} follows {} again", palette::sources(key)));
        } else {
            let c = self.palette.get(key);
            self.palette.set(key, c);
            self.info(format!("{key} pinned"));
        }
        self.load_picker();
    }

    fn undo(&mut self) {
        match self.undo.pop() {
            Some(p) => {
                self.redo.push(std::mem::replace(&mut self.palette, p));
                self.last_edit = None;
                self.load_picker();
            }
            None => self.info("Nothing to undo"),
        }
    }

    fn redo(&mut self) {
        match self.redo.pop() {
            Some(p) => {
                self.undo.push(std::mem::replace(&mut self.palette, p));
                self.last_edit = None;
                self.load_picker();
            }
            None => self.info("Nothing to redo"),
        }
    }

    /// Copies text to the system clipboard with OSC 52 (Ghostty supports it).
    fn copy(&mut self, text: &str) {
        let seq = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
        let mut out = std::io::stdout();
        let _ = out.write_all(seq.as_bytes()).and_then(|_| out.flush());
        self.info(format!("Copied {text}"));
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
        match self.store.save(name, &self.palette) {
            Ok(()) => {
                self.name = name.to_string();
                self.is_new = false;
                self.saved = self.palette.clone();
                if then_apply {
                    self.apply_saved();
                } else {
                    self.notify_saved(format!("Saved \"{name}\""));
                }
            }
            Err(e) => self.error(format!("Save failed: {e:#}")),
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
                self.system_palette = Some(self.palette.clone());
                self.notify_saved(format!("Saved and applied \"{name}\""));
            }
            Err(e) => self.error(format!("Apply failed: {e:#}")),
        }
    }

    fn open(&mut self, name: &str) {
        match self.store.load(name) {
            Ok(p) => {
                self.name = name.to_string();
                self.is_new = false;
                self.saved = p.clone();
                self.palette = p;
                self.reset_history();
                self.info(format!("Opened \"{name}\""));
            }
            Err(e) => self.error(format!("{e:#}")),
        }
    }

    fn new_palette(&mut self) {
        self.name = "untitled".to_string();
        self.is_new = true;
        self.palette = Palette::blank();
        self.saved = self.palette.clone();
        self.reset_history();
        self.info("New palette: edit the 8 base colours first, the rest follow");
    }

    pub(crate) fn reset_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.last_edit = None;
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
                self.info("New font preset (a copy of the current fonts); s asks for a name");
            }
            Pending::OpenFonts(n) => match self.fonts.open(&n) {
                Ok(()) => self.info(format!("Opened font preset \"{n}\"")),
                Err(e) => self.error(format!("{e:#}")),
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

    fn open_overlay(&mut self) {
        self.overlay_for = For::Palette;
        match self.store.load_all() {
            Ok(all) => {
                let entries = all
                    .iter()
                    .map(|(name, v)| {
                        let sw = [
                            "bg0", "fg0", "red", "orange", "yellow", "green", "aqua", "blue",
                            "purple",
                        ]
                        .iter()
                        .filter_map(|k| v.get(*k).and_then(|x| x.as_str()).and_then(Rgb::parse))
                        .collect();
                        (name.clone(), sw)
                    })
                    .collect::<Vec<_>>();
                let sel = entries
                    .iter()
                    .position(|(n, _)| *n == self.name)
                    .unwrap_or(0);
                // each palette, shown while it's highlighted
                self.open_palettes = all
                    .values()
                    .map(|v| v.as_object().and_then(|o| Palette::from_json(o).ok()))
                    .collect();
                self.open_original = Some(self.palette.clone());
                self.overlay = Overlay::Open {
                    filter: String::new(),
                    sel,
                    entries,
                };
            }
            Err(e) => self.error(format!("{e:#}")),
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
                self.palette = shown.unwrap_or_else(|| original.clone());
            }
            _ => self.end_palette_preview(),
        }
    }

    /// Back to the palette from before the palette list opened.
    fn end_palette_preview(&mut self) {
        if let Some(p) = self.open_original.take() {
            self.palette = p;
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
            self.page_tabs_focused = true;
        }
        self.page_tabs_focused
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
            Action::Down if top => self.page_tabs_focused = false,
            Action::Up if !top => self.page_tabs_focused = false,
            Action::Confirm | Action::Back => self.page_tabs_focused = false,
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
                    _ if !self.wide => {} // not drawn at this width
                    _ => {
                        self.sample_visible = true;
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
                if !self.main_preview_visible {
                    self.main_preview_visible = true;
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
                    self.info("That preview window isn't shown at this window size");
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
        let columns: [Vec<u8>; 2] = [
            [1, 2, 3]
                .into_iter()
                .filter(|&p| p != 3 || (self.sample_visible && self.wide))
                .collect(),
            [4, 5]
                .into_iter()
                .filter(|&p| p != 5 || self.picker_visible)
                .collect(),
        ];
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
        self.picker_visible = true;
        if self.focus == Focus::List {
            self.focus = Focus::Square;
        }
    }

    /// Back to the colour list from the picker: Esc (`discard`) puts back what
    /// the colours were when you started changing them, Enter keeps them. The
    /// picker hides again
    /// unless Settings says it stays open.
    fn leave_picker(&mut self, discard: bool) {
        if discard
            && let Some((before, undo_len)) = self.edit_session.take()
            && before != self.palette
        {
            self.palette = before;
            self.undo.truncate(undo_len);
            self.redo.clear();
            self.last_edit = None;
            self.load_picker();
            self.info(format!("Discarded the change to {}", self.key()));
        }
        self.edit_session = None;
        self.focus = Focus::List;
        if self.settings.colours.picker_hides_on_esc() {
            self.picker_visible = false;
        }
    }

    /// The mouse is over this pane (and no pop-up or drag is in the way).
    pub fn pane_hovered(&self, area: Rect) -> bool {
        self.hits.over(area)
            && self.dragging.is_none()
            && matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. })
    }

    /// The keymap contexts for the pane with the keyboard, most specific first.
    pub fn contexts(&self) -> [Context; 3] {
        use crate::fonts::FontFocus;
        if self.resize_mode {
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

    /// Sets the size share a border controls.
    fn set_share(&mut self, id: crate::hit::BorderId, share: f64) {
        use crate::hit::BorderId as B;
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
    fn move_border(&mut self, id: crate::hit::BorderId, pos: u16) {
        if let Some(b) = self.borders.iter_mut().find(|b| b.id == id) {
            let share = b.share_at(pos);
            // remember where it went, so quick repeats add up before the next draw
            if b.vertical {
                b.hit.x = pos
            } else {
                b.hit.y = pos
            }
            self.set_share(id, share);
        }
    }

    /// The border resize mode moves for the focused pane: its side edge
    /// (`vertical`) or its lower edge.
    fn border_for_pane(&self, vertical: bool) -> Option<crate::hit::BorderId> {
        use crate::hit::BorderId as B;
        let pane = self.current_pane();
        let id = match (self.wide, vertical, pane) {
            (true, true, 1..=3) => B::FontsColours,
            (true, true, _) => B::ColoursPreview,
            (true, false, 1..=3) => B::ListSample,
            (true, false, 4 | 5) => B::ListPicker,
            (true, false, _) => B::PreviewSplit,
            (false, true, 6 | 7) => B::PreviewSplit,
            (false, true, _) => B::NarrowFontsColours,
            (false, false, 4 | 5) if self.picker_visible => B::ListPicker,
            (false, false, _) => B::TopPreview,
        };
        self.borders.iter().any(|b| b.id == id).then_some(id)
    }

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
                self.sizes = self.settings.layout.sizes.clone();
                return;
            }
            Action::Back | Action::Confirm | Action::ResizeMode => {
                self.resize_mode = false;
                return;
            }
            _ => return,
        };
        let vertical = dx != 0;
        let Some(id) = self.border_for_pane(vertical) else {
            return;
        };
        let b = self.borders.iter().find(|b| b.id == id).copied().unwrap();
        let pos = (b.pos() as i32 + (dx + dy) * n).max(0) as u16;
        self.move_border(id, pos);
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
        if self.palette_menu_focused && !copying {
            match a {
                Action::Down | Action::Back => self.palette_menu_focused = false,
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
                    self.palette_menu_focused = true;
                    return;
                }
                Action::Down
                    if self.row_sel as i32 >= last
                        && !self.picker_visible
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

    fn on_key(&mut self, k: KeyEvent) {
        self.handle_key(k);
        self.sync_palette_preview();
        self.end_session_outside_picker();
    }

    /// Only Enter keeps a picker change: leaving the picker any other way
    /// (Esc, another pane or column, a preview window, a click elsewhere,
    /// hiding it) puts the colours back.
    fn end_session_outside_picker(&mut self) {
        let left =
            self.focus == Focus::List || self.side != Side::Colours || self.preview.focus.is_some();
        if left && self.edit_session.is_some() {
            self.leave_picker(true);
        }
    }

    fn handle_key(&mut self, k: KeyEvent) {
        use crate::fonts::FontFocus;
        let key = Key::from_event(k);
        // A save confirmation box: any key closes it (Ctrl+C still goes on
        // to quit).
        if self.notice.take().is_some()
            && key != Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        {
            return;
        }
        // A preview window has the keyboard: everything but the key bound to
        // "type into the preview / next window / back" goes to it.
        if let Some(id) = self.preview.focus
            && matches!(self.overlay, Overlay::None)
        {
            if self.keymap.lookup(&[Context::Global], &[key]) == Lookup::Run(Action::PreviewCycle) {
                self.preview.cycle_focus();
            } else {
                self.preview.send_key(id, k);
            }
            return;
        }
        // Ctrl+C quits (asking first about unsaved changes), unless the
        // keymap uses it here (Emacs's leader, copying in VS Code)
        if key == Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
            && self.pending_keys.is_empty()
            && !matches!(self.overlay, Overlay::Capture { .. })
            && (!matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. })
                || self.keymap.lookup(&self.contexts(), &[key]) == Lookup::None)
        {
            self.pending_keys.clear();
            self.guard(Pending::Quit);
            return;
        }
        let copying = matches!(self.overlay, Overlay::CopyFrom { .. });
        if !matches!(self.overlay, Overlay::None) && !copying {
            self.on_overlay_key(k);
            return;
        }
        self.message = None;
        // Text fields: keys are typing, not actions.
        if self.pending_keys.is_empty() {
            if self.page == Page::Downloads && self.dl.searching {
                return self.dl_search_key(k);
            }
            if self.side == Side::Colours && self.focus == Focus::Hex {
                return self.on_hex_key(k);
            }
            if self.side == Side::Fonts && self.fonts.focus == FontFocus::Search {
                return self.fonts.on_search_key(k);
            }
        }
        self.pending_keys.push(key);
        let contexts = self.contexts();
        match self.keymap.lookup(&contexts, &self.pending_keys) {
            Lookup::Pending => {}
            Lookup::Run(a) => {
                self.pending_keys.clear();
                self.run_action(a);
            }
            Lookup::None => self.pending_keys.clear(),
        }
    }

    /// Does what an action means in the pane with the keyboard.
    pub fn run_action(&mut self, a: Action) {
        use crate::fonts::FontFocus;
        if self.resize_mode {
            return self.run_resize(a);
        }
        if let Overlay::CopyFrom { target } = self.overlay {
            return self.run_copy_from(target, a);
        }
        if a == Action::UndoConfig {
            return self.undo_config();
        }
        if self.page_tabs_focused && self.page_tabs_action(a) {
            return;
        }
        let fonts = self.side == Side::Fonts;
        let page_action = matches!(
            a,
            Action::Quit
                | Action::Help
                | Action::PageMain
                | Action::PageDownloads
                | Action::PageSettings
                | Action::ToggleHints
                | Action::TogglePreview
        );
        if self.page == Page::Settings && !page_action {
            return self.run_settings(a);
        }
        if self.page == Page::Downloads && !page_action {
            return self.run_downloads(a);
        }
        if self.page != Page::Main && !page_action {
            return;
        }
        match a {
            Action::Quit => self.guard(Pending::Quit),
            Action::Help => self.overlay = Overlay::Help { page: 0 },
            Action::PageMain => self.page = Page::Main,
            Action::PageDownloads => self.page = Page::Downloads,
            Action::PageSettings => self.page = Page::Settings,
            Action::PreviewCycle => {
                self.preview.mode = PreviewMode::Desktop;
                self.preview.cycle_focus();
            }
            Action::TogglePicker => {
                if self.picker_visible {
                    self.picker_visible = false;
                    self.focus = Focus::List;
                } else {
                    self.picker_visible = true;
                }
            }
            Action::ToggleHints => self.hints_visible = !self.hints_visible,
            Action::TogglePreview => match self.page {
                Page::Main => {
                    self.main_preview_visible = !self.main_preview_visible;
                    // a hidden window can't keep the keyboard
                    if !self.main_preview_visible {
                        self.preview.focus = None;
                    }
                }
                Page::Downloads => self.fonts_preview_visible = !self.fonts_preview_visible,
                Page::Settings => self.info("This page has no preview"),
            },
            Action::ToggleSample => {
                use crate::fonts::FontFocus;
                self.sample_visible = !self.sample_visible;
                if !self.sample_visible && self.fonts.focus == FontFocus::Sample {
                    self.fonts.focus = FontFocus::Slots;
                }
            }
            Action::ResizeMode => self.resize_mode = true,
            Action::PaneLeft => self.move_pane(-1, 0),
            Action::PaneRight => self.move_pane(1, 0),
            Action::PaneUp => self.move_pane(0, -1),
            Action::PaneDown => self.move_pane(0, 1),
            Action::SavePalette => self.save(false),
            Action::SaveFonts => self.fonts_save(false),
            Action::ApplyPalette => self.apply(),
            Action::ApplyFonts => self.fonts_apply(),
            // s / S / a / o / n act on the theme (palette and fonts together)
            Action::Save => self.theme_save(false),
            Action::SaveAs => {
                self.overlay_for = For::Theme;
                self.overlay = Overlay::SaveAs {
                    input: self.theme.clone(),
                    overwrite: false,
                    then_apply: false,
                };
            }
            Action::Apply => self.theme_apply(),
            Action::Open => self.theme_open_overlay(),
            Action::New => self.guard(Pending::NewTheme),
            // a part of the theme: another palette or font preset, or a new one
            Action::OpenPalette => self.open_overlay(),
            Action::OpenFonts => self.fonts_open_overlay(),
            Action::NewPalette => self.guard(Pending::New),
            Action::NewFonts => self.guard(Pending::NewFonts),
            Action::Panel(n) => self.focus_pane(n),
            // Tab in the fonts column: roles → list → sample → roles
            Action::FocusNext | Action::FocusPrev if fonts => {
                let order: Vec<u8> = [1, 2, 3]
                    .into_iter()
                    .filter(|&p| p != 3 || (self.sample_visible && self.wide))
                    .collect();
                let i = order
                    .iter()
                    .position(|&p| p == self.current_pane())
                    .unwrap_or(0) as i32;
                let d = if a == Action::FocusNext { 1 } else { -1 };
                let next = order[(i + d).rem_euclid(order.len() as i32) as usize];
                self.focus_pane(next);
                if next == 1 {
                    self.fonts.focus = FontFocus::Slots;
                }
            }
            Action::FillFamily if fonts && self.fonts.focus == FontFocus::List => {
                match self.fonts.fill_from_family() {
                    Ok(fill) => self.after_fill(fill),
                    Err(e) => self.info(e),
                }
            }
            Action::FixRoleFont if fonts && self.fonts.focus == FontFocus::Slots => {
                match self.fonts.fix_role() {
                    Ok(fill) => self.after_fill(fill),
                    Err(e) => self.info(e),
                }
            }
            // picking, and no installed font fits the role: the Fonts page,
            // where one can be had
            Action::Confirm
                if fonts
                    && self.fonts.focus == FontFocus::List
                    && self.fonts.search.is_empty()
                    && self.fonts.rows().is_empty()
                    && let Some(r) = self.fonts.picking =>
            {
                self.fonts.end_pick();
                self.fonts.focus = FontFocus::Slots;
                self.dl.tab = if r == 1 {
                    crate::fonts_dl::DlTab::Google
                } else {
                    crate::fonts_dl::DlTab::Nerd
                };
                self.dl.sel = 0;
                self.page = Page::Downloads;
            }
            // the top of the roles, the bottom of the font list (without the
            // sample under it): on to the page tabs, when they're that way
            Action::Up
                if fonts
                    && self.fonts.focus == FontFocus::Slots
                    && self.fonts.slot == 0
                    && self.focus_page_tabs(true) => {}
            Action::Down
                if fonts
                    && self.fonts.focus == FontFocus::List
                    && !(self.sample_visible && self.wide)
                    && self.fonts.list_sel + 1 >= self.fonts.rows().len()
                    && self.focus_page_tabs(false) => {}
            _ if fonts => {
                let role = self.fonts.slot;
                self.fonts.act(a);
                if self.fonts.picked {
                    let what = format!(
                        "{}: {}",
                        crate::fonts::ROLE_NAMES[role],
                        self.fonts.preset.fonts[role].family
                    );
                    self.apply_picked_fonts(what);
                }
            }
            _ => self.run_colours(a),
        }
    }

    /// A font was picked (or a family filled the roles): save the theme's
    /// font preset and apply it straight away.
    pub(crate) fn apply_picked_fonts(&mut self, what: String) {
        self.fonts.picked = false;
        let name = if self.fonts.is_new {
            self.theme.clone()
        } else {
            self.fonts.name.clone()
        };
        let result = self
            .fonts
            .save_as(&name)
            .and_then(|_| self.fonts.store.apply(&name, self.fonts.preset.sizes()));
        match result {
            Ok(()) => {
                self.fonts.active = Some(name.clone());
                self.info(format!("{what}: saved to \"{name}\" and applied"));
            }
            Err(e) => self.error(format!("{what}: {e:#}")),
        }
    }

    /// Colour-tab actions: the list, the picker and the preview toggles.
    fn run_colours(&mut self, a: Action) {
        match a {
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::Pin => self.toggle_pin(),
            Action::CopyHex => {
                let hex = self.palette.get(self.key()).hex();
                self.copy(&hex);
            }
            Action::CopyFrom => {
                self.overlay = Overlay::CopyFrom { target: self.key() };
                self.focus = Focus::List;
            }
            Action::HexEntry => {
                self.focus = Focus::Hex;
                self.hex_input = "#".into();
            }
            Action::FocusNext => self.cycle_focus(1),
            Action::FocusPrev => self.cycle_focus(-1),
            Action::Back if self.focus != Focus::List => self.leave_picker(true),
            Action::Confirm if self.focus != Focus::List => self.leave_picker(false),
            Action::ToggleYazi => {
                self.preview.mode = PreviewMode::Desktop;
                self.preview.toggle_yazi();
            }
            Action::ToggleRofi => {
                self.preview.mode = PreviewMode::Desktop;
                self.preview.show_rofi = !self.preview.show_rofi;
            }
            Action::ToggleView => {
                self.preview.mode = match self.preview.mode {
                    PreviewMode::Desktop => PreviewMode::Swatches,
                    PreviewMode::Swatches => PreviewMode::Desktop,
                };
            }
            _ if self.focus == Focus::List => self.list_action(a),
            _ => {
                let (dx, dy, big) = match a {
                    Action::Left => (-1, 0, false),
                    Action::Right => (1, 0, false),
                    Action::Up => (0, -1, false),
                    Action::Down => (0, 1, false),
                    Action::LeftBig => (-1, 0, true),
                    Action::RightBig => (1, 0, true),
                    Action::UpBig => (0, -1, true),
                    Action::DownBig => (0, 1, true),
                    _ => return,
                };
                self.nudge(dx, dy, big);
            }
        }
    }

    /// Moves the focused picker control: the square by saturation/value, the
    /// hue strip by hue, sliders by their value (up/down picks a slider).
    fn nudge(&mut self, dx: i32, dy: i32, big: bool) {
        match self.focus {
            Focus::Square => {
                let step = if big { 0.1 } else { 0.02 };
                let (h, s, v) = self.hsv;
                self.set_hsv(h, s + dx as f64 * step, v - dy as f64 * step);
            }
            Focus::Hue => {
                let step = if big { 15.0 } else { 2.0 };
                let (h, s, v) = self.hsv;
                self.set_hsv(h + (dy + dx) as f64 * step, s, v);
            }
            Focus::SliderH | Focus::SliderS | Focus::SliderL if dx != 0 => {
                let n = dx as f64 * if big { 10.0 } else { 1.0 };
                let (s, l) = self.hsl();
                match self.focus {
                    Focus::SliderH => {
                        let (h, s, v) = self.hsv;
                        self.set_hsv(h + n, s, v);
                    }
                    Focus::SliderS => self.set_hsl(s + n / 100.0, l),
                    _ => self.set_hsl(s, l + n / 100.0),
                }
            }
            Focus::SliderH | Focus::SliderS | Focus::SliderL => self.cycle_focus(dy),
            _ => {}
        }
    }

    /// While picking the key to copy a colour from: move, Enter copies, Esc cancels.
    fn run_copy_from(&mut self, target: &'static str, a: Action) {
        let on_key = matches!(
            self.colour_rows().get(self.row_sel),
            Some(ColourRow::Key(_))
        );
        match a {
            // a colour row: copy it; a group row: open or close it
            Action::Confirm | Action::Right if on_key => {
                let from = self.key();
                let c = self.palette.get(from);
                self.overlay = Overlay::None;
                let to = self.keys.iter().position(|k| *k == target).unwrap_or(0);
                self.select(to);
                self.set_key(target, c);
                self.info(format!("{target} = {from} ({})", c.hex()));
            }
            Action::Back => self.overlay = Overlay::None,
            Action::Up
            | Action::Down
            | Action::Top
            | Action::Bottom
            | Action::PageUp
            | Action::PageDown
            | Action::Confirm
            | Action::Right
            | Action::Left => self.list_action(a),
            _ => {}
        }
    }

    fn fonts_apply(&mut self) {
        if self.fonts.dirty() {
            self.fonts_save(true);
        } else {
            let name = self.fonts.name.clone();
            self.fonts_save_as(&name, true);
        }
    }

    fn on_hex_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Enter => match Rgb::parse(&self.hex_input) {
                Some(c) => {
                    self.last_edit = None;
                    self.edit(c);
                    self.load_picker();
                    self.leave_picker(false);
                }
                None => self.error(format!("Not a colour: {} (use #rrggbb)", self.hex_input)),
            },
            KeyCode::Esc => {
                self.load_picker();
                self.leave_picker(true);
            }
            KeyCode::Tab => self.cycle_focus(1),
            KeyCode::BackTab => self.cycle_focus(-1),
            KeyCode::Backspace => {
                self.hex_input.pop();
            }
            KeyCode::Char(c) if c.is_ascii_hexdigit() || c == '#' => {
                if c == '#' {
                    self.hex_input.clear();
                }
                if self.hex_input.len() < 7 {
                    if self.hex_input.is_empty() && c != '#' {
                        self.hex_input.push('#');
                    }
                    self.hex_input.push(c.to_ascii_lowercase());
                }
            }
            _ => {}
        }
    }

    fn on_overlay_key(&mut self, k: KeyEvent) {
        let overlay = std::mem::replace(&mut self.overlay, Overlay::None);
        self.overlay = match overlay {
            // arrows and page keys page through the columns; any other key closes
            Overlay::Help { page } => match k.code {
                KeyCode::Right | KeyCode::PageDown | KeyCode::Char('l') => {
                    Overlay::Help { page: page + 1 }
                }
                KeyCode::Left | KeyCode::PageUp | KeyCode::Char('h') => Overlay::Help {
                    page: page.saturating_sub(1),
                },
                _ => Overlay::None,
            },
            Overlay::Menu { menu, mut sel, at } => {
                let last = crate::fonts_preview::menu_items(menu).len() - 1;
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                match k.code {
                    KeyCode::Esc => return,
                    KeyCode::Enter => return self.choose_menu(menu, sel),
                    KeyCode::Down | KeyCode::Char('j') if !ctrl => sel = (sel + 1).min(last),
                    KeyCode::Char('n') if ctrl => sel = (sel + 1).min(last),
                    KeyCode::Up | KeyCode::Char('k') if !ctrl => sel = sel.saturating_sub(1),
                    KeyCode::Char('p') if ctrl => sel = sel.saturating_sub(1),
                    KeyCode::Home | KeyCode::Char('g') => sel = 0,
                    KeyCode::End | KeyCode::Char('G') => sel = last,
                    _ => {}
                }
                Overlay::Menu { menu, sel, at }
            }
            // Settings page pop-ups edit the draft (see draft.rs)
            Overlay::ChooseKey { row, sel, original } => {
                self.with_draft(|app| app.choose_key_key(row, sel, original, k));
                return;
            }
            Overlay::Capture { what, keys } => {
                // capture_key sets the overlay itself
                self.with_draft(|app| app.capture_key(what, keys, k));
                return;
            }
            Overlay::Prompt {
                title,
                mut input,
                purpose,
            } => match k.code {
                KeyCode::Esc => Overlay::None,
                KeyCode::Enter => {
                    // finish_prompt may open something else
                    self.with_draft(|app| app.finish_prompt(purpose, input));
                    return;
                }
                KeyCode::Backspace => {
                    input.pop();
                    Overlay::Prompt {
                        title,
                        input,
                        purpose,
                    }
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    Overlay::Prompt {
                        title,
                        input,
                        purpose,
                    }
                }
                _ => Overlay::Prompt {
                    title,
                    input,
                    purpose,
                },
            },
            Overlay::Ask { question, then } => match k.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.with_draft(|app| app.run_deferred(then));
                    return;
                }
                KeyCode::Esc | KeyCode::Char('n') => Overlay::None,
                _ => Overlay::Ask { question, then },
            },
            Overlay::Confirm(p) => match k.code {
                KeyCode::Char('y') => {
                    self.run_pending(p);
                    Overlay::None
                }
                KeyCode::Char('s') => {
                    self.save_before(&p);
                    self.run_pending(p);
                    Overlay::None
                }
                KeyCode::Esc | KeyCode::Char('n') => Overlay::None,
                _ => Overlay::Confirm(p),
            },
            // keys while copying go through the keymap (run_copy_from)
            Overlay::CopyFrom { target } => Overlay::CopyFrom { target },
            Overlay::SaveAs {
                mut input,
                overwrite,
                then_apply,
            } => match k.code {
                KeyCode::Esc => Overlay::None,
                KeyCode::Enter => {
                    let name = input.trim().to_string();
                    if self.overlay_for == For::Theme {
                        let exists = self.themes.get(&name).is_some();
                        return self.overlay = if name.is_empty() {
                            Overlay::SaveAs {
                                input,
                                overwrite,
                                then_apply,
                            }
                        } else if exists && name != self.theme && !overwrite {
                            self.error(format!(
                                "The theme \"{name}\" exists. Enter again to overwrite it"
                            ));
                            Overlay::SaveAs {
                                input,
                                overwrite: true,
                                then_apply,
                            }
                        } else {
                            self.theme_save_as(&name, then_apply);
                            std::mem::replace(&mut self.overlay, Overlay::None)
                        };
                    }
                    let (exists, current) = if self.overlay_for == For::Fonts {
                        let all = self.fonts.store.load_all();
                        (
                            all.map(|a| a.contains_key(&name)).unwrap_or(false),
                            self.fonts.name.clone(),
                        )
                    } else {
                        let all = self.store.load_all();
                        (
                            all.map(|a| a.contains_key(&name)).unwrap_or(false),
                            self.name.clone(),
                        )
                    };
                    if name.is_empty() {
                        Overlay::SaveAs {
                            input,
                            overwrite,
                            then_apply,
                        }
                    } else if self.overlay_for == For::Fonts
                        && (!exists || name == current || overwrite)
                    {
                        self.fonts_save_as(&name, then_apply);
                        Overlay::None
                    } else if exists && name != current && !overwrite {
                        self.error(format!("\"{name}\" exists. Enter again to overwrite it"));
                        Overlay::SaveAs {
                            input,
                            overwrite: true,
                            then_apply,
                        }
                    } else {
                        self.save_as(&name, then_apply);
                        Overlay::None
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                    Overlay::SaveAs {
                        input,
                        overwrite: false,
                        then_apply,
                    }
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    Overlay::SaveAs {
                        input,
                        overwrite: false,
                        then_apply,
                    }
                }
                _ => Overlay::SaveAs {
                    input,
                    overwrite,
                    then_apply,
                },
            },
            Overlay::Open {
                mut filter,
                mut sel,
                entries,
            } => {
                let n = open_matches(&entries, &filter).len();
                match k.code {
                    KeyCode::Esc => Overlay::None,
                    KeyCode::Enter => match open_matches(&entries, &filter).get(sel) {
                        Some(&i) => {
                            let name = entries[i].0.clone();
                            // the unsaved-changes check sees the real palette
                            self.end_palette_preview();
                            self.guard(match self.overlay_for {
                                For::Fonts => Pending::OpenFonts(name),
                                For::Palette => Pending::Open(name),
                                For::Theme => Pending::OpenTheme(name),
                            });
                            // guard() may have opened a confirmation instead
                            std::mem::replace(&mut self.overlay, Overlay::None)
                        }
                        None => Overlay::Open {
                            filter,
                            sel,
                            entries,
                        },
                    },
                    KeyCode::Down => {
                        sel = (sel + 1).min(n.saturating_sub(1));
                        Overlay::Open {
                            filter,
                            sel,
                            entries,
                        }
                    }
                    KeyCode::Up => {
                        sel = sel.saturating_sub(1);
                        Overlay::Open {
                            filter,
                            sel,
                            entries,
                        }
                    }
                    KeyCode::Char('n') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        sel = (sel + 1).min(n.saturating_sub(1));
                        Overlay::Open {
                            filter,
                            sel,
                            entries,
                        }
                    }
                    KeyCode::Char('p') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        sel = sel.saturating_sub(1);
                        Overlay::Open {
                            filter,
                            sel,
                            entries,
                        }
                    }
                    KeyCode::Backspace => {
                        filter.pop();
                        Overlay::Open {
                            filter,
                            sel: 0,
                            entries,
                        }
                    }
                    KeyCode::Char(c) => {
                        filter.push(c);
                        Overlay::Open {
                            filter,
                            sel: 0,
                            entries,
                        }
                    }
                    _ => Overlay::Open {
                        filter,
                        sel,
                        entries,
                    },
                }
            }
            Overlay::None => Overlay::None,
        };
    }

    fn on_mouse(&mut self, m: MouseEvent) {
        self.handle_mouse(m);
        self.sync_palette_preview();
        self.end_session_outside_picker();
    }

    fn handle_mouse(&mut self, m: MouseEvent) {
        use crate::hit::Target as T;
        let (x, y) = (m.column, m.row);
        self.hits.mouse = Some((x, y));
        // a save confirmation box: a click anywhere closes it
        if self.notice.is_some() {
            if matches!(m.kind, MouseEventKind::Down(_)) {
                self.notice = None;
            }
            return;
        }
        let target = self.hits.at(x, y);
        // With a pop-up open (other than "copy from"), only the pop-up responds.
        let popup = !matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. });
        if popup && !matches!(target, Some((_, T::OverlayRow(_) | T::ConfirmChoice(_)))) {
            // a click outside a dropdown closes it
            if matches!(self.overlay, Overlay::Menu { .. })
                && matches!(m.kind, MouseEventKind::Down(_))
            {
                self.overlay = Overlay::None;
            }
            return;
        }
        match m.kind {
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let d = if m.kind == MouseEventKind::ScrollDown {
                    1
                } else {
                    -1
                };
                match target {
                    Some((_, T::ColourRow(_))) => self.move_row(d),
                    Some((_, T::FontRow(_))) => self.fonts.scroll(d),
                    Some((_, t @ (T::SettingsRow(_) | T::SettingsDiff))) => {
                        self.settings_scroll(t, d)
                    }
                    Some((_, T::DlRow(_))) => {
                        self.run_downloads(if d > 0 { Action::Down } else { Action::Up })
                    }
                    Some((_, T::DlPreview)) => self.preview_scroll(d),
                    _ => {}
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // a click outside a preview window gives the keyboard back
                self.page_tabs_focused = false;
                if !matches!(target, Some((_, T::Preview(_)))) {
                    self.preview.focus = None;
                }
                if let Some((r, t)) = target {
                    self.click(r, t, x, y, false);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) if self.dragging.is_some() => {
                let id = self.dragging.unwrap();
                let vertical = self
                    .borders
                    .iter()
                    .find(|b| b.id == id)
                    .is_some_and(|b| b.vertical);
                self.move_border(id, if vertical { x } else { y });
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // dragging works on the picker's controls only
                if let Some((r, t @ (T::Square | T::Hue | T::SliderH | T::SliderS | T::SliderL))) =
                    target
                {
                    self.click(r, t, x, y, true);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.last_edit = None;
                self.dragging = None;
            }
            _ => {}
        }
    }

    /// A click (or a drag step) on a target at `r`.
    fn click(&mut self, r: Rect, t: crate::hit::Target, x: u16, y: u16, drag: bool) {
        use crate::fonts::FontFocus;
        use crate::hit::Target as T;
        let frac = |v: u16, start: u16, len: u16| {
            if len <= 1 {
                0.0
            } else {
                (v.saturating_sub(start) as f64 / (len - 1) as f64).clamp(0.0, 1.0)
            }
        };
        match t {
            T::PageTab(p) => self.page = p,
            T::Border(id) => self.dragging = Some(id),
            T::PaneTitle(n) => self.focus_pane(n),
            T::PaneBody(3) => self.focus_pane(3),
            T::PaneBody(_) => {}
            T::Preview(id) => {
                self.preview.mode = PreviewMode::Desktop;
                self.preview.focus = Some(id);
            }
            T::ColourRow(row) => {
                self.palette_menu_focused = false;
                let copying = matches!(self.overlay, Overlay::CopyFrom { .. });
                let again =
                    row == self.row_sel && self.focus == Focus::List && self.side == Side::Colours;
                self.side = Side::Colours;
                self.focus = Focus::List;
                self.row_sel = row;
                match self.colour_rows().get(row).copied() {
                    Some(ColourRow::Group { index, expanded }) if again => {
                        self.set_group_open(index, !expanded)
                    }
                    Some(ColourRow::Key(k)) => {
                        self.select_key(k);
                        if again && !copying {
                            self.open_picker();
                        }
                    }
                    _ => {}
                }
            }
            T::Square => {
                self.side = Side::Colours;
                self.focus = Focus::Square;
                let s = frac(x, r.x, r.width);
                let v = 1.0 - frac(y, r.y, r.height.max(2));
                self.set_hsv(self.hsv.0, s, v);
            }
            T::Hue => {
                self.side = Side::Colours;
                self.focus = Focus::Hue;
                let (_, s, v) = self.hsv;
                self.set_hsv(frac(y, r.y, r.height.max(2)) * 360.0, s, v);
            }
            T::SliderH => {
                self.side = Side::Colours;
                self.focus = Focus::SliderH;
                let (_, s, v) = self.hsv;
                self.set_hsv(frac(x, r.x, r.width) * 360.0, s, v);
            }
            T::SliderS | T::SliderL => {
                self.side = Side::Colours;
                self.focus = if t == T::SliderS {
                    Focus::SliderS
                } else {
                    Focus::SliderL
                };
                let (s, l) = self.hsl();
                let u = frac(x, r.x, r.width);
                if t == T::SliderS {
                    self.set_hsl(u, l)
                } else {
                    self.set_hsl(s, u)
                }
            }
            T::Hex if !drag => {
                self.side = Side::Colours;
                self.focus = Focus::Hex;
                self.hex_input = "#".into();
            }
            T::Hex => {}
            T::FontRole(i) => {
                self.side = Side::Fonts;
                self.fonts.click_role(i);
            }
            T::FontRow(i) => {
                self.side = Side::Fonts;
                let role = self.fonts.slot;
                self.fonts.click_row(i);
                if self.fonts.picked {
                    let what = format!(
                        "{}: {}",
                        crate::fonts::ROLE_NAMES[role],
                        self.fonts.preset.fonts[role].family
                    );
                    self.apply_picked_fonts(what);
                }
            }
            T::FontSearch => {
                self.side = Side::Fonts;
                self.fonts.focus = FontFocus::Search;
            }
            T::Chip(c) => {
                self.side = Side::Fonts;
                self.fonts.click_chip(c);
            }
            T::OverlayRow(i) => {
                if let Overlay::ChooseKey { row, sel, original } = &self.overlay {
                    let (row, sel, original) = (*row, *sel, original.clone());
                    // a click previews the key; a second click keeps it
                    let key = crate::behaviour::theme_keys()[i];
                    if sel == i {
                        self.overlay = Overlay::None;
                    } else {
                        self.overlay = Overlay::ChooseKey {
                            row,
                            sel: i,
                            original,
                        };
                    }
                    self.with_draft(|app| app.set_colour_setting(row, key));
                } else if let Overlay::Open { sel, .. } = &mut self.overlay {
                    if *sel == i {
                        // a second click opens it, like Enter
                        self.on_overlay_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                    } else {
                        *sel = i;
                    }
                } else if let Overlay::Menu { menu, .. } = self.overlay {
                    // one click chooses
                    self.choose_menu(menu, i);
                }
            }
            T::SettingsTab(_)
            | T::SettingsRow(_)
            | T::HistFile(_)
            | T::SettingsDiff
            | T::PresetRow(_) => self.settings_click(t),
            T::ThemeName => self.theme_open_overlay(),
            T::PaletteMenu => {
                self.focus_pane(4);
                self.palette_menu_focused = true;
                self.open_overlay();
            }
            T::DlTab(_) | T::DlRow(_) | T::DlChip(_) | T::DlSearch | T::DlButton(_) => {
                self.dl_click(t)
            }
            T::PreviewMenu(n) => self.open_menu(n, r),
            T::TourPane(i) => self.tour_click_pane(i),
            T::TourRow(i) => self.tour_click_row(i),
            T::DlPreview => {}
            T::SettingsButton('s') => self.settings_save(),
            T::SettingsButton(_) => self.settings_discard(),
            T::ConfirmChoice(c) => {
                let code = if c == 'n' {
                    KeyCode::Esc
                } else {
                    KeyCode::Char(c)
                };
                self.on_overlay_key(KeyEvent::new(code, KeyModifiers::NONE));
            }
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
                self.info(format!("{key} = {}", c.hex()));
            }
            (Overlay::None, None) => {
                self.error(format!("Pasted text isn't a colour: {}", s.trim()))
            }
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
            self.picker_visible = true;
        }
        if self.focus == Focus::Hex {
            self.hex_input = "#".into(); // typing replaces the colour; Esc restores it
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
