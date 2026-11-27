//! Overlays drawn on top of a page: the dialogs, the key-sequence hint and the save notice.
//!
//! - `dialogs.rs`: the dialogs, and `help.rs`: the help text.
//! - `notice.rs`: the save notice.
//! - `pending.rs`: the hint for a half-typed key sequence.
//! - `boxes.rs`: `popup` and `centered`, the shared box helpers; only overlays use them.

/// The shared box helpers.
mod boxes;
/// The dialogs.
pub mod dialogs;
/// The help text.
mod help;
/// The save notice.
pub mod notice;
/// The hint for a half-typed key sequence.
pub mod pending;
