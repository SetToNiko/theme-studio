//! App's methods for the Settings page's Keymaps tab: the key list rows, its
//! keys, setting an action's keys, and recording new keys.
//!
//! This is another `impl App { ... }` block (see `settings_actions.rs`). The
//! row type `KmRow` and the page data stay in `settings.rs`; this file only
//! holds the methods that need the whole `App`.

use super::App;
use super::nav::{Capture, Deferred, Naming, Overlay};
use crate::keymap::{Action, Context, Key, Keymap, all_actions, display_seq};
use crate::settings::{KmRow, move_over};

impl App {
    /// Keymaps tab rows: the preset, then each context's actions (those it
    /// has keys for, in this preset or the shipped one, and unbound ones).
    pub fn keymap_rows(&self) -> Vec<KmRow> {
        let shipped = Keymap::vim();
        let mut rows = vec![KmRow::Preset];
        for ctx in Context::ALL {
            let actions: Vec<Action> = all_actions()
                .into_iter()
                .filter(|a| {
                    !self.keymap.keys_of(ctx, *a).is_empty()
                        || !shipped.keys_of(ctx, *a).is_empty()
                        || self.keymap.unbound.contains(&(ctx, *a))
                })
                .collect();
            if actions.is_empty() {
                continue;
            }
            // a blank line before each group's title; closed groups show
            // only the title
            rows.push(KmRow::Gap);
            rows.push(KmRow::Header(ctx));
            if !self.settings_page.km_closed.contains(&ctx) {
                rows.extend(actions.into_iter().map(|a| KmRow::Binding(ctx, a)));
            }
        }
        rows
    }

    /// The cursor can rest on everything but the blank lines.
    pub fn keymap_selectable(rows: &[KmRow]) -> Vec<bool> {
        rows.iter().map(|r| !matches!(r, KmRow::Gap)).collect()
    }

    pub(crate) fn move_sel(sel: &mut usize, n: usize, a: Action) -> bool {
        let last = n.saturating_sub(1) as i32;
        let d: i32 = match a {
            Action::Up => -1,
            Action::Down => 1,
            Action::PageUp => -10,
            Action::PageDown => 10,
            Action::Top => -(*sel as i32),
            Action::Bottom => last - *sel as i32,
            _ => return false,
        };
        *sel = (*sel as i32 + d).clamp(0, last) as usize;
        true
    }

    pub(crate) fn run_keymaps_tab(&mut self, a: Action) {
        if self.settings_page.browse.is_some() {
            return self.run_browse(a);
        }
        let rows = self.keymap_rows();
        if move_over(
            &mut self.settings_page.km_sel,
            &Self::keymap_selectable(&rows),
            a,
        ) {
            return;
        }
        let row = rows.get(self.settings_page.km_sel).copied();
        let builtin = Keymap::builtin().iter().any(|b| b.name == self.keymap.name);
        match (row, a) {
            // the preset button opens the preset list
            (Some(KmRow::Preset), Action::Right | Action::Confirm) => self.open_browse(),
            // a group's title: Enter toggles it, right opens, left closes
            (Some(KmRow::Header(ctx)), Action::Confirm | Action::Right | Action::Left) => {
                let closed = &mut self.settings_page.km_closed;
                let open = !closed.contains(&ctx);
                if (a == Action::Confirm && open) || a == Action::Left {
                    closed.insert(ctx);
                } else {
                    closed.remove(&ctx);
                }
            }
            (_, Action::SaveNamed) => {
                self.prompt("Save the keymap as a new preset", Naming::Preset, "")
            }
            (_, Action::ResetPreset) if builtin => self.ask(
                format!("Reset \"{}\" to the shipped keys?", self.keymap.name),
                Deferred::ResetPreset(self.keymap.name.clone()),
            ),
            (_, Action::ResetPreset) => self
                .messages
                .info("Only shipped presets can be reset; this one is yours"),
            (Some(KmRow::Binding(ctx, action)), Action::Confirm | Action::Right) => {
                self.overlay = Overlay::Capture {
                    what: Capture::Binding {
                        ctx,
                        action,
                        add: false,
                    },
                    keys: Vec::new(),
                }
            }
            (Some(KmRow::Binding(ctx, action)), Action::Add) => {
                self.overlay = Overlay::Capture {
                    what: Capture::Binding {
                        ctx,
                        action,
                        add: true,
                    },
                    keys: Vec::new(),
                }
            }
            (Some(KmRow::Binding(ctx, action)), Action::EditText) => {
                let current: Vec<String> = self
                    .keymap
                    .keys_of(ctx, action)
                    .iter()
                    .map(|k| display_seq(k, self.keymap.leader))
                    .collect();
                self.prompt(
                    &format!(
                        "Keys for \"{}\" (Vim notation; space between several)",
                        action.describe()
                    ),
                    Naming::BindingText {
                        ctx,
                        action,
                        add: false,
                    },
                    &current.join(" "),
                )
            }
            (Some(KmRow::Binding(ctx, action)), Action::Delete) => {
                self.keymap.set_keys(ctx, action, Vec::new());
                self.sync_preset();
                self.messages.info(format!(
                    "\"{}\" has no keys now ({})",
                    action.describe(),
                    ctx.title()
                ));
            }
            _ => {}
        }
    }

    /// Sets an action's keys from the capture or the text prompt.
    pub fn bind(&mut self, ctx: Context, action: Action, keys: Vec<Vec<Key>>, add: bool) {
        let mut all = if add {
            self.keymap.keys_of(ctx, action)
        } else {
            Vec::new()
        };
        let clashes: Vec<String> = keys
            .iter()
            .flat_map(|k| self.keymap.conflicts(ctx, action, k))
            .collect();
        all.extend(keys);
        self.keymap.set_keys(ctx, action, all);
        self.sync_preset();
        if clashes.is_empty() {
            self.messages
                .info(format!("New keys for \"{}\" (unsaved)", action.describe()));
        } else {
            self.messages
                .error(format!("New keys (unsaved), but: {}", clashes.join("; ")));
        }
    }

    /// Keys while recording new keys (Enter finishes, Esc cancels,
    /// Backspace takes the last one back).
    pub fn capture_key(
        &mut self,
        what: Capture,
        mut keys: Vec<Key>,
        k: ratatui::crossterm::event::KeyEvent,
    ) {
        use ratatui::crossterm::event::KeyCode;
        let key = Key::from_event(k);
        match k.code {
            KeyCode::Esc => self.overlay = Overlay::None,
            KeyCode::Enter if !keys.is_empty() => {
                self.overlay = Overlay::None;
                match what {
                    Capture::Binding { ctx, action, add } => {
                        self.bind(ctx, action, vec![keys], add)
                    }
                    Capture::Leader => {}
                }
            }
            KeyCode::Backspace if !keys.is_empty() => {
                keys.pop();
                self.overlay = Overlay::Capture { what, keys };
            }
            _ => match what {
                Capture::Leader => {
                    self.overlay = Overlay::None;
                    self.keymap = self.keymap.with_leader(key);
                    self.sync_preset();
                    self.messages
                        .info(format!("Leader is now {}", key.display()));
                }
                _ => {
                    keys.push(key);
                    self.overlay = Overlay::Capture { what, keys };
                }
            },
        }
    }
}
