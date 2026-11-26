//! Fetching font previews ahead of time (settings.json: fonts-page.prefetch
//! and prefetch-categories), so the Fonts page and the tournament draw at
//! once. Everything slow happens off the main thread: the queue is made on a
//! thread (it checks ~1,700 folders), each download is a thread of its own
//! (curl and tar under nice), and the screen is redrawn for the progress at
//! most a few times a second. Fonts already fetched are skipped, so it
//! carries on where it stopped after a restart.

use std::collections::HashSet;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::system::download::{Catalogue, Entry, Msg, Source};

/// How many previews download at once (Nerd Font archives: one).
const AT_ONCE: usize = 3;

/// The status line's progress is redrawn at most this often.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// The Behaviour tab's "space used" is measured again after this long.
const SIZE_EVERY: Duration = Duration::from_secs(5);

/// A Google / Fontsource preview file, about.
const GOOGLE_FILE: u64 = 100_000;
/// A Nerd Font's Regular, kept once its archive is unpacked, about.
const NERD_KEPT: u64 = 2_500_000;

/// What fetches a preview (a stub in tests).
pub type Fetch = fn(Entry, Sender<Msg>);

#[derive(Debug, Default)]
pub struct Prefetch {
    /// The categories the queue was made for, and ones it's being made for.
    built_for: Option<Vec<String>>,
    building: Option<Vec<String>>,
    /// Still to fetch (taken from the end).
    queue: Vec<Entry>,
    /// Fetched first, prefetching on or off: a tournament's fonts.
    pub urgent: Vec<Entry>,
    /// Those of them on their way (they don't count in the progress).
    urgent_in_flight: HashSet<String>,
    in_flight: HashSet<String>,
    /// Nerd Font archives among them.
    pub(crate) nerd_in_flight: HashSet<String>,
    /// Fonts in the chosen categories, and how many have a preview.
    pub total: usize,
    pub done: usize,
    /// The progress last drawn, and when.
    drawn: (usize, Option<Instant>),
    /// The space the previews take, and when it was measured (None while
    /// it's being measured for the first time).
    pub used: Option<u64>,
    measured: Option<Instant>,
    measuring: bool,
    /// Tests swap the downloads for a stub.
    pub fetch: Option<Fetch>,
}

impl Prefetch {
    /// Downloading (or about to).
    pub fn active(&self) -> bool {
        !self.queue.is_empty()
            || !self.urgent.is_empty()
            || !self.in_flight.is_empty()
            || self.building.is_some()
    }

    /// Prefetching is off: forgets the queue and the thread making it, so
    /// nothing new starts (downloads on their way still land). True when
    /// that made it go from active to idle.
    pub fn stop_new(&mut self) -> bool {
        let was = self.active();
        self.queue.clear();
        self.built_for = None;
        self.building = None;
        was && !self.active()
    }

    /// The queue was made for exactly these categories.
    pub fn is_built_for(&self, categories: &[String]) -> bool {
        self.built_for.as_deref() == Some(categories)
    }

    /// Notes that a thread starts making the queue for `categories`. False
    /// when one is already making it (nothing to start).
    pub fn start_building(&mut self, categories: &[String]) -> bool {
        if self.building.as_deref() == Some(categories) {
            return false;
        }
        self.building = Some(categories.to_vec());
        true
    }

    /// True when the status line's progress should be redrawn now (it
    /// changed, and the last draw was long enough ago). Notes the draw.
    pub fn progress_due(&mut self) -> bool {
        let due = self.drawn.1.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY);
        if self.drawn.0 != self.done && due {
            self.drawn = (self.done, Some(Instant::now()));
            return true;
        }
        false
    }

    /// True when the space used should be measured now: the Behaviour tab is
    /// `shown`, no measuring runs, and the last one is old enough. Notes that
    /// a measuring starts.
    pub fn start_measuring_due(&mut self, shown: bool) -> bool {
        if !shown || self.measuring || self.measured.is_some_and(|t| t.elapsed() < SIZE_EVERY) {
            return false;
        }
        self.measuring = true;
        true
    }

    /// Takes the next font to download from the urgent queue or the prefetch
    /// queue. None when `AT_ONCE` downloads are on their way, the queue is
    /// empty, or the next one is a Nerd Font archive and one is already on
    /// its way (they go one at a time).
    pub fn take_next(&mut self, urgent: bool) -> Option<Entry> {
        if self.in_flight.len() >= AT_ONCE {
            return None;
        }
        let q = if urgent {
            &mut self.urgent
        } else {
            &mut self.queue
        };
        let nerd = matches!(q.last()?.source, Source::Nerd { .. });
        if nerd && !self.nerd_in_flight.is_empty() {
            return None;
        }
        q.pop()
    }

    /// This font is on its way already.
    pub fn is_in_flight(&self, id: &str) -> bool {
        self.in_flight.contains(id)
    }

    /// A taken font was already there: counts as done (unless urgent: those
    /// don't count in the progress).
    pub fn mark_already_fetched(&mut self, urgent: bool) {
        if !urgent {
            self.done += 1;
        }
    }

    /// A taken font's download started.
    pub fn mark_in_flight(&mut self, e: &Entry, urgent: bool) {
        if matches!(e.source, Source::Nerd { .. }) {
            self.nerd_in_flight.insert(e.id.clone());
        }
        if urgent {
            self.urgent_in_flight.insert(e.id.clone());
        }
        self.in_flight.insert(e.id.clone());
    }

    /// A preview finished; true if it was one of these.
    pub fn finished(&mut self, id: &str) -> bool {
        self.nerd_in_flight.remove(id);
        let ours = self.in_flight.remove(id);
        if ours && !self.urgent_in_flight.remove(id) {
            self.done += 1;
        }
        ours
    }

    /// The queue made on a thread arrived.
    pub fn queued(&mut self, categories: Vec<String>, queue: Vec<Entry>, total: usize) {
        if self.building.as_ref() != Some(&categories) {
            return; // the choice changed since: another is on its way
        }
        self.building = None;
        self.done = total - queue.len();
        self.total = total;
        self.queue = queue;
        self.built_for = Some(categories);
    }

    /// The space used, measured.
    pub fn measured(&mut self, bytes: u64) {
        self.used = Some(bytes);
        self.measured = Some(Instant::now());
        self.measuring = false;
    }

    /// Starts over (after the previews were deleted).
    pub fn reset(&mut self) {
        let fetch = self.fetch;
        *self = Prefetch::default();
        self.fetch = fetch;
    }
}

/// The fonts of `categories` still to fetch (in catalogue order, Google's
/// before the Nerd Fonts' archives), and how many there are in all.
pub fn queue(
    cat: &Catalogue,
    categories: &[String],
    has: impl Fn(&str) -> bool,
) -> (Vec<Entry>, usize) {
    let all: Vec<&Entry> = cat
        .google
        .iter()
        .chain(cat.nerd.iter())
        .filter(|e| categories.contains(&e.category))
        .collect();
    let total = all.len();
    let mut q: Vec<Entry> = all.into_iter().filter(|e| !has(&e.id)).cloned().collect();
    // taken with pop(): the first ones last
    q.reverse();
    (q, total)
}

/// A category's fonts: (how many, bytes to download, bytes kept), about.
pub fn estimate(cat: &Catalogue, category: &str) -> (usize, u64, u64) {
    let mut out = (0, 0, 0);
    for e in cat.google.iter().chain(cat.nerd.iter()) {
        if e.category != category {
            continue;
        }
        out.0 += 1;
        if let Source::Nerd { size, .. } = e.source {
            out.1 += size;
            out.2 += NERD_KEPT;
        } else {
            out.1 += GOOGLE_FILE;
            out.2 += GOOGLE_FILE;
        }
    }
    out
}

/// Bytes, readably: "850 KB", "7.2 MB", "1.5 GB".
#[expect(
    clippy::cast_precision_loss,
    reason = "the number is only shown with one decimal; a size would need more than 2^52 bytes to lose precision"
)]
pub fn size(b: u64) -> String {
    match b {
        b if b >= 1_000_000_000 => format!("{:.1} GB", b as f64 / 1e9),
        b if b >= 1_000_000 => format!("{:.1} MB", b as f64 / 1e6),
        b => format!("{} KB", b.div_ceil(1000)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, category: &str, source: Source) -> Entry {
        Entry {
            id: id.into(),
            family: id.into(),
            category: category.into(),
            license: String::new(),
            weights: vec![400],
            italic: false,
            variable: false,
            source,
            subset: "latin".into(),
        }
    }

    #[test]
    fn queues_the_chosen_categories_and_skips_fetched_ones() {
        let nerd = Source::Nerd {
            asset: "Hack".into(),
            url: String::new(),
            size: 40_000_000,
        };
        let cat = Catalogue {
            google: vec![
                entry("fira-code", "monospace", Source::Google),
                entry("inter", "sans-serif", Source::Google),
                entry("iosevka", "monospace", Source::Fontsource),
            ],
            nerd: vec![entry("hack", "nerd font", nerd)],
            nerd_tag: String::new(),
        };
        let mono = vec!["monospace".to_string(), "nerd font".to_string()];
        let (q, total) = queue(&cat, &mono, |id| id == "iosevka");
        assert_eq!(total, 3);
        // popped in catalogue order: fira-code first, the archive last
        let order: Vec<&str> = q.iter().rev().map(|e| e.id.as_str()).collect();
        assert_eq!(order, ["fira-code", "hack"]);
        assert_eq!(estimate(&cat, "monospace"), (2, 200_000, 200_000));
        assert_eq!(estimate(&cat, "nerd font"), (1, 40_000_000, NERD_KEPT));
        assert_eq!(size(200_000), "200 KB");
        assert_eq!(size(7_250_000), "7.2 MB");
        assert_eq!(size(1_500_000_000), "1.5 GB");
    }
}
