//! Theme Studio: edit themes (a palette from palettes.json plus a font preset
//! from fonts.json) with a live preview of the desktop, and download fonts.
//! See docs/theme-studio.md.

/// The application state, the input handling, and the pages.
mod app;
/// The command line: argument handling, `--init-config`, and the start.
pub mod cli;
/// The loading and writing of the settings files.
mod config;
/// The font tab, the font pages, and the font presets.
mod fonts;
/// The key bindings and the keymap presets.
mod keymap;
/// The data types, the stores, and the paths.
mod model;
/// The terminal panes of the preview.
mod preview_term;
/// The state of the Settings page.
mod settings_page;
/// The access to the system: downloads, the clipboard, and the output.
mod system;
/// The drawing of the screen.
mod ui;
