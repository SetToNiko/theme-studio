//! The keys that ship with the studio: the Vim preset and the presets
//! derived from it. This is data, kept apart from the keymap logic.

use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use super::action::Action;
use super::context::Context;
use super::key::Key;
use super::map::{Binding, Keymap};

/// The movement actions: the ones that `moves_without_letters` rebinds.
const MOVES: [Action; 12] = [
    Action::Up,
    Action::Down,
    Action::Left,
    Action::Right,
    Action::Top,
    Action::Bottom,
    Action::UpBig,
    Action::DownBig,
    Action::LeftBig,
    Action::RightBig,
    Action::PageUp,
    Action::PageDown,
];

/// Whether a key sequence is one plain letter or symbol, with no modifier.
fn is_plain_letter(keys: &[Key]) -> bool {
    keys.len() == 1 && matches!(keys[0].code, KeyCode::Char(_)) && keys[0].mods.is_empty()
}

impl Keymap {
    /// The presets that ship with the studio.
    pub fn builtin() -> Vec<Keymap> {
        vec![
            Keymap::vim(),
            Keymap::helix(),
            Keymap::emacs(),
            Keymap::vscode(),
            Keymap::jetbrains(),
        ]
    }

    /// A preset made from Vim's: its keys with some replaced (an empty list
    /// unbinds); `<leader>` sequences follow the new leader.
    fn derived(name: &str, leader: &str, overrides: &[(Context, Action, &[&str])]) -> Keymap {
        let space = Key::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let leader = Key::parse_seq(leader, space).expect("leader parses")[0];
        let mut km = Keymap::vim().with_leader(leader);
        km.name = name.to_string();
        for (ctx, action, seqs) in overrides {
            let keys = seqs
                .iter()
                .map(|s| Key::parse_seq(s, leader).unwrap_or_else(|e| panic!("{name}: {s}: {e}")))
                .collect();
            km.set_keys(*ctx, *action, keys);
        }
        km
    }

    /// Movement without letters (h/j/k/l, g/G, H/J/K/L): arrows and the
    /// like stay, and every context that moves gets the keys in `add`.
    fn moves_without_letters(mut self, add: &[(Action, &[&str])]) -> Keymap {
        let moving: Vec<(Context, Action)> = self
            .bindings
            .iter()
            .filter(|b| MOVES.contains(&b.action))
            .map(|b| (b.context, b.action))
            .collect();
        self.bindings
            .retain(|b| !(MOVES.contains(&b.action) && is_plain_letter(&b.keys)));
        for (ctx, action) in moving {
            for (a, seqs) in add.iter().filter(|(a, _)| *a == action) {
                for s in *seqs {
                    let keys = Key::parse_seq(s, self.leader).expect("move keys parse");
                    if !self
                        .bindings
                        .iter()
                        .any(|b| b.context == ctx && b.action == *a && b.keys == keys)
                    {
                        self.bindings.push(Binding {
                            context: ctx,
                            keys,
                            action: *a,
                        });
                    }
                }
            }
        }
        self
    }

    /// Helix: Vim's keys with Helix's differences (U redo, gg / ge, :w / :q,
    /// Space ? for the key list).
    pub fn helix() -> Keymap {
        use Action as A;
        use Context as C;
        let mut rows: Vec<(Context, Action, &[&str])> = vec![
            (C::Global, A::Quit, &["q", ":q"]),
            (C::Global, A::Save, &["s", ":w"]),
            (C::Global, A::Help, &["?", "<leader>?"]),
            (C::Colours, A::Redo, &["U"]),
        ];
        for ctx in [C::ColourList, C::FontList, C::Settings, C::Downloads] {
            rows.push((ctx, A::Top, &["gg", "<Home>"]));
            rows.push((ctx, A::Bottom, &["ge", "<End>"]));
        }
        Keymap::derived("Helix", "<Space>", &rows)
    }

    /// Emacs: C-n / C-p / C-f / C-b to move, C-x prefixes for files, C-g to
    /// back out, C-/ to undo; the leader is C-c.
    // Long function (reason verified, phase 6): one table, one row per Emacs key, then the table of movement keys.
    pub fn emacs() -> Keymap {
        use Action as A;
        use Context as C;
        let rows: Vec<(Context, Action, &[&str])> = vec![
            (C::Global, A::Quit, &["<C-x><C-c>"]),
            (C::Global, A::Help, &["?", "<C-h>b"]),
            (C::Global, A::Save, &["<C-x><C-s>"]),
            (C::Global, A::SaveAs, &["<C-x><C-w>"]),
            (C::Global, A::Apply, &["<leader><C-c>"]),
            (C::Global, A::Open, &["<C-x><C-f>"]),
            (C::Global, A::New, &["<C-x>b"]),
            (C::Global, A::PaneLeft, &["<C-x><Left>"]),
            (C::Global, A::PaneRight, &["<C-x><Right>", "<C-x>o"]),
            (C::Global, A::PaneUp, &["<C-x><Up>"]),
            (C::Global, A::PaneDown, &["<C-x><Down>"]),
            (C::Colours, A::Undo, &["<C-/>", "<C-x>u"]),
            (C::Colours, A::Redo, &["<C-x>U"]),
            (C::Colours, A::CopyHex, &["y", "<leader>w"]),
            (C::Colours, A::Back, &["<Esc>", "<C-g>"]),
            (C::FontList, A::Back, &["<Esc>", "<C-g>"]),
            (C::FontList, A::Search, &["<C-s>", "/"]),
            (C::Resize, A::Back, &["<Esc>", "<CR>", "<C-g>"]),
            (C::Settings, A::Back, &["<Esc>", "<C-g>"]),
            (C::Downloads, A::Back, &["<Esc>", "<C-g>"]),
            (C::Downloads, A::Search, &["<C-s>", "/"]),
        ];
        Keymap::derived("Emacs", "<C-c>", &rows).moves_without_letters(&[
            (A::Up, &["<C-p>"]),
            (A::Down, &["<C-n>"]),
            (A::Left, &["<C-b>"]),
            (A::Right, &["<C-f>"]),
            (A::Top, &["<C-Home>"]),
            (A::Bottom, &["<C-End>"]),
            (A::PageDown, &["<C-v>"]),
            (A::UpBig, &["<S-Up>"]),
            (A::DownBig, &["<S-Down>"]),
            (A::LeftBig, &["<S-Left>"]),
            (A::RightBig, &["<S-Right>"]),
        ])
    }

    /// Arrows and the Ctrl keys `VS Code` and `JetBrains` share.
    fn arrows(km: Keymap) -> Keymap {
        use Action as A;
        km.moves_without_letters(&[
            (A::Top, &["<C-Home>"]),
            (A::Bottom, &["<C-End>"]),
            (A::UpBig, &["<S-Up>"]),
            (A::DownBig, &["<S-Down>"]),
            (A::LeftBig, &["<S-Left>"]),
            (A::RightBig, &["<S-Right>"]),
        ])
    }

    /// VS Code: Ctrl+S / Ctrl+O / Ctrl+N, Ctrl+Z / Ctrl+Y, F5 to apply,
    /// Ctrl+1…7 for panes (with the kitty keyboard protocol), arrows to
    /// move; the leader is Ctrl+K (its chord key).
    pub fn vscode() -> Keymap {
        use Action as A;
        use Context as C;
        let rows: Vec<(Context, Action, &[&str])> = vec![
            (C::Global, A::Quit, &["<C-q>"]),
            (C::Global, A::Help, &["?", "<leader><C-s>"]),
            (C::Global, A::Save, &["<C-s>"]),
            (C::Global, A::SaveAs, &["<C-S-s>"]),
            (C::Global, A::Apply, &["<F5>"]),
            (C::Global, A::Open, &["<C-o>"]),
            (C::Global, A::New, &["<C-n>"]),
            (C::Global, A::PaneLeft, &["<leader><Left>"]),
            (C::Global, A::PaneRight, &["<leader><Right>"]),
            (C::Global, A::PaneUp, &["<leader><Up>"]),
            (C::Global, A::PaneDown, &["<leader><Down>"]),
            (C::Global, A::Panel(1), &["<C-1>", "<leader>1"]),
            (C::Global, A::Panel(2), &["<C-2>", "<leader>2"]),
            (C::Global, A::Panel(3), &["<C-3>", "<leader>3"]),
            (C::Global, A::Panel(4), &["<C-4>", "<leader>4"]),
            (C::Global, A::Panel(5), &["<C-5>", "<leader>5"]),
            (C::Global, A::Panel(6), &["<C-6>", "<leader>6"]),
            (C::Global, A::Panel(7), &["<C-7>", "<leader>7"]),
            (C::Colours, A::Undo, &["<C-z>"]),
            (C::Colours, A::Redo, &["<C-y>", "<C-S-z>"]),
            (C::Colours, A::CopyHex, &["<C-c>"]),
            (C::FontList, A::Search, &["<C-f>", "/"]),
            (C::Downloads, A::Search, &["<C-f>", "/"]),
        ];
        Keymap::arrows(Keymap::derived("VS Code", "<C-k>", &rows))
    }

    /// `JetBrains`: Ctrl+S, Ctrl+E (recent files) to open, Ctrl+Z /
    /// Ctrl+Shift+Z, Shift+F10 (run) to apply, arrows to move. Its
    /// Alt+number tool windows would clash with i3, so panes stay on
    /// Space 1…7.
    pub fn jetbrains() -> Keymap {
        use Action as A;
        use Context as C;
        let rows: Vec<(Context, Action, &[&str])> = vec![
            (C::Global, A::Quit, &["<C-q>"]),
            (C::Global, A::Save, &["<C-s>"]),
            (C::Global, A::SaveAs, &["<C-S-s>"]),
            (C::Global, A::Apply, &["<S-F10>"]),
            (C::Global, A::Open, &["<C-e>", "<C-o>"]),
            (C::Global, A::New, &["<C-n>"]),
            (C::Global, A::PaneLeft, &["<leader><Left>"]),
            (C::Global, A::PaneRight, &["<leader><Right>"]),
            (C::Global, A::PaneUp, &["<leader><Up>"]),
            (C::Global, A::PaneDown, &["<leader><Down>"]),
            (C::Colours, A::Undo, &["<C-z>"]),
            (C::Colours, A::Redo, &["<C-S-z>"]),
            (C::Colours, A::CopyHex, &["<C-c>"]),
            (C::FontList, A::Search, &["<C-f>", "/"]),
            (C::Downloads, A::Search, &["<C-f>", "/"]),
        ];
        Keymap::arrows(Keymap::derived("JetBrains", "<Space>", &rows))
    }

    /// The shipped Vim preset: the studio's original keys, plus leader
    /// sequences for saving and applying either side.
    // Long function (reason verified, phase 6): one table, one row per default key of the Vim preset.
    #[expect(
        clippy::too_many_lines,
        reason = "one table, one row per default key of the Vim preset; splitting it would only scatter the rows"
    )]
    pub fn vim() -> Keymap {
        use Action as A;
        use Context as C;
        const CHIP_KEYS: [&str; 8] = ["m", "p", "n", "M", "P", "x", "b", "c"];
        let mut rows: Vec<(Context, Action, &[&str])> = vec![
            (C::Global, A::Quit, &["q"]),
            (C::Global, A::Help, &["?"]),
            (C::Global, A::PageMain, &["<F1>"]),
            (C::Global, A::PageDownloads, &["<F2>"]),
            (C::Global, A::PageSettings, &["<F3>"]),
            (C::Global, A::PaneLeft, &["<C-w>h"]),
            (C::Global, A::PaneRight, &["<C-w>l"]),
            (C::Global, A::PaneUp, &["<C-w>k"]),
            (C::Global, A::PaneDown, &["<C-w>j"]),
            (C::Global, A::Panel(5), &["<leader>5"]),
            (C::Global, A::Panel(6), &["<leader>6"]),
            (C::Global, A::Panel(7), &["<leader>7"]),
            (C::Global, A::ResizeMode, &["<leader>r"]),
            (C::Global, A::UndoConfig, &["<leader>u"]),
            (C::Global, A::OpenPalette, &["<leader>op"]),
            (C::Global, A::OpenFonts, &["<leader>of"]),
            (C::Global, A::NewPalette, &["<leader>np"]),
            (C::Global, A::NewFonts, &["<leader>nf"]),
            (C::Downloads, A::Up, &["k", "<Up>"]),
            (C::Downloads, A::Down, &["j", "<Down>"]),
            (C::Downloads, A::Left, &["h", "<Left>"]),
            (C::Downloads, A::Right, &["l", "<Right>"]),
            (C::Downloads, A::Top, &["g", "<Home>"]),
            (C::Downloads, A::Bottom, &["G", "<End>"]),
            (C::Downloads, A::PageUp, &["<PageUp>"]),
            (C::Downloads, A::PageDown, &["<PageDown>"]),
            (C::Downloads, A::Confirm, &["<CR>"]),
            (C::Downloads, A::Back, &["<Esc>"]),
            (C::Downloads, A::Search, &["/"]),
            (C::Downloads, A::Delete, &["d", "<Del>"]),
            (C::Downloads, A::TabNext, &["]"]),
            (C::Downloads, A::TabPrev, &["["]),
            (C::Downloads, A::PreviewFetch, &["p"]),
            (C::Downloads, A::PreviewText, &["t"]),
            (C::Downloads, A::PreviewLang, &["L"]),
            (C::Downloads, A::PreviewNext, &[">"]),
            (C::Downloads, A::PreviewPrev, &["<"]),
            (C::Downloads, A::SizeUp, &["+", "="]),
            (C::Downloads, A::SizeDown, &["-"]),
            (C::Settings, A::Up, &["k", "<Up>"]),
            (C::Settings, A::Down, &["j", "<Down>"]),
            (C::Settings, A::Left, &["h", "<Left>"]),
            (C::Settings, A::Right, &["l", "<Right>"]),
            (C::Settings, A::Top, &["g", "<Home>"]),
            (C::Settings, A::Bottom, &["G", "<End>"]),
            (C::Settings, A::PageUp, &["<PageUp>"]),
            (C::Settings, A::PageDown, &["<PageDown>"]),
            (C::Settings, A::Confirm, &["<CR>"]),
            (C::Settings, A::Back, &["<Esc>"]),
            (C::Settings, A::FocusNext, &["<Tab>"]),
            (C::Settings, A::FocusPrev, &["<S-Tab>"]),
            (C::Settings, A::TabNext, &["]"]),
            (C::Settings, A::TabPrev, &["["]),
            (C::Settings, A::Add, &["a"]),
            (C::Settings, A::Delete, &["d", "<Del>"]),
            (C::Settings, A::Restore, &["r"]),
            (C::Settings, A::SaveNamed, &["n"]),
            (C::Settings, A::ResetPreset, &["R"]),
            (C::Settings, A::ToggleAll, &["A"]),
            (C::Settings, A::EditText, &["i"]),
            (C::Settings, A::Discard, &["D"]),
            (C::Settings, A::NextHeading, &["}"]),
            (C::Settings, A::PrevHeading, &["{"]),
            (C::Resize, A::Left, &["h", "<Left>"]),
            (C::Resize, A::Right, &["l", "<Right>"]),
            (C::Resize, A::Up, &["k", "<Up>"]),
            (C::Resize, A::Down, &["j", "<Down>"]),
            (C::Resize, A::LeftBig, &["H", "<S-Left>"]),
            (C::Resize, A::RightBig, &["L", "<S-Right>"]),
            (C::Resize, A::UpBig, &["K", "<S-Up>"]),
            (C::Resize, A::DownBig, &["J", "<S-Down>"]),
            (C::Resize, A::ResetSizes, &["="]),
            (C::Resize, A::Back, &["<Esc>", "<CR>"]),
            (C::Colours, A::TogglePicker, &["<leader>cp"]),
            (C::Fonts, A::ToggleSample, &["<leader>fs"]),
            (C::Global, A::ToggleHints, &["<leader>th"]),
            (C::Global, A::TogglePreview, &["<leader>tp"]),
            (C::Global, A::PreviewCycle, &["<F12>"]),
            (C::Global, A::SavePalette, &["<leader>sp"]),
            (C::Global, A::SaveFonts, &["<leader>sf"]),
            (C::Global, A::ApplyPalette, &["<leader>ap"]),
            (C::Global, A::ApplyFonts, &["<leader>af"]),
            (C::Global, A::Save, &["s"]),
            (C::Global, A::SaveAs, &["S"]),
            (C::Global, A::Apply, &["a"]),
            (C::Global, A::Open, &["o"]),
            (C::Global, A::New, &["n"]),
            (C::Global, A::Panel(1), &["<leader>1"]),
            (C::Global, A::Panel(2), &["<leader>2"]),
            (C::Global, A::Panel(3), &["<leader>3"]),
            (C::Global, A::Panel(4), &["<leader>4"]),
            (C::Colours, A::Undo, &["u"]),
            (C::Colours, A::Redo, &["<C-r>"]),
            (C::Colours, A::Pin, &["p"]),
            (C::Colours, A::CopyHex, &["y"]),
            (C::Colours, A::CopyFrom, &["c"]),
            (C::Colours, A::HexEntry, &["#"]),
            (C::Colours, A::FocusNext, &["<Tab>"]),
            (C::Colours, A::FocusPrev, &["<S-Tab>"]),
            (C::Colours, A::Back, &["<Esc>"]),
            (C::Colours, A::ToggleYazi, &["Y"]),
            (C::Colours, A::ToggleRofi, &["R"]),
            (C::Colours, A::ToggleView, &["V"]),
            (C::ColourList, A::Up, &["k", "<Up>"]),
            (C::ColourList, A::Down, &["j", "<Down>"]),
            (C::ColourList, A::Right, &["l", "<Right>"]),
            (C::ColourList, A::Confirm, &["<CR>"]),
            (C::ColourList, A::Top, &["g", "<Home>"]),
            (C::ColourList, A::Bottom, &["G", "<End>"]),
            (C::ColourList, A::PageUp, &["<PageUp>"]),
            (C::ColourList, A::PageDown, &["<PageDown>"]),
            (C::Picker, A::Confirm, &["<CR>"]),
            (C::Picker, A::Up, &["k", "<Up>"]),
            (C::Picker, A::Down, &["j", "<Down>"]),
            (C::Picker, A::Left, &["h", "<Left>"]),
            (C::Picker, A::Right, &["l", "<Right>"]),
            (C::Picker, A::UpBig, &["K", "<S-Up>"]),
            (C::Picker, A::DownBig, &["J", "<S-Down>"]),
            (C::Picker, A::LeftBig, &["H", "<S-Left>"]),
            (C::Picker, A::RightBig, &["L", "<S-Right>"]),
            (C::Fonts, A::FocusNext, &["<Tab>"]),
            (C::Fonts, A::FocusPrev, &["<S-Tab>"]),
            (C::FontRoles, A::Up, &["k", "<Up>"]),
            (C::FontRoles, A::Down, &["j", "<Down>"]),
            (C::FontRoles, A::Left, &["h", "<Left>"]),
            (C::FontRoles, A::Right, &["l", "<Right>"]),
            (C::FontRoles, A::LeftBig, &["H"]),
            (C::FontRoles, A::RightBig, &["L"]),
            (C::FontRoles, A::SizeUp, &["+", "="]),
            (C::FontRoles, A::SizeDown, &["-"]),
            (C::FontRoles, A::PickFont, &["<CR>", "f"]),
            (C::FontRoles, A::FixRoleFont, &["r"]),
            (C::FontList, A::Up, &["k", "<Up>"]),
            (C::FontList, A::Down, &["j", "<Down>"]),
            (C::FontList, A::Left, &["h", "<Left>"]),
            (C::FontList, A::Right, &["l", "<Right>"]),
            (C::FontList, A::Top, &["g", "<Home>"]),
            (C::FontList, A::Bottom, &["G", "<End>"]),
            (C::FontList, A::PageUp, &["<PageUp>"]),
            (C::FontList, A::PageDown, &["<PageDown>"]),
            (C::FontList, A::Confirm, &["<CR>"]),
            (C::FontList, A::Back, &["<Esc>"]),
            (C::FontList, A::Search, &["/"]),
            (C::FontList, A::FillFamily, &["F"]),
            (C::Downloads, A::FillFamily, &["F"]),
            (C::Downloads, A::TourNames, &["n"]),
            (C::Downloads, A::TourBracket, &["b"]),
            (C::Downloads, A::TourUndo, &["u"]),
            (C::Downloads, A::UseFor(0), &["1"]),
            (C::Downloads, A::UseFor(1), &["2"]),
            (C::Downloads, A::UseFor(2), &["3"]),
        ];
        for (i, (c, _)) in crate::fonts::types::CHIPS.iter().enumerate() {
            rows.push((
                C::FontList,
                A::Filter(*c),
                std::slice::from_ref(&CHIP_KEYS[i]),
            ));
        }
        Keymap::from_rows(
            "Vim",
            Key::new(KeyCode::Char(' '), KeyModifiers::NONE),
            &rows,
        )
        .expect("the Vim preset parses")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::key::display_seq;
    use crate::keymap::lookup::Lookup;
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
