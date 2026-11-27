//! The keymap data: `Binding`, the `Keymap` struct, and its plain accessors.
//!
//! `lookup.rs` turns pressed keys into actions, `json.rs` reads and writes
//! keymaps.json, and `edit.rs` changes a keymap.

use std::collections::HashSet;

use anyhow::Result;

use super::action::Action;
use super::context::Context;
use super::key::{Key, display_seq};

/// One key sequence that runs one action in one context.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub context: Context,
    pub keys: Vec<Key>,
    pub action: Action,
}

/// A keymap preset: a leader key and a list of bindings. `lookup` turns the
/// keys pressed so far into an action.
#[derive(Clone, Debug)]
pub struct Keymap {
    /// (context, action) pairs deliberately left without keys.
    pub unbound: HashSet<(Context, Action)>,
    pub name: String,
    pub leader: Key,
    pub bindings: Vec<Binding>,
}

impl Keymap {
    /// Builds a keymap from (context, action, sequences) rows.
    ///
    /// # Errors
    /// Fails when one key sequence cannot be parsed (see `Key::parse_seq`).
    pub fn from_rows(
        name: &str,
        leader: Key,
        rows: &[(Context, Action, &[&str])],
    ) -> Result<Keymap> {
        let mut bindings = Vec::new();
        for (context, action, seqs) in rows {
            for s in *seqs {
                bindings.push(Binding {
                    context: *context,
                    keys: Key::parse_seq(s, leader)?,
                    action: *action,
                });
            }
        }
        Ok(Keymap {
            name: name.to_string(),
            leader,
            bindings,
            unbound: HashSet::new(),
        })
    }

    /// The first sequence bound to `action` in one of the contexts, as text.
    pub fn key_for(&self, contexts: &[Context], action: Action) -> Option<String> {
        contexts.iter().find_map(|c| {
            self.bindings
                .iter()
                .find(|b| b.context == *c && b.action == action)
                .map(|b| display_seq(&b.keys, self.leader))
        })
    }

    /// An action's key sequences in a context.
    pub fn keys_of(&self, ctx: Context, action: Action) -> Vec<Vec<Key>> {
        self.bindings
            .iter()
            .filter(|b| b.context == ctx && b.action == action)
            .map(|b| b.keys.clone())
            .collect()
    }
}
