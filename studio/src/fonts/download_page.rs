//! The Download fonts page (F2): Google Fonts, Nerd Fonts and Installed tabs;
//! search, category chips, and installing / removing (see download.rs).

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use serde_json::Value;

use crate::system::download::{self, Catalogue, Msg};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DlTab {
    Google,
    Nerd,
    Installed,
    Tournament,
}

pub const DL_TABS: [(DlTab, &str); 4] = [
    (DlTab::Google, "Google Fonts"),
    (DlTab::Nerd, "Nerd Fonts"),
    (DlTab::Installed, "Installed"),
    (DlTab::Tournament, "Tournament"),
];

/// The Google tab's category chips: (label, Fontsource category).
pub const CATEGORIES: [(&str, &str); 6] = [
    ("All", ""),
    ("Sans", "sans-serif"),
    ("Serif", "serif"),
    ("Mono", "monospace"),
    ("Display", "display"),
    ("Handwriting", "handwriting"),
];

/// A row of the list.
#[derive(Clone, Debug, PartialEq)]
pub enum DlRow {
    /// A catalogue entry (index into the tab's catalogue).
    Entry(usize),
    /// An entry of the installed-fonts list, by name.
    Listed(String),
}

/// Where a font stands.
#[derive(Clone, Debug, PartialEq)]
pub enum DlStatus {
    None,
    /// Downloading: bytes so far, total (0 if unknown).
    Downloading(u64, u64),
    /// In the installed-fonts list (downloaded here, or provided by Nix).
    Listed,
    /// Already on the system some other way.
    System,
}

pub struct DlPage {
    pub tab: DlTab,
    pub sel: usize,
    pub scroll: usize,
    pub search: String,
    pub searching: bool,
    pub category: usize,
    pub tabs_focused: bool,
    pub catalogue: Option<Catalogue>,
    pub loading: bool,
    pub load_error: Option<String>,
    /// Running installs: id → (family, done, total).
    pub jobs: HashMap<String, (String, u64, u64)>,
    /// The installed-fonts list as last read (None: read it again).
    pub(crate) listed: Option<serde_json::Map<String, Value>>,
    /// The preview pane (made on first use).
    pub preview: Option<crate::fonts::preview_page::FontPreview>,
    /// After this download (id), fill these roles from the family (a
    /// Nerd Font fetched for roles that had no fitting face).
    pub fill_after: Option<(String, String, Vec<usize>)>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl Default for DlPage {
    fn default() -> Self {
        let (tx, rx) = download::channel_pair();
        DlPage {
            tab: DlTab::Google,
            sel: 0,
            scroll: 0,
            search: String::new(),
            searching: false,
            category: 0,
            tabs_focused: false,
            catalogue: None,
            loading: false,
            load_error: None,
            jobs: HashMap::new(),
            listed: None,
            preview: None,
            fill_after: None,
            tx,
            rx,
        }
    }
}

/// A name for comparing: letters and digits only, lower case.
pub(crate) fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

impl DlPage {
    /// Starts loading the catalogue in the background (results arrive as
    /// messages; see `App::poll_downloads`).
    pub(crate) fn start_catalogue_load(&mut self) {
        self.loading = true;
        download::load_catalogue(self.tx.clone());
    }

    /// Where background work reports back.
    pub(crate) fn sender(&self) -> Sender<Msg> {
        self.tx.clone()
    }

    /// The next waiting background message, if any.
    pub(crate) fn try_recv(&self) -> Result<Msg, TryRecvError> {
        self.rx.try_recv()
    }
}

/// Bytes as megabytes.
pub fn mb(b: u64) -> String {
    format!("{:.1} MB", b as f64 / 1_000_000.0)
}
