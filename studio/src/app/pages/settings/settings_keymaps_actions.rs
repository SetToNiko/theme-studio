//! App's methods for the Settings page's Keymaps tab: the key list rows, its
//! keys, setting an action's keys, and recording new keys.
//!
//! This is another `impl App { ... }` block (see `settings_actions.rs`). The
//! row type `KmRow` and the page data stay in `settings_page/mod.rs`; this file only
//! holds the methods that need the whole `App`.

use crate::app::app_state::App;
use crate::app::state::nav::{Capture, Deferred, Naming, Overlay};
use crate::keymap::action::{Action, all_actions};
use crate::keymap::key::{Key, display_seq};
use crate::keymap::{context::Context, map::Keymap};
use crate::settings_page::rows::move_over;
use crate::settings_page::state::KmRow;

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

    // Long function (reason verified, phase 6): one match over the Keymaps tab's row kinds and keys.
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
                self.prompt("Save the keymap as a new preset", Naming::Preset, "");
            }
            (_, Action::ResetPreset) if builtin => self.ask(
                format!("Reset \"{}\" to the shipped keys?", self.keymap.name),
                Deferred::ResetPreset(self.keymap.name.clone()),
            ),
            (_, Action::ResetPreset) => self
                .messages
                .info("Only shipped presets can be reset; this one is yours"),
            (Some(KmRow::Binding(ctx, action)), Action::Confirm | Action::Right) => {
                self.start_capture(ctx, action, false);
            }
            (Some(KmRow::Binding(ctx, action)), Action::Add) => {
                self.start_capture(ctx, action, true);
            }
            (Some(KmRow::Binding(ctx, action)), Action::EditText) => {
                self.edit_binding_text(ctx, action);
            }
            (Some(KmRow::Binding(ctx, action)), Action::Delete) => self.unbind(ctx, action),
            _ => {}
        }
    }

    /// Opens the key-recording pop-up for an action (`add`: keep its old keys).
    fn start_capture(&mut self, ctx: Context, action: Action, add: bool) {
        self.overlay = Overlay::Capture {
            what: Capture::Binding { ctx, action, add },
            keys: Vec::new(),
        };
    }

    /// Opens the text prompt for an action's keys, filled with its current keys.
    fn edit_binding_text(&mut self, ctx: Context, action: Action) {
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
        );
    }

    /// Removes all keys of an action (it stays in the list, unbound).
    fn unbind(&mut self, ctx: Context, action: Action) {
        self.keymap.set_keys(ctx, action, Vec::new());
        self.sync_preset();
        self.messages.info(format!(
            "\"{}\" has no keys now ({})",
            action.describe(),
            ctx.title()
        ));
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
                        self.bind(ctx, action, vec![keys], add);
                    }
                    Capture::Leader => {}
                }
            }
            KeyCode::Backspace if !keys.is_empty() => {
                keys.pop();
                self.overlay = Overlay::Capture { what, keys };
            }
            _ => {
                if matches!(what, Capture::Leader) {
                    self.overlay = Overlay::None;
                    self.keymap = self.keymap.with_leader(key);
                    self.sync_preset();
                    self.messages
                        .info(format!("Leader is now {}", key.display()));
                } else {
                    keys.push(key);
                    self.overlay = Overlay::Capture { what, keys };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::app_state::App;
    use crate::app::startup::Start;
    use crate::app::state::nav::Pending;
    use crate::app::state::nav::{Overlay, Page};
    use crate::app::test_support::*;
    use crate::keymap::action::Action;
    use crate::keymap::context::Context;
    use crate::keymap::key::Key;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

    #[test]
    fn settings_changes_wait_for_save() {
        use crate::settings_page::state::KmRow;
        let store = temp_store("settings-keys");
        let dir = store.dir.clone();
        crate::model::history::init_for_test(dir.join("history"));
        let mut app = test_app(store, Start::Named("One".into()));
        app.on_key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
        assert_eq!(app.page, Page::Settings);
        assert_eq!(app.contexts()[0], Context::Settings);
        let row_of = |app: &mut App, r: KmRow| {
            app.show_draft(|a| a.keymap_rows().iter().position(|x| *x == r).unwrap())
        };
        let draft_keys =
            |app: &mut App, c: Context, a: Action| app.show_draft(|x| x.keymap.keys_of(c, a));
        let on_disk =
            |dir: &std::path::Path| crate::config::load::load(&dir.join("studio-config"), false);
        let q = Key::parse_seq("Q", app.keymap.leader).unwrap();
        // Enter on "quit" records new keys: Q, then Enter keeps them, unsaved
        app.settings_page.km_sel = row_of(&mut app, KmRow::Binding(Context::Global, Action::Quit));
        press(&mut app, "\n");
        assert!(matches!(app.overlay, Overlay::Capture { .. }));
        press(&mut app, "Q");
        assert!(render(&mut app, 160, 45).contains("Q▏"));
        press(&mut app, "\n");
        assert_eq!(
            draft_keys(&mut app, Context::Global, Action::Quit),
            vec![q.clone()]
        );
        assert_ne!(
            app.keymap.keys_of(Context::Global, Action::Quit),
            vec![q.clone()],
            "not live yet"
        );
        assert_ne!(
            on_disk(&dir).keymap.keys_of(Context::Global, Action::Quit),
            vec![q.clone()]
        );
        let screen = render(&mut app, 160, 45);
        assert!(
            screen.contains("● 1 unsaved")
                && screen.contains("s Save")
                && screen.contains("Settings ●"),
            "{screen}"
        );
        // s saves: live and on disk
        press(&mut app, "s");
        assert!(app.settings_page.draft.is_none());
        assert_eq!(
            app.keymap.keys_of(Context::Global, Action::Quit),
            vec![q.clone()]
        );
        assert_eq!(
            on_disk(&dir).keymap.keys_of(Context::Global, Action::Quit),
            vec![q]
        );
        // binding pin to y (copy-hex in Colours) warns; D discards
        app.settings_page.km_sel = row_of(&mut app, KmRow::Binding(Context::Colours, Action::Pin));
        press(&mut app, "ay\n");
        assert!(
            app.messages
                .message
                .as_ref()
                .is_some_and(|(m, err)| *err && m.contains("is also")),
            "{:?}",
            app.messages.message
        );
        assert_eq!(draft_keys(&mut app, Context::Colours, Action::Pin).len(), 2);
        press(&mut app, "D");
        assert!(app.settings_page.draft.is_none());
        assert_eq!(app.keymap.keys_of(Context::Colours, Action::Pin).len(), 1);
        // d unbinds, s saves; <leader>u undoes that save, again redoes
        press(&mut app, "ds");
        assert_eq!(
            app.keymap.keys_of(Context::Colours, Action::Pin),
            Vec::<Vec<Key>>::new()
        );
        press(&mut app, " u");
        assert_eq!(app.keymap.keys_of(Context::Colours, Action::Pin).len(), 1);
        press(&mut app, " u");
        assert_eq!(
            app.keymap.keys_of(Context::Colours, Action::Pin),
            Vec::<Vec<Key>>::new()
        );
        // Behaviour tab: the leader, by pressing it
        press(&mut app, "]");
        app.settings_page.bh_sel = 1;
        press(&mut app, "\n,");
        let comma = Key::new(KeyCode::Char(','), KeyModifiers::NONE);
        assert_eq!(app.show_draft(|a| a.keymap.leader), comma);
        assert_ne!(app.keymap.leader, comma, "not live yet");
        // quitting asks, and "save first" saves the settings
        press(&mut app, "Q"); // quit is Q now
        assert!(matches!(app.overlay, Overlay::Confirm(Pending::Quit)));
        let screen = render(&mut app, 160, 45);
        assert!(
            screen.contains("Unsaved: ") && screen.contains("settings."),
            "{screen}"
        );
        press(&mut app, "s");
        assert!(app.quit);
        assert_eq!(app.keymap.leader, comma);
        assert_eq!(on_disk(&dir).keymap.leader, comma);
        assert_eq!(
            app.keymap.keys_of(Context::Global, Action::SavePalette),
            vec![Key::parse_seq(",sp", comma).unwrap()]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn preset_browser_previews_loads_creates_and_deletes() {
        use crate::app::state::hit::Target as T;
        let store = temp_store("presets");
        let dir = store.dir.clone();
        crate::model::history::init_for_test(dir.join("history"));
        let mut app = test_app(store, Start::Named("One".into()));
        app.page = Page::Settings;
        let screen = render(&mut app, 160, 45);
        assert!(screen.contains("[ Vim ▾ ]"), "{screen}");
        // Enter on the preset button opens the list, on the active preset
        app.settings_page.km_sel = 0;
        press(&mut app, "\n");
        assert_eq!(app.settings_page.browse, Some(0));
        assert!(render(&mut app, 160, 45).contains("Vim (the current keymap)"));
        // j highlights Helix: the left side shows what would change
        press(&mut app, "j");
        let screen = render(&mut app, 160, 45);
        assert!(
            screen.contains("Vim → Helix:") && screen.contains("differ"),
            "{screen}"
        );
        assert!(
            screen.contains("redo") && !screen.contains("main page"),
            "only differences:\n{screen}"
        );
        press(&mut app, "A");
        assert!(
            render(&mut app, 160, 45).contains("main page"),
            "everything"
        );
        // n makes a new preset from the highlighted one
        press(&mut app, "nMy Helix\n");
        let names = |app: &mut App| {
            app.show_draft(|a| a.presets.iter().map(|p| p.name.clone()).collect::<Vec<_>>())
        };
        assert!(names(&mut app).contains(&"My Helix".to_string()));
        let mine = names(&mut app)
            .iter()
            .position(|n| n == "My Helix")
            .unwrap();
        assert_eq!(app.settings_page.browse, Some(mine));
        // Enter loads it (unsaved); s saves
        press(&mut app, "\n");
        assert_eq!(app.settings_page.browse, None);
        assert_eq!(app.show_draft(|a| a.keymap.name.clone()), "My Helix");
        assert_eq!(app.keymap.name, "Vim", "not live yet");
        press(&mut app, "s");
        assert_eq!(app.keymap.name, "My Helix");
        let saved = crate::config::load::load(&dir.join("studio-config"), false);
        assert_eq!(saved.keymap.name, "My Helix");
        // shipped presets can't be deleted; yours can (asks first)
        app.settings_page.km_sel = 0;
        press(&mut app, "\n");
        assert_eq!(
            app.settings_page.browse,
            Some(mine),
            "opens on the active one"
        );
        press(&mut app, "ggd"); // Helix keys are live now: gg, ge
        assert!(
            app.messages
                .message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("can't be deleted"))
        );
        press(&mut app, "ge");
        assert_eq!(app.settings_page.browse, Some(mine));
        press(&mut app, "dy");
        assert!(!names(&mut app).contains(&"My Helix".to_string()));
        assert_eq!(
            app.show_draft(|a| a.keymap.name.clone()),
            "Vim",
            "the active one went: Vim"
        );
        press(&mut app, "s");
        assert!(
            !crate::config::load::load(&dir.join("studio-config"), false)
                .presets
                .iter()
                .any(|p| p.name == "My Helix")
        );
        // the mouse: a click highlights, a second loads
        render(&mut app, 160, 45);
        click_on(&mut app, T::PresetRow(2));
        assert_eq!(app.settings_page.browse, Some(2));
        render(&mut app, 160, 45);
        click_on(&mut app, T::PresetRow(2));
        assert_eq!(app.show_draft(|a| a.keymap.name.clone()), "Emacs");
        press(&mut app, "D");
        // Esc closes the list first, then leaves the page
        app.settings_page.km_sel = 0;
        press(&mut app, "\n\x1b");
        assert_eq!((app.settings_page.browse, app.page), (None, Page::Settings));
        press(&mut app, "\x1b");
        assert_eq!(app.page, Page::Main);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn settings_groups_fold_and_only_names_are_underlined() {
        use crate::app::state::hit::Target as T;
        use crate::app::state::nav::BhRow;
        use crate::settings_page::behaviour::visible_behaviour_rows;
        use crate::settings_page::state::KmRow;
        let mut app = test_app(temp_store("fold"), Start::Named("One".into()));
        app.page = Page::Settings;
        // Keymaps: Enter on a group's title closes it, again opens it
        let n = app.keymap_rows().len();
        let g = app
            .keymap_rows()
            .iter()
            .position(|r| *r == KmRow::Header(Context::Global))
            .unwrap();
        app.settings_page.km_sel = g;
        press(&mut app, "\n");
        assert!(app.keymap_rows().len() < n);
        assert!(render(&mut app, 160, 45).contains("▸ Everywhere"));
        press(&mut app, "l");
        assert_eq!(app.keymap_rows().len(), n);
        // the cursor stops on titles but skips blank lines
        press(&mut app, "k");
        assert_eq!(app.keymap_rows()[app.settings_page.km_sel], KmRow::Preset);
        // Behaviour: a click on the selected title folds it
        app.settings_page.tab = crate::settings_page::state::Tab::Behaviour;
        render(&mut app, 160, 45);
        let rows = visible_behaviour_rows(&app.settings_page.bh_closed);
        let hover = rows
            .iter()
            .position(|r| *r == BhRow::Header("Hover (everything clickable)"))
            .unwrap();
        app.settings_page.bh_sel = hover;
        render(&mut app, 160, 45);
        click_on(&mut app, T::SettingsRow(hover));
        assert!(
            app.settings_page
                .bh_closed
                .contains("Hover (everything clickable)")
        );
        assert!(visible_behaviour_rows(&app.settings_page.bh_closed).len() < rows.len());
        // hover: only the option's name is underlined, not the row
        let mut t = Terminal::new(TestBackend::new(160, 45)).unwrap();
        t.draw(|f| crate::ui::screen::frame::draw(f, &mut app))
            .unwrap();
        let leader = visible_behaviour_rows(&app.settings_page.bh_closed)
            .iter()
            .position(|r| *r == BhRow::Leader)
            .unwrap();
        let r = app.hits.rect_of(T::SettingsRow(leader)).unwrap();
        mouse(&mut app, MouseEventKind::Moved, r.x + 5, r.y);
        t.draw(|f| crate::ui::screen::frame::draw(f, &mut app))
            .unwrap();
        let buf = t.backend().buffer().clone();
        let under = |x: u16| {
            buf[(x, r.y)]
                .modifier
                .contains(ratatui::style::Modifier::UNDERLINED)
        };
        let row: String = (r.x..r.right())
            .map(|x| buf[(x, r.y)].symbol().to_string())
            .collect();
        let start = r.x + u16::try_from(row.find("Leader").unwrap()).unwrap();
        assert!(under(start) && under(start + 10), "the name");
        assert!(
            !under(r.x) && !under(start + 45) && !under(start + 52),
            "not the padding or value:\n{row}"
        );
        std::fs::remove_dir_all(app.store.dir.clone()).unwrap();
    }
}
