//! The Fonts tab (`FontsTab`) split by purpose: state, list, presets, and keys.

/// The keys, the clicks on the roles, and the actions of the tab.
pub mod keys;
/// The font list: rows, groups, and picking.
pub mod list;
/// Open, new, and save of the presets.
pub mod presets;
/// The struct `FontsTab`, its construction, and the checks of the role fonts.
pub mod state;
