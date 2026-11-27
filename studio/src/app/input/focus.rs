//! App's methods for pane focus: which pane has the keyboard and how it moves.
//!
//! This is a second `impl App { ... }` block. Rust allows an `impl` of a type
//! in another file of the same crate. `app/mod.rs` has the other `App`
//! methods and the struct itself.
//!
//! The numbered panes are the font roles, font list, font sample, colour
//! list, colour picker, and the two preview windows. This file also has the
//! page tabs focus, the keymap contexts that follow the focus, and the
//! resize-mode keys (they move the borders of the focused pane).

use crate::app::App;
use crate::app::state::nav::{Focus, Page, Side};
use crate::app::state::steps::offset;
use crate::keymap::Context;
use crate::keymap::action::{Action, ActionGroup};
use crate::preview_term::types::{Mode as PreviewMode, PaneId};

impl App {
    /// Starts with the font roles focused (`--fonts`).
    pub fn open_fonts_tab(&mut self) {
        self.focus_pane(1);
    }

    /// Moving off the top (`up`) or bottom of the page: the keyboard goes to
    /// the page tabs if they're that way (settings: layout.tabs). True if so.
    pub fn focus_page_tabs(&mut self, up: bool) -> bool {
        if up == self.settings.layout.tabs_on_top() {
            self.layout.page_tabs_focused = true;
        }
        self.layout.page_tabs_focused
    }

    /// Keys with the keyboard on the page tabs: left / right change pages
    /// (the keyboard stays on the tabs); back towards the page, Enter or Esc
    /// return to it. True if handled here.
    pub(in crate::app) fn page_tabs_action(&mut self, a: Action) -> bool {
        const PAGES: [Page; 3] = [Page::Main, Page::Downloads, Page::Settings];
        let top = self.settings.layout.tabs_on_top();
        let i = PAGES.iter().position(|p| *p == self.page).unwrap_or(0);
        match a {
            Action::Left | Action::TabPrev => self.page = PAGES[(i + 2) % 3],
            Action::Right | Action::TabNext => self.page = PAGES[(i + 1) % 3],
            Action::Down if top => self.layout.page_tabs_focused = false,
            Action::Up if !top => self.layout.page_tabs_focused = false,
            Action::Confirm | Action::Back => self.layout.page_tabs_focused = false,
            // page keys, quitting and help work as anywhere
            _ if a.group() == ActionGroup::Page => return false,
            _ => {}
        }
        true
    }

    /// Number keys jump between the numbered panes: ¹ font roles, ² font
    /// list, ³ font sample, ⁴ colour list, ⁵ colour picker, ⁶ nvim, ⁷ shell
    /// (or yazi). 6 and 7 give that preview window the keyboard; F12 hands it
    /// back. A hidden picker or sample is shown.
    pub fn focus_pane(&mut self, n: u8) {
        use crate::fonts::types::FontFocus;
        self.page = Page::Main;
        match n {
            1..=3 => {
                self.preview.focus = None;
                self.side = Side::Fonts;
                match n {
                    1 => self.fonts.focus = FontFocus::Slots,
                    2 => self.fonts.open_list(),
                    _ if !self.layout.wide => {} // not drawn at this width
                    _ => {
                        self.layout.sample_visible = true;
                        self.fonts.focus = FontFocus::Sample;
                    }
                }
            }
            4 => {
                self.preview.focus = None;
                self.side = Side::Colours;
                if self.focus != Focus::List {
                    self.leave_picker(true); // only Enter keeps a change
                }
            }
            5 => {
                self.preview.focus = None;
                self.open_picker();
            }
            6 | 7 => self.focus_preview_pane(n),
            _ => {}
        }
    }

    /// Gives a preview window the keyboard: 6 is nvim, 7 is the shell (or yazi).
    ///
    /// A hidden preview is shown again. If that window is not shown at this
    /// window size, the keyboard stays where it is and a message says so.
    fn focus_preview_pane(&mut self, n: u8) {
        // jumping to a hidden preview shows it again
        if !self.layout.main_preview_visible {
            self.layout.main_preview_visible = true;
            self.preview.rects.clear();
        }
        self.preview.mode = PreviewMode::Desktop;
        let id = match (n, self.preview.show_yazi) {
            (6, _) => PaneId::Nvim,
            (_, true) => PaneId::Yazi,
            _ => PaneId::Shell,
        };
        // the lower slot counts whether it last showed the shell or yazi
        let shown = self
            .preview
            .rects
            .iter()
            .any(|(p, _)| *p == id || (n == 7 && *p != PaneId::Nvim));
        if !shown && !self.preview.rects.is_empty() {
            self.messages
                .info("That preview window isn't shown at this window size");
            return;
        }
        self.preview.focus = Some(id);
    }

    /// The number of the pane with the keyboard.
    pub fn current_pane(&self) -> u8 {
        use crate::fonts::types::FontFocus;
        match (self.preview.focus, self.side) {
            (Some(PaneId::Nvim), _) => 6,
            (Some(_), _) => 7,
            (None, Side::Fonts) => match self.fonts.focus {
                FontFocus::Slots => 1,
                FontFocus::List | FontFocus::Search => 2,
                FontFocus::Sample => 3,
            },
            (None, Side::Colours) if self.focus == Focus::List => 4,
            (None, Side::Colours) => 5,
        }
    }

    /// `<C-w>h/j/k/l`: the neighbouring pane in the fonts and colour columns
    /// (the preview windows take the keyboard, so they're only entered on
    /// purpose: 6, 7, F12 or a click). Hidden panes are skipped.
    pub(in crate::app) fn move_pane(&mut self, dx: i32, dy: i32) {
        let columns = self.layout.pane_columns();
        let cur = self.current_pane();
        let Some((column, row)) = columns.iter().enumerate().find_map(|(column, entries)| {
            entries
                .iter()
                .position(|&p| p == cur)
                .map(|row| (column, row))
        }) else {
            return;
        };
        let next_column = offset(column, dx).min(1);
        let entries = &columns[next_column];
        let next_row = if dx != 0 {
            row.min(entries.len() - 1)
        } else {
            offset(row, dy).min(entries.len() - 1)
        };
        let target = entries[next_row];
        if target != cur {
            self.focus_pane(target);
        }
    }

    /// Shows the picker and gives it the keyboard (editing a colour).
    pub(in crate::app) fn open_picker(&mut self) {
        self.side = Side::Colours;
        self.layout.picker_visible = true;
        if self.focus == Focus::List {
            self.focus = Focus::Square;
        }
    }

    /// Back to the colour list from the picker: Esc (`discard`) puts back what
    /// the colours were when you started changing them, Enter keeps them. The
    /// picker hides again unless Settings says it stays open.
    pub(in crate::app) fn leave_picker(&mut self, discard: bool) {
        if self.editor.end_session(self.key(), discard) {
            self.messages
                .info(format!("Discarded the change to {}", self.key()));
        }
        self.focus = Focus::List;
        if self.settings.colours.picker_hides_on_esc() {
            self.layout.picker_visible = false;
        }
    }

    /// The keymap contexts for the pane with the keyboard, most specific first.
    pub fn contexts(&self) -> [Context; 3] {
        use crate::fonts::types::FontFocus;
        if self.layout.resize_mode {
            return [Context::Resize; 3];
        }
        match self.page {
            Page::Main => {}
            Page::Settings => return [Context::Settings, Context::Global, Context::Global],
            Page::Downloads => return [Context::Downloads, Context::Global, Context::Global],
        }
        match self.side {
            Side::Fonts => match self.fonts.focus {
                FontFocus::Slots => [Context::FontRoles, Context::Fonts, Context::Global],
                FontFocus::List | FontFocus::Search => {
                    [Context::FontList, Context::Fonts, Context::Global]
                }
                FontFocus::Sample => [Context::Fonts, Context::Global, Context::Global],
            },
            Side::Colours if self.focus == Focus::List => {
                [Context::ColourList, Context::Colours, Context::Global]
            }
            Side::Colours => [Context::Picker, Context::Colours, Context::Global],
        }
    }

    // ----- resizing --------------------------------------------------------

    /// Resize-mode keys: h/l move the focused pane's side edge, j/k its lower
    /// edge, by one cell (capitals: five); = resets; Esc / Enter leave.
    // Long function (reason verified, phase 3): one match from the action to a step size, then the border move.
    pub(in crate::app) fn run_resize(&mut self, action: Action) {
        let (dx, dy, step): (i32, i32, i32) = match action {
            Action::Left => (-1, 0, 1),
            Action::Right => (1, 0, 1),
            Action::Up => (0, -1, 1),
            Action::Down => (0, 1, 1),
            Action::LeftBig => (-1, 0, 5),
            Action::RightBig => (1, 0, 5),
            Action::UpBig => (0, -1, 5),
            Action::DownBig => (0, 1, 5),
            Action::ResetSizes => {
                self.layout.sizes = self.settings.layout.sizes.clone();
                return;
            }
            Action::Back | Action::Confirm | Action::ResizeMode => {
                self.layout.resize_mode = false;
                return;
            }
            _ => return,
        };
        let vertical = dx != 0;
        let Some(id) = self.layout.border_for_pane(self.current_pane(), vertical) else {
            return;
        };
        let border = self
            .layout
            .borders
            .iter()
            .find(|border| border.id == id)
            .copied()
            .unwrap();
        let pos =
            u16::try_from((i32::from(border.pos()) + (dx + dy) * step).max(0)).unwrap_or(u16::MAX);
        self.layout.move_border(id, pos);
    }
}
