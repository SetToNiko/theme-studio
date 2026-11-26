//! The Preview pane: the live desktop, or the swatch view.
//!
//! The live desktop (nvim, shell or yazi, i3 borders and bar, rofi) is drawn
//! by `crate::ui::desktop_preview`; this file draws its frame and the click targets.
//! The swatch view (V) lists every key as a swatch plus static samples.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::app::state::hit::Target;
use crate::keymap::Context;
use crate::keymap::action::Action;
use crate::model::palette::{self, GROUPS, Palette};
use crate::preview_term::Mode as PreviewMode;
use crate::ui::widgets::friendly;
use crate::ui::{focused_block, swatch};

/// A line of sample code: (text, palette key) tokens, and an optional line background key.
type CodeLine<'a> = (Vec<(&'a str, &'a str)>, Option<&'a str>);

/// The live desktop (nvim, shell or yazi, i3 borders and bar, rofi), or the
/// swatch view.
/// `stack`: nvim above the shell (a preview column); otherwise side by side.
pub fn draw_preview(f: &mut Frame, app: &mut App, area: Rect, stack: bool) {
    if app.preview.mode == PreviewMode::Swatches {
        return draw_swatches(f, app, area);
    }
    // While nvim or the shell has the keyboard the frame turns magenta (your
    // palette's purple, as the terminal shows it): keys go there, not to the
    // studio, until F12 hands them back.
    let capture = Style::new().magenta();
    let title = preview_title(app, capture);
    let block = match app.preview.focus {
        Some(_) => Block::bordered().title(title).border_style(capture.bold()),
        None => focused_block(app, area, title, false),
    };
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.preview.draw_desktop(
        f,
        inner,
        &app.editor.palette,
        app.terminal.as_ref(),
        &app.name,
        crate::ui::desktop_preview::Windows {
            stack,
            split: app.layout.sizes.preview_split,
        },
    );
    add_window_hits(f, app);
}

/// The frame title: key hints, or which window has the keyboard.
fn preview_title(app: &App, capture: Style) -> Line<'static> {
    let key = |a: Action| {
        app.keymap
            .key_for(&[Context::Global], a)
            .map(|k| friendly(&k))
            .unwrap_or_default()
    };
    match app.preview.focus {
        Some(id) => Line::from(vec![
            Span::styled(" Preview: typing into ", capture),
            Span::styled(format!("{id:?}"), capture.bold()),
            Span::styled(
                format!(
                    "  {} next window / back to the studio ",
                    key(Action::PreviewCycle)
                ),
                capture,
            ),
        ]),
        None => Line::raw(format!(
            " Preview  {} / {}, {} or click: use a window   Y yazi   R rofi   V swatches ",
            key(Action::Panel(6)),
            key(Action::Panel(7)),
            key(Action::PreviewCycle)
        )),
    }
}

/// nvim and the shell are clickable (a click hands them the keyboard);
/// hovering one lights its frame in the capture colour.
fn add_window_hits(f: &mut Frame, app: &mut App) {
    let rects = app.preview.rects.clone();
    for (id, r) in rects {
        if app.hits.add(r, Target::Preview(id)) && app.preview.focus != Some(id) {
            let frame = crate::app::state::hit::looks().preview;
            let buf = f.buffer_mut();
            for x in r.x..r.x + r.width {
                for y in [r.y, r.y + r.height.saturating_sub(1)] {
                    if let Some(c) = buf.cell_mut((x, y)) {
                        c.set_fg(frame);
                    }
                }
            }
            for y in r.y..r.y + r.height {
                for x in [r.x, r.x + r.width.saturating_sub(1)] {
                    if let Some(c) = buf.cell_mut((x, y)) {
                        c.set_fg(frame);
                    }
                }
            }
        }
    }
}

/// Every key as a swatch, plus static samples (V switches to this view).
fn draw_swatches(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.editor.palette;
    let block = Block::bordered()
        .title(" Preview ")
        .border_style(Style::new().dark_gray());
    let inner = block.inner(area);
    f.render_widget(block, area);
    let bg = p.get("bg0");
    f.render_widget(
        Block::new().style(Style::new().bg(bg.color()).fg(p.get("fg0").color())),
        inner,
    );

    let mut lines: Vec<Line> = Vec::new();
    terminal_lines(&mut lines, app);
    lines.push(Line::raw(""));
    editor_lines(&mut lines, p);
    lines.push(Line::raw(""));
    i3_lines(&mut lines, p);
    lines.push(Line::raw(""));
    all_colour_lines(&mut lines, app, inner.width);
    f.render_widget(Paragraph::new(lines), inner);
}

/// A text style with the palette colour of `k` as foreground.
fn fg(p: &Palette, k: &str) -> Style {
    Style::new().fg(p.get(k).color())
}

/// A section heading of the swatch view.
fn heading(p: &Palette, t: &str) -> Line<'static> {
    Line::styled(format!(" {t}"), fg(p, "grey1").bold())
}

/// Terminal colours, as Ghostty gets them (terminal-colors.json).
// Long function: one section of the swatch view: the 16 ANSI swatches and the sample line, in drawing order.
fn terminal_lines(lines: &mut Vec<Line<'static>>, app: &App) {
    let p = &app.editor.palette;
    let bg = p.get("bg0");
    lines.push(heading(p, "Terminal colours"));
    let Some(map) = &app.terminal else {
        lines.push(Line::styled(
            "  (terminal-colors.json not found)",
            fg(p, "red"),
        ));
        return;
    };
    for half in [0..8, 8..16] {
        let mut spans: Vec<Span> = vec!["  ".into()];
        for i in half {
            let c = map
                .ansi
                .get(i)
                .and_then(|k| palette::key(k))
                .map_or(bg, |k| p.get(k));
            spans.push(Span::styled(format!("{i:>2} "), fg(p, "grey0")));
            spans.push(swatch(c, 4));
            spans.push(" ".into());
        }
        lines.push(Line::from(spans));
    }
    let sample: Vec<Span> = ["red", "green", "yellow", "blue", "purple", "aqua"]
        .iter()
        .map(|k| Span::styled(format!(" {k}"), fg(p, k)))
        .collect();
    let term = Style::new()
        .fg(palette::key(&map.foreground)
            .map_or(bg, |k| p.get(k))
            .color())
        .bg(palette::key(&map.background)
            .map_or(bg, |k| p.get(k))
            .color());
    let mut spans = vec![Span::styled("  $ ls --color ", term)];
    spans.extend(sample);
    lines.push(Line::from(spans));
}

/// The sample code: tokens and line backgrounds as nvim shows them
/// (mini.base16 mapping from nvim/lua/theme.lua).
// Long function: a data table: the six sample code lines with their token colours.
fn code_sample() -> Vec<CodeLine<'static>> {
    vec![
        (
            vec![("-- Re-apply when the colours file changes", "grey1")],
            None,
        ),
        (
            vec![
                ("local ", "purple"),
                ("function ", "purple"),
                ("apply", "blue"),
                ("(", "grey2"),
                ("path", "red"),
                (", ", "grey2"),
                ("opts", "red"),
                (")", "grey2"),
            ],
            None,
        ),
        (
            vec![
                ("  if not ", "purple"),
                ("opts", "red"),
                (".", "grey2"),
                ("force ", "red"),
                ("then return ", "purple"),
                ("nil ", "orange"),
                ("end", "purple"),
            ],
            Some("bg1"),
        ),
        (
            vec![
                ("  local ", "purple"),
                ("count", "red"),
                (" = ", "fg0"),
                ("42", "orange"),
                (" + ", "fg0"),
                ("#", "aqua"),
                ("path", "red"),
            ],
            None,
        ),
        (
            vec![
                ("  vim", "red"),
                (".", "grey2"),
                ("notify", "blue"),
                ("(", "grey2"),
                ("\"theme applied\"", "green"),
                (", ", "grey2"),
                ("Level", "yellow"),
                (".", "grey2"),
                ("INFO", "orange"),
                (")", "grey2"),
            ],
            Some("bg3"),
        ),
        (vec![("end", "purple")], None),
    ]
}

/// Code as nvim shows it, then its status line.
// Long function: one section of the swatch view: code lines, then the nvim status line of four coloured spans.
fn editor_lines(lines: &mut Vec<Line<'static>>, p: &Palette) {
    let bg = p.get("bg0");
    lines.push(heading(p, "Editor"));
    let num = |n: u32, cur: bool| {
        Span::styled(
            format!("  {n:>3} "),
            if cur { fg(p, "yellow") } else { fg(p, "grey0") },
        )
    };
    for (n, (tokens, line_bg)) in code_sample().into_iter().enumerate() {
        let mut spans = vec![num(
            u32::try_from(n + 1).unwrap_or(u32::MAX),
            line_bg == Some("bg1"),
        )];
        for (text, k) in tokens {
            let mut st = fg(p, k);
            if let Some(b) = line_bg {
                st = st.bg(p.get(b).color());
            }
            spans.push(Span::styled(text, st));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(vec![
        "  ".into(),
        Span::styled(
            " NORMAL ",
            Style::new()
                .bg(p.get("green").color())
                .fg(bg.color())
                .bold(),
        ),
        Span::styled(
            " theme.lua ",
            Style::new()
                .bg(p.get("bg_statusline2").color())
                .fg(p.get("fg0").color()),
        ),
        Span::styled(
            " ".repeat(20),
            Style::new().bg(p.get("bg_statusline1").color()),
        ),
        Span::styled(
            " 3:14 ",
            Style::new()
                .bg(p.get("bg_statusline3").color())
                .fg(p.get("fg0").color()),
        ),
    ]));
}

/// i3 window borders and bar (i3/theme roles).
// Long function: one section of the swatch view: the seven coloured spans of the i3 borders and bar.
fn i3_lines(lines: &mut Vec<Line<'static>>, p: &Palette) {
    let bg = p.get("bg0");
    lines.push(heading(p, "Window borders and bar"));
    lines.push(Line::from(vec![
        "  ".into(),
        Span::styled(
            " focused ",
            Style::new().bg(p.get("yellow").color()).fg(bg.color()),
        ),
        "  ".into(),
        Span::styled(
            " inactive ",
            Style::new()
                .bg(p.get("bg3").color())
                .fg(p.get("fg0").color()),
        ),
        "  ".into(),
        Span::styled(
            " urgent ",
            Style::new()
                .bg(p.get("red").color())
                .fg(p.get("fg0").color()),
        ),
        "  ".into(),
        Span::styled(
            " 1 ",
            Style::new().bg(p.get("yellow").color()).fg(bg.color()),
        ),
        Span::styled(
            " 2 ",
            Style::new()
                .bg(p.get("bg3").color())
                .fg(p.get("fg0").color()),
        ),
        Span::styled(" 3 ", fg(p, "grey2")),
        Span::styled("   vol 40%  wifi  17:42", fg(p, "fg0")),
    ]));
}

/// Every key as a swatch with its name, as many per line as fit in `width`.
fn all_colour_lines(lines: &mut Vec<Line<'static>>, app: &App, width: u16) {
    let p = &app.editor.palette;
    lines.push(heading(p, "All colours"));
    let cols = ((width.saturating_sub(2)) / 24).max(1) as usize;
    let all: Vec<&str> = GROUPS
        .iter()
        .flat_map(|(_, ks)| ks.iter().copied())
        .collect();
    for chunk in all.chunks(cols) {
        let mut spans: Vec<Span> = vec!["  ".into()];
        for k in chunk {
            spans.push(swatch(p.get(k), 3));
            spans.push(Span::styled(
                format!(" {k:<19}"),
                if *k == app.key() {
                    fg(p, "yellow").bold()
                } else {
                    fg(p, "grey2")
                },
            ));
        }
        lines.push(Line::from(spans));
    }
}
