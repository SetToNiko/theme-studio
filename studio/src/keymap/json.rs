//! Reading a keymap from keymaps.json and writing it back.
//!
//! A file can name unknown contexts, unknown actions, or keys that do not
//! parse. The reader skips them and returns a warning for each.

use std::collections::HashSet;

use serde_json::{Map, Value};

use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use super::action::{Action, LegacyId, all_actions};
use super::context::Context;
use super::key::{Key, display_seq};
use super::map::{Binding, Keymap};

/// The leader key of a preset: the file's `leader`, else the base preset's,
/// else Space. A leader that is not one key is reported and replaced by Space.
fn read_leader(name: &str, v: &Value, base: Option<&Keymap>, warnings: &mut Vec<String>) -> Key {
    let space = Key::new(KeyCode::Char(' '), KeyModifiers::NONE);
    match v.get("leader").and_then(Value::as_str) {
        Some(s) => match Key::parse_seq(s, space) {
            Ok(k) if k.len() == 1 => k[0],
            _ => {
                warnings.push(format!("{name}: leader \"{s}\" isn't one key; using Space"));
                space
            }
        },
        None => base.map_or(space, |b| b.leader),
    }
}

/// The action named `aid` in a preset file. A renamed action gives its new
/// name; an action that no longer exists, or an unknown one, gives `None`
/// (the unknown one with a warning).
fn resolve_action(name: &str, aid: &str, warnings: &mut Vec<String>) -> Option<Action> {
    match (Action::from_id(aid), Action::legacy(aid)) {
        (Some(a), _) => Some(a),
        (None, Some(LegacyId::Renamed(renamed))) => Some(renamed),
        (None, Some(LegacyId::Dropped)) => None, // no longer exists
        (None, None) => {
            warnings.push(format!("{name}: unknown action \"{aid}\""));
            None
        }
    }
}

impl Keymap {
    /// The preset as stored in keymaps.json:
    /// `{ "leader": "<Space>", "bindings": { context: { action: [keys…] } } }`.
    /// Actions with no keys in a context are left out of that context.
    pub fn to_json(&self) -> Value {
        let mut contexts = Map::new();
        for ctx in Context::ALL {
            let mut actions = Map::new();
            for a in all_actions() {
                let seqs: Vec<Value> = self
                    .bindings
                    .iter()
                    .filter(|b| b.context == ctx && b.action == a)
                    .map(|b| Value::String(display_seq(&b.keys, self.leader)))
                    .collect();
                if !seqs.is_empty() || self.unbound.contains(&(ctx, a)) {
                    actions.insert(a.id(), Value::Array(seqs));
                }
            }
            if !actions.is_empty() {
                contexts.insert(ctx.id().to_string(), Value::Object(actions));
            }
        }
        let mut m = Map::new();
        m.insert("leader".into(), Value::String(self.leader.display()));
        m.insert("bindings".into(), Value::Object(contexts));
        Value::Object(m)
    }

    /// Reads a preset from keymaps.json. Problems (unknown names, keys that
    /// don't parse) are skipped and reported. With `base`, actions the file
    /// doesn't mention for a context keep `base`'s keys, so actions added in
    /// a newer version show up; an empty list means deliberately unbound.
    pub fn from_json(name: &str, v: &Value, base: Option<&Keymap>) -> (Keymap, Vec<String>) {
        let mut warnings = Vec::new();
        let leader = read_leader(name, v, base, &mut warnings);
        let mut km = Keymap {
            name: name.to_string(),
            leader,
            bindings: Vec::new(),
            unbound: HashSet::new(),
        };
        let mut mentioned: HashSet<(Context, Action)> = HashSet::new();
        if let Some(contexts) = v.get("bindings").and_then(Value::as_object) {
            km.read_contexts(name, contexts, &mut mentioned, &mut warnings);
        }
        if let Some(base) = base {
            km.add_missing_from(base, &mentioned);
        }
        (km, warnings)
    }

    /// Reads the `bindings` object of a preset: one entry for each context,
    /// each with its actions. Fills `mentioned` with every action the file names.
    fn read_contexts(
        &mut self,
        name: &str,
        contexts: &Map<String, Value>,
        mentioned: &mut HashSet<(Context, Action)>,
        warnings: &mut Vec<String>,
    ) {
        for (cid, actions) in contexts {
            let Some(ctx) = Context::from_id(cid) else {
                warnings.push(format!("{name}: unknown context \"{cid}\""));
                continue;
            };
            let Some(actions) = actions.as_object() else {
                continue;
            };
            self.read_actions(name, (cid, ctx), actions, mentioned, warnings);
        }
    }

    /// Reads the actions of one context: `context` is its id in the file and
    /// the context itself. Problems are reported in `warnings`.
    fn read_actions(
        &mut self,
        name: &str,
        context: (&str, Context),
        actions: &Map<String, Value>,
        mentioned: &mut HashSet<(Context, Action)>,
        warnings: &mut Vec<String>,
    ) {
        let (cid, ctx) = context;
        for (aid, seqs) in actions {
            let Some(action) = resolve_action(name, aid, warnings) else {
                continue;
            };
            mentioned.insert((ctx, action));
            let list = seqs.as_array().cloned().unwrap_or_default();
            if list.is_empty() {
                self.unbound.insert((ctx, action));
            }
            for s in list.iter().filter_map(Value::as_str) {
                match Key::parse_seq(s, self.leader) {
                    Ok(keys) => self.bindings.push(Binding {
                        context: ctx,
                        keys,
                        action,
                    }),
                    Err(e) => warnings.push(format!("{name}: {cid}.{aid}: {e}")),
                }
            }
        }
    }

    /// Copies the bindings of `base` for the actions that the file did not
    /// mention, re-read with this preset's leader.
    fn add_missing_from(&mut self, base: &Keymap, mentioned: &HashSet<(Context, Action)>) {
        for b in &base.bindings {
            if !mentioned.contains(&(b.context, b.action)) {
                let text = display_seq(&b.keys, base.leader);
                if let Ok(keys) = Key::parse_seq(&text, self.leader) {
                    self.bindings.push(Binding {
                        context: b.context,
                        keys,
                        action: b.action,
                    });
                }
            }
        }
    }
}
