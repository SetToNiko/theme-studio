//! Input handling: turns keys and mouse events into changes of `App`.
//!
//! - `actions.rs`: `run_action`, the one place that runs a key action.
//! - `focus.rs`: moving the focus between panes.
//! - `mouse.rs`: clicks, drags and scrolling.
//! - `overlay.rs`: the pop-ups (naming, open lists, confirmations).
//! - `colours.rs`: picking and changing colours.

/// Run one key action.
pub(super) mod actions;
/// Pick and change colours.
pub(super) mod colours;
/// Move the focus between panes.
pub(super) mod focus;
/// Clicks, drags, and scrolling.
pub(super) mod mouse;
/// The pop-ups.
pub(super) mod overlay;
