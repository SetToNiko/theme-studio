//! The Fonts page's Tournament tab: choosing a pool, fetching its previews,
//! playing the matches, and the results (tournament.rs has the bracket).
//! Stages: Setup → Fetching → Playing → Results; Esc goes back to Setup,
//! where an unfinished tournament can be picked up again.
//! The `impl App` methods are in `app/tour_actions.rs` and `app/tour_rounds.rs`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use ratatui_image::protocol::StatefulProtocol;

use crate::fonts::SampleRenderer;
use crate::samples::Kind;
use crate::settle::Settle;
use crate::tournament::{Entrant, Rng, Tournament};

/// The pools: (key, label). Google's categories and Nerd Fonts come from
/// the catalogue; the installed ones from the system's fonts.
pub const POOLS: [(&str, &str); 8] = [
    ("monospace", "Google monospace"),
    ("sans-serif", "Google sans-serif"),
    ("serif", "Google serif"),
    ("display", "Google display"),
    ("handwriting", "Google handwriting"),
    ("nerd font", "Nerd Fonts"),
    ("installed-mono", "Installed monospaced"),
    ("installed-propo", "Installed proportional"),
];

/// Pool sizes: all the category's fonts (0), or some picked at random.
pub const SIZES: [usize; 4] = [0, 16, 32, 64];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Setup,
    /// The pool's previews being fetched before the first match.
    Fetching,
    Playing,
    Results,
}

/// A setup row.
#[derive(Clone, Debug, PartialEq)]
pub enum SetupRow {
    Pool(usize),
    Size,
    /// A past (or unfinished) tournament.
    Past(usize),
}

/// One of the two preview panes (the results use the first).
pub struct Pane {
    pub renderer: SampleRenderer,
    pub image: Option<StatefulProtocol>,
    pub error: Option<String>,
    pub settle: Settle,
    /// Its size in cells (from the last draw, once settled).
    pub area: (u16, u16),
    /// The entrant it shows.
    pub shows: Option<usize>,
}

pub struct TourPage {
    pub stage: Stage,
    /// The cursor: setup rows, or the results' fonts.
    pub sel: usize,
    pub size: usize,
    /// Fetching: the pool, before the bracket is drawn up.
    pub pending: Vec<Entrant>,
    pub pool: (String, String),
    pub t: Option<Tournament>,
    pub names: bool,
    pub bracket: bool,
    pub bracket_scroll: usize,
    pub kind: Kind,
    pub lang: usize,
    pub pt: u32,
    pub panes: [Pane; 2],
    /// Past tournaments (read again when the setup shows).
    pub past: Option<Vec<(PathBuf, Tournament)>>,
    rng: Rng,
}

impl TourPage {
    /// A new page at the Setup stage, with its two preview panes.
    pub(crate) fn new(draft_dir: &Path, dirty: &Arc<AtomicBool>) -> TourPage {
        let pane = |name| Pane {
            renderer: SampleRenderer::named(draft_dir, dirty.clone(), name),
            image: None,
            error: None,
            settle: Settle::default(),
            area: (60, 20),
            shows: None,
        };
        TourPage {
            stage: Stage::Setup,
            sel: 0,
            size: 0,
            pending: Vec::new(),
            pool: Default::default(),
            t: None,
            names: false,
            bracket: false,
            bracket_scroll: 0,
            kind: Kind::Code,
            lang: 0,
            pt: 13,
            panes: [pane("tour-a"), pane("tour-b")],
            past: None,
            rng: Rng::new(),
        }
    }

    /// Shuffles a pool with the page's random numbers.
    pub(crate) fn shuffle<T>(&mut self, pool: &mut [T]) {
        self.rng.shuffle(pool);
    }

    /// Draws up the bracket for `entrants` and makes it the current tournament.
    pub(crate) fn start_bracket(&mut self, entrants: Vec<Entrant>) {
        let (key, title) = self.pool.clone();
        self.t = Some(Tournament::new(&key, &title, entrants, &mut self.rng));
    }

    /// Picks the left or right font of the current match. True when that
    /// was the last match (no tournament: false).
    pub(crate) fn pick(&mut self, left: bool) -> bool {
        let Some(x) = self.t.as_mut() else {
            return false;
        };
        x.pick(left, &mut self.rng);
        x.finished()
    }
}
