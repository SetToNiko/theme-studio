//! The panes of the Main page, one file per pane.
//!
//! - `colours.rs`: the Palette pane (the palette dropdown and the colour list).
//! - `picker.rs`: the Colour pane (colour square, hue strip, sliders, hex input).
//! - `preview.rs`: the Preview pane (live desktop, or the swatch view).

/// The Palette pane.
pub mod colours;
/// The Colour pane.
pub mod picker;
/// The Preview pane.
pub mod preview;
