//! The live terminal behind the preview: the hidden programs and their draft palette.
//!
//! [`crate::preview_term::preview::Preview`] starts nvim (your config), an interactive shell (starship) and yazi
//! in hidden terminals ([`crate::preview_term::pane::Pane`]) and writes the draft palette to a colours
//! file that they read. `ui/desktop_preview.rs` draws their screens with the draft
//! palette, so nothing has to be reloaded except nvim, which re-reads the draft
//! colours file itself. The frame and click targets of the Preview pane are in
//! `ui/panes/preview.rs`.
//!
//! `types.rs` has `PaneId` and `Mode`, `preview.rs` the `Preview` type, `bar.rs`
//! the background read of the bar, and `drafts.rs` the draft colours file.

/// Reads the status-bar blocks on a background thread.
mod bar;
/// The text of the draft colours file and the clean-up of old draft folders.
mod drafts;
/// One program in a hidden terminal.
pub mod pane;
/// The `Preview` type: the hidden programs and the draft palette.
pub(crate) mod preview;
/// The names of the panes and the preview modes.
pub(crate) mod types;
