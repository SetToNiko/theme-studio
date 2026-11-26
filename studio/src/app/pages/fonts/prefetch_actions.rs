//! App's methods that drive font-preview prefetching.
//!
//! This is a second `impl App { ... }` block (see `persist.rs` for the same
//! style). The prefetch data (`Prefetch`, the queue, the size estimates) and
//! its logic stay in `prefetch.rs`; this file only holds the methods that
//! need the whole `App`: the per-loop poll, feeding downloads, and measuring
//! the space the previews use.

use crate::app::App;
use crate::app::state::nav::Page;
use crate::download::{self, Msg};
use crate::prefetch::queue;

impl App {
    // Long function: one top-to-bottom sequence of early exits (off, no catalogue, queue not built), then feed and progress.
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
            return self.prefetch.stop_new();
        }
        let Some(cat) = &self.dl.catalogue else {
            if !self.dl.loading {
                self.dl.loading = true;
                download::load_catalogue(self.dl_sender());
            }
            return false;
        };
        let cats = s.prefetch_categories.clone();
        if !self.prefetch.is_built_for(&cats) {
            // the queue is made on a thread (it checks every font's folder)
            if self.prefetch.start_building(&cats) {
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
        self.prefetch.progress_due()
    }

    /// Starts downloads from the urgent queue or the prefetch queue, up to
    /// `AT_ONCE` at a time (Nerd Font archives one at a time; see
    /// [`crate::prefetch::Prefetch::take_next`]).
    fn feed(&mut self, urgent: bool) {
        let fetch = self.prefetch.fetch.unwrap_or(download::fetch_preview);
        while let Some(e) = self.prefetch.take_next(urgent) {
            if self.prefetch.is_in_flight(&e.id) {
                continue;
            }
            let tx = self.dl_sender();
            let pv = self.pv();
            if pv.families.contains_key(&e.id) {
                self.prefetch.mark_already_fetched(urgent);
                continue;
            }
            // the page may be fetching it already; its result counts here too
            if pv.fetching.insert(e.id.clone()) {
                fetch(e.clone(), tx);
            }
            self.prefetch.mark_in_flight(&e, urgent);
        }
    }

    /// The Behaviour tab shows the space the previews take: measured on a
    /// thread while it's open, every few seconds.
    fn measure_previews(&mut self) {
        let shown = self.page == Page::Settings
            && self.settings_page.tab == crate::settings_page::Tab::Behaviour;
        if !self.prefetch.start_measuring_due(shown) {
            return;
        }
        let tx = self.dl_sender();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::PreviewsSize(download::previews_size()));
        });
    }
}
