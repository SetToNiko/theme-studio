//! Input handling: turns keys and mouse events into changes of `App`.
//!
//! - `actions.rs`: `run_action`, the one place that runs a key action.
//! - `focus.rs`: moving the focus between panes.
//! - `mouse.rs`: clicks, drags and scrolling.
//! - `overlay.rs`: the pop-ups (naming, open lists, confirmations).
//! - `colours.rs`: picking and changing colours.

pub(super) mod actions;
pub(super) mod colours;
pub(super) mod focus;
pub(super) mod mouse;
pub(super) mod overlay;
