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
use crate::app::state::hit::{Target, hover};
use crate::app::state::nav::{Focus, Side};
use crate::model::color::{self, Rgb};
use crate::model::palette;
use crate::ui::focused_block;
use crate::ui::pane_chrome::pane_title;
use crate::ui::widgets::{f64_to_u16, panel_number, to_u16};

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

/// The rectangles of the picker for an inside `inner`: the rows of the
/// colour square, the square, the hue strip and the width of a slider track.
fn part_rects(inner: Rect) -> (u16, Rect, Rect, u16) {
    // Colour square rows: whatever the 3 sliders, a gap and the hex row leave
    let rows: u16 = inner.height.saturating_sub(5).max(2);
    let square = Rect {
        x: inner.x + 1,
        y: inner.y,
        width: inner.width.saturating_sub(7),
        height: rows,
    };
    let hue = Rect {
        x: square.x + square.width + 1,
        y: inner.y,
        width: 2,
        height: rows,
    };
    (rows, square, hue, inner.width.saturating_sub(10))
}

/// Works out where each part goes and registers the clickable ones.
// Long function: six `hits.add` calls that share `inner`, `rows` and `track_w`; splitting would only pass these three values around.
fn register_hits(app: &mut App, inner: Rect) -> PickerParts {
    let (rows, sq, hue, track_w) = part_rects(inner);
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

/// The colour grid of the square: saturation left to right, value top to
/// bottom, for hue `hue_deg`. Each cell is two pixels (`▀`: top and bottom).
fn draw_square_grid(buf: &mut Buffer, parts: &PickerParts, hue_deg: f64) {
    let (sq, rows) = (parts.sq, parts.rows);
    let pixels = f64::from(rows * 2 - 1);
    for row in 0..rows {
        for col in 0..sq.width {
            let sat = f64::from(col) / f64::from((sq.width - 1).max(1));
            let top = color::hsv_to_rgb(hue_deg, sat, 1.0 - f64::from(2 * row) / pixels);
            let bottom = color::hsv_to_rgb(hue_deg, sat, 1.0 - f64::from(2 * row + 1) / pixels);
            set_cell(buf, sq.x + col, sq.y + row, "▀", top, bottom);
        }
    }
}

/// Saturation (left to right) × value (top to bottom), two pixels per cell,
/// with the marker on the current colour and a crosshair under the mouse.
fn draw_square(
    buf: &mut Buffer,
    app: &App,
    parts: &PickerParts,
    current: Rgb,
    mouse: Option<(u16, u16)>,
) {
    let (sq, rows) = (parts.sq, parts.rows);
    let (hue_deg, sat, val) = app.editor.hsv;
    draw_square_grid(buf, parts, hue_deg);
    let pixels = f64::from(rows * 2 - 1);
    let marker_x = sq.x + f64_to_u16((sat * f64::from(sq.width - 1)).round());
    let marker_y = sq.y + (f64_to_u16(((1.0 - val) * pixels).round()) / 2).min(rows - 1);
    let marker = if app.focus == Focus::Square {
        "◉"
    } else {
        "○"
    };
    set_cell(buf, marker_x, marker_y, marker, current.text_on(), current);
    // hovering the square: a crosshair where a click would pick
    if parts.sq_hover
        && let Some((x, y)) = mouse
    {
        draw_crosshair(buf, (x, y), current);
    }
}

/// The crosshair cell at `(x, y)`: it keeps the colour already drawn there (`current` if none).
fn draw_crosshair(buf: &mut Buffer, (x, y): (u16, u16), current: Rgb) {
    let under = buf
        .cell((x, y))
        .and_then(|cell| match cell.fg {
            Color::Rgb(red, green, blue) => Some(Rgb(red, green, blue)),
            _ => None,
        })
        .unwrap_or(current);
    set_cell(buf, x, y, "┼", under.text_on(), under);
}

/// The hue strip, with a marker at the current hue and a pale one under the mouse.
fn draw_hue(buf: &mut Buffer, app: &App, parts: &PickerParts, mouse: Option<(u16, u16)>) {
    let (hue, rows) = (parts.hue, parts.rows);
    let (hue_deg, _, _) = app.editor.hsv;
    let pixels = f64::from(rows * 2 - 1);
    for row in 0..rows {
        let top = color::hsv_to_rgb(f64::from(2 * row) / pixels * 360.0, 1.0, 1.0);
        let bottom = color::hsv_to_rgb(f64::from(2 * row + 1) / pixels * 360.0, 1.0, 1.0);
        for col in 0..hue.width {
            set_cell(buf, hue.x + col, hue.y + row, "▀", top, bottom);
        }
    }
    let marker_y = hue.y + (f64_to_u16((hue_deg / 360.0 * pixels).round()) / 2).min(rows - 1);
    if let Some(cell) = buf.cell_mut((hue.x + hue.width, marker_y)) {
        cell.set_symbol("◀");
        cell.set_style(if app.focus == Focus::Hue {
            Style::new().yellow()
        } else {
            Style::new()
        });
    }
    // hovering the hue strip: a pale marker at the hue a click would pick
    if parts.hue_hover
        && let Some((_, y)) = mouse
        && let Some(cell) = buf.cell_mut((hue.x + hue.width, y))
        && cell.symbol() != "◀"
    {
        cell.set_symbol("◁").set_style(Style::new().dark_gray());
    }
}

/// A slider value as a whole number for its text (degrees or percent).
fn whole(value: f64) -> i32 {
    i32::from(f64_to_u16(value.round()))
}

/// H, S, L sliders, each showing what moving it would give.
fn draw_sliders(buf: &mut Buffer, app: &App, parts: &PickerParts, key: &'static str) {
    let (hue_deg, _, _) = app.editor.hsv;
    let (sat, lum) = app.editor.hsl(key);
    let sliders: [(Focus, &str, f64, String); 3] = [
        (
            Focus::SliderH,
            "H",
            hue_deg / 360.0,
            format!("{:>3}°", whole(hue_deg)),
        ),
        (
            Focus::SliderS,
            "S",
            sat,
            format!("{:>3}%", whole(sat * 100.0)),
        ),
        (
            Focus::SliderL,
            "L",
            lum,
            format!("{:>3}%", whole(lum * 100.0)),
        ),
    ];
    for (row, slider) in sliders.into_iter().enumerate() {
        draw_slider(buf, app, parts, (row, slider), (hue_deg, sat, lum));
    }
}

/// One slider row: its label, the colour track, the thumb and the value text.
/// `hsl` is the colour that the other two sliders show.
// Long function: label, track, thumb and value text of one slider are drawn in this order into the same buffer row, so later cells overwrite earlier ones.
fn draw_slider(
    buf: &mut Buffer,
    app: &App,
    parts: &PickerParts,
    (row, (focus, label, position, value)): (usize, (Focus, &str, f64, String)),
    (hue_deg, sat, lum): (f64, f64, f64),
) {
    let inner = parts.inner;
    let y = inner.y + parts.rows + to_u16(row);
    let mut label_style = if app.focus == focus {
        Style::new().yellow().bold()
    } else {
        Style::new()
    };
    if parts.slider_hover[row] {
        label_style = hover(label_style);
    }
    if let Some(cell) = buf.cell_mut((inner.x + 1, y)) {
        cell.set_symbol(label).set_style(label_style);
    }
    let track = Rect {
        x: inner.x + 3,
        y,
        width: parts.track_w,
        height: 1,
    };
    for col in 0..track.width {
        let along = f64::from(col) / f64::from((track.width - 1).max(1));
        let colour = match focus {
            Focus::SliderH => color::hsv_to_rgb(along * 360.0, 0.85, 0.9),
            Focus::SliderS => color::hsl_to_rgb(hue_deg, along, lum),
            _ => color::hsl_to_rgb(hue_deg, sat, along),
        };
        set_cell(buf, track.x + col, y, " ", colour, colour);
    }
    let thumb_x = track.x + f64_to_u16((position * f64::from(track.width - 1)).round());
    let under = buf.cell((thumb_x, y)).map_or(Color::Reset, |cell| cell.bg);
    let under = match under {
        Color::Rgb(red, green, blue) => Rgb(red, green, blue),
        _ => Rgb(0, 0, 0),
    };
    set_cell(buf, thumb_x, y, "┃", under.text_on(), under);
    buf.set_string(track.x + track.width + 1, y, value, label_style);
}

/// The hex input row, and the colour before and after editing.
// Long function: one line of six spans (label, input, saved and current swatches) plus its row rectangle.
fn draw_hex(f: &mut Frame, app: &App, parts: &PickerParts, key: &str, current: Rgb) {
    let inner = parts.inner;
    let y = inner.y + parts.rows + 4;
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
    let label_style = if editing {
        Style::new().yellow().bold()
    } else {
        Style::new()
    };
    let line = Line::from(vec![
        Span::styled(
            " hex ",
            if parts.hex_hover {
                hover(label_style)
            } else {
                label_style
            },
        ),
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
