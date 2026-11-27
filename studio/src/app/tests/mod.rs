//! Tests for the app, one file per area. The shared helpers are in `app/test_support.rs`.
//!
//! - `palette.rs`: palette editing (picker, pinning, derived colours, palette list)
//! - `layout.rs`: window sizes, panes, borders, number keys, status line, toggles
//! - `input.rs`: leader keys, mouse hover and clicks, hover looks
//! - `pages_settings.rs`: the Settings page (tabs, behaviour rows, keymap presets, guide)
//! - `pages_fonts.rs`: the Fonts page (roles, families, presets, sizes)
//! - `font_downloads.rs`: font downloads, installed-font previews, prefetching, the tournament
//! - `lifecycle.rs`: saving, opening and forking themes, quitting

/// Font downloads, installed-font previews, prefetching, the tournament.
mod font_downloads;
/// Leader keys, mouse hover and clicks, hover looks.
mod input;
/// Window sizes, panes, borders, number keys, status line, toggles.
mod layout;
/// Saving, opening and forking themes, quitting.
mod lifecycle;
/// The Fonts page.
mod pages_fonts;
/// The Settings page.
mod pages_settings;
/// Palette editing.
mod palette;
