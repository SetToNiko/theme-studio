//! App's methods that drive font-preview prefetching.
//!
//! This is a second `impl App { ... }` block (see `persist.rs` for the same
//! style). The prefetch data (`Prefetch`, the queue, the size estimates) and
//! its logic stay in `system/prefetch.rs`; this file only holds the methods that
//! need the whole `App`: the per-loop poll, feeding downloads, and measuring
//! the space the previews use.

use crate::app::app_state::App;
use crate::app::state::nav::Page;
use crate::system::download;
use crate::system::download::types::Msg;
use crate::system::prefetch::queue;

impl App {
    // Long function (reason verified, phase 6): one top-to-bottom sequence of early exits (off, no catalogue, queue not built), then feed and progress.
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
                download::catalogue::load_catalogue(self.dl_sender());
            }
            return false;
        };
        let cats = s.prefetch_categories.clone();
        if !self.prefetch.is_built_for(&cats) {
            // the queue is made on a thread (it checks every font's folder)
            if self.prefetch.start_building(&cats) {
                let (cat, tx) = (cat.clone(), self.dl_sender());
                std::thread::spawn(move || {
                    let (queue, total) = queue(&cat, &cats, download::preview::has_preview);
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
    /// [`crate::system::prefetch::Prefetch::take_next`]).
    fn feed(&mut self, urgent: bool) {
        let fetch = self
            .prefetch
            .fetch
            .unwrap_or(download::preview::fetch_preview);
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
            && self.settings_page.tab == crate::settings_page::state::Tab::Behaviour;
        if !self.prefetch.start_measuring_due(shown) {
            return;
        }
        let tx = self.dl_sender();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::PreviewsSize(download::preview::previews_size()));
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::app::startup::Start;
    use crate::app::test_support::*;
    use std::time::{Duration, Instant};

    #[test]
    fn prefetching_keeps_the_main_loop_quick() {
        use crate::system::download::types::{Catalogue, Entry, Msg, Source};
        use std::sync::mpsc::Sender;
        // downloads answer at once (no network)
        #[expect(
            clippy::needless_pass_by_value,
            reason = "the prefetch field is a function pointer that takes the entry and the sender by value"
        )]
        fn stub(e: Entry, tx: Sender<Msg>) {
            let _ = tx.send(Msg::Preview {
                id: e.id,
                result: Ok(e.family),
            });
        }
        let store = temp_store("prefetch-loop");
        let dir = store.dir.clone();
        let mut app = test_app(store, Start::Named("One".into()));
        let entry = |i: usize, category: &str, source: Source| Entry {
            id: format!("font-{i}"),
            family: format!("Font {i}"),
            category: category.into(),
            license: String::new(),
            weights: vec![400],
            italic: false,
            variable: false,
            source,
            subset: "latin".into(),
        };
        let nerd = |i| Source::Nerd {
            asset: format!("N{i}"),
            url: String::new(),
            size: 1,
        };
        app.dl.catalogue = Some(Catalogue {
            google: (0..2000)
                .map(|i| entry(i, "monospace", Source::Google))
                .collect(),
            nerd: (2000..2005)
                .map(|i| entry(i, "nerd font", nerd(i)))
                .collect(),
            nerd_tag: String::new(),
        });
        app.settings.fonts_page.prefetch = true;
        app.settings.fonts_page.prefetch_categories = vec!["monospace".into(), "nerd font".into()];
        app.prefetch.fetch = Some(stub);
        let (start, mut slowest) = (Instant::now(), Duration::ZERO);
        while app.prefetch.done < 2005 || app.prefetch.active() {
            let t = Instant::now();
            app.poll_downloads();
            app.poll_prefetch();
            slowest = slowest.max(t.elapsed());
            assert!(
                app.prefetch.nerd_in_flight.len() <= 1,
                "archives one at a time"
            );
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "stuck at {}",
                app.prefetch.done
            );
        }
        assert_eq!((app.prefetch.done, app.prefetch.total), (2005, 2005));
        // a tick never takes long (generous for a debug build)
        assert!(
            slowest < Duration::from_millis(20),
            "a tick took {slowest:?}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
