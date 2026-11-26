//! App's methods for the mouse.
//!
//! This is a second `impl App { ... }` block. Rust allows an `impl` of a type
//! in another file of the same crate. `app/mod.rs` has the other `App`
//! methods and the struct itself.
//!
//! [`App::on_mouse`] is the entry point. It sends the event to
//! `handle_mouse`, which checks the notice box and the pop-ups first, then
//! sends the event by kind: scroll, left button down, drag, or button up. A
//! click goes to `click`, which looks at the target under the pointer
//! ([`crate::app::state::hit::Target`]) and calls one method for that kind of target.

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use crate::app::state::hit::Target as T;
use crate::app::state::nav::{ColourRow, Focus, Overlay, Side};
use crate::app::{App, PreviewMode};
use crate::keymap::action::Action;

/// Where `v` lies between `start` and the end of a control that is `len` cells long (0.0 to 1.0).
fn frac(v: u16, start: u16, len: u16) -> f64 {
    if len <= 1 {
        0.0
    } else {
        (f64::from(v.saturating_sub(start)) / f64::from(len - 1)).clamp(0.0, 1.0)
    }
}

impl App {
    /// Handles one mouse event, then keeps the palette preview and the picker session in step.
    pub(in crate::app) fn on_mouse(&mut self, m: MouseEvent) {
        self.handle_mouse(m);
        self.sync_palette_preview();
        self.end_session_outside_picker();
    }

    /// Sends a mouse event to the scroll, click, drag, or button-up handler.
    ///
    /// A notice box or an open pop-up takes the event first.
    // Long function (reason verified, phase 3): one match over the mouse event kinds, after the notice and pop-up checks.
    fn handle_mouse(&mut self, m: MouseEvent) {
        let (x, y) = (m.column, m.row);
        self.hits.mouse = Some((x, y));
        // a save confirmation box: a click anywhere closes it
        if self.messages.notice.is_some() {
            if matches!(m.kind, MouseEventKind::Down(_)) {
                self.messages.notice = None;
            }
            return;
        }
        let target = self.hits.at(x, y);
        // With a pop-up open (other than "copy from"), only the pop-up responds.
        let popup = !matches!(self.overlay, Overlay::None | Overlay::CopyFrom { .. });
        if popup && !matches!(target, Some((_, T::OverlayRow(_) | T::ConfirmChoice(_)))) {
            // a click outside a dropdown closes it
            if matches!(self.overlay, Overlay::Menu { .. })
                && matches!(m.kind, MouseEventKind::Down(_))
            {
                self.overlay = Overlay::None;
            }
            return;
        }
        match m.kind {
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let d = if m.kind == MouseEventKind::ScrollDown {
                    1
                } else {
                    -1
                };
                self.mouse_scroll(target.map(|(_, t)| t), d);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // a click outside a preview window gives the keyboard back
                self.layout.page_tabs_focused = false;
                if !matches!(target, Some((_, T::Preview(_)))) {
                    self.preview.focus = None;
                }
                if let Some((r, t)) = target {
                    self.click(r, t, x, y, false);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) if self.layout.dragging.is_some() => {
                self.drag_border(x, y);
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // dragging works on the picker's controls only
                if let Some((r, t @ (T::Square | T::Hue | T::SliderH | T::SliderS | T::SliderL))) =
                    target
                {
                    self.click(r, t, x, y, true);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.editor.end_edit_run();
                self.layout.dragging = None;
            }
            _ => {}
        }
    }

    /// Scrolls the list under the pointer by `d` rows (1 is down, -1 is up).
    fn mouse_scroll(&mut self, target: Option<T>, d: i32) {
        match target {
            Some(T::ColourRow(_)) => self.move_row(d),
            Some(T::FontRow(_)) => self.fonts.scroll(d),
            Some(t @ (T::SettingsRow(_) | T::SettingsDiff)) => self.settings_scroll(t, d),
            Some(T::DlRow(_)) => self.run_downloads(if d > 0 { Action::Down } else { Action::Up }),
            Some(T::DlPreview) => self.preview_scroll(d),
            _ => {}
        }
    }

    /// Moves the pane border that the user drags to the pointer position.
    fn drag_border(&mut self, x: u16, y: u16) {
        let Some(id) = self.layout.dragging else {
            return;
        };
        let vertical = self
            .layout
            .borders
            .iter()
            .find(|b| b.id == id)
            .is_some_and(|b| b.vertical);
        self.layout.move_border(id, if vertical { x } else { y });
    }

    /// A click (or a drag step) on a target in `area`.
    ///
    /// Picks the method for the kind of target. Short targets are done here.
    // Long function (reason verified, phase 3): one match with one arm per click target kind.
    fn click(&mut self, area: Rect, target: T, column: u16, line: u16, drag: bool) {
        match target {
            T::PageTab(page) => self.page = page,
            T::Border(id) => self.layout.dragging = Some(id),
            T::PaneTitle(pane) => self.focus_pane(pane),
            T::PaneBody(3) => self.focus_pane(3),
            T::PaneBody(_) | T::DlPreview => {}
            T::Preview(id) => {
                self.preview.mode = PreviewMode::Desktop;
                self.preview.focus = Some(id);
            }
            T::ColourRow(row) => self.click_colour_row(row),
            T::Square | T::Hue | T::SliderH | T::SliderS | T::SliderL | T::Hex => {
                self.click_picker_control(area, target, column, line, drag);
            }
            T::FontRole(_) | T::FontRow(_) | T::FontSearch | T::Chip(_) => self.click_fonts(target),
            T::OverlayRow(index) => self.click_overlay_row(index),
            T::SettingsTab(_)
            | T::SettingsRow(_)
            | T::HistFile(_)
            | T::SettingsDiff
            | T::PresetRow(_) => self.settings_click(target),
            T::ThemeName => self.theme_open_overlay(),
            T::PaletteMenu => {
                self.focus_pane(4);
                self.layout.palette_menu_focused = true;
                self.open_overlay();
            }
            T::DlTab(_) | T::DlRow(_) | T::DlChip(_) | T::DlSearch | T::DlButton(_) => {
                self.dl_click(target);
            }
            T::PreviewMenu(menu) => self.open_menu(menu, area),
            T::TourPane(index) => self.tour_click_pane(index),
            T::TourRow(index) => self.tour_click_row(index),
            T::SettingsButton('s') => self.settings_save(),
            T::SettingsButton(_) => self.settings_discard(),
            T::ConfirmChoice(choice) => {
                let code = if choice == 'n' {
                    KeyCode::Esc
                } else {
                    KeyCode::Char(choice)
                };
                self.on_overlay_key(KeyEvent::new(code, KeyModifiers::NONE));
            }
        }
    }

    /// A click on a row of the colour list: selects it, and a second click opens it.
    fn click_colour_row(&mut self, row: usize) {
        self.layout.palette_menu_focused = false;
        let copying = matches!(self.overlay, Overlay::CopyFrom { .. });
        let again = row == self.row_sel && self.focus == Focus::List && self.side == Side::Colours;
        self.side = Side::Colours;
        self.focus = Focus::List;
        self.row_sel = row;
        match self.colour_rows().get(row).copied() {
            Some(ColourRow::Group { index, expanded }) if again => {
                self.set_group_open(index, !expanded);
            }
            Some(ColourRow::Key(k)) => {
                self.select_key(k);
                if again && !copying {
                    self.open_picker();
                }
            }
            _ => {}
        }
    }

    /// A click or drag step on a picker control: the square, the hue bar, a slider, or the hex box.
    ///
    /// The position inside the control at `area` sets the colour. `drag` is true for a drag step.
    // Long function (reason verified, phase 3): one match over the five picker controls, which share the focus and colour update.
    fn click_picker_control(&mut self, area: Rect, target: T, column: u16, line: u16, drag: bool) {
        self.side = Side::Colours;
        match target {
            T::Square => {
                self.focus = Focus::Square;
                let sat = frac(column, area.x, area.width);
                let val = 1.0 - frac(line, area.y, area.height.max(2));
                self.set_hsv(self.editor.hsv.0, sat, val);
            }
            T::Hue => {
                self.focus = Focus::Hue;
                let (_, sat, val) = self.editor.hsv;
                self.set_hsv(frac(line, area.y, area.height.max(2)) * 360.0, sat, val);
            }
            T::SliderH => {
                self.focus = Focus::SliderH;
                let (_, sat, val) = self.editor.hsv;
                self.set_hsv(frac(column, area.x, area.width) * 360.0, sat, val);
            }
            T::SliderS | T::SliderL => {
                self.focus = if target == T::SliderS {
                    Focus::SliderS
                } else {
                    Focus::SliderL
                };
                let (sat, light) = self.editor.hsl(self.key());
                let along = frac(column, area.x, area.width);
                if target == T::SliderS {
                    self.set_hsl(along, light);
                } else {
                    self.set_hsl(sat, along);
                }
            }
            T::Hex if !drag => {
                self.focus = Focus::Hex;
                self.editor.hex_input = "#".into();
            }
            _ => {}
        }
    }

    /// A click on the fonts tab: a role, a font row, the search box, or a filter chip.
    fn click_fonts(&mut self, t: T) {
        use crate::fonts::types::FontFocus;
        self.side = Side::Fonts;
        match t {
            T::FontRole(i) => self.fonts.click_role(i),
            T::FontRow(i) => {
                let role = self.fonts.slot;
                self.fonts.click_row(i);
                if self.fonts.picked {
                    let what = format!(
                        "{}: {}",
                        crate::fonts::types::ROLE_NAMES[role],
                        self.fonts.preset.fonts[role].family
                    );
                    self.apply_picked_fonts(&what);
                }
            }
            T::FontSearch => self.fonts.focus = FontFocus::Search,
            T::Chip(c) => self.fonts.click_chip(c),
            _ => {}
        }
    }

    /// A click on a row of the open pop-up (key chooser, open list, or dropdown menu).
    fn click_overlay_row(&mut self, i: usize) {
        if let Overlay::ChooseKey { row, sel, original } = &self.overlay {
            let (row, sel, original) = (*row, *sel, original.clone());
            // a click previews the key; a second click keeps it
            let key = crate::settings_page::behaviour::theme_keys()[i];
            if sel == i {
                self.overlay = Overlay::None;
            } else {
                self.overlay = Overlay::ChooseKey {
                    row,
                    sel: i,
                    original,
                };
            }
            self.with_draft(|app| app.set_colour_setting(row, key));
        } else if let Overlay::Open { sel, .. } = &mut self.overlay {
            if *sel == i {
                // a second click opens it, like Enter
                self.on_overlay_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            } else {
                *sel = i;
            }
        } else if let Overlay::Menu { menu, .. } = self.overlay {
            // one click chooses
            self.choose_menu(menu, i);
        }
    }
}
