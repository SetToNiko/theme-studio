//! The whole window: what `draw` puts on the screen.
//!
//! - `frame.rs`: `draw`, the entry point. It picks the arrangement for the window size.
//! - `tabs.rs`: the page tabs and the theme name.
//! - `pages.rs`: the page inside the arrangement, and the "window too small" screen.

/// The entry point of drawing: one frame of the window.
pub mod frame;
/// The page for the window size (Main, Download fonts, Settings).
mod pages;
/// The page tabs and the theme name in one row.
pub mod tabs;
