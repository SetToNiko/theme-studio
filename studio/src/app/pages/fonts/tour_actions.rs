//! App's methods for the Tournament tab's set-up: choosing a pool, asking
//! before it starts, fetching its previews, and polling until the bracket
//! can be drawn up.
//!
//! This is another `impl App { ... }` block (see `fonts_preview_actions.rs`
//! for the same style). `fonts/tournament_page.rs` keeps the tab's own data (`TourPage`,
//! its stages and setup rows); `tour_rounds.rs` has the matches and results.
//! Stages: Setup → Fetching → Playing → Results.

use std::path::PathBuf;

use crate::app::app_state::App;
use crate::app::state::nav::{Deferred, Overlay, Page};
use crate::fonts::pages::download::DlTab;
use crate::fonts::pages::tournament::{POOLS, SIZES, SetupRow, Stage, TourPage};
use crate::keymap::action::Action;
use crate::model::tournament::{self, Entrant, Tournament};
use crate::system::download;
use crate::system::download::types::Entry;

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
    pub fn tour_pool(&self, key: &str) -> Vec<(Entrant, Option<Entry>)> {
        match key {
            "installed-mono" => self.installed_pool(true),
            "installed-propo" => self.installed_pool(false),
            cat => self.catalogue_pool(cat),
        }
    }

    /// The installed fonts that are monospace (`mono`) or proportional,
    /// without bitmap and special fonts. They have no catalogue entry.
    fn installed_pool(&self, mono: bool) -> Vec<(Entrant, Option<Entry>)> {
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
    }

    /// The catalogue fonts (Google and Nerd) of one `category`, with their entries.
    fn catalogue_pool(&self, category: &str) -> Vec<(Entrant, Option<Entry>)> {
        self.dl
            .catalogue
            .iter()
            .flat_map(|c| c.google.iter().chain(c.nerd.iter()))
            .filter(|e| e.category == category)
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
            .collect()
    }

    /// Enter on a pool: says what it takes and asks first.
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
        let fetch = if key.starts_with("installed") {
            "nothing to fetch".to_string()
        } else {
            self.fetch_estimate(key, n, &all)
        };
        self.overlay = Overlay::Ask {
            question: format!("{label}: {n} fonts, {} picks; {fetch}. Start?", n - 1),
            then: Deferred::StartTournament(pool),
        };
    }

    /// Text for the question: about how many previews a tournament with `n`
    /// of the pool's `pool_len` fonts must fetch, and how large they are.
    /// (A guess for the random pick: the share of fonts without a preview.)
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a rough guess shown as text: font counts and byte sizes are far below 2^52, and the values are never negative"
    )]
    fn fetch_estimate(&self, key: &str, n: usize, pool: &[(Entrant, Option<Entry>)]) -> String {
        let pool_len = pool.len();
        let missing = pool
            .iter()
            .filter_map(|(_, e)| e.as_ref())
            .filter(|e| !download::preview::has_preview(&e.id))
            .count();
        let share = n as f64 / pool_len as f64;
        let (_, bytes, _) =
            crate::system::prefetch::estimate(self.dl.catalogue.as_ref().unwrap(), key);
        let left = missing as f64 * share;
        let bytes = bytes as f64 * missing as f64 / pool_len.max(1) as f64 * share;
        format!(
            "previews to fetch: about {} ({})",
            left.round() as usize,
            crate::system::prefetch::size(bytes as u64)
        )
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
    pub fn poll_tournament(&mut self) -> bool {
        if self.page != Page::Downloads
            || self.dl.tab != DlTab::Tournament
            || self.tour_page.is_none()
        {
            return false;
        }
        let mut changed = false;
        if self.tour().stage == Stage::Fetching {
            match self.finish_fetching() {
                None => return false,
                Some(done) => changed = done,
            }
        }
        for (i, show) in self.tour_pane_fonts().into_iter().enumerate() {
            changed |= self.tour_draw_pane(i, show);
        }
        changed
    }

    /// Ends the Fetching stage once every preview is in: starts the bracket
    /// with the fonts that can be drawn. `None`: keep waiting. `Some(true)`:
    /// something changed (the bracket started, or too few previews came).
    fn finish_fetching(&mut self) -> Option<bool> {
        let (ok, failed, n) = self.tour_fetched();
        if ok + failed < n {
            return None;
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
            self.messages
                .error("Too few previews could be fetched for a tournament");
            return Some(true);
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
        Some(true)
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

    // Long function (reason verified, phase 6): one match over the set-up page's keys.
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
                t.size = crate::settings_page::rows::wrap(t.size, d, SIZES.len());
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

#[cfg(test)]
mod tests {
    use crate::app::app_state::App;
    use crate::app::startup::Start;
    use crate::app::state::nav::{Overlay, Page};
    use crate::app::test_support::*;

    #[test]
    fn a_tournament_of_installed_fonts_from_setup_to_results() {
        use crate::fonts::pages::tournament::Stage;
        let store = temp_store("tournament");
        let dir = store.dir.clone();
        let mut app = test_app(store, Start::Named("One".into()));
        let _ = std::fs::remove_dir_all(crate::model::tournament::dir());
        app.fonts.installed = (0..5)
            .map(|i| crate::fonts::types::Installed {
                family: format!("Test Mono {i}"),
                mono: true,
                nerd: false,
                bitmap: false,
                special: false,
            })
            .collect();
        app.page = Page::Downloads;
        app.dl.tab = crate::fonts::pages::download::DlTab::Tournament;
        let screen = |app: &mut App| render(app, 160, 45);
        assert!(screen(&mut app).contains("Installed monospaced"));
        // the installed monospaced pool: what it takes, then it starts
        app.tour().sel = 6;
        press(&mut app, "\n");
        let Overlay::Ask { question, .. } = &app.overlay else {
            panic!("asks first")
        };
        assert!(
            question.contains("5 fonts, 4 picks; nothing to fetch"),
            "{question}"
        );
        press(&mut app, "y");
        assert_eq!(app.tour().stage, Stage::Fetching);
        app.poll_tournament();
        assert_eq!(app.tour().stage, Stage::Playing);
        let s = screen(&mut app);
        assert!(s.contains("round 1 of 3") && s.contains("← A"), "{s}");
        assert!(!s.contains("Test Mono"), "names hidden");
        press(&mut app, "n");
        assert!(screen(&mut app).contains("Test Mono"));
        // picks: four in all; u takes one back
        press(&mut app, "hl");
        press(&mut app, "u");
        assert_eq!(app.tour().t.as_ref().unwrap().picks.len(), 1);
        press(&mut app, "lhh");
        assert_eq!(app.tour().stage, Stage::Results);
        let s = screen(&mut app);
        assert!(
            s.contains("the top 10") && s.contains("1st") && s.contains("2nd"),
            "{s}"
        );
        // the bracket
        press(&mut app, "b");
        assert!(screen(&mut app).contains("Round 1 (5 fonts)"));
        press(&mut app, "\x1b");
        assert!(!app.tour().bracket);
        // back to the setup: the result is listed
        press(&mut app, "\x1b");
        assert_eq!(app.tour().stage, Stage::Setup);
        assert!(screen(&mut app).contains("won by Test Mono"));
        assert_eq!(app.tour_past().len(), 1);
        let _ = std::fs::remove_dir_all(crate::model::tournament::dir());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
