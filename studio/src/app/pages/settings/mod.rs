//! The Settings page of `App`.
//!
//! - `settings_actions.rs`: the page as a whole.
//! - `settings_keymaps_actions.rs`: the keymap tab (capture and edit bindings).
//! - `settings_history_actions.rs`: the history tab.
//! - `draft_actions.rs`: Settings drafts (changes not saved yet).
//! - `behaviour_actions.rs`: the Behaviour tab keys and value changes.
//! - `behaviour_colours_actions.rs`: looks and colour rows of that tab.
//! - `behaviour_labels_actions.rs`: row values and labels of that tab.

/// Behaviour tab keys and value changes.
mod behaviour_actions;
/// Look and colour rows of the Behaviour tab.
mod behaviour_colours_actions;
/// Row values and labels of the Behaviour tab.
mod behaviour_labels_actions;
/// Settings drafts, not saved yet.
mod draft_actions;
/// The Settings page as a whole.
mod settings_actions;
/// The history tab.
mod settings_history_actions;
/// The keymap tab.
mod settings_keymaps_actions;
