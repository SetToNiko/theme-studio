//! Drawing of the live desktop preview: the bar, the rofi menu and the two windows.
//!
//! The Preview pane (`ui/panes/preview.rs`) calls it for its desktop view.
//!
//! A model of the i3 desktop with real programs in it. The hidden terminals
//! (`Preview`, `Pane`) are in `crate::preview_term`; this file draws their screens
//! with the draft palette. The pane's frame and click targets are in
//! `panes/preview.rs`, not here.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::widgets::{f64_to_u16, to_u16};
use crate::model::color::Rgb;
use crate::model::palette::Palette;
use crate::model::store::TerminalMap;
use crate::preview_term::pane::{Pane, TermColors};
use crate::preview_term::{PaneId, Preview};
use crate::system::desktop::Roles;

/// The i3 border of window `id`: the number and name on it, and the colour
/// of the focused or unfocused client.
fn window_block(id: PaneId, focused: bool, palette: &Palette, roles: &Roles) -> Block<'static> {
    // column 4 is child_border, what a `border pixel` window shows
    let border = if focused {
        roles.get(palette, "client.focused", 4)
    } else {
        roles.get(palette, "client.unfocused", 4)
    };
    // Number and name on the border: 6 / 7 jump here (i3 itself shows no
    // title with `border pixel`; this is studio chrome).
    let (number, label) = match id {
        PaneId::Nvim => (6, "nvim"),
        PaneId::Shell => (7, "shell"),
        PaneId::Yazi => (7, "yazi"),
    };
    let label_fg = if focused {
        border
    } else {
        palette.get("grey1")
    };
    Block::bordered()
        .title(Line::from(vec![
            crate::ui::widgets::panel_number(number),
            Span::styled(format!("{label} "), Style::new().fg(label_fg.color())),
        ]))
        .border_style(
            Style::new()
                .fg(border.color())
                .bg(palette.get("bg_dim").color()),
        )
}

impl Preview {
    /// Draws the model desktop: two tiled windows with i3 borders, the bar,
    /// and optionally the rofi menu on top.
    pub fn draw_desktop(
        &mut self,
        f: &mut Frame,
        area: Rect,
        palette: &Palette,
        map: Option<&TerminalMap>,
        name: &str,
        windows: Windows,
    ) {
        let colors = TermColors::new(palette, map);

        // Desktop and bar
        let bar_area = Rect {
            y: area.y + area.height.saturating_sub(1),
            height: 1.min(area.height),
            ..area
        };
        let desk = Rect {
            height: area.height.saturating_sub(1),
            ..area
        };
        f.render_widget(
            Block::new().style(Style::new().bg(palette.get("bg_dim").color())),
            desk,
        );
        self.draw_bar(f, bar_area, palette);

        // Two windows: side by side when wide enough, else stacked
        let rects = split_windows(desk, windows.stack, windows.split).0;
        self.draw_windows(f, rects, palette, &colors);
        self.draw_errors(f, desk, palette);
        if self.show_rofi {
            self.draw_rofi(f, desk, palette, name);
        }
    }

    /// Draws the two windows in `(left_rect, right_rect)` and remembers where they are.
    fn draw_windows(
        &mut self,
        f: &mut Frame,
        (left_rect, right_rect): (Rect, Rect),
        palette: &Palette,
        colors: &TermColors,
    ) {
        let right_id = if self.show_yazi {
            PaneId::Yazi
        } else {
            PaneId::Shell
        };
        self.rects.clear();
        // The window with the keyboard looks focused; otherwise nvim does.
        let focused = self.focus.unwrap_or(PaneId::Nvim);
        for (id, rect) in [(PaneId::Nvim, left_rect), (right_id, right_rect)] {
            if rect.width < 4 || rect.height < 3 {
                continue;
            }
            self.draw_window(f, (id, rect), id == focused, palette, colors);
            self.rects.push((id, rect));
        }
    }

    /// Draws the error lines of the hidden terminals at the top of `desk`, in red.
    fn draw_errors(&self, f: &mut Frame, desk: Rect, palette: &Palette) {
        if self.errors.is_empty() {
            return;
        }
        let rect = Rect {
            height: to_u16(self.errors.len()).min(desk.height),
            ..desk
        };
        let lines: Vec<Line> = self
            .errors
            .iter()
            .map(|e| Line::styled(e.clone(), Style::new().fg(palette.get("red").color())))
            .collect();
        f.render_widget(Paragraph::new(lines), rect);
    }

    /// The screen of the hidden terminal `id`, if it runs.
    fn pane_for(&self, id: PaneId) -> Option<&Pane> {
        match id {
            PaneId::Nvim => self.nvim.as_ref(),
            PaneId::Shell => self.shell.as_ref(),
            PaneId::Yazi => self.yazi.as_ref(),
        }
    }

    /// Draws one window (`id` in `rect`): the i3 border with its number and
    /// name, then the screen of the hidden terminal inside it.
    fn draw_window(
        &mut self,
        f: &mut Frame,
        (id, rect): (PaneId, Rect),
        focused: bool,
        palette: &Palette,
        colors: &TermColors,
    ) {
        let block = window_block(id, focused, palette, &self.roles);
        let inner = block.inner(rect);
        f.render_widget(block, rect);
        self.ensure(id, inner.height, inner.width);
        if let Some(pane) = self.pane_mut(id) {
            pane.flush_pending();
        }
        f.render_widget(
            Block::new().style(Style::new().bg(colors.bg.color()).fg(colors.fg.color())),
            inner,
        );
        if let Some(pane) = self.pane_for(id) {
            pane.render(f.buffer_mut(), inner, colors, focused);
        } else if self.headless {
            f.render_widget(Paragraph::new(format!("{id:?}")), inner);
        }
    }

    /// The workspace buttons at the left end of the bar.
    fn bar_workspaces(&self, palette: &Palette) -> Vec<Span<'static>> {
        let role = |r: &str, i: usize| self.roles.get(palette, r, i);
        let button = |text: &'static str, kind: &str| {
            Span::styled(
                text,
                Style::new()
                    .bg(role(kind, 1).color())
                    .fg(role(kind, 2).color()),
            )
        };
        vec![
            button(" 1 ", "focused_workspace"),
            button(" 2 ", "inactive_workspace"),
            button(" 3 ", "inactive_workspace"),
        ]
    }

    /// The i3blocks status blocks at the right end of the bar.
    fn bar_blocks(&self, palette: &Palette) -> Vec<Span<'static>> {
        let role = |r: &str, i: usize| self.roles.get(palette, r, i);
        let statusline = role("statusline", 0);
        let sep = Span::styled(" │ ", Style::new().fg(role("separator", 0).color()));
        let mut right: Vec<Span> = Vec::new();
        match &*self.bar.lock().unwrap() {
            Some(blocks) => {
                for (i, block) in blocks.iter().enumerate() {
                    if i > 0 {
                        right.push(sep.clone());
                    }
                    right.push(Span::styled(
                        block.text.clone(),
                        Style::new().fg(block.color.unwrap_or(statusline).color()),
                    ));
                }
            }
            None => right.push(Span::styled(
                "reading i3blocks…",
                Style::new().fg(role("separator", 0).color()),
            )),
        }
        right.push(" ".into());
        right
    }

    fn draw_bar(&self, f: &mut Frame, area: Rect, palette: &Palette) {
        let bg = self.roles.get(palette, "background", 0);
        let statusline = self.roles.get(palette, "statusline", 0);
        let mut left = self.bar_workspaces(palette);
        let right = self.bar_blocks(palette);
        let used: usize = left.iter().chain(&right).map(Span::width).sum();
        left.push(
            " ".repeat(usize::from(area.width).saturating_sub(used))
                .into(),
        );
        left.extend(right);
        f.render_widget(
            Paragraph::new(Line::from(left))
                .style(Style::new().bg(bg.color()).fg(statusline.color())),
            area,
        );
    }

    /// A rofi window with a neutral list of entries, drawn with this palette's rofi roles.
    // Long function (reason verified, phase 1): one linear drawing sequence (window, swatches, row, lines) that shares the role colours.
    fn draw_rofi(&self, f: &mut Frame, area: Rect, palette: &Palette, name: &str) {
        let role = |r: &str| self.roles.get(palette, &format!("rofi.{r}"), 0);
        let width = 54.min(area.width);
        let height = 8.min(area.height);
        let rect = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };
        let (bg, fg) = (role("background"), role("foreground"));
        f.render_widget(Clear, rect);
        let block = Block::bordered()
            .border_style(Style::new().fg(role("border-color").color()).bg(bg.color()))
            .style(Style::new().bg(bg.color()).fg(fg.color()));
        let inner = block.inner(rect);
        f.render_widget(block, rect);
        let row = |label: &str, sel: bool| {
            let style = if sel {
                Style::new()
                    .bg(role("selected-normal-background").color())
                    .fg(role("selected-normal-foreground").color())
            } else {
                Style::new().fg(fg.color())
            };
            let mut spans = vec![Span::styled(
                format!(" {label:<22}"),
                style.add_modifier(Modifier::BOLD),
            )];
            // only the palette being edited has known colours; the other
            // rows are just names
            if sel {
                spans.extend(rofi_swatches(palette, role("selected-normal-background")));
            }
            spans.push(Span::styled(" ".repeat(usize::from(inner.width)), style));
            Line::from(spans)
        };
        let lines = vec![
            Line::from(vec![
                Span::styled(" Search: ", Style::new().fg(fg.color())),
                Span::styled("▏", Style::new().fg(fg.color())),
            ]),
            Line::styled(
                "─".repeat(usize::from(inner.width)),
                Style::new().fg(role("separatorcolor").color()),
            ),
            row(name, true),
            row("Gruvbox Dark", false),
            row("Nord", false),
            row("Tokyo Night", false),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }
}

/// The palette's colours as swatches on the selected rofi row (`selected_bg`).
fn rofi_swatches(palette: &Palette, selected_bg: Rgb) -> Vec<Span<'static>> {
    [
        "bg0", "fg0", "red", "orange", "yellow", "green", "aqua", "blue", "purple",
    ]
    .iter()
    .map(|k| {
        Span::styled(
            "██",
            Style::new()
                .fg(palette.get(k).color())
                .bg(selected_bg.color()),
        )
    })
    .collect()
}

/// How nvim and the shell share the preview.
#[derive(Clone, Copy, Debug)]
pub struct Windows {
    /// One above the other (a preview column) rather than side by side.
    pub stack: bool,
    /// nvim's share of the space.
    pub split: f64,
}

/// The preview's desktop inside its frame (above the bar), for a preview
/// pane at `area`.
pub fn desk_area(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(3),
    }
}

/// nvim's and the shell's windows: one above the other when `stack` (or
/// when they wouldn't fit side by side), `share` of the space for nvim.
/// Returns the two rects and whether they're stacked.
pub fn split_windows(desk: Rect, stack: bool, share: f64) -> ((Rect, Rect), bool) {
    let share = share.clamp(0.0, 1.0);
    if !stack && desk.width >= 2 * 30 {
        (side_by_side(desk, share), false)
    } else {
        (stacked(desk, share), true)
    }
}

/// `desk` cut into a left window of `share` of the width and a right window.
fn side_by_side(desk: Rect, share: f64) -> (Rect, Rect) {
    let left_w = f64_to_u16(f64::from(desk.width) * share).clamp(10, desk.width - 10);
    (
        Rect {
            width: left_w,
            ..desk
        },
        Rect {
            x: desk.x + left_w,
            width: desk.width - left_w,
            ..desk
        },
    )
}

/// `desk` cut into a top window of `share` of the height and a bottom window.
fn stacked(desk: Rect, share: f64) -> (Rect, Rect) {
    let top_h = if desk.height >= 6 {
        f64_to_u16(f64::from(desk.height) * share).clamp(3, desk.height - 3)
    } else {
        desk.height / 2
    };
    (
        Rect {
            height: top_h,
            ..desk
        },
        Rect {
            y: desk.y + top_h,
            height: desk.height - top_h,
            ..desk
        },
    )
}
