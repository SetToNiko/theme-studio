//! App's methods for the Tournament tab's set-up: choosing a pool, asking
//! before it starts, fetching its previews, and polling until the bracket
//! can be drawn up.
//!
//! This is another `impl App { ... }` block (see `fonts_preview_actions.rs`
//! for the same style). `tour_page.rs` keeps the tab's own data (`TourPage`,
//! its stages and setup rows); `tour_rounds.rs` has the matches and results.
//! Stages: Setup → Fetching → Playing → Results.

use std::path::PathBuf;

use crate::app::App;
use crate::app::state::nav::{Deferred, Overlay, Page};
use crate::download::{self, Entry};
use crate::fonts::download_page::DlTab;
use crate::keymap::action::Action;
use crate::tour_page::{POOLS, SIZES, SetupRow, Stage, TourPage};
use crate::tournament::{self, Entrant, Tournament};

impl App {
    pub fn tour(&mut self) -> &mut TourPage {
        if self.tour_page.is_none() {
            self.tour_page = Some(TourPage::new(
                self.preview.draft_dir(),
                &self.preview.dirty_flag(),
            ));
        }
        self.tour_page.as_mut().unwrap()
    }

    /// The setup's rows.
    pub fn tour_setup_rows(&mut self) -> Vec<SetupRow> {
        let past = self.tour_past().len();
        let mut v: Vec<SetupRow> = (0..POOLS.len()).map(SetupRow::Pool).collect();
        v.push(SetupRow::Size);
        v.extend((0..past).map(SetupRow::Past));
        v
    }

    pub fn tour_past(&mut self) -> &Vec<(PathBuf, Tournament)> {
        let t = self.tour();
        if t.past.is_none() {
            t.past = Some(tournament::past(&tournament::dir()));
        }
        t.past.as_ref().unwrap()
    }

    /// A pool's fonts (all of them).
    // Long function: builds the `Entrant`s of both sources (installed fonts, catalogue) in one match.
    pub fn tour_pool(&self, key: &str) -> Vec<(Entrant, Option<Entry>)> {
        let installed = |mono: bool| -> Vec<(Entrant, Option<Entry>)> {
            self.fonts
                .installed
                .iter()
                .filter(|f| f.mono == mono && !f.bitmap && !f.special)
                .map(|f| {
                    (
                        Entrant {
                            id: f.family.clone(),
                            name: f.family.clone(),
                            installed: true,
                        },
                        None,
                    )
                })
                .collect()
        };
        match key {
            "installed-mono" => installed(true),
            "installed-propo" => installed(false),
            cat => self
                .dl
                .catalogue
                .iter()
                .flat_map(|c| c.google.iter().chain(c.nerd.iter()))
                .filter(|e| e.category == cat)
                .map(|e| {
                    (
                        Entrant {
                            id: e.id.clone(),
                            name: e.family.clone(),
                            installed: false,
                        },
                        Some(e.clone()),
                    )
                })
                .collect(),
        }
    }

    /// Enter on a pool: says what it takes and asks first.
    // Long function: checks the pool, estimates the fetch, then builds the question, in that order.
    fn tour_ask(&mut self, pool: usize) {
        let (key, label) = POOLS[pool];
        let all = self.tour_pool(key);
        if self.dl.catalogue.is_none() && !key.starts_with("installed") {
            return self.messages.info("The catalogue is still loading");
        }
        let size = SIZES[self.tour().size];
        let n = if size == 0 {
            all.len()
        } else {
            size.min(all.len())
        };
        if n < 2 {
            return self
                .messages
                .info(format!("{label}: not enough fonts for a tournament"));
        }
        // what fetching the previews takes (a guess for the random pick)
        let missing: Vec<&Entry> = all
            .iter()
            .filter_map(|(_, e)| e.as_ref())
            .filter(|e| !download::has_preview(&e.id))
            .collect();
        let fetch = if key.starts_with("installed") {
            "nothing to fetch".to_string()
        } else {
            let share = n as f64 / all.len() as f64;
            let (_, bytes, _) = crate::prefetch::estimate(self.dl.catalogue.as_ref().unwrap(), key);
            let left = missing.len() as f64 * share;
            let bytes = bytes as f64 * missing.len() as f64 / all.len().max(1) as f64 * share;
            format!(
                "previews to fetch: about {} ({})",
                left.round() as usize,
                crate::prefetch::size(bytes as u64)
            )
        };
        self.overlay = Overlay::Ask {
            question: format!("{label}: {n} fonts, {} picks; {fetch}. Start?", n - 1),
            then: Deferred::StartTournament(pool),
        };
    }

    /// Starts a tournament of a pool: picks the fonts, fetches what's
    /// missing (the bracket is drawn up once the previews are in).
    pub fn tour_start(&mut self, pool: usize) {
        let (key, label) = POOLS[pool];
        let mut all = self.tour_pool(key);
        let size = SIZES[self.tour().size];
        self.tour().shuffle(&mut all);
        if size > 0 {
            all.truncate(size);
        }
        let n = all.len();
        let title = if size == 0 {
            format!("{label}, all {n}")
        } else {
            format!("{label}, {n} at random")
        };
        for (e, entry) in &all {
            match entry {
                Some(entry) => self.prefetch.urgent.push(entry.clone()),
                // installed: drawn from the system's fonts as they are
                None => {
                    self.pv().families.insert(e.id.clone(), Ok(e.name.clone()));
                }
            }
        }
        let t = self.tour();
        t.pending = all.into_iter().map(|(e, _)| e).collect();
        t.pool = (key.to_string(), title);
        t.stage = Stage::Fetching;
        t.t = None;
    }

    /// Fetching: how many previews are in (or failed), of how many.
    pub fn tour_fetched(&mut self) -> (usize, usize, usize) {
        let ids: Vec<String> = self.tour().pending.iter().map(|e| e.id.clone()).collect();
        let pv = self.pv();
        let ok = ids
            .iter()
            .filter(|id| matches!(pv.families.get(*id), Some(Ok(_))))
            .count();
        let failed = ids
            .iter()
            .filter(|id| matches!(pv.families.get(*id), Some(Err(_))))
            .count();
        (ok, failed, ids.len())
    }

    /// Each loop on the Tournament tab: the bracket once the previews are
    /// in, the two fonts drawn. True when something changed.
    // Long function: two stages in order (finish fetching and start the bracket, then draw the panes).
    pub fn poll_tournament(&mut self) -> bool {
        if self.page != Page::Downloads
            || self.dl.tab != DlTab::Tournament
            || self.tour_page.is_none()
        {
            return false;
        }
        let mut changed = false;
        if self.tour().stage == Stage::Fetching {
            let (ok, failed, n) = self.tour_fetched();
            if ok + failed < n {
                return false;
            }
            // every preview in: the fonts that can be drawn play
            let ready: Vec<Entrant> = {
                let pending = std::mem::take(&mut self.tour().pending);
                let pv = self.pv();
                pending
                    .into_iter()
                    .filter(|e| matches!(pv.families.get(&e.id), Some(Ok(_))))
                    .collect()
            };
            let t = self.tour();
            if ready.len() < 2 {
                t.stage = Stage::Setup;
                return {
                    self.messages
                        .error("Too few previews could be fetched for a tournament");
                    true
                };
            }
            t.start_bracket(ready);
            t.stage = Stage::Playing;
            t.past = None;
            self.tour_save();
            if failed > 0 {
                self.messages.info(format!(
                    "{failed} fonts left out: their previews couldn't be fetched"
                ));
            }
            changed = true;
        }
        for (i, show) in self.tour_pane_fonts().into_iter().enumerate() {
            changed |= self.tour_draw_pane(i, show);
        }
        changed
    }

    /// Which entrants the two panes show: the current pairing while playing,
    /// the selected result on the results page.
    fn tour_pane_fonts(&mut self) -> [Option<usize>; 2] {
        let t = self.tour();
        match (t.stage, &t.t) {
            (Stage::Playing, Some(x)) => match x.pairing() {
                Some((a, b)) => [Some(a), Some(b)],
                None => [None, None],
            },
            (Stage::Results, Some(x)) => {
                [x.top().into_iter().flat_map(|(_, v)| v).nth(t.sel), None]
            }
            _ => [None, None],
        }
    }

    // Long function: one match over the set-up page's keys.
    pub(in crate::app) fn run_tour_setup(&mut self, a: Action) -> bool {
        let rows = self.tour_setup_rows();
        let last = rows.len().saturating_sub(1);
        let t = self.tour();
        t.sel = t.sel.min(last);
        let row = rows[t.sel].clone();
        match a {
            Action::Up if t.sel == 0 => return false, // on to the tabs
            Action::Up => t.sel -= 1,
            Action::Down => t.sel = (t.sel + 1).min(last),
            Action::Top => t.sel = 0,
            Action::Bottom => t.sel = last,
            Action::Left | Action::Right if row == SetupRow::Size => {
                let d = if a == Action::Left { -1 } else { 1 };
                t.size = crate::settings::wrap(t.size, d, SIZES.len());
            }
            Action::Confirm => match row {
                SetupRow::Pool(p) => self.tour_ask(p),
                SetupRow::Size => t.size = (t.size + 1) % SIZES.len(),
                SetupRow::Past(i) => {
                    let Some((_, past)) = self.tour_past().get(i).cloned() else {
                        return true;
                    };
                    let t = self.tour();
                    t.stage = if past.finished() {
                        Stage::Results
                    } else {
                        Stage::Playing
                    };
                    t.pool = (past.category.clone(), past.title.clone());
                    t.t = Some(past);
                    t.sel = 0;
                }
            },
            Action::Delete => {
                if let SetupRow::Past(i) = row
                    && let Some((path, _)) = self.tour_past().get(i).cloned()
                {
                    let _ = std::fs::remove_file(&path);
                    self.tour().past = None;
                    self.messages.info("Deleted that tournament");
                }
            }
            _ => return false,
        }
        true
    }
}
