//! The Settings page (F3): Keymaps, Behaviour, History and README tabs.
//! Keymaps and Behaviour changes wait in a draft until Save (see `draft.rs`);
//! History acts at once. Every write goes through the history, so it can be
//! undone.
//!
//! Files in this folder:
//! - `state.rs`: the page state (`SettingsPage`), the tabs and the row kinds.
//! - `diff.rs`: the difference view of the History tab (`DiffLine`, `DiffView`).
//! - `rows.rs`: cursor moves over rows (`move_over`, `ends`, `wrap`).
//! - `behaviour.rs`: the Behaviour tab (settings.json values in groups).
//! - `draft.rs`: the unsaved changes (`Draft`) of the Keymaps and Behaviour tabs.
//! - `keymap_presets.rs`: the preset browser of the Keymaps tab.
//! - `readme.rs`: the README tab (Markdown rendered to styled lines).
//! - `readme_layout.rs`: word wrapping and table layout for the README tab.

/// The Behaviour tab: settings.json values in groups.
pub mod behaviour;
/// The difference view of the History tab.
pub mod diff;
/// The unsaved changes of the Keymaps and Behaviour tabs.
pub mod draft;
/// The preset browser of the Keymaps tab.
pub mod keymap_presets;
/// The README tab: Markdown rendered to styled lines.
pub mod readme;
/// Word wrapping and table layout for the README tab.
pub mod readme_layout;
/// Cursor moves over the rows of a tab.
pub mod rows;
/// The page state, the tabs and the row kinds.
pub mod state;
