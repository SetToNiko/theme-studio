//! What the editor remembers: the data structs behind `App`, not its input handling.
//!
//! - `nav.rs`: where the user is (`Page`, `Focus`, `Overlay`, ...).
//! - `palette_editor.rs`: the palette being edited, its undo history and picker numbers.
//! - `hit.rs`: what is clickable where on screen (rebuilt on every draw).
//! - `layout.rs`: pane sizes, borders, resize and drag state.
//! - `messages.rs`: the status and error messages that the UI shows.
//! - `steps.rs`: index arithmetic for list cursors (no `as` casts).

/// What is clickable where on screen.
pub mod hit;
/// Pane sizes, borders, and drag state.
pub(super) mod layout;
/// The status and error messages.
pub(super) mod messages;
/// Where the user is: page, focus, pop-up.
pub mod nav;
/// The palette being edited and its undo history.
pub(super) mod palette_editor;
/// Index arithmetic for list cursors.
pub(super) mod steps;
