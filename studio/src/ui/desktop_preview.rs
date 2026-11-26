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

use crate::model::palette::Palette;
use crate::model::store::TerminalMap;
use crate::preview_term::pane::TermColors;
use crate::preview_term::{PaneId, Preview};

impl Preview {
    /// Draws the model desktop: two tiled windows with i3 borders, the bar,
    /// and optionally the rofi menu on top.
    pub fn draw_desktop(
        &mut self,
        f: &mut Frame,
        area: Rect,
        p: &Palette,
        map: Option<&TerminalMap>,
        name: &str,
        windows: Windows,
    ) {
        let colors = TermColors::new(p, map);
        let roles = self.roles.clone();
        let role = |r: &str, i: usize| roles.get(p, r, i);

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
            Block::new().style(Style::new().bg(p.get("bg_dim").color())),
            desk,
        );
        self.draw_bar(f, bar_area, p);

        // Two windows: side by side when wide enough, else stacked
        let right_id = if self.show_yazi {
            PaneId::Yazi
        } else {
            PaneId::Shell
        };
        let (a, b) = split_windows(desk, windows.stack, windows.split).0;
        self.rects.clear();
        // The window with the keyboard looks focused; otherwise nvim does.
        let focused = self.focus.unwrap_or(PaneId::Nvim);
        for (id, r) in [(PaneId::Nvim, a), (right_id, b)] {
            if r.width < 4 || r.height < 3 {
                continue;
            }
            // column 4 is child_border, what a `border pixel` window shows
            let border = if id == focused {
                role("client.focused", 4)
            } else {
                role("client.unfocused", 4)
            };
            // Number and name on the border: 6 / 7 jump here (i3 itself shows no
            // title with `border pixel`; this is studio chrome).
            let (n, label) = match id {
                PaneId::Nvim => (6, "nvim"),
                PaneId::Shell => (7, "shell"),
                PaneId::Yazi => (7, "yazi"),
            };
            let label_fg = if id == focused {
                border
            } else {
                p.get("grey1")
            };
            let block = Block::bordered()
                .title(Line::from(vec![
                    crate::ui::widgets::panel_number(n),
                    Span::styled(format!("{label} "), Style::new().fg(label_fg.color())),
                ]))
                .border_style(Style::new().fg(border.color()).bg(p.get("bg_dim").color()));
            let inner = block.inner(r);
            f.render_widget(block, r);
            self.ensure(id, inner.height, inner.width);
            if let Some(pane) = self.pane_mut(id) {
                pane.flush_pending();
            }
            f.render_widget(
                Block::new().style(Style::new().bg(colors.bg.color()).fg(colors.fg.color())),
                inner,
            );
            if let Some(pane) = match id {
                PaneId::Nvim => self.nvim.as_ref(),
                PaneId::Shell => self.shell.as_ref(),
                PaneId::Yazi => self.yazi.as_ref(),
            } {
                pane.render(f.buffer_mut(), inner, &colors, id == focused);
            } else if self.headless {
                f.render_widget(Paragraph::new(format!("{id:?}")), inner);
            }
            self.rects.push((id, r));
        }
        if !self.errors.is_empty() {
            let r = Rect {
                height: (self.errors.len() as u16).min(desk.height),
                ..desk
            };
            let lines: Vec<Line> = self
                .errors
                .iter()
                .map(|e| Line::styled(e.clone(), Style::new().fg(p.get("red").color())))
                .collect();
            f.render_widget(Paragraph::new(lines), r);
        }
        if self.show_rofi {
            self.draw_rofi(f, desk, p, name);
        }
    }

    fn draw_bar(&self, f: &mut Frame, area: Rect, p: &Palette) {
        let role = |r: &str, i: usize| self.roles.get(p, r, i);
        let bg = role("background", 0);
        let mut left = vec![
            Span::styled(
                " 1 ",
                Style::new()
                    .bg(role("focused_workspace", 1).color())
                    .fg(role("focused_workspace", 2).color()),
            ),
            Span::styled(
                " 2 ",
                Style::new()
                    .bg(role("inactive_workspace", 1).color())
                    .fg(role("inactive_workspace", 2).color()),
            ),
            Span::styled(
                " 3 ",
                Style::new()
                    .bg(role("inactive_workspace", 1).color())
                    .fg(role("inactive_workspace", 2).color()),
            ),
        ];
        let statusline = role("statusline", 0);
        let sep = Span::styled(" │ ", Style::new().fg(role("separator", 0).color()));
        let mut right: Vec<Span> = Vec::new();
        match &*self.bar.lock().unwrap() {
            Some(blocks) => {
                for (i, b) in blocks.iter().enumerate() {
                    if i > 0 {
                        right.push(sep.clone());
                    }
                    right.push(Span::styled(
                        b.text.clone(),
                        Style::new().fg(b.color.unwrap_or(statusline).color()),
                    ));
                }
            }
            None => right.push(Span::styled(
                "reading i3blocks…",
                Style::new().fg(role("separator", 0).color()),
            )),
        }
        right.push(" ".into());
        let used: usize = left.iter().chain(&right).map(|s| s.width()).sum();
        left.push(
            " ".repeat((area.width as usize).saturating_sub(used))
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
    fn draw_rofi(&self, f: &mut Frame, area: Rect, p: &Palette, name: &str) {
        let role = |r: &str| self.roles.get(p, &format!("rofi.{r}"), 0);
        let w = 54.min(area.width);
        let h = 8.min(area.height);
        let r = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        let (bg, fg) = (role("background"), role("foreground"));
        f.render_widget(Clear, r);
        let block = Block::bordered()
            .border_style(Style::new().fg(role("border-color").color()).bg(bg.color()))
            .style(Style::new().bg(bg.color()).fg(fg.color()));
        let inner = block.inner(r);
        f.render_widget(block, r);
        let swatches = |sel: bool| -> Vec<Span<'static>> {
            [
                "bg0", "fg0", "red", "orange", "yellow", "green", "aqua", "blue", "purple",
            ]
            .iter()
            .map(|k| {
                Span::styled(
                    "██",
                    Style::new().fg(p.get(k).color()).bg(if sel {
                        role("selected-normal-background")
                    } else {
                        bg
                    }
                    .color()),
                )
            })
            .collect()
        };
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
                spans.extend(swatches(sel));
            }
            spans.push(Span::styled(" ".repeat(inner.width as usize), style));
            Line::from(spans)
        };
        let lines = vec![
            Line::from(vec![
                Span::styled(" Search: ", Style::new().fg(fg.color())),
                Span::styled("▏", Style::new().fg(fg.color())),
            ]),
            Line::styled(
                "─".repeat(inner.width as usize),
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
        let w = ((desk.width as f64 * share) as u16).clamp(10, desk.width - 10);
        (
            (
                Rect { width: w, ..desk },
                Rect {
                    x: desk.x + w,
                    width: desk.width - w,
                    ..desk
                },
            ),
            false,
        )
    } else {
        let h = if desk.height >= 6 {
            ((desk.height as f64 * share) as u16).clamp(3, desk.height - 3)
        } else {
            desk.height / 2
        };
        (
            (
                Rect { height: h, ..desk },
                Rect {
                    y: desk.y + h,
                    height: desk.height - h,
                    ..desk
                },
            ),
            true,
        )
    }
}
