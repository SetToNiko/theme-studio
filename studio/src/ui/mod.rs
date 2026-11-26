//! Drawing. The studio's own chrome uses named terminal colours (so it
//! follows the system theme); only swatches and the preview use the palette
//! being edited.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::App;
use crate::app::state::nav::{ColourRow, Focus, Overlay, Page, Side};
use crate::color::{self, Rgb};
use crate::hit::{Target, hover};
use crate::keymap::Context;
use crate::keymap::action::Action;
use crate::palette::{self, GROUPS, TEXT_KEYS};
use crate::preview::Mode as PreviewMode;
mod pages;
pub mod pane_chrome;
pub mod widgets;
use pages::{draw_body_page, draw_main_page, draw_too_small};
use pane_chrome::{pane_border, pane_title};
use widgets::{friendly, panel_number};
mod overlays;
use overlays::dialogs;
pub mod layout;
mod status;
use layout::{Arrangement, arrange};
use status::draw_status;

pub fn draw(f: &mut Frame, app: &mut App) {
    app.hits.clear();
    // images still waiting for a size to settle set this again
    app.redraw_at = None;
    // set again if the Palette pane is drawn (the palette list hangs from it)
    app.layout.palette_pane = None;
    app.messages.hover_note = None;
    crate::hit::set_looks(app.looks(&app.settings));
    let full = f.area();
    // tabs on top: their own row; at the bottom they share the status line
    let top = app.settings.layout.tabs_on_top();
    let (tabs_row, area) = if top && full.height > 1 {
        let [t, rest] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(full);
        (Some(t), rest)
    } else {
        (None, full)
    };
    let arrangement = arrange(
        area,
        app.page,
        app.layout.picker_visible,
        app.layout.sample_visible,
        app.layout.main_preview_visible,
        &app.layout.sizes,
    );
    if let Arrangement::TooSmall { need_w, need_h } = arrangement {
        return draw_too_small(f, full, need_w, need_h + top as u16);
    }
    if let Some(t) = tabs_row {
        draw_tabs(f, app, t);
    }
    match arrangement {
        Arrangement::TooSmall { .. } => {}
        Arrangement::Page { body, status } => {
            draw_body_page(f, app, body);
            draw_status(f, app, status);
        }
        Arrangement::Main(l) => {
            draw_main_page(f, app, &l);
            draw_status(f, app, l.status);
        }
    }
    dialogs::draw_overlay(f, app, full);
    overlays::draw_pending(f, app, full);
    overlays::draw_notice(f, app, full);
}

fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) -> u16 {
    let mut x = area.x;
    for (label, action, page) in [
        ("Main", Action::PageMain, Page::Main),
        ("Fonts", Action::PageDownloads, Page::Downloads),
        ("Settings", Action::PageSettings, Page::Settings),
    ] {
        let key = app
            .keymap
            .key_for(&[Context::Global], action)
            .map(|k| friendly(&k))
            .unwrap_or_default();
        // a dot on Settings while it has unsaved changes
        let dot = if page == Page::Settings && app.settings_page.draft.is_some() {
            " ●"
        } else {
            ""
        };
        let text = format!(" {key} {label}{dot} ");
        let w = (text.chars().count() as u16).min((area.x + area.width).saturating_sub(x));
        let r = Rect {
            x,
            y: area.y,
            width: w,
            height: 1,
        };
        // with the keyboard on the tabs, the current one is yellow (like a
        // page's own tabs)
        let mut style = match (app.page == page, app.layout.page_tabs_focused) {
            (true, true) => Style::new().black().on_yellow().bold(),
            (true, false) => Style::new().reversed().bold(),
            _ => Style::new().dark_gray(),
        };
        if app.hits.add(r, Target::PageTab(page)) {
            style = hover(style);
        }
        f.render_widget(Paragraph::new(Span::styled(text, style)), r);
        x += w;
    }
    // the theme: its name and unsaved dot (a click opens the theme list)
    let dot = if app.theme_dirty() { " ●" } else { "" };
    let label = format!("  Theme: {}{dot} ", app.theme);
    let w = (label.chars().count() as u16).min((area.x + area.width).saturating_sub(x));
    if w > 0 {
        let r = Rect {
            x,
            y: area.y,
            width: w,
            height: 1,
        };
        let mut line = Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("Theme: {}", app.theme), Style::new().bold()),
            Span::styled(dot, Style::new().yellow()),
            Span::raw(" "),
        ]);
        if app.hits.add(r, Target::ThemeName) {
            line = crate::hit::hover_line(line, 1);
        }
        f.render_widget(Paragraph::new(line), r);
        x += w;
    }
    x - area.x
}

fn focused_block(app: &App, area: Rect, title: Line<'static>, focused: bool) -> Block<'static> {
    Block::bordered()
        .title(title)
        .border_style(pane_border(app, area, focused))
}

/// A swatch with a thin frame, visible even when it matches the background.
fn framed(c: Rgb) -> Span<'static> {
    Span::styled("▕    ▏", Style::new().bg(c.color()).fg(Color::DarkGray))
}

/// A swatch: `width` cells of background colour.
fn swatch(c: Rgb, width: usize) -> Span<'static> {
    Span::styled(" ".repeat(width), Style::new().bg(c.color()))
}

// ----- key list -----------------------------------------------------------

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    app.layout.palette_pane = Some(area);
    // the title names the palette; the dropdown is the pane's first row
    let dot = if app.dirty() {
        Span::styled(" ●", Style::new().yellow())
    } else {
        "".into()
    };
    let title = Line::from(vec![
        panel_number(4),
        "Palette: ".into(),
        app.name.clone().into(),
        dot,
        " ".into(),
    ]);
    let title = pane_title(app, area, title, 4);
    let focused = app.side == Side::Colours
        && app.focus == Focus::List
        && app.preview.focus.is_none()
        && !app.layout.page_tabs_focused;
    let block = focused_block(app, area, title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let [menu_row, inner] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    draw_palette_menu(f, app, menu_row, focused && app.layout.palette_menu_focused);
    // the colour rows have the cursor unless the dropdown row does
    let focused = focused && !app.layout.palette_menu_focused;

    let bg0 = app.editor.palette.get("bg0");
    let copy_target = match app.overlay {
        Overlay::CopyFrom { target } => Some(target),
        _ => None,
    };
    let rows = app.colour_rows();
    // the selected colour's relatives by derivation (pinned keys are set by
    // hand, so they're left out) get the derived-colour highlight
    let selected = app.keys[app.sel];
    let related: Vec<&str> = if app.editor.palette.is_pinned(selected) {
        Vec::new()
    } else {
        palette::related(selected)
            .into_iter()
            .filter(|k| palette::is_base(k) || !app.editor.palette.is_pinned(k))
            .collect()
    };
    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            let cursor = r == app.row_sel && (focused || copy_target.is_some());
            let cursor_style = if cursor {
                Style::new().add_modifier(Modifier::REVERSED)
            } else {
                Style::new()
            };
            match *row {
                ColourRow::Group { index, expanded } => {
                    let (name, keys) = GROUPS[index];
                    let line = Line::from(vec![
                        Span::styled(if expanded { " ▾ " } else { " ▸ " }, Style::new().yellow()),
                        Span::styled(name, cursor_style.bold()),
                        Span::styled(format!("  {}", keys.len()), Style::new().dark_gray()),
                    ]);
                    // a closed group stands in for its related keys
                    if !expanded && keys.iter().any(|k| related.contains(k)) {
                        line.patch_style(crate::hit::derived(Style::new()))
                    } else {
                        line
                    }
                }
                ColourRow::Key(i) => {
                    let key = app.keys[i];
                    let c = app.editor.palette.get(key);
                    let pinned = app.editor.palette.is_pinned(key);
                    // the colour the picker shows, when the cursor is elsewhere
                    let name_style = if cursor {
                        cursor_style
                    } else if i == app.sel {
                        Style::new().bold().underlined()
                    } else {
                        Style::new()
                    };
                    let marker = if copy_target == Some(key) {
                        Span::styled("→", Style::new().yellow())
                    } else if pinned {
                        Span::styled("", Style::new().yellow())
                    } else {
                        " ".into()
                    };
                    let mut spans = vec!["  ".into(), marker, " ".into()];
                    // the cursor on the name, not the padding before the swatch
                    spans.extend(crate::hit::cursor_spans(
                        format!("{key:<16}"),
                        Style::new(),
                        name_style,
                    ));
                    spans.push(swatch(c, 4));
                    spans.push(Span::raw(format!(" {} ", c.hex())));
                    let info = if palette::is_base(key) || pinned {
                        String::new()
                    } else {
                        format!("⟵ {}", palette::sources(key))
                    };
                    spans.push(Span::styled(
                        format!("{info:<11}"),
                        Style::new().dark_gray(),
                    ));
                    if TEXT_KEYS.contains(&key) {
                        let ratio = color::contrast(c, bg0);
                        let style = if ratio < 3.0 {
                            Style::new().red().bold()
                        } else if ratio < 4.5 {
                            Style::new().yellow()
                        } else {
                            Style::new().dark_gray()
                        };
                        spans.push(Span::styled(format!("{ratio:>4.1}:1"), style));
                    }
                    if related.contains(&key) {
                        Line::from(spans).patch_style(crate::hit::derived(Style::new()))
                    } else {
                        Line::from(spans)
                    }
                }
            }
        })
        .collect();

    // Scroll so the cursor stays visible.
    let h = inner.height as usize;
    if app.row_sel < app.list_scroll {
        app.list_scroll = app.row_sel;
    } else if h > 0 && app.row_sel >= app.list_scroll + h {
        app.list_scroll = app.row_sel + 1 - h;
    }
    app.list_scroll = app.list_scroll.min(rows.len().saturating_sub(h.max(1)));
    let shown: Vec<Line> = lines.into_iter().skip(app.list_scroll).take(h).collect();
    // each row is clickable; the one under the mouse is highlighted
    let shown: Vec<Line> = shown
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let r = Rect {
                y: inner.y + i as u16,
                height: 1,
                ..inner
            };
            if app.hits.add(r, Target::ColourRow(app.list_scroll + i)) {
                // only the group's or key's name is underlined
                let name = match rows.get(app.list_scroll + i) {
                    Some(ColourRow::Group { .. }) => 1,
                    _ => 3,
                };
                crate::hit::hover_line(line, name)
            } else {
                line
            }
        })
        .collect();
    f.render_widget(Paragraph::new(shown), inner);
}

/// The Palette pane's dropdown row: " CRT Amber ▾ " and the palette's base
/// colours. Enter (with the keyboard on it) or a click opens the list.
fn draw_palette_menu(f: &mut Frame, app: &mut App, area: Rect, cursor: bool) {
    let text = format!(" {} ▾ ", app.name);
    let w = (text.chars().count() as u16).min(area.width);
    let hit = Rect { width: w, ..area };
    let base = Style::new().yellow().bold();
    let mut spans = if cursor {
        crate::hit::cursor_spans(text, base, Style::new().reversed())
    } else {
        vec![Span::styled(text, base)]
    };
    if app.hits.add(hit, Target::PaletteMenu) && !cursor {
        spans = vec![Span::styled(spans[0].content.to_string(), hover(base))];
    }
    spans.push(" ".into());
    for k in [
        "bg0", "fg0", "red", "orange", "yellow", "green", "aqua", "blue", "purple",
    ] {
        spans.push(swatch(app.editor.palette.get(k), 2));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ----- picker -------------------------------------------------------------

fn set_cell(buf: &mut Buffer, x: u16, y: u16, sym: &str, fg: Rgb, bg: Rgb) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(sym).set_fg(fg.color()).set_bg(bg.color());
    }
}

fn draw_picker(f: &mut Frame, app: &mut App, area: Rect) {
    let key = app.key();
    let state = if palette::is_base(key) {
        Span::styled(" base ", Style::new().dark_gray())
    } else if app.editor.palette.is_pinned(key) {
        Span::styled("  pinned (p to follow again) ", Style::new().yellow())
    } else {
        Span::styled(
            format!(" follows {} until edited ", palette::sources(key)),
            Style::new().dark_gray(),
        )
    };
    let title = Line::from(vec![
        panel_number(5),
        "Colour: ".into(),
        Span::styled(key, Style::new().bold()),
        " ".into(),
        state,
    ]);
    let focused = app.side == Side::Colours
        && !matches!(app.focus, Focus::List)
        && app.preview.focus.is_none();
    let title = pane_title(app, area, title, 5);
    let block = focused_block(app, area, title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Colour square rows: whatever the 3 sliders, a gap and the hex row leave
    let rows: u16 = inner.height.saturating_sub(5).max(2);
    let sq = Rect {
        x: inner.x + 1,
        y: inner.y,
        width: inner.width.saturating_sub(7),
        height: rows,
    };
    let hue = Rect {
        x: sq.x + sq.width + 1,
        y: inner.y,
        width: 2,
        height: rows,
    };
    let (h, s, v) = app.editor.hsv;
    let current = app.editor.palette.get(key);
    // clickable parts (registered before drawing, so hover can be shown)
    let track_w = inner.width.saturating_sub(10);
    let track = |row: u16| Rect {
        x: inner.x + 3,
        y: inner.y + rows + row,
        width: track_w,
        height: 1,
    };
    let sq_hover = app.hits.add(sq, Target::Square);
    let hue_hover = app.hits.add(hue, Target::Hue);
    let slider_hover = [
        app.hits.add(track(0), Target::SliderH),
        app.hits.add(track(1), Target::SliderS),
        app.hits.add(track(2), Target::SliderL),
    ];
    let hex_hover = app.hits.add(
        Rect {
            x: inner.x,
            y: inner.y + rows + 4,
            width: 14,
            height: 1,
        },
        Target::Hex,
    );
    let mouse = app.hits.mouse;
    let buf = f.buffer_mut();

    // Saturation (left to right) × value (top to bottom), two pixels per cell
    let px = (rows * 2 - 1) as f64;
    for cy in 0..rows {
        for cx in 0..sq.width {
            let sx = cx as f64 / (sq.width - 1).max(1) as f64;
            let top = color::hsv_to_rgb(h, sx, 1.0 - (2 * cy) as f64 / px);
            let bot = color::hsv_to_rgb(h, sx, 1.0 - (2 * cy + 1) as f64 / px);
            set_cell(buf, sq.x + cx, sq.y + cy, "▀", top, bot);
        }
    }
    let mx = sq.x + (s * (sq.width - 1) as f64).round() as u16;
    let my = sq.y + (((1.0 - v) * px).round() as u16 / 2).min(rows - 1);
    let marker = if app.focus == Focus::Square {
        "◉"
    } else {
        "○"
    };
    set_cell(buf, mx, my, marker, current.text_on(), current);
    // hovering the square: a crosshair where a click would pick
    if sq_hover && let Some((x, y)) = mouse {
        let under = buf
            .cell((x, y))
            .and_then(|c| match c.fg {
                Color::Rgb(r, g, b) => Some(Rgb(r, g, b)),
                _ => None,
            })
            .unwrap_or(current);
        set_cell(buf, x, y, "┼", under.text_on(), under);
    }

    // Hue strip
    for cy in 0..rows {
        let top = color::hsv_to_rgb((2 * cy) as f64 / px * 360.0, 1.0, 1.0);
        let bot = color::hsv_to_rgb((2 * cy + 1) as f64 / px * 360.0, 1.0, 1.0);
        for cx in 0..hue.width {
            set_cell(buf, hue.x + cx, hue.y + cy, "▀", top, bot);
        }
    }
    let hy = hue.y + ((h / 360.0 * px).round() as u16 / 2).min(rows - 1);
    if let Some(cell) = buf.cell_mut((hue.x + hue.width, hy)) {
        cell.set_symbol("◀");
        cell.set_style(if app.focus == Focus::Hue {
            Style::new().yellow()
        } else {
            Style::new()
        });
    }
    // hovering the hue strip: a pale marker at the hue a click would pick
    if hue_hover
        && let Some((_, y)) = mouse
        && let Some(cell) = buf.cell_mut((hue.x + hue.width, y))
        && cell.symbol() != "◀"
    {
        cell.set_symbol("◁").set_style(Style::new().dark_gray());
    }

    // H, S, L sliders, each showing what moving it would give
    let (sl_s, sl_l) = app.editor.hsl(key);
    let sliders: [(Focus, &str, f64, String); 3] = [
        (
            Focus::SliderH,
            "H",
            h / 360.0,
            format!("{:>3}°", h.round() as i32),
        ),
        (
            Focus::SliderS,
            "S",
            sl_s,
            format!("{:>3}%", (sl_s * 100.0).round() as i32),
        ),
        (
            Focus::SliderL,
            "L",
            sl_l,
            format!("{:>3}%", (sl_l * 100.0).round() as i32),
        ),
    ];
    for (row, (focus, label, t, value)) in sliders.into_iter().enumerate() {
        let y = inner.y + rows + row as u16;
        let mut label_style = if app.focus == focus {
            Style::new().yellow().bold()
        } else {
            Style::new()
        };
        if slider_hover[row] {
            label_style = hover(label_style);
        }
        if let Some(cell) = buf.cell_mut((inner.x + 1, y)) {
            cell.set_symbol(label).set_style(label_style);
        }
        let track = Rect {
            x: inner.x + 3,
            y,
            width: track_w,
            height: 1,
        };
        for cx in 0..track.width {
            let u = cx as f64 / (track.width - 1).max(1) as f64;
            let c = match focus {
                Focus::SliderH => color::hsv_to_rgb(u * 360.0, 0.85, 0.9),
                Focus::SliderS => color::hsl_to_rgb(h, u, sl_l),
                _ => color::hsl_to_rgb(h, sl_s, u),
            };
            set_cell(buf, track.x + cx, y, " ", c, c);
        }
        let tx = track.x + (t * (track.width - 1) as f64).round() as u16;
        let under = buf.cell((tx, y)).map(|c| c.bg).unwrap_or(Color::Reset);
        let under = match under {
            Color::Rgb(r, g, b) => Rgb(r, g, b),
            _ => Rgb(0, 0, 0),
        };
        set_cell(buf, tx, y, "┃", under.text_on(), under);
        buf.set_string(track.x + track.width + 1, y, value, label_style);
    }

    // Hex input, and the colour before and after editing
    let y = inner.y + rows + 4;
    let editing = app.focus == Focus::Hex;
    let hex_style = if editing {
        Style::new().yellow().underlined()
    } else {
        Style::new()
    };
    let hex_text = if editing {
        format!("{:<7}▏", app.editor.hex_input)
    } else {
        format!("{:<8}", current.hex())
    };
    let old = app.editor.saved.get(key);
    let line = Line::from(vec![
        Span::styled(" hex ", {
            let s = if editing {
                Style::new().yellow().bold()
            } else {
                Style::new()
            };
            if hex_hover { hover(s) } else { s }
        }),
        Span::styled(hex_text, hex_style),
        "   saved ".dark_gray(),
        framed(old),
        "  now ".dark_gray(),
        framed(current),
    ]);
    let hex_row = Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(Paragraph::new(line), hex_row);
}

// ----- preview (simple, until the live desktop preview) --------------------

/// A line of sample code: (text, palette key) tokens, and an optional line background key.
type CodeLine<'a> = (Vec<(&'a str, &'a str)>, Option<&'a str>);

// ----- preview ------------------------------------------------------------

/// The live desktop (nvim, shell or yazi, i3 borders and bar, rofi), or the
/// swatch view.
/// `stack`: nvim above the shell (a preview column); otherwise side by side.
fn draw_preview(f: &mut Frame, app: &mut App, area: Rect, stack: bool) {
    if app.preview.mode == PreviewMode::Swatches {
        return draw_swatches(f, app, area);
    }
    let key = |a: Action| {
        app.keymap
            .key_for(&[Context::Global], a)
            .map(|k| friendly(&k))
            .unwrap_or_default()
    };
    // While nvim or the shell has the keyboard the frame turns magenta (your
    // palette's purple, as the terminal shows it): keys go there, not to the
    // studio, until F12 hands them back.
    let capture = Style::new().magenta();
    let title = match app.preview.focus {
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
    };
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
        crate::preview::Windows {
            stack,
            split: app.layout.sizes.preview_split,
        },
    );
    // nvim and the shell are clickable (a click hands them the keyboard);
    // hovering one lights its frame in the capture colour
    let rects = app.preview.rects.clone();
    for (id, r) in rects {
        if app.hits.add(r, Target::Preview(id)) && app.preview.focus != Some(id) {
            let frame = crate::hit::looks().preview;
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

    let fg = |k: &str| Style::new().fg(p.get(k).color());
    let mut lines: Vec<Line> = Vec::new();
    let heading = |t: &str| Line::styled(format!(" {t}"), fg("grey1").bold());

    // Terminal colours, as Ghostty gets them (terminal-colors.json)
    lines.push(heading("Terminal colours"));
    if let Some(map) = &app.terminal {
        for half in [0..8, 8..16] {
            let mut spans: Vec<Span> = vec!["  ".into()];
            for i in half {
                let c = map
                    .ansi
                    .get(i)
                    .and_then(|k| palette::key(k))
                    .map(|k| p.get(k))
                    .unwrap_or(bg);
                spans.push(Span::styled(format!("{i:>2} "), fg("grey0")));
                spans.push(swatch(c, 4));
                spans.push(" ".into());
            }
            lines.push(Line::from(spans));
        }
        let sample: Vec<Span> = ["red", "green", "yellow", "blue", "purple", "aqua"]
            .iter()
            .map(|k| Span::styled(format!(" {k}"), fg(k)))
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
    } else {
        lines.push(Line::styled(
            "  (terminal-colors.json not found)",
            fg("red"),
        ));
    }
    lines.push(Line::raw(""));

    // Code as nvim shows it (mini.base16 mapping from nvim/lua/theme.lua)
    lines.push(heading("Editor"));
    let num = |n: u32, cur: bool| {
        Span::styled(
            format!("  {n:>3} "),
            if cur { fg("yellow") } else { fg("grey0") },
        )
    };
    let code: Vec<CodeLine> = vec![
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
    ];
    for (n, (tokens, line_bg)) in code.into_iter().enumerate() {
        let mut spans = vec![num(n as u32 + 1, line_bg == Some("bg1"))];
        for (text, k) in tokens {
            let mut st = fg(k);
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
    lines.push(Line::raw(""));

    // i3 window borders and bar (i3/theme roles)
    lines.push(heading("Window borders and bar"));
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
        Span::styled(" 3 ", fg("grey2")),
        Span::styled("   vol 40%  wifi  17:42", fg("fg0")),
    ]));
    lines.push(Line::raw(""));

    // Every key
    lines.push(heading("All colours"));
    let cols = ((inner.width.saturating_sub(2)) / 24).max(1) as usize;
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
                    fg("yellow").bold()
                } else {
                    fg("grey2")
                },
            ));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>() + "…"
    }
}
