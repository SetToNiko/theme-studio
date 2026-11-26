//! Editor state and input handling.

mod actions;
mod focus;
mod layout;
mod messages;
mod mouse;
mod overlay;
mod palette_editor;
mod persist;

use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::Result;
/// The terminal the studio draws on.
pub type Term = ratatui::Terminal<ratatui::backend::CrosstermBackend<crate::out::Out>>;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::Rect;

use crate::color::Rgb;
use crate::fonts::FontsTab;
use crate::keymap::{Action, Key, Keymap};
use crate::palette::{self, GROUPS, Palette};
use crate::preview::{Mode as PreviewMode, Preview};
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

    // ----- input -----------------------------------------------------------

    /// The mouse is over this pane (and no pop-up or drag is in the way).
    pub fn pane_hovered(&self, area: Rect) -> bool {
        self.hits.over(area)
            && self.layout.dragging.is_none()
            && matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. })
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
