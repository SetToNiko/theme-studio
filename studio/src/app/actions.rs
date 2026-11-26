//! App's methods for the keyboard: key dispatch and the actions.
//!
//! This is a second `impl App { ... }` block. Rust allows an `impl` of a type
//! in another file of the same crate. `app/mod.rs` has the other `App`
//! methods and the struct itself.
//!
//! [`App::on_key`] is the entry point. It sends the key to `handle_key`,
//! which checks, in this order: the notice box, a focused preview window,
//! Ctrl+C, the pop-ups, the text fields, and last the keymap. The keymap
//! turns the keys into an [`Action`]. [`App::run_action`] then does what the
//! action means. One `match` sends each action to the method of its group:
//! the view (windows and panes), the theme files, the other pages, or the
//! fonts column. What is left goes to the colours column (`run_colours`).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, ColourRow, Focus, For, Overlay, Page, Pending, PreviewMode, Side};
use crate::keymap::{Action, Context, Key, Lookup};

impl App {
    pub(super) fn on_key(&mut self, k: KeyEvent) {
        self.handle_key(k);
        self.sync_palette_preview();
        self.end_session_outside_picker();
    }

    /// Only Enter keeps a picker change: leaving the picker any other way
    /// (Esc, another pane or column, a preview window, a click elsewhere,
    /// hiding it) puts the colours back.
    pub(super) fn end_session_outside_picker(&mut self) {
        let left =
            self.focus == Focus::List || self.side != Side::Colours || self.preview.focus.is_some();
        if left && self.editor.in_edit_session() {
            self.leave_picker(true);
        }
    }

    /// Turns one key press into an action, checking the special cases first.
    ///
    /// The order of the checks matters: notice box, focused preview window,
    /// Ctrl+C, pop-ups, text fields, and last the keymap with its pending keys.
    // Long function (reason verified, phase 3): one early return per special case, in a fixed order.
    fn handle_key(&mut self, k: KeyEvent) {
        use crate::fonts::FontFocus;
        let key = Key::from_event(k);
        // A save confirmation box: any key closes it (Ctrl+C still goes on
        // to quit).
        if self.messages.notice.take().is_some()
            && key != Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        {
            return;
        }
        // A preview window has the keyboard: everything but the key bound to
        // "type into the preview / next window / back" goes to it.
        if let Some(id) = self.preview.focus
            && matches!(self.overlay, Overlay::None)
        {
            if self.keymap.lookup(&[Context::Global], &[key]) == Lookup::Run(Action::PreviewCycle) {
                self.preview.cycle_focus();
            } else {
                self.preview.send_key(id, k);
            }
            return;
        }
        // Ctrl+C quits (asking first about unsaved changes), unless the
        // keymap uses it here (Emacs's leader, copying in VS Code)
        if key == Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
            && self.pending_keys.is_empty()
            && !matches!(self.overlay, Overlay::Capture { .. })
            && (!matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. })
                || self.keymap.lookup(&self.contexts(), &[key]) == Lookup::None)
        {
            self.pending_keys.clear();
            self.guard(Pending::Quit);
            return;
        }
        let copying = matches!(self.overlay, Overlay::CopyFrom { .. });
        if !matches!(self.overlay, Overlay::None) && !copying {
            self.on_overlay_key(k);
            return;
        }
        self.messages.message = None;
        // Text fields: keys are typing, not actions.
        if self.pending_keys.is_empty() {
            if self.page == Page::Downloads && self.dl.searching {
                return self.dl_search_key(k);
            }
            if self.side == Side::Colours && self.focus == Focus::Hex {
                return self.on_hex_key(k);
            }
            if self.side == Side::Fonts && self.fonts.focus == FontFocus::Search {
                return self.fonts.on_search_key(k);
            }
        }
        self.pending_keys.push(key);
        let contexts = self.contexts();
        match self.keymap.lookup(&contexts, &self.pending_keys) {
            Lookup::Pending => {}
            Lookup::Run(a) => {
                self.pending_keys.clear();
                self.run_action(a);
            }
            Lookup::None => self.pending_keys.clear(),
        }
    }

    /// Does what an action means in the pane with the keyboard.
    ///
    /// The checks that come first (resize mode, the copy-from pop-up, undo of
    /// a config change, the page tabs) can end the call. Then one `match`
    /// sends the action to the method of its group.
    // Long function: four early returns, then one match that lists every action group.
    pub fn run_action(&mut self, a: Action) {
        if self.layout.resize_mode {
            return self.run_resize(a);
        }
        if let Overlay::CopyFrom { target } = self.overlay {
            return self.run_copy_from(target, a);
        }
        if a == Action::UndoConfig {
            return self.undo_config();
        }
        if self.layout.page_tabs_focused && self.page_tabs_action(a) {
            return;
        }
        match a {
            // quit, help, page switch, hints and preview toggles: on every page
            _ if is_page_action(a) => self.run_view_action(a),
            // the other pages take every other action themselves
            _ if self.page == Page::Settings => self.run_settings(a),
            _ if self.page == Page::Downloads => self.run_downloads(a),
            _ if self.page != Page::Main => {}
            // from here on: the Main page
            Action::PreviewCycle
            | Action::TogglePicker
            | Action::ToggleSample
            | Action::ResizeMode
            | Action::PaneLeft
            | Action::PaneRight
            | Action::PaneUp
            | Action::PaneDown
            | Action::Panel(_) => self.run_view_action(a),
            Action::SavePalette
            | Action::SaveFonts
            | Action::ApplyPalette
            | Action::ApplyFonts
            | Action::Save
            | Action::SaveAs
            | Action::Apply
            | Action::Open
            | Action::New
            | Action::OpenPalette
            | Action::OpenFonts
            | Action::NewPalette
            | Action::NewFonts => self.run_theme_action(a),
            _ if self.side == Side::Fonts => self.run_fonts(a),
            _ => self.run_colours(a),
        }
    }

    /// Actions for the window: quit, help, page switch, the toggles, resize
    /// mode and the pane focus. Other actions do nothing.
    // Long function (reason verified, phase 3): one match with one arm per window action.
    fn run_view_action(&mut self, a: Action) {
        match a {
            Action::Quit => self.guard(Pending::Quit),
            Action::Help => self.overlay = Overlay::Help { page: 0 },
            Action::PageMain => self.page = Page::Main,
            Action::PageDownloads => self.page = Page::Downloads,
            Action::PageSettings => self.page = Page::Settings,
            Action::PreviewCycle => {
                self.preview.mode = PreviewMode::Desktop;
                self.preview.cycle_focus();
            }
            Action::TogglePicker => {
                if self.layout.picker_visible {
                    self.layout.picker_visible = false;
                    self.focus = Focus::List;
                } else {
                    self.layout.picker_visible = true;
                }
            }
            Action::ToggleHints => self.layout.hints_visible = !self.layout.hints_visible,
            Action::TogglePreview => match self.page {
                Page::Main => {
                    self.layout.main_preview_visible = !self.layout.main_preview_visible;
                    // a hidden window can't keep the keyboard
                    if !self.layout.main_preview_visible {
                        self.preview.focus = None;
                    }
                }
                Page::Downloads => {
                    self.layout.fonts_preview_visible = !self.layout.fonts_preview_visible;
                }
                Page::Settings => self.messages.info("This page has no preview"),
            },
            Action::ToggleSample => {
                use crate::fonts::FontFocus;
                self.layout.sample_visible = !self.layout.sample_visible;
                if !self.layout.sample_visible && self.fonts.focus == FontFocus::Sample {
                    self.fonts.focus = FontFocus::Slots;
                }
            }
            Action::ResizeMode => self.layout.resize_mode = true,
            Action::PaneLeft => self.move_pane(-1, 0),
            Action::PaneRight => self.move_pane(1, 0),
            Action::PaneUp => self.move_pane(0, -1),
            Action::PaneDown => self.move_pane(0, 1),
            Action::Panel(n) => self.focus_pane(n),
            _ => {}
        }
    }

    /// Actions for the theme and its parts (save, apply, open, new). Other
    /// actions do nothing.
    fn run_theme_action(&mut self, a: Action) {
        match a {
            Action::SavePalette => self.save(false),
            Action::SaveFonts => self.fonts_save(false),
            Action::ApplyPalette => self.apply(),
            Action::ApplyFonts => self.fonts_apply(),
            // s / S / a / o / n act on the theme (palette and fonts together)
            Action::Save => self.theme_save(false),
            Action::SaveAs => {
                self.overlay_for = For::Theme;
                self.overlay = Overlay::SaveAs {
                    input: self.theme.clone(),
                    overwrite: false,
                    then_apply: false,
                };
            }
            Action::Apply => self.theme_apply(),
            Action::Open => self.theme_open_overlay(),
            Action::New => self.guard(Pending::NewTheme),
            // a part of the theme: another palette or font preset, or a new one
            Action::OpenPalette => self.open_overlay(),
            Action::OpenFonts => self.fonts_open_overlay(),
            Action::NewPalette => self.guard(Pending::New),
            Action::NewFonts => self.guard(Pending::NewFonts),
            _ => {}
        }
    }

    /// Fonts-column actions: the focus order, filling the roles, the jump to
    /// the Downloads page, and the rest for the fonts tab itself.
    // Long function (reason verified, phase 3): one match with one arm per fonts-column action; the guards keep the original order.
    fn run_fonts(&mut self, a: Action) {
        use crate::fonts::FontFocus;

        match a {
            Action::FocusNext | Action::FocusPrev => self.fonts_cycle_panes(a),
            Action::FillFamily if self.fonts.focus == FontFocus::List => {
                match self.fonts.fill_from_family() {
                    Ok(fill) => self.after_fill(fill),
                    Err(e) => self.messages.info(e),
                }
            }
            Action::FixRoleFont if self.fonts.focus == FontFocus::Slots => {
                match self.fonts.fix_role() {
                    Ok(fill) => self.after_fill(fill),
                    Err(e) => self.messages.info(e),
                }
            }
            // picking, and no installed font fits the role: the Fonts page,
            // where one can be had
            Action::Confirm
                if self.fonts.focus == FontFocus::List
                    && self.fonts.search.is_empty()
                    && self.fonts.rows().is_empty()
                    && let Some(r) = self.fonts.picking =>
            {
                self.fonts.end_pick();
                self.fonts.focus = FontFocus::Slots;
                self.dl.tab = if r == 1 {
                    crate::fonts_dl::DlTab::Google
                } else {
                    crate::fonts_dl::DlTab::Nerd
                };
                self.dl.sel = 0;
                self.page = Page::Downloads;
            }
            // the top of the roles, the bottom of the font list (without the
            // sample under it): on to the page tabs, when they're that way
            Action::Up
                if self.fonts.focus == FontFocus::Slots
                    && self.fonts.slot == 0
                    && self.focus_page_tabs(true) => {}
            Action::Down
                if self.fonts.focus == FontFocus::List
                    && !(self.layout.sample_visible && self.layout.wide)
                    && self.fonts.list_sel + 1 >= self.fonts.rows().len()
                    && self.focus_page_tabs(false) => {}
            _ => self.fonts_act(a),
        }
    }

    /// Tab in the fonts column: roles, then list, then sample, then roles.
    fn fonts_cycle_panes(&mut self, a: Action) {
        use crate::fonts::FontFocus;
        let order: Vec<u8> = [1, 2, 3]
            .into_iter()
            .filter(|&p| p != 3 || (self.layout.sample_visible && self.layout.wide))
            .collect();
        let len = order.len();
        let i = order
            .iter()
            .position(|&p| p == self.current_pane())
            .unwrap_or(0);
        let next = if a == Action::FocusNext {
            order[(i + 1) % len]
        } else {
            order[(i + len - 1) % len]
        };
        self.focus_pane(next);
        if next == 1 {
            self.fonts.focus = FontFocus::Slots;
        }
    }

    /// Gives the action to the fonts tab. When a font was picked, saves and
    /// applies it.
    fn fonts_act(&mut self, a: Action) {
        let role = self.fonts.slot;
        self.fonts.act(a);
        if self.fonts.picked {
            let what = format!(
                "{}: {}",
                crate::fonts::ROLE_NAMES[role],
                self.fonts.preset.fonts[role].family
            );
            self.apply_picked_fonts(what);
        }
    }

    /// A font was picked (or a family filled the roles): save the theme's
    /// font preset and apply it straight away.
    pub(crate) fn apply_picked_fonts(&mut self, what: String) {
        self.fonts.picked = false;
        let name = if self.fonts.is_new {
            self.theme.clone()
        } else {
            self.fonts.name.clone()
        };
        let result = self
            .fonts
            .save_as(&name)
            .and_then(|_| self.fonts.store.apply(&name, self.fonts.preset.sizes()));
        match result {
            Ok(()) => {
                self.fonts.active = Some(name.clone());
                self.messages
                    .info(format!("{what}: saved to \"{name}\" and applied"));
            }
            Err(e) => self.messages.error(format!("{what}: {e:#}")),
        }
    }

    /// Colour-tab actions: the list, the picker and the preview toggles.
    // Long function: one match with one arm per colour-column action.
    fn run_colours(&mut self, a: Action) {
        match a {
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::Pin => self.toggle_pin(),
            Action::CopyHex => {
                let hex = self.editor.palette.get(self.key()).hex();
                self.copy(&hex);
            }
            Action::CopyFrom => {
                self.overlay = Overlay::CopyFrom { target: self.key() };
                self.focus = Focus::List;
            }
            Action::HexEntry => {
                self.focus = Focus::Hex;
                self.editor.hex_input = "#".into();
            }
            Action::FocusNext => self.cycle_focus(1),
            Action::FocusPrev => self.cycle_focus(-1),
            Action::Back if self.focus != Focus::List => self.leave_picker(true),
            Action::Confirm if self.focus != Focus::List => self.leave_picker(false),
            Action::ToggleYazi => {
                self.preview.mode = PreviewMode::Desktop;
                self.preview.toggle_yazi();
            }
            Action::ToggleRofi => {
                self.preview.mode = PreviewMode::Desktop;
                self.preview.show_rofi = !self.preview.show_rofi;
            }
            Action::ToggleView => {
                self.preview.mode = match self.preview.mode {
                    PreviewMode::Desktop => PreviewMode::Swatches,
                    PreviewMode::Swatches => PreviewMode::Desktop,
                };
            }
            _ if self.focus == Focus::List => self.list_action(a),
            _ => {
                if let Some((dx, dy, big)) = nudge_step(a) {
                    self.nudge(dx, dy, big);
                }
            }
        }
    }

    /// Moves the focused picker control: the square by saturation/value, the
    /// hue strip by hue, sliders by their value (up/down picks a slider).
    fn nudge(&mut self, dx: i32, dy: i32, big: bool) {
        match self.focus {
            Focus::Square => {
                let step = if big { 0.1 } else { 0.02 };
                let (h, s, v) = self.editor.hsv;
                self.set_hsv(h, s + dx as f64 * step, v - dy as f64 * step);
            }
            Focus::Hue => {
                let step = if big { 15.0 } else { 2.0 };
                let (h, s, v) = self.editor.hsv;
                self.set_hsv(h + (dy + dx) as f64 * step, s, v);
            }
            Focus::SliderH | Focus::SliderS | Focus::SliderL if dx != 0 => {
                let n = dx as f64 * if big { 10.0 } else { 1.0 };
                let (s, l) = self.editor.hsl(self.key());
                match self.focus {
                    Focus::SliderH => {
                        let (h, s, v) = self.editor.hsv;
                        self.set_hsv(h + n, s, v);
                    }
                    Focus::SliderS => self.set_hsl(s + n / 100.0, l),
                    _ => self.set_hsl(s, l + n / 100.0),
                }
            }
            Focus::SliderH | Focus::SliderS | Focus::SliderL => self.cycle_focus(dy),
            _ => {}
        }
    }

    /// While picking the key to copy a colour from: move, Enter copies, Esc cancels.
    fn run_copy_from(&mut self, target: &'static str, a: Action) {
        let on_key = matches!(
            self.colour_rows().get(self.row_sel),
            Some(ColourRow::Key(_))
        );
        match a {
            // a colour row: copy it; a group row: open or close it
            Action::Confirm | Action::Right if on_key => {
                let from = self.key();
                let c = self.editor.palette.get(from);
                self.overlay = Overlay::None;
                let to = self.keys.iter().position(|k| *k == target).unwrap_or(0);
                self.select(to);
                self.set_key(target, c);
                self.messages
                    .info(format!("{target} = {from} ({})", c.hex()));
            }
            Action::Back => self.overlay = Overlay::None,
            Action::Up
            | Action::Down
            | Action::Top
            | Action::Bottom
            | Action::PageUp
            | Action::PageDown
            | Action::Confirm
            | Action::Right
            | Action::Left => self.list_action(a),
            _ => {}
        }
    }

    fn fonts_apply(&mut self) {
        if self.fonts.dirty() {
            self.fonts_save(true);
        } else {
            let name = self.fonts.name.clone();
            self.fonts_save_as(&name, true);
        }
    }
}

/// Says if an action works on every page: quit, help, page switch, and the
/// hints and preview toggles.
///
/// `run_action` and `page_tabs_action` both use it, so the list is in one place.
pub(super) fn is_page_action(a: Action) -> bool {
    matches!(
        a,
        Action::Quit
            | Action::Help
            | Action::PageMain
            | Action::PageDownloads
            | Action::PageSettings
            | Action::ToggleHints
            | Action::TogglePreview
    )
}

/// Turns a move action of the picker into a step `(dx, dy, big)`: the
/// direction (-1, 0 or 1 on each axis) and `true` for the large step.
/// Returns `None` for any other action.
fn nudge_step(a: Action) -> Option<(i32, i32, bool)> {
    Some(match a {
        Action::Left => (-1, 0, false),
        Action::Right => (1, 0, false),
        Action::Up => (0, -1, false),
        Action::Down => (0, 1, false),
        Action::LeftBig => (-1, 0, true),
        Action::RightBig => (1, 0, true),
        Action::UpBig => (0, -1, true),
        Action::DownBig => (0, 1, true),
        _ => return None,
    })
}
