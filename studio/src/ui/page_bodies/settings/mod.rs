//! Drawing the Settings page (F3): tab strip, then the Keymaps, Behaviour,
//! History or README tab. Everything clickable goes into the hit map with a
//! hover look.
//!
//! - `page.rs`: the frame and the tab strip, then the current tab.
//! - `buttons.rs`: the Save and Discard buttons.
//! - `keymaps.rs`: the Keymaps tab.
//! - `behaviour.rs`: the Behaviour tab.
//! - `history.rs`: the History tab.
//! - `readme.rs`: the README tab.
//! - `values.rs`: a value as coloured text, shared by the tabs.

/// The Behaviour tab.
mod behaviour;
/// The Save and Discard buttons.
pub mod buttons;
/// The History tab.
mod history;
/// The Keymaps tab.
mod keymaps;
/// The frame, the tab strip and the current tab.
pub mod page;
/// The README tab.
mod readme;
/// A value as coloured text, shared by the tabs.
mod values;
