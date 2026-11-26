//! App's methods that drive font-preview prefetching.
//!
//! This is a second `impl App { ... }` block (see `persist.rs` for the same
//! style). The prefetch data (`Prefetch`, the queue, the size estimates) and
//! its logic stay in `prefetch.rs`; this file only holds the methods that
//! need the whole `App`: the per-loop poll, feeding downloads, and measuring
//! the space the previews use.

use std::time::Instant;

use super::App;
use super::nav::Page;
use crate::download::{self, Msg, Source};
use crate::prefetch::{AT_ONCE, PROGRESS_EVERY, SIZE_EVERY, queue};

impl App {
    /// Each loop: keeps a few previews downloading while prefetching is on,
    /// and the Behaviour tab's "space used" up to date. Quick: the slow
    /// parts run on threads. True when the screen should be redrawn.
    pub fn poll_prefetch(&mut self) -> bool {
        self.measure_previews();
        // a tournament's fonts first, whatever the settings say
        self.feed(true);
        let s = &self.settings.fonts_page;
        if !s.prefetch {
            // ones on their way still land; nothing new starts
            let was = self.prefetch.active();
            self.prefetch.queue.clear();
            self.prefetch.built_for = None;
            self.prefetch.building = None;
            return was && !self.prefetch.active();
        }
        let Some(cat) = &self.dl.catalogue else {
            if !self.dl.loading {
                self.dl.loading = true;
                download::load_catalogue(self.dl_sender());
            }
            return false;
        };
        let cats = s.prefetch_categories.clone();
        if self.prefetch.built_for.as_ref() != Some(&cats) {
            // the queue is made on a thread (it checks every font's folder)
            if self.prefetch.building.as_ref() != Some(&cats) {
                self.prefetch.building = Some(cats.clone());
                let (cat, tx) = (cat.clone(), self.dl_sender());
                std::thread::spawn(move || {
                    let (queue, total) = queue(&cat, &cats, download::has_preview);
                    let _ = tx.send(Msg::PrefetchQueue {
                        categories: cats,
                        queue,
                        total,
                    });
                });
            }
            return false;
        }
        self.feed(false);
        // the status line's "fetching previews 12/72", a few times a second
        let p = &mut self.prefetch;
        let due = p.drawn.1.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY);
        if p.drawn.0 != p.done && due {
            p.drawn = (p.done, Some(Instant::now()));
            return true;
        }
        false
    }

    /// Starts downloads from the urgent queue or the prefetch queue, up to
    /// AT_ONCE at a time (Nerd Font archives one at a time).
    fn feed(&mut self, urgent: bool) {
        let fetch = self.prefetch.fetch.unwrap_or(download::fetch_preview);
        while self.prefetch.in_flight.len() < AT_ONCE {
            let q = if urgent {
                &self.prefetch.urgent
            } else {
                &self.prefetch.queue
            };
            let Some(e) = q.last() else {
                break;
            };
            // Nerd Font archives one at a time
            let nerd = matches!(e.source, Source::Nerd { .. });
            if nerd && !self.prefetch.nerd_in_flight.is_empty() {
                break;
            }
            let e = if urgent {
                self.prefetch.urgent.pop()
            } else {
                self.prefetch.queue.pop()
            }
            .unwrap();
            if self.prefetch.in_flight.contains(&e.id) {
                continue;
            }
            let tx = self.dl_sender();
            let pv = self.pv();
            if pv.families.contains_key(&e.id) {
                if !urgent {
                    self.prefetch.done += 1;
                }
                continue;
            }
            // the page may be fetching it already; its result counts here too
            if pv.fetching.insert(e.id.clone()) {
                fetch(e.clone(), tx);
            }
            if nerd {
                self.prefetch.nerd_in_flight.insert(e.id.clone());
            }
            if urgent {
                self.prefetch.urgent_in_flight.insert(e.id.clone());
            }
            self.prefetch.in_flight.insert(e.id);
        }
    }

    /// The Behaviour tab shows the space the previews take: measured on a
    /// thread while it's open, every few seconds.
    fn measure_previews(&mut self) {
        let shown = self.page == Page::Settings
            && self.settings_page.tab == crate::settings::Tab::Behaviour;
        let p = &mut self.prefetch;
        if !shown || p.measuring || p.measured.is_some_and(|t| t.elapsed() < SIZE_EVERY) {
            return;
        }
        p.measuring = true;
        let tx = self.dl_sender();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::PreviewsSize(download::previews_size()));
        });
    }
}
