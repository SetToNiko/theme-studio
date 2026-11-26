//! App's methods for the overlay keys.
//!
//! This is a second `impl App { ... }` block. Rust allows an `impl` of a type
//! in another file of the same crate. `app/mod.rs` has the other `App`
//! methods and the struct itself (and the [`Overlay`] enum).
//!
//! An overlay is a pop-up over the page: help, a name prompt, a list, a
//! question. [`App::on_overlay_key`] takes the open overlay, gives the key to
//! one method for that kind of overlay, and stores the overlay that results.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::state::nav::{For, Overlay, Pending};
use crate::app::{App, open_matches};
use crate::model::color::Rgb;
use crate::model::palette::Palette;

impl App {
    /// Opens the palette list (`For::Palette`) with the current palette highlighted.
    ///
    /// Shows an error message instead if the stored palettes cannot be read.
    pub(in crate::app) fn open_overlay(&mut self) {
        self.overlay_for = For::Palette;
        match self.store.load_all() {
            Ok(all) => {
                let entries = all
                    .iter()
                    .map(|(name, v)| (name.clone(), swatches(v)))
                    .collect::<Vec<_>>();
                let sel = entries
                    .iter()
                    .position(|(n, _)| *n == self.name)
                    .unwrap_or(0);
                // each palette, shown while it's highlighted
                self.open_palettes = all
                    .values()
                    .map(|v| v.as_object().and_then(|o| Palette::from_json(o).ok()))
                    .collect();
                self.open_original = Some(self.editor.palette.clone());
                self.overlay = Overlay::Open {
                    filter: String::new(),
                    sel,
                    entries,
                };
            }
            Err(e) => self.messages.error(format!("{e:#}")),
        }
    }

    /// Gives one key to the open overlay.
    ///
    /// Takes the overlay out of `self.overlay` with
    /// `std::mem::replace(&mut x, new)`. That call moves the old value out of
    /// `x` and leaves `new` in its place. Without it, `self.overlay` would stay
    /// borrowed, and the per-kind methods below could not take `&mut self`.
    ///
    /// The rule for each per-kind method: it returns `Some(overlay)` to show
    /// that overlay next, and `None` when it already set `self.overlay`
    /// itself (it opened something else). Never both.
    // Long function (reason verified, phase 3): one match over all 11 overlay kinds, one short arm each.
    pub(in crate::app) fn on_overlay_key(&mut self, k: KeyEvent) {
        let overlay = std::mem::replace(&mut self.overlay, Overlay::None);
        let next = match overlay {
            Overlay::Help { page } => Some(help_key(page, k)),
            Overlay::Menu { menu, sel, at } => self.menu_key(menu, sel, at, k),
            // Settings page pop-ups edit the draft (see `settings_page/draft.rs`)
            Overlay::ChooseKey { row, sel, original } => {
                self.with_draft(|app| app.choose_colour_key_press(row, sel, original, k));
                None
            }
            Overlay::Capture { what, keys } => {
                // capture_key sets the overlay itself
                self.with_draft(|app| app.capture_key(what, keys, k));
                None
            }
            Overlay::Prompt {
                title,
                input,
                purpose,
            } => self.prompt_key(title, input, purpose, k),
            Overlay::Ask { question, then } => self.ask_key(question, then, k),
            Overlay::Confirm(p) => Some(self.confirm_key(p, k)),
            // keys while copying go through the keymap (run_copy_from)
            Overlay::CopyFrom { target } => Some(Overlay::CopyFrom { target }),
            Overlay::SaveAs {
                input,
                overwrite,
                then_apply,
            } => Some(self.save_as_key(input, overwrite, then_apply, k)),
            Overlay::Open {
                filter,
                sel,
                entries,
            } => Some(self.open_key(filter, sel, entries, k)),
            Overlay::None => Some(Overlay::None),
        };
        if let Some(next) = next {
            self.overlay = next;
        }
    }

    /// Keys in the dropdown list under the preview: move, choose with Enter, close with Esc.
    fn menu_key(
        &mut self,
        menu: u8,
        mut sel: usize,
        at: ratatui::layout::Rect,
        k: KeyEvent,
    ) -> Option<Overlay> {
        let last = crate::fonts::preview_page::menu_items(menu).len() - 1;
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc => return Some(Overlay::None),
            KeyCode::Enter => {
                self.choose_menu(menu, sel);
                return None;
            }
            KeyCode::Down | KeyCode::Char('j') if !ctrl => sel = (sel + 1).min(last),
            KeyCode::Char('n') if ctrl => sel = (sel + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') if !ctrl => sel = sel.saturating_sub(1),
            KeyCode::Char('p') if ctrl => sel = sel.saturating_sub(1),
            KeyCode::Home | KeyCode::Char('g') => sel = 0,
            KeyCode::End | KeyCode::Char('G') => sel = last,
            _ => {}
        }
        Some(Overlay::Menu { menu, sel, at })
    }

    /// Keys in a settings-page text prompt: type, Backspace, Enter to finish, Esc to close.
    fn prompt_key(
        &mut self,
        title: String,
        mut input: String,
        purpose: crate::app::state::nav::Naming,
        k: KeyEvent,
    ) -> Option<Overlay> {
        match k.code {
            KeyCode::Esc => return Some(Overlay::None),
            KeyCode::Enter => {
                // finish_prompt may open something else
                self.with_draft(|app| app.finish_prompt(purpose, &input));
                return None;
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c) => input.push(c),
            _ => {}
        }
        Some(Overlay::Prompt {
            title,
            input,
            purpose,
        })
    }

    /// Keys in a settings-page yes / no question: y or Enter runs it, n or Esc closes.
    fn ask_key(
        &mut self,
        question: String,
        then: crate::app::state::nav::Deferred,
        k: KeyEvent,
    ) -> Option<Overlay> {
        match k.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                self.with_draft(|app| app.run_deferred(then));
                None
            }
            KeyCode::Esc | KeyCode::Char('n') => Some(Overlay::None),
            _ => Some(Overlay::Ask { question, then }),
        }
    }

    /// Keys in the unsaved-changes box: y goes on, s saves first, n or Esc closes.
    fn confirm_key(&mut self, p: Pending, k: KeyEvent) -> Overlay {
        match k.code {
            KeyCode::Char('y') => {
                self.run_pending(p);
                Overlay::None
            }
            KeyCode::Char('s') => {
                self.save_before(&p);
                self.run_pending(p);
                Overlay::None
            }
            KeyCode::Esc | KeyCode::Char('n') => Overlay::None,
            _ => Overlay::Confirm(p),
        }
    }

    /// Keys in the "save as" name box: type, Backspace, Enter to save, Esc to close.
    // Long function (reason verified, phase 3): one match over the keys of the box; Enter hands over to two methods.
    fn save_as_key(
        &mut self,
        mut input: String,
        mut overwrite: bool,
        then_apply: bool,
        k: KeyEvent,
    ) -> Overlay {
        match k.code {
            KeyCode::Esc => return Overlay::None,
            KeyCode::Enter => {
                return if self.overlay_for == For::Theme {
                    self.save_as_enter_theme(input, overwrite, then_apply)
                } else {
                    self.save_as_enter(input, overwrite, then_apply)
                };
            }
            // typing changes the name, so a second Enter no longer overwrites
            KeyCode::Backspace => {
                input.pop();
                overwrite = false;
            }
            KeyCode::Char(c) => {
                input.push(c);
                overwrite = false;
            }
            _ => {}
        }
        Overlay::SaveAs {
            input,
            overwrite,
            then_apply,
        }
    }

    /// Enter in the "save as" box for a theme.
    ///
    /// An empty name keeps the box open. An existing name asks for a second
    /// Enter (`overwrite`). Otherwise it saves. Returns the overlay to show next.
    fn save_as_enter_theme(&mut self, input: String, overwrite: bool, then_apply: bool) -> Overlay {
        let name = input.trim().to_string();
        let exists = self.themes.get(&name).is_some();
        if name.is_empty() {
            Overlay::SaveAs {
                input,
                overwrite,
                then_apply,
            }
        } else if exists && name != self.theme && !overwrite {
            self.messages.error(format!(
                "The theme \"{name}\" exists. Enter again to overwrite it"
            ));
            Overlay::SaveAs {
                input,
                overwrite: true,
                then_apply,
            }
        } else {
            self.theme_save_as(&name, then_apply);
            // theme_save_as may have opened another overlay: take it, leave `None`
            std::mem::replace(&mut self.overlay, Overlay::None)
        }
    }

    /// Enter in the "save as" box for a palette or a fonts setup.
    ///
    /// Same rules as for a theme: an empty name keeps the box open, an
    /// existing name needs a second Enter. Returns the overlay to show next.
    // Long function (reason verified, phase 3): the fonts and palette checks share one set of "exists" values.
    fn save_as_enter(&mut self, input: String, overwrite: bool, then_apply: bool) -> Overlay {
        let name = input.trim().to_string();
        let (exists, current) = if self.overlay_for == For::Fonts {
            let all = self.fonts.store.load_all();
            (
                all.is_ok_and(|a| a.contains_key(&name)),
                self.fonts.name.clone(),
            )
        } else {
            let all = self.store.load_all();
            (all.is_ok_and(|a| a.contains_key(&name)), self.name.clone())
        };
        if name.is_empty() {
            Overlay::SaveAs {
                input,
                overwrite,
                then_apply,
            }
        } else if self.overlay_for == For::Fonts && (!exists || name == current || overwrite) {
            self.fonts_save_as(&name, then_apply);
            Overlay::None
        } else if exists && name != current && !overwrite {
            self.messages
                .error(format!("\"{name}\" exists. Enter again to overwrite it"));
            Overlay::SaveAs {
                input,
                overwrite: true,
                then_apply,
            }
        } else {
            self.save_as(&name, then_apply);
            Overlay::None
        }
    }

    /// Keys in the "open" list: filter by typing, move, Enter opens, Esc closes.
    // Long function (reason verified, phase 3): one match over the keys of the list; Enter hands over to `open_enter`.
    fn open_key(
        &mut self,
        mut filter: String,
        mut sel: usize,
        entries: Vec<(String, Vec<Rgb>)>,
        k: KeyEvent,
    ) -> Overlay {
        let n = open_matches(&entries, &filter).len();
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc => return Overlay::None,
            KeyCode::Enter => return self.open_enter(filter, sel, entries),
            KeyCode::Down => sel = (sel + 1).min(n.saturating_sub(1)),
            KeyCode::Up => sel = sel.saturating_sub(1),
            KeyCode::Char('n') if ctrl => sel = (sel + 1).min(n.saturating_sub(1)),
            KeyCode::Char('p') if ctrl => sel = sel.saturating_sub(1),
            // typing changes the filter, so the highlight goes back to the top
            KeyCode::Backspace => {
                filter.pop();
                sel = 0;
            }
            KeyCode::Char(c) => {
                filter.push(c);
                sel = 0;
            }
            _ => {}
        }
        Overlay::Open {
            filter,
            sel,
            entries,
        }
    }

    /// Enter in the "open" list: opens the highlighted entry, or keeps the list if none matches.
    fn open_enter(
        &mut self,
        filter: String,
        sel: usize,
        entries: Vec<(String, Vec<Rgb>)>,
    ) -> Overlay {
        match open_matches(&entries, &filter).get(sel) {
            Some(&i) => {
                let name = entries[i].0.clone();
                // the unsaved-changes check sees the real palette
                self.end_palette_preview();
                self.guard(match self.overlay_for {
                    For::Fonts => Pending::OpenFonts(name),
                    For::Palette => Pending::Open(name),
                    For::Theme => Pending::OpenTheme(name),
                });
                // guard() may have opened a confirmation instead: take it, leave `None`
                std::mem::replace(&mut self.overlay, Overlay::None)
            }
            None => Overlay::Open {
                filter,
                sel,
                entries,
            },
        }
    }
}

/// Keys in the help pop-up: arrows and page keys page through the columns; any other key closes.
fn help_key(page: usize, k: KeyEvent) -> Overlay {
    match k.code {
        KeyCode::Right | KeyCode::PageDown | KeyCode::Char('l') => Overlay::Help { page: page + 1 },
        KeyCode::Left | KeyCode::PageUp | KeyCode::Char('h') => Overlay::Help {
            page: page.saturating_sub(1),
        },
        _ => Overlay::None,
    }
}

/// Picks the nine swatch colours (backgrounds, foreground and accents) from a
/// palette's JSON object. Keys that are missing or not a colour are skipped.
pub(in crate::app) fn swatches(v: &serde_json::Value) -> Vec<Rgb> {
    let mut out = Vec::new();
    for key in [
        "bg0", "fg0", "red", "orange", "yellow", "green", "aqua", "blue", "purple",
    ] {
        if let Some(c) = v.get(key).and_then(|x| x.as_str()).and_then(Rgb::parse) {
            out.push(c);
        }
    }
    out
}
