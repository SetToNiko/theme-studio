//! What the editor remembers: the data structs behind `App`, not its input handling.
//!
//! - `nav.rs`: where the user is (`Page`, `Focus`, `Overlay`, ...).
//! - `palette_editor.rs`: the palette being edited, its undo history and picker numbers.
//! - `layout.rs`: pane sizes, borders, resize and drag state.
//! - `messages.rs`: the status and error messages that the UI shows.

pub(super) mod layout;
pub(super) mod messages;
pub mod nav;
pub(super) mod palette_editor;
