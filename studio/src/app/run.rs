//! The main loop of `App`: run until quit, refresh and poll before each draw,
//! and send each terminal event to the key, mouse or paste handler.

use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use super::state::nav::Page;
use super::{App, Term};

impl App {
    /// Runs the main loop until the user quits: timers, drawing, one event.
    ///
    /// # Errors
    /// Fails when drawing or reading terminal events fails.
    pub fn run(
        &mut self,
        terminal: &mut Term,
        // The drawing code is in ui.rs; main.rs passes it in, so app does not depend on ui.
        draw: fn(&mut Frame, &mut App),
    ) -> Result<()> {
        let mut redraw = true;
        while !self.quit {
            if self.refresh_before_draw() {
                redraw = true;
            }
            if self.preview.take_dirty() || redraw {
                terminal.draw(|f| draw(f, self))?;
                self.update_pointer();
                redraw = false;
            }
            // Short poll: the preview's programs can print at any time.
            if event::poll(Duration::from_millis(25))? {
                self.handle_event(&event::read()?);
                redraw = true;
            }
        }
        Ok(())
    }

    /// Brings the preview and the sample up to date before a frame is drawn.
    /// True when the screen must be drawn again.
    fn refresh_before_draw(&mut self) -> bool {
        // nvim in the preview re-reads this whenever it changes
        self.preview.sync_draft(&self.editor.palette);
        if self.page == Page::Main && self.layout.sample_visible {
            self.update_sample();
        }
        self.poll_background()
    }

    /// Checks the background work and the timers. True when the screen must
    /// be drawn again.
    fn poll_background(&mut self) -> bool {
        // `|=` runs every poll even when an earlier one asked for a redraw
        // downloads: the catalogue, progress, finished installs
        let mut redraw = self.poll_downloads();
        redraw |= self.poll_prefetch();
        redraw |= self.poll_tournament();
        redraw |= self.messages.expire(self.settings.messages.seconds);
        // a role's size, once h / l have been still for a moment: saved
        // to the theme and applied (one i3 restart, not one per step)
        if self
            .fonts
            .sized_at
            .is_some_and(|t| t.elapsed() >= Duration::from_millis(400))
        {
            self.apply_sizes();
            redraw = true;
        }
        // an image waiting for its pane's size to settle
        if self.redraw_at.is_some_and(|t| Instant::now() >= t) {
            redraw = true;
        }
        redraw
    }

    /// Sends one terminal event to the key, mouse or paste handler.
    fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Key(k) if k.kind == KeyEventKind::Press => self.on_key(*k),
            Event::Mouse(m) => self.on_mouse(*m),
            Event::Paste(s) => {
                self.on_paste(s);
                self.sync_palette_preview();
            }
            _ => {}
        }
    }
}
