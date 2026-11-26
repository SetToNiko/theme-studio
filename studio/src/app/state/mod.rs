//! What the editor remembers: the data structs behind `App`, not its input handling.
//!
//! - `nav.rs`: where the user is (`Page`, `Focus`, `Overlay`, ...).
//! - `palette_editor.rs`: the palette being edited, its undo history and picker numbers.
//! - `hit.rs`: what is clickable where on screen (rebuilt on every draw).
//! - `layout.rs`: pane sizes, borders, resize and drag state.
//! - `messages.rs`: the status and error messages that the UI shows.
//! - `steps.rs`: index arithmetic for list cursors (no `as` casts).

pub mod hit;
pub(super) mod layout;
pub(super) mod messages;
pub mod nav;
pub(super) mod palette_editor;
pub(super) mod steps;
