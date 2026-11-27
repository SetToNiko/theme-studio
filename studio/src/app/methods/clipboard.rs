//! Copying text to the system clipboard.

use std::io::Write;

use crate::app::app_state::App;

impl App {
    /// Copies text to the system clipboard with OSC 52 (Ghostty supports it).
    pub(in crate::app) fn copy(&mut self, text: &str) {
        let seq = format!(
            "\x1b]52;c;{}\x07",
            crate::model::encoding::base64(text.as_bytes())
        );
        let mut out = std::io::stdout();
        let _ = out.write_all(seq.as_bytes()).and_then(|()| out.flush());
        self.messages.info(format!("Copied {text}"));
    }
}
