//! App's methods for the Download fonts page (F2): rows, status, polling
//! downloads, keys and clicks. Install and remove are in `fonts_install_actions.rs`.
//!
//! This is another `impl App { ... }` block (see `behaviour_actions.rs` for
//! the same style). The page's own data (`DlPage`, `DlTab`, `DlRow`,
//! `DlStatus`, the tab and chip lists) stays in `fonts/download_page.rs`; this file only
//! holds the methods that need the whole `App`.

use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;

use crate::app::App;
use crate::app::state::nav::Page;
use crate::fonts::pages::download::{CATEGORIES, DL_TABS, DlRow, DlStatus, DlTab, mb, norm};
use crate::keymap::action::Action;
use crate::system::download;
use crate::system::download::types::{Catalogue, Entry, Msg, Source};

impl App {
    /// fonts-installed.json (a setting; default next to settings.json).
    pub fn installed_list_path(&self) -> PathBuf {
        let p = &self.settings.fonts.installed_list;
        if p.is_empty() {
            self.config_dir.join("fonts-installed.json")
        } else {
            download::files::expand(p)
        }
    }

    /// The installed-fonts list's name for an entry.
    pub(super) fn list_name(e: &Entry) -> String {
        match &e.source {
            Source::Nerd { asset, .. } => asset.clone(),
            _ => e.family.clone(),
        }
    }

    /// The installed-fonts list (read once, again after changes).
    pub fn dl_listed(&mut self) -> serde_json::Map<String, Value> {
        if self.dl.listed.is_none() {
            self.dl.listed = Some(download::list::read_list(&self.installed_list_path()));
        }
        self.dl.listed.clone().unwrap_or_default()
    }

    pub fn dl_status(&self, e: &Entry, listed: &serde_json::Map<String, Value>) -> DlStatus {
        if let Some((_, d, t)) = self.dl.jobs.get(&e.id) {
            return DlStatus::Downloading(*d, *t);
        }
        if listed.contains_key(&Self::list_name(e)) {
            return DlStatus::Listed;
        }
        let want = match &e.source {
            Source::Nerd { asset, .. } => format!("{}nerdfont", norm(asset)),
            _ => norm(&e.family),
        };
        let on_system = self.fonts.installed.iter().any(|f| {
            let n = norm(&f.family);
            if matches!(e.source, Source::Nerd { .. }) {
                n.starts_with(&want)
            } else {
                n == want
            }
        });
        if on_system {
            DlStatus::System
        } else {
            DlStatus::None
        }
    }

    /// The tab's catalogue entries.
    pub fn dl_entries(&self) -> &[Entry] {
        match (&self.dl.catalogue, self.dl.tab) {
            (Some(c), DlTab::Google) => &c.google,
            (Some(c), DlTab::Nerd) => &c.nerd,
            _ => &[],
        }
    }

    /// The rows shown: search and category applied.
    pub fn dl_rows(&mut self) -> Vec<DlRow> {
        let q = self.dl.search.to_lowercase();
        if self.dl.tab == DlTab::Installed {
            return self
                .dl_listed()
                .keys()
                .filter(|k| k.to_lowercase().contains(&q))
                .map(|k| DlRow::Listed(k.clone()))
                .collect();
        }
        let cat = if self.dl.tab == DlTab::Google {
            CATEGORIES[self.dl.category].1
        } else {
            ""
        };
        self.dl_entries()
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.family.to_lowercase().contains(&q) && (cat.is_empty() || e.category == cat)
            })
            .map(|(i, _)| DlRow::Entry(i))
            .collect()
    }

    /// The highlighted catalogue entry.
    pub fn dl_selected(&mut self) -> Option<Entry> {
        match self.dl_rows().get(self.dl.sel) {
            Some(DlRow::Entry(i)) => self.dl_entries().get(*i).cloned(),
            _ => None,
        }
    }

    /// Background results: the catalogue, progress, finished installs. True
    /// when something changed (redraw).
    pub fn poll_downloads(&mut self) -> bool {
        if self.page == Page::Downloads && self.dl.catalogue.is_none() && !self.dl.loading {
            self.dl.start_catalogue_load();
        }
        let mut changed = false;
        // only what shows on screen asks for a redraw (prefetching sends a lot)
        while let Ok(m) = self.dl.try_recv() {
            changed |= self.handle_dl_message(m);
        }
        // the preview: fetch, draw, pick up a finished image
        self.poll_preview() || changed
    }

    /// Handles one background message. True when the screen must be redrawn.
    fn handle_dl_message(&mut self, m: Msg) -> bool {
        match m {
            Msg::Catalogue(Ok(c)) => {
                self.dl.loading = false;
                self.dl.catalogue = Some(c);
            }
            Msg::Catalogue(Err(e)) => {
                self.dl.loading = false;
                self.dl.catalogue = Some(Catalogue::default());
                self.dl.load_error = Some(e);
            }
            Msg::Progress { id, done, total } => return self.on_progress(&id, done, total),
            Msg::Preview { id, result } => return self.on_preview(id, result),
            Msg::PrefetchQueue {
                categories,
                queue,
                total,
            } => self.prefetch.queued(categories, queue, total),
            Msg::PreviewsSize(bytes) => self.prefetch.measured(bytes),
            Msg::Installed {
                id,
                result,
                warning,
            } => self.on_installed(id, result, warning),
        }
        true
    }

    /// A preview family of font `id` came (or failed). True when it is the
    /// font on screen, or one that the user asked for (not a prefetch).
    fn on_preview(&mut self, id: String, result: Result<String, String>) -> bool {
        let ours = self.prefetch.finished(&id);
        let pv = self.pv();
        let shown = !ours || pv.last.as_deref() == Some(id.as_str());
        pv.fetching.remove(&id);
        pv.families.insert(id, result);
        shown
    }

    /// Download progress of job `id`: `done` of `total` bytes (0 if unknown).
    /// True when the job exists (the screen must be redrawn).
    fn on_progress(&mut self, id: &str, done: u64, total: u64) -> bool {
        let Some(job) = self.dl.jobs.get_mut(id) else {
            return false;
        };
        (job.1, job.2) = (done, total);
        let pct = if total > 0 {
            format!("{}%", done * 100 / total.max(1))
        } else {
            mb(done)
        };
        let family = job.0.clone();
        self.messages.info(format!("Downloading {family}: {pct}"));
        true
    }

    fn dl_switch(&mut self, d: i32) {
        let i = DL_TABS
            .iter()
            .position(|(t, _)| *t == self.dl.tab)
            .unwrap_or(0);
        self.dl.tab = DL_TABS[crate::settings_page::wrap(i, d, DL_TABS.len())].0;
        // read the list again (it may have changed outside the studio)
        self.dl.listed = None;
        self.dl.sel = 0;
        self.dl.scroll = 0;
    }

    /// Keys on the page (not while typing in the search box).
    pub fn run_downloads(&mut self, a: Action) {
        let top = self.settings.layout.inner_tabs_on_top();
        if self.dl.tabs_focused {
            return self.run_dl_tab_strip(a, top);
        }
        // the Tournament tab has its own keys (what it doesn't use goes on)
        if self.dl.tab == DlTab::Tournament {
            if self.run_tournament(a) {
                return;
            }
            if !matches!(
                a,
                Action::Up | Action::Down | Action::TabNext | Action::TabPrev
            ) {
                return;
            }
        }
        // the preview's keys (text, language, size, pages, fetch)
        if self.preview_action(a) {
            return;
        }
        self.run_dl_list(a, top);
    }

    /// Keys while the keyboard is on the tab strip: left/right change tabs;
    /// moving back towards the page, Enter or Esc return to it. `top` is
    /// whether the strip is above the page.
    fn run_dl_tab_strip(&mut self, a: Action, top: bool) {
        match a {
            Action::Left | Action::TabPrev => self.dl_switch(-1),
            Action::Right | Action::TabNext => self.dl_switch(1),
            Action::Down if top => self.dl.tabs_focused = false,
            Action::Up if !top => self.dl.tabs_focused = false,
            Action::Up if top => {
                self.focus_page_tabs(true);
            }
            Action::Down if !top => {
                self.focus_page_tabs(false);
            }
            Action::Confirm | Action::Back => self.dl.tabs_focused = false,
            _ => {}
        }
    }

    /// Keys on the font list itself: move, change tab or category, search,
    /// install, remove, leave. `top` is whether the tab strip is above the page.
    fn run_dl_list(&mut self, a: Action, top: bool) {
        let n = self.dl_rows().len();
        let ok = vec![true; n];
        let edge = |sel: usize, up: bool| if up { sel == 0 } else { sel + 1 >= n };
        match a {
            Action::Up if top && edge(self.dl.sel, true) => self.dl.tabs_focused = true,
            Action::Down if !top && edge(self.dl.sel, false) => self.dl.tabs_focused = true,
            Action::Up if edge(self.dl.sel, true) && self.focus_page_tabs(true) => {}
            Action::Down if edge(self.dl.sel, false) && self.focus_page_tabs(false) => {}
            Action::TabNext => self.dl_switch(1),
            Action::TabPrev => self.dl_switch(-1),
            Action::Left | Action::Right if self.dl.tab == DlTab::Google => {
                let d = if a == Action::Left { -1 } else { 1 };
                self.dl.category =
                    crate::settings_page::wrap(self.dl.category, d, CATEGORIES.len());
                self.dl.sel = 0;
            }
            Action::Search => self.dl.searching = true,
            Action::Confirm => self.dl_install(),
            Action::Delete => self.dl_remove(),
            Action::Back => self.page = Page::Main,
            _ => {
                crate::settings_page::move_over(&mut self.dl.sel, &ok, a);
            }
        }
    }

    /// Typing into the search box (Enter or Esc leave it).
    pub fn dl_search_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Down => self.dl.searching = false,
            KeyCode::Backspace => {
                self.dl.search.pop();
            }
            KeyCode::Char(c) => self.dl.search.push(c),
            _ => {}
        }
        self.dl.sel = 0;
        self.dl.scroll = 0;
    }

    /// A click on the page.
    pub fn dl_click(&mut self, t: crate::app::state::hit::Target) {
        use crate::app::state::hit::Target as T;
        self.dl.tabs_focused = false;
        match t {
            T::DlTab(i) => {
                self.dl.tab = DL_TABS[i as usize].0;
                self.dl.listed = None;
                self.dl.sel = 0;
                self.dl.scroll = 0;
            }
            T::DlRow(i) => {
                self.dl.searching = false;
                self.dl.sel = i;
            }
            T::DlChip(i) => {
                self.dl.category = i as usize;
                self.dl.sel = 0;
            }
            T::DlSearch => self.dl.searching = true,
            T::DlButton('i') => self.dl_install(),
            T::DlButton(_) => self.dl_remove(),
            _ => {}
        }
    }
}
