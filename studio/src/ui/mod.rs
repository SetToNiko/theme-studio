//! Drawing. The studio's own chrome uses named terminal colours (so it
//! follows the system theme); only swatches and the preview use the palette
//! being edited.
//!
//! - `screen/`: the whole window: `frame.rs` is the entry point (`draw`).
//! - `status/`: the status line.
//! - `overlays/`: the dialogs and boxes on top of a page.
//! - `page_bodies/`, `panes/`: the bodies of the pages and the panes of the Main page.
//! - `layout.rs`, `desktop_preview.rs`: the sizes and places of the areas.
//! - `widgets.rs`, `pane_chrome.rs`: small pieces that several screens share.

/// The live desktop preview inside the Preview pane.
pub mod desktop_preview;
/// The sizes and places of the areas for each window size.
pub mod layout;
/// The dialogs, the key-sequence hint and the save notice.
mod overlays;
/// The bodies of the pages and tabs.
mod page_bodies;
/// The border and the title of a pane.
pub mod pane_chrome;
/// The panes of the Main page.
mod panes;
/// The whole window: the frame, the page tabs and the page for each window size.
pub mod screen;
/// The status line.
mod status;
/// Small drawing pieces that several screens share.
pub mod widgets;
