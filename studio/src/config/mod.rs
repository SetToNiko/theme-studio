//! The studio's own settings: `$XDG_CONFIG_HOME/theme-studio/` (default
//! `~/.config/theme-studio/`) with keymaps.json and settings.json. Missing
//! files are created and missing entries filled in from the defaults, so
//! things added in newer versions show up while your changes are kept.

/// Reads and writes the JSON files of the config folder.
mod files;
/// The keymaps.json file: defaults, presets, and loading.
pub(crate) mod keymaps_file;
/// Loads the config folder: keymaps.json and settings.json together.
pub(crate) mod load;
/// The upgrade of an older keymaps.json to the current format.
mod migrate;
/// The settings data: the structs saved in settings.json and their defaults.
pub mod settings;
/// The settings.json file: reading, topping up, and saving.
pub(crate) mod settings_file;
