//! The data and rules of the studio. No screen code here: nothing in `model`
//! depends on `app` or `ui`. Some modules read or write the studio's own data
//! files (palettes, themes, history, saved tournaments). None uses the network.
//! Only `store` starts a program: `Store::apply` runs `theme-switch` (the
//! command comes from `paths::switch_command`) and waits for it. In the
//! tests, `paths::switch_command` gives a harmless command instead.
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

pub mod color;
pub mod encoding;
pub mod history;
pub mod palette;
pub mod paths;
pub mod samples;
pub mod settle;
pub mod store;
pub mod theme;
pub mod tournament;
