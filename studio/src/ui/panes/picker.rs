//! The Colour pane: the colour square, hue strip, H/S/L sliders and hex input.
//!
//! The picker draws straight into the terminal buffer (two pixels per cell
//! with half blocks), so it has its own cell helper. Clickable parts are
//! registered before drawing, so hover can be shown.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::app::state::nav::{Focus, Side};
use crate::hit::{Target, hover};
use crate::model::color::{self, Rgb};
use crate::model::palette;
use crate::ui::focused_block;
use crate::ui::pane_chrome::pane_title;
use crate::ui::widgets::panel_number;

/// Where the parts of the picker are, and which of them the mouse is over.
struct PickerParts {
    /// The pane's inside (without the border).
    inner: Rect,
    /// Rows of the colour square and the hue strip.
    rows: u16,
    sq: Rect,
    hue: Rect,
    /// Width of each slider track.
    track_w: u16,
    sq_hover: bool,
    hue_hover: bool,
    slider_hover: [bool; 3],
    hex_hover: bool,
}

/// Draws one cell of the terminal buffer: symbol, foreground, background.
fn set_cell(buf: &mut Buffer, x: u16, y: u16, sym: &str, fg: Rgb, bg: Rgb) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(sym).set_fg(fg.color()).set_bg(bg.color());
    }
}

/// A swatch with a thin frame, visible even when it matches the background.
fn framed(c: Rgb) -> Span<'static> {
    Span::styled("▕    ▏", Style::new().bg(c.color()).fg(Color::DarkGray))
}

/// Draws the Colour pane for the selected key.
pub fn draw_picker(f: &mut Frame, app: &mut App, area: Rect) {
    let key = app.key();
    let focused = app.side == Side::Colours
        && !matches!(app.focus, Focus::List)
        && app.preview.focus.is_none();
    let title = pane_title(app, area, picker_title(app, key), 5);
    let block = focused_block(app, area, title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let parts = register_hits(app, inner);
    let current = app.editor.palette.get(key);
    let mouse = app.hits.mouse;
    let buf = f.buffer_mut();
    draw_square(buf, app, &parts, current, mouse);
    draw_hue(buf, app, &parts, mouse);
    draw_sliders(buf, app, &parts, key);
    draw_hex(f, app, &parts, key, current);
}

/// The pane title: the key's name and whether it is base, pinned or derived.
fn picker_title(app: &App, key: &'static str) -> Line<'static> {
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
    Line::from(vec![
        panel_number(5),
        "Colour: ".into(),
        Span::styled(key, Style::new().bold()),
        " ".into(),
        state,
    ])
}

/// Works out where each part goes and registers the clickable ones.
// Long function: one place computes the six clickable rectangles and registers them, because each rectangle depends on the same rows and inner size.
fn register_hits(app: &mut App, inner: Rect) -> PickerParts {
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
    PickerParts {
        inner,
        rows,
        sq,
        hue,
        track_w,
        sq_hover,
        hue_hover,
        slider_hover,
        hex_hover,
    }
}

/// Saturation (left to right) × value (top to bottom), two pixels per cell,
/// with the marker on the current colour and a crosshair under the mouse.
// Long function: two pixels per cell: the colour grid, the marker and the hover crosshair share the same grid maths.
fn draw_square(
    buf: &mut Buffer,
    app: &App,
    p: &PickerParts,
    current: Rgb,
    mouse: Option<(u16, u16)>,
) {
    let (sq, rows) = (p.sq, p.rows);
    let (h, s, v) = app.editor.hsv;
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
    if p.sq_hover
        && let Some((x, y)) = mouse
    {
        let under = buf
            .cell((x, y))
            .and_then(|c| match c.fg {
                Color::Rgb(r, g, b) => Some(Rgb(r, g, b)),
                _ => None,
            })
            .unwrap_or(current);
        set_cell(buf, x, y, "┼", under.text_on(), under);
    }
}

/// The hue strip, with a marker at the current hue and a pale one under the mouse.
fn draw_hue(buf: &mut Buffer, app: &App, p: &PickerParts, mouse: Option<(u16, u16)>) {
    let (hue, rows) = (p.hue, p.rows);
    let (h, _, _) = app.editor.hsv;
    let px = (rows * 2 - 1) as f64;
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
    if p.hue_hover
        && let Some((_, y)) = mouse
        && let Some(cell) = buf.cell_mut((hue.x + hue.width, y))
        && cell.symbol() != "◀"
    {
        cell.set_symbol("◁").set_style(Style::new().dark_gray());
    }
}

/// H, S, L sliders, each showing what moving it would give.
// Long function: the three sliders (H, S, L) are one loop over a table; each iteration draws label, track, thumb and value in order.
fn draw_sliders(buf: &mut Buffer, app: &App, p: &PickerParts, key: &'static str) {
    let inner = p.inner;
    let (h, _, _) = app.editor.hsv;
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
        let y = inner.y + p.rows + row as u16;
        let mut label_style = if app.focus == focus {
            Style::new().yellow().bold()
        } else {
            Style::new()
        };
        if p.slider_hover[row] {
            label_style = hover(label_style);
        }
        if let Some(cell) = buf.cell_mut((inner.x + 1, y)) {
            cell.set_symbol(label).set_style(label_style);
        }
        let track = Rect {
            x: inner.x + 3,
            y,
            width: p.track_w,
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
}

/// The hex input row, and the colour before and after editing.
// Long function: one line of six spans (label, input, saved and current swatches) plus its row rectangle.
fn draw_hex(f: &mut Frame, app: &App, p: &PickerParts, key: &str, current: Rgb) {
    let inner = p.inner;
    let y = inner.y + p.rows + 4;
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
            if p.hex_hover { hover(s) } else { s }
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
