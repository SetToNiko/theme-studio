//! Lookup: turns the keys pressed so far into an action, and lists what can
//! follow a key prefix for the leader popup.

use std::collections::HashSet;

use super::action::Action;
use super::context::Context;
use super::key::Key;
use super::map::{Binding, Keymap};

/// The result of looking up the keys pressed so far.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lookup {
    Run(Action),
    /// The keys so far start a longer sequence: wait for more.
    Pending,
    None,
}

impl Keymap {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    fn space() -> Key {
        Key::new(KeyCode::Char(' '), KeyModifiers::NONE)
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
