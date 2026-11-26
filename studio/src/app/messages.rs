//! The status-line message, the save confirmation box, and the hover note.
//!
//! [`Messages`] holds what the user is told and when it goes away. It knows
//! nothing about the rest of the studio. The settings it needs (how long a
//! message stays) come in as parameters. [`App`](super::App) calls it.

use std::time::{Duration, Instant};

/// What the studio tells the user: the status-line message, the save confirmation box, and the hover note.
pub struct Messages {
    /// Status-line message and whether it's an error.
    pub message: Option<(String, bool)>,
    /// When an info message was shown (it goes after settings.messages.seconds).
    pub message_at: Option<Instant>,
    /// A save confirmation in a box (settings.messages.saved = "popup");
    /// stays until a key or a click.
    pub notice: Option<String>,
    /// A note for the status line about what the mouse is over (set while
    /// drawing, e.g. a font role.s requirement).
    pub hover_note: Option<String>,
}

impl Messages {
    /// Makes an empty set: nothing is shown.
    pub fn new() -> Self {
        Self {
            message: None,
            message_at: None,
            notice: None,
            hover_note: None,
        }
    }

    /// Shows an info message in the status line. It goes away after a while (see [`Messages::expire`]).
    pub fn info(&mut self, s: impl Into<String>) {
        self.message = Some((s.into(), false));
        self.message_at = Some(Instant::now());
    }

    /// Shows an error message in the status line. It stays until the next key.
    pub fn error(&mut self, s: impl Into<String>) {
        self.message = Some((s.into(), true));
        self.message_at = None;
    }

    /// Shows a save confirmation in a box. It replaces the status-line message
    /// and stays until a key or a click.
    pub fn popup(&mut self, s: impl Into<String>) {
        self.message = None;
        self.notice = Some(s.into());
    }

    /// Takes an info message away once it has been shown for `secs` seconds
    /// (0: never). Returns whether it went.
    pub fn expire(&mut self, secs: u32) -> bool {
        match self.message_at {
            Some(at) if secs > 0 && at.elapsed() >= Duration::from_secs(secs.into()) => {
                self.message = None;
                self.message_at = None;
                true
            }
            _ => false,
        }
    }
}
