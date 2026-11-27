//! The data and rules of the studio. No screen code here: nothing in `model`
//! depends on `app` or `ui`. Some modules read or write the studio's own data
//! files (palettes, themes, history, saved tournaments). None uses the network.
//! No module here starts a program: `system::switch` runs the switch scripts.
//! Nothing in `model` depends on `fonts`, `system` or `settings_page`.
//!
//! - `color`: colour type and conversions (hex, HSV, HSL, WCAG contrast).
//! - `encoding`: base64 encoding.
//! - `history`: version history of the config files.
//! - `palette`: a palette, its 34 keys, and the derive rules.
//! - `paths`: where the data, share, config and state folders are.
//! - `samples`: preview texts for fonts, as Pango markup.
//! - `settle`: images wait for a pane size to settle.
//! - `store`: reading and writing palettes.json and the other data files.
//! - `theme`: a name for a palette, a font preset and sizes (themes.json).
//! - `tournament`: the font tournament bracket and its saved state.

/// Colour type and conversions (hex, HSV, HSL, WCAG contrast).
pub mod color;
/// Base64 encoding.
pub mod encoding;
/// Version history of the config files.
pub mod history;
/// A palette, its 34 keys, and the derive rules.
pub mod palette;
/// Where the data, share, config and state folders are.
pub mod paths;
/// Preview texts for fonts, as Pango markup.
pub mod samples;
/// Waiting for a pane size to settle before images are drawn.
pub mod settle;
/// Reading and writing palettes.json and the other data files.
pub mod store;
/// A name for a palette, a font preset and sizes (themes.json).
pub mod theme;
/// The font tournament bracket and its saved state.
pub mod tournament;
