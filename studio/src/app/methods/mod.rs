//! Core methods of `App`: queries, editing, the font sample, the pointer, and the clipboard.
//!
//! Each file holds one `impl App` block for one topic. The files in this
//! folder share nothing except `App` itself.

/// Copy text to the system clipboard.
mod clipboard;
/// Colour editing: picker, undo, redo, pins.
mod editing;
/// The mouse pointer shape.
pub(super) mod pointer;
/// Questions about the state of `App`.
mod queries;
/// The font sample image.
mod sample;
