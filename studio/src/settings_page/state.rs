//! The state of the Settings page: the tabs, the row kinds and `SettingsPage`.
//!
//! `SettingsPage` holds the cursor, the scroll and the cached data of each tab.
//! The difference view that it caches comes from `diff.rs`.

use std::path::PathBuf;

use crate::keymap::action::Action;
use crate::keymap::context::Context;
use crate::model::history::SnapshotInfo;
use crate::settings_page::diff::DiffView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Keymaps,
    Behaviour,
    History,
    /// docs/theme-studio.md, rendered (`readme.rs`).
    Readme,
}

pub const TABS: [(Tab, &str); 4] = [
    (Tab::Keymaps, "Keymaps"),
    (Tab::Behaviour, "Behaviour"),
    (Tab::History, "History"),
    (Tab::Readme, "README"),
];

/// A row of the Keymaps tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KmRow {
    /// The preset button (opens the preset list).
    Preset,
    /// A blank line before a group (not selectable).
    Gap,
    /// A group's title (not selectable).
    Header(Context),
    Binding(Context, Action),
}

/// The files with a history, as the History tab lists them.
pub const FILES: [&str; 5] = [
    "themes.json",
    "palettes.json",
    "fonts.json",
    "keymaps.json",
    "settings.json",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistFocus {
    Files,
    Versions,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent flags of the page (tab strip focus, reloaded config, two difference views); an enum would not fit them, because any mix can be true together: the tab strip can be focused while a draft is reloaded, and `browse_all` (Keymaps tab) and `show_all` (History tab) stay set when you change tabs"
)]
pub struct SettingsPage {
    pub tab: Tab,
    pub km_sel: usize,
    pub km_scroll: usize,
    pub bh_sel: usize,
    pub bh_scroll: usize,
    pub hist_file: usize,
    pub hist_focus: HistFocus,
    pub hist_sel: usize,
    pub hist_scroll: usize,
    /// The keyboard is on the tab strip (left/right change tabs).
    pub tabs_focused: bool,
    /// Unsaved Keymaps / Behaviour changes.
    pub draft: Option<crate::settings_page::draft::Draft>,
    /// A config file was reloaded during a draft edit (see `with_draft`).
    pub reloaded: bool,
    /// Keymaps tab: closed groups.
    pub km_closed: std::collections::HashSet<Context>,
    /// Behaviour tab: closed groups, by title.
    pub bh_closed: std::collections::HashSet<&'static str>,
    /// Keymaps tab: browsing presets, with this one highlighted.
    pub browse: Option<usize>,
    pub browse_scroll: usize,
    /// The preset list's difference shows every action.
    pub browse_all: bool,
    /// The difference view shows everything, not just what differs.
    pub show_all: bool,
    pub diff_scroll: usize,
    /// README tab: the guide's text with the file's modified time (read
    /// again when that changes), its render for the last width, and the
    /// first line shown.
    pub readme: Option<(
        Option<std::time::SystemTime>,
        std::result::Result<String, String>,
    )>,
    pub readme_doc: Option<(u16, crate::settings_page::readme::Doc)>,
    pub readme_scroll: usize,
    /// Cached until something changes (see `history::generation`).
    versions: Option<(VersionsKey, Vec<(PathBuf, SnapshotInfo)>)>,
    view: Option<(ViewKey, DiffView)>,
}

/// (history generation, file)
type VersionsKey = (u64, usize);
/// (history generation, file, selected version, show everything, palette)
type ViewKey = (u64, usize, Option<PathBuf>, bool, String);

impl Default for SettingsPage {
    fn default() -> Self {
        SettingsPage {
            tab: Tab::Keymaps,
            km_sel: 0,
            km_scroll: 0,
            bh_sel: 0,
            bh_scroll: 0,
            hist_file: 0,
            hist_focus: HistFocus::Versions,
            hist_sel: 0,
            hist_scroll: 0,
            tabs_focused: false,
            draft: None,
            reloaded: false,
            km_closed: std::collections::HashSet::default(),
            bh_closed: std::collections::HashSet::default(),
            browse: None,
            browse_scroll: 0,
            browse_all: false,
            show_all: false,
            diff_scroll: 0,
            readme: None,
            readme_doc: None,
            readme_scroll: 0,
            versions: None,
            view: None,
        }
    }
}

impl SettingsPage {
    /// The cached versions of a file, if they were read at this history
    /// generation (see `history::generation`).
    pub(crate) fn cached_versions(
        &self,
        generation: u64,
        file: usize,
    ) -> Option<Vec<(PathBuf, SnapshotInfo)>> {
        match &self.versions {
            Some((k, v)) if *k == (generation, file) => Some(v.clone()),
            _ => None,
        }
    }

    /// Keeps a file's versions until the history generation changes.
    pub(crate) fn store_versions(
        &mut self,
        generation: u64,
        file: usize,
        v: Vec<(PathBuf, SnapshotInfo)>,
    ) {
        self.versions = Some(((generation, file), v));
    }

    /// The cached difference view, if it was built for exactly this
    /// generation, file, selected version, "show everything" flag and palette.
    pub(crate) fn cached_view(
        &self,
        generation: u64,
        file: usize,
        version: Option<&std::path::Path>,
        show_all: bool,
        palette: &str,
    ) -> Option<DiffView> {
        match &self.view {
            Some((k, v))
                if k.0 == generation
                    && k.1 == file
                    && k.2.as_deref() == version
                    && k.3 == show_all
                    && k.4 == palette =>
            {
                Some(v.clone())
            }
            _ => None,
        }
    }

    /// Keeps the difference view for the same five values.
    pub(crate) fn store_view(
        &mut self,
        generation: u64,
        file: usize,
        version: Option<PathBuf>,
        show_all: bool,
        palette: String,
        view: DiffView,
    ) {
        self.view = Some(((generation, file, version, show_all, palette), view));
    }
}
