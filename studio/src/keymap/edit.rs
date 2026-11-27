//! Changing a keymap: set keys, change the leader, find clashes, and give
//! warnings about the whole keymap.

use ratatui::crossterm::event::KeyModifiers;

use super::action::Action;
use super::context::Context;
use super::key::{Key, display_seq};
use super::map::{Binding, Keymap};

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

impl Keymap {
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
    use ratatui::crossterm::event::KeyCode;

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
