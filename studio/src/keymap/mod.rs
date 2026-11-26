//! Keymaps: every key-driven action has a name and a context; bindings are
//! keys or sequences in Vim notation (`j`, `<C-r>`, `<leader>sp`). Text
//! fields (hex, search, names) don't go through the keymap.
//!
//! `key.rs` has the key press, `action.rs` the actions, `table.rs` the table
//! with the id, label and group of each action, and `defaults.rs` the keys
//! that ship with the studio. This file has the contexts and the lookup,
//! loading and saving of a keymap.

use std::collections::HashSet;

use serde_json::{Map, Value};

use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};

pub mod action;
mod defaults;
pub mod key;
mod table;

use action::{Action, LegacyId, all_actions};
use key::{Key, display_seq};

/// Where a binding applies. Panes look in their own context first, then
/// their column's, then Global.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Context {
    Global,
    /// The colour column (list and picker)
    Colours,
    ColourList,
    Picker,
    /// The Download fonts page.
    Downloads,
    /// The Settings page (Keymaps, Behaviour, History tabs).
    Settings,
    /// Resize mode (`<leader>r`): keys move the focused pane's edges.
    Resize,
    /// The fonts column (roles and list)
    Fonts,
    FontRoles,
    FontList,
}

impl Context {
    pub const ALL: [Context; 10] = [
        Context::Global,
        Context::Colours,
        Context::ColourList,
        Context::Picker,
        Context::Fonts,
        Context::FontRoles,
        Context::FontList,
        Context::Downloads,
        Context::Settings,
        Context::Resize,
    ];

    /// The context's name in keymaps.json.
    pub fn id(self) -> &'static str {
        match self {
            Context::Global => "global",
            Context::Colours => "colours",
            Context::ColourList => "colour-list",
            Context::Picker => "picker",
            Context::Fonts => "fonts",
            Context::FontRoles => "font-roles",
            Context::FontList => "font-list",
            Context::Downloads => "downloads",
            Context::Settings => "settings",
            Context::Resize => "resize",
        }
    }

    pub fn from_id(id: &str) -> Option<Context> {
        Context::ALL.into_iter().find(|c| c.id() == id)
    }

    pub fn title(self) -> &'static str {
        match self {
            Context::Global => "Everywhere",
            Context::Colours => "Colours",
            Context::ColourList => "Colour list",
            Context::Picker => "Colour picker",
            Context::Fonts => "Fonts",
            Context::FontRoles => "Font roles",
            Context::FontList => "Font list",
            Context::Downloads => "Download fonts page",
            Context::Settings => "Settings page",
            Context::Resize => "Resize mode",
        }
    }
}

/// One key sequence that runs one action in one context.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub context: Context,
    pub keys: Vec<Key>,
    pub action: Action,
}

/// The result of looking up the keys pressed so far.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lookup {
    Run(Action),
    /// The keys so far start a longer sequence: wait for more.
    Pending,
    None,
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

/// Whether two contexts are active at the same time (so their keys can clash).
fn active_together(a: Context, b: Context) -> bool {
    use Context as C;
    a == b
        || (a == C::Global && b != C::Resize)
        || (b == C::Global && a != C::Resize)
        || matches!(
            (a, b),
            (C::Colours, C::ColourList | C::Picker)
                | (C::ColourList | C::Picker, C::Colours)
                | (C::Fonts, C::FontRoles | C::FontList)
                | (C::FontRoles | C::FontList, C::Fonts)
        )
}

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

    /// The keys pressed so far, in the given contexts (most specific first).
    pub fn lookup(&self, contexts: &[Context], seq: &[Key]) -> Lookup {
        let active = |b: &&Binding| contexts.contains(&b.context);
        if self
            .bindings
            .iter()
            .filter(active)
            .any(|b| b.keys.len() > seq.len() && b.keys.starts_with(seq))
        {
            return Lookup::Pending;
        }
        for c in contexts {
            if let Some(b) = self
                .bindings
                .iter()
                .find(|b| b.context == *c && b.keys == seq)
            {
                return Lookup::Run(b.action);
            }
        }
        Lookup::None
    }

    /// What can follow `prefix` (for the leader popup): next key, and what
    /// it does (or "…" when it starts a longer sequence).
    pub fn continuations(&self, contexts: &[Context], prefix: &[Key]) -> Vec<(String, String)> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for c in contexts {
            for b in self.bindings.iter().filter(|b| b.context == *c) {
                if b.keys.len() > prefix.len() && b.keys.starts_with(prefix) {
                    let next = b.keys[prefix.len()];
                    if !seen.insert(next) {
                        continue;
                    }
                    let what = if b.keys.len() == prefix.len() + 1 {
                        b.action.describe()
                    } else {
                        "…".to_string()
                    };
                    out.push((next.display(), what));
                }
            }
        }
        // digits, then letters, then named keys: easy to scan
        out.sort_by(|a, b| (a.0.starts_with('<'), &a.0).cmp(&(b.0.starts_with('<'), &b.0)));
        out
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

    /// An action's key sequences in a context.
    pub fn keys_of(&self, ctx: Context, action: Action) -> Vec<Vec<Key>> {
        self.bindings
            .iter()
            .filter(|b| b.context == ctx && b.action == action)
            .map(|b| b.keys.clone())
            .collect()
    }

    /// Replaces an action's keys in a context (none: deliberately unbound).
    pub fn set_keys(&mut self, ctx: Context, action: Action, seqs: Vec<Vec<Key>>) {
        self.bindings
            .retain(|b| !(b.context == ctx && b.action == action));
        if seqs.is_empty() {
            self.unbound.insert((ctx, action));
        } else {
            self.unbound.remove(&(ctx, action));
        }
        for keys in seqs {
            self.bindings.push(Binding {
                context: ctx,
                keys,
                action,
            });
        }
    }

    /// The same keymap with another leader: sequences written with
    /// `<leader>` follow it.
    pub fn with_leader(&self, leader: Key) -> Keymap {
        let mut km = self.clone();
        km.leader = leader;
        for b in &mut km.bindings {
            if let Ok(keys) = Key::parse_seq(&display_seq(&b.keys, self.leader), leader) {
                b.keys = keys;
            }
        }
        km
    }

    /// Bindings that would clash with `keys` for `action` in `ctx`: the same
    /// keys for another action in the same context, or (in contexts that are
    /// active together) one sequence starting the other, which can't work
    /// without a timeout.
    pub fn conflicts(&self, ctx: Context, action: Action, keys: &[Key]) -> Vec<String> {
        let mut out = Vec::new();
        for b in self
            .bindings
            .iter()
            .filter(|b| !(b.context == ctx && b.action == action))
        {
            let same = b.context == ctx && b.keys == keys;
            let prefix = active_together(b.context, ctx)
                && b.keys != keys
                && (b.keys.starts_with(keys) || keys.starts_with(&b.keys));
            if same || prefix {
                out.push(format!(
                    "{} {} {} ({})",
                    display_seq(&b.keys, self.leader),
                    if same { "is also" } else { "overlaps" },
                    b.action.describe(),
                    b.context.title()
                ));
            }
        }
        out
    }

    /// Things to know about the whole keymap.
    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self
            .keys_of(Context::Global, Action::PreviewCycle)
            .is_empty()
        {
            out.push("nothing gets you out of a preview window (preview-cycle has no key)".into());
        }
        if self
            .bindings
            .iter()
            .any(|b| b.keys.iter().any(|k| k.mods.contains(KeyModifiers::ALT)))
        {
            out.push("Alt is i3's $mod: i3 takes Alt+key combinations first".into());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEvent;

    fn space() -> Key {
        Key::new(KeyCode::Char(' '), KeyModifiers::NONE)
    }

    #[test]
    fn parses_and_displays_vim_notation() {
        let l = space();
        let k = |s| Key::parse_seq(s, l).unwrap();
        assert_eq!(k("gg").len(), 2);
        assert_eq!(k("<leader>cp")[0], l);
        assert_eq!(
            k("<C-r>")[0],
            Key::new(KeyCode::Char('r'), KeyModifiers::CONTROL)
        );
        assert_eq!(
            k("<S-Tab>")[0],
            Key::new(KeyCode::BackTab, KeyModifiers::NONE)
        );
        assert_eq!(k("<F12>")[0], Key::new(KeyCode::F(12), KeyModifiers::NONE));
        assert_eq!(k("<C-x><C-s>").len(), 2);
        assert_eq!(k("<C-S-z>"), k("<C-Z>"));
        for s in [
            "j", "<C-r>", "<S-Tab>", "<F5>", "<CR>", "<Esc>", "<S-Up>", "<M-x>", "<C-Z>",
            "<Space>", "#",
        ] {
            assert_eq!(
                display_seq(&k(s), Key::new(KeyCode::Char('\\'), KeyModifiers::NONE)),
                s,
                "round trip"
            );
        }
        assert_eq!(display_seq(&k("<leader>sp"), l), "<leader>sp");
        assert!(Key::parse_seq("<nope>", l).is_err());
    }

    #[test]
    fn key_events_are_normalised() {
        let e = |c, m| Key::from_event(KeyEvent::new(c, m));
        assert_eq!(
            e(KeyCode::Char('J'), KeyModifiers::SHIFT),
            Key::new(KeyCode::Char('J'), KeyModifiers::NONE)
        );
        assert_eq!(
            e(KeyCode::BackTab, KeyModifiers::SHIFT),
            Key::new(KeyCode::BackTab, KeyModifiers::NONE)
        );
        assert_eq!(
            e(KeyCode::Up, KeyModifiers::SHIFT),
            Key::new(KeyCode::Up, KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn lookup_waits_for_sequences_and_prefers_specific_contexts() {
        let km = Keymap::vim();
        let l = space();
        let seq = |s: &str| Key::parse_seq(s, l).unwrap();
        let list = [Context::FontList, Context::Fonts, Context::Global];
        let roles = [Context::FontRoles, Context::Fonts, Context::Global];
        // n: a filter in the font list, "new" elsewhere
        assert_eq!(
            km.lookup(&list, &seq("n")),
            Lookup::Run(Action::Filter('n'))
        );
        assert_eq!(km.lookup(&roles, &seq("n")), Lookup::Run(Action::New));
        // the leader waits, then runs the sequence
        assert_eq!(km.lookup(&roles, &seq(" ")), Lookup::Pending);
        assert_eq!(km.lookup(&roles, &seq(" s")), Lookup::Pending);
        assert_eq!(
            km.lookup(&roles, &seq(" sf")),
            Lookup::Run(Action::SaveFonts)
        );
        assert_eq!(km.lookup(&roles, &seq(" z")), Lookup::None);
        let next: Vec<String> = km
            .continuations(&roles, &seq(" "))
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(
            next,
            [
                "1", "2", "3", "4", "5", "6", "7", "a", "f", "n", "o", "r", "s", "t", "u"
            ]
        );
        assert_eq!(
            km.key_for(&roles, Action::SaveFonts).as_deref(),
            Some("<leader>sf")
        );
    }

    /// No key in a pane both runs something and starts a longer sequence
    /// (there's no timeout to tell them apart).
    #[test]
    fn vim_preset_has_no_ambiguous_prefixes() {
        let km = Keymap::vim();
        let stacks: [&[Context]; 4] = [
            &[Context::ColourList, Context::Colours, Context::Global],
            &[Context::Picker, Context::Colours, Context::Global],
            &[Context::FontRoles, Context::Fonts, Context::Global],
            &[Context::FontList, Context::Fonts, Context::Global],
        ];
        for stack in stacks {
            let active: Vec<&Binding> = km
                .bindings
                .iter()
                .filter(|b| stack.contains(&b.context))
                .collect();
            for a in &active {
                for b in &active {
                    assert!(
                        !(b.keys.len() > a.keys.len() && b.keys.starts_with(&a.keys)),
                        "{:?} {:?} is a prefix of {:?} {:?}",
                        a.context,
                        a.action,
                        b.context,
                        b.action
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod edit_tests {
    use super::*;

    // Long function (reason verified, phase 6): one scenario; each step changes the same keymap and the next assert depends on it.
    #[test]
    fn set_keys_leader_change_and_conflicts() {
        let mut km = Keymap::vim();
        let l = km.leader;
        let seq = |s: &str, l: Key| Key::parse_seq(s, l).unwrap();
        km.set_keys(Context::Global, Action::Quit, vec![seq("Q", l)]);
        assert_eq!(km.keys_of(Context::Global, Action::Quit), vec![seq("Q", l)]);
        km.set_keys(Context::Colours, Action::Undo, vec![]);
        assert!(km.unbound.contains(&(Context::Colours, Action::Undo)));
        // a new leader moves every <leader> sequence with it
        let comma = Key::new(KeyCode::Char(','), KeyModifiers::NONE);
        let km2 = km.with_leader(comma);
        assert_eq!(
            km2.keys_of(Context::Global, Action::SavePalette),
            vec![seq(",sp", comma)]
        );
        assert_eq!(
            km2.keys_of(Context::Global, Action::Quit),
            vec![seq("Q", comma)]
        );
        // y is copy-hex in Colours; binding pin to y clashes, binding it to Space overlaps the leader
        assert_eq!(
            km.conflicts(Context::Colours, Action::Pin, &seq("y", l))
                .len(),
            1
        );
        assert_ne!(
            km.conflicts(Context::Colours, Action::Pin, &seq(" ", l))
                .len(),
            0
        );
        // the font list's n deliberately hides the global n: not a conflict
        assert_eq!(
            km.conflicts(Context::FontList, Action::Filter('n'), &seq("n", l))
                .len(),
            0
        );
        assert_eq!(km.warnings().len(), 0);
        km.set_keys(Context::Global, Action::PreviewCycle, vec![]);
        km.set_keys(Context::Global, Action::Help, vec![seq("<M-h>", l)]);
        assert_eq!(km.warnings().len(), 2);
    }
}

#[cfg(test)]
mod preset_tests {
    use super::*;
    use ratatui::crossterm::event::KeyEvent;

    #[test]
    fn shipped_presets_are_clash_free_and_round_trip() {
        let all = Keymap::builtin();
        let names: Vec<&str> = all.iter().map(|k| k.name.as_str()).collect();
        assert_eq!(names, ["Vim", "Helix", "Emacs", "VS Code", "JetBrains"]);
        for km in &all {
            for b in &km.bindings {
                assert!(
                    km.conflicts(b.context, b.action, &b.keys).is_empty(),
                    "{}: {} {:?}: {:?}",
                    km.name,
                    display_seq(&b.keys, km.leader),
                    b.action,
                    km.conflicts(b.context, b.action, &b.keys)
                );
                // no Alt (i3), no keys Ghostty keeps for itself
                assert!(
                    !b.keys.iter().any(|k| k.mods.contains(KeyModifiers::ALT)),
                    "{}",
                    km.name
                );
            }
            assert!(km.warnings().is_empty(), "{}: {:?}", km.name, km.warnings());
            // every action the Vim preset has keys for has keys here too
            for b in &Keymap::vim().bindings {
                assert!(
                    !km.keys_of(b.context, b.action).is_empty(),
                    "{}: {:?} in {:?} has no keys",
                    km.name,
                    b.action,
                    b.context
                );
            }
            let (back, w) = Keymap::from_json(&km.name, &km.to_json(), Some(km));
            assert!(w.is_empty(), "{w:?}");
            assert_eq!(back.to_json(), km.to_json());
        }
        let key = |km: &Keymap, s: &str| Key::parse_seq(s, km.leader).unwrap();
        let emacs = Keymap::emacs();
        assert_eq!(
            emacs.leader,
            Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        );
        assert_eq!(
            emacs.lookup(
                &[Context::ColourList, Context::Colours, Context::Global],
                &key(&emacs, "<C-n>")
            ),
            Lookup::Run(Action::Down)
        );
        assert!(
            emacs
                .keys_of(Context::ColourList, Action::Down)
                .iter()
                .all(|k| k[0].code != KeyCode::Char('j'))
        );
        assert_eq!(
            emacs.lookup(&[Context::Global], &key(&emacs, "<C-c>sp")),
            Lookup::Run(Action::SavePalette),
            "leader sequences follow the leader"
        );
        let vs = Keymap::vscode();
        assert_eq!(
            vs.lookup(&[Context::Global], &key(&vs, "<C-S-s>")),
            Lookup::Run(Action::SaveAs)
        );
        assert_eq!(
            vs.lookup(&[Context::Global], &key(&vs, "<C-3>")),
            Lookup::Run(Action::Panel(3))
        );
        let hx = Keymap::helix();
        assert_eq!(
            hx.lookup(&[Context::Colours], &key(&hx, "U")),
            Lookup::Run(Action::Redo)
        );
        assert_eq!(
            hx.lookup(&[Context::FontList], &key(&hx, "ge")),
            Lookup::Run(Action::Bottom)
        );
    }

    #[test]
    fn kitty_protocol_keys_match_their_notation() {
        let l = Key::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let ev = |c, m| Key::from_event(KeyEvent::new(c, m));
        let cs = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
        // kitty: s + Ctrl + Shift; legacy-style: S + Ctrl + Shift
        assert_eq!(
            vec![ev(KeyCode::Char('s'), cs)],
            Key::parse_seq("<C-S-s>", l).unwrap()
        );
        assert_eq!(
            vec![ev(KeyCode::Char('S'), cs)],
            Key::parse_seq("<C-S-s>", l).unwrap()
        );
        assert_eq!(
            vec![ev(KeyCode::Char('1'), KeyModifiers::CONTROL)],
            Key::parse_seq("<C-1>", l).unwrap()
        );
        assert_eq!(
            vec![ev(KeyCode::Char('A'), KeyModifiers::SHIFT)],
            Key::parse_seq("A", l).unwrap()
        );
    }
}

#[cfg(test)]
mod leader_display_tests {
    use super::*;

    #[test]
    fn only_a_leading_leader_shows_as_leader() {
        let cc = Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let k = |s| Key::parse_seq(s, cc).unwrap();
        assert_eq!(display_seq(&k("<C-x><C-c>"), cc), "<C-x><C-c>");
        assert_eq!(display_seq(&k("<leader><C-c>"), cc), "<leader><C-c>");
        assert_eq!(display_seq(&k("<leader>sp"), cc), "<leader>sp");
    }
}
