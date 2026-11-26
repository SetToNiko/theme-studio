//! The data and rules of the studio. No screen code here: nothing in `core`
//! depends on `app` or `ui`. Some modules read or write the studio's own data
//! files (palettes, themes, history, saved tournaments); none starts a program
//! or uses the network, except `history` (one `date` call) and `store` (one
//! command value for `theme-switch`).
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
