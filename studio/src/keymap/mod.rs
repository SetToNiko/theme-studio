//! Keymaps: every key-driven action has a name and a context; bindings are
//! keys or sequences in Vim notation (`j`, `<C-r>`, `<leader>sp`). Text
//! fields (hex, search, names) don't go through the keymap.
//!
//! `key.rs` has the key press, `action.rs` the actions, `table.rs` the table
//! with the id, label and group of each action, and `defaults.rs` the keys
//! that ship with the studio. `context.rs` has the contexts, `map.rs` the
//! keymap data, `lookup.rs` the lookup, `json.rs` the loading and saving,
//! and `edit.rs` the changes.

/// The actions that a key can run.
pub mod action;
/// The contexts where a binding applies.
pub mod context;
/// The keys that ship with the studio.
mod defaults;
/// Changing a keymap: set keys, leader, clashes, warnings.
mod edit;
/// Reading and writing keymaps.json.
mod json;
/// One key press in Vim notation.
pub mod key;
/// Lookup of pressed keys, and the leader popup rows.
pub mod lookup;
/// The keymap data and its accessors.
pub mod map;
/// The table with the id, label and group of each action.
mod table;
