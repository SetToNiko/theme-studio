//! The status-line message, the save confirmation box, and the hover note.
//!
//! [`Messages`](crate::app::state::messages::Messages) holds what the user is told and when it goes away. It knows
//! nothing about the rest of the studio. The settings it needs (how long a
//! message stays) come in as parameters. [`App`](crate::app::app_state::App) calls it.

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

#[cfg(test)]
mod tests {
    use crate::app::startup::Start;
    use crate::app::test_support::*;
    use ratatui::crossterm::event::{MouseButton, MouseEventKind};
    use std::time::{Duration, Instant};

    #[test]
    fn messages_time_out_and_saves_can_pop_up() {
        let store = temp_store("messages");
        let dir = store.dir.clone();
        crate::model::history::init_for_test(dir.join("history"));
        let mut app = test_app(store, Start::Named("One".into()));
        let ago = |s: u64| Instant::now().checked_sub(Duration::from_secs(s));
        // info messages go after settings.messages.seconds (3 by default)
        app.messages.info("hello");
        assert!(!app.messages.expire(app.settings.messages.seconds));
        app.messages.message_at = ago(3);
        assert!(app.messages.expire(app.settings.messages.seconds));
        assert_eq!(app.messages.message, None);
        // errors stay until a key
        app.messages.error("bad");
        assert!(!app.messages.expire(app.settings.messages.seconds));
        assert!(app.messages.message.is_some());
        // 0: until a key
        app.settings.messages.seconds = 0;
        app.messages.info("stays");
        app.messages.message_at = ago(100);
        assert!(!app.messages.expire(app.settings.messages.seconds));
        app.settings.messages.seconds = 3;
        // saving the settings says "Saved"; switched to popup, that save
        // already shows in a box
        app.with_draft(|a| a.settings.messages.saved = "popup".into());
        app.settings_save();
        assert_eq!(app.messages.notice.as_deref(), Some("Saved"));
        assert_eq!(app.messages.message, None);
        assert!(render(&mut app, 160, 45).contains("press any key to dismiss"));
        // a key closes it and does nothing else
        let sel = app.sel;
        press(&mut app, "j");
        assert_eq!(app.messages.notice, None);
        assert_eq!(app.sel, sel);
        // so does a click
        app.notify_saved("Saved \"One\"");
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 1);
        assert_eq!(app.messages.notice, None);
        // bar: the status line, like any message
        app.settings.messages.saved = "bar".into();
        app.notify_saved("Saved");
        assert_eq!(app.messages.message, Some(("Saved".into(), false)));
        assert_eq!(app.messages.notice, None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
