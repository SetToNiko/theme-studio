//! The fonts column of the Main page: font presets (fonts.json, the data folder;
//! applied by font-switch) with their Monospace, Proportional and Icons
//! roles, the installed fonts (fontconfig), and the sample image
//! (`ImageMagick` + `Pango`, shown with the kitty graphics protocol).

/// The list filters: spacing, Nerd Font kind, and role.
pub mod filters;
/// The installed fonts, read through fontconfig.
pub mod installed;
/// Picking fonts for the roles.
pub mod pick;
/// The font presets and fonts.json.
pub mod presets;
/// The sample image of the fonts.
pub mod render;
/// The grouped list of the installed fonts.
pub mod tree;
/// The shared types of the Fonts tab.
pub mod types;

/// The pages of the Fonts area: download, preview, and tournament.
pub mod pages;
/// The Fonts tab: its state, list, presets, and keys.
pub mod tab;
