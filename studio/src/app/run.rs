//! The main loop of `App` and the terminal it draws on: run until quit, refresh and poll before
//! each draw, and send each terminal event to the key, mouse or paste handler.
//!
//! [`Term`](crate::app::run::Term), [`setup_terminal`](crate::app::run::setup_terminal) and [`restore_terminal`](crate::app::run::restore_terminal) are here too: the type, its set-up, and
//! its restore are in one file.

use std::io::stdout;
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::Frame;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;

use super::app_state::App;
use super::methods::pointer::set_pointer;
use super::state::nav::Page;
use crate::system;

/// The terminal the studio draws on.
pub type Term = ratatui::Terminal<ratatui::backend::CrosstermBackend<crate::system::out::Out>>;

impl App {
    /// Runs the main loop until the user quits: timers, drawing, one event.
    ///
    /// # Errors
    /// Fails when drawing or reading terminal events fails.
    pub fn run(
        &mut self,
        terminal: &mut Term,
        // The drawing code is in `ui`; `cli.rs` passes it in, so `app` does not depend on `ui`.
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

/// Takes over the terminal: raw mode, the alternate screen, the mouse, and the paste events.
///
/// The second value is `true` when the terminal speaks the kitty keyboard protocol.
/// [`restore_terminal`] needs it.
///
/// # Errors
/// Fails when the terminal cannot be made, or the mouse and paste events cannot be turned on.
pub(crate) fn setup_terminal() -> Result<(Term, bool)> {
    // raw mode, the alternate screen and the panic hook; drawing goes
    // through system::out::Out (for the hover underline styles)
    drop(ratatui::init());
    let terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(
        system::out::Out::default(),
    ))?;
    execute!(stdout(), EnableMouseCapture, EnableBracketedPaste)?;
    // The kitty keyboard protocol (Ghostty, kitty, foot, WezTerm…), so keys
    // like Ctrl+Shift+Z, Ctrl+1 or Ctrl+I (not Tab) arrive as themselves.
    let enhanced = ratatui::crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        let _ = execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        );
    }
    Ok((terminal, enhanced))
}

/// Gives the terminal back to the shell: the mouse pointer, the key protocol, the events, and the
/// screen.
///
/// `enhanced` is the second value of [`setup_terminal`].
pub(crate) fn restore_terminal(enhanced: bool) {
    set_pointer("default");
    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();
}
