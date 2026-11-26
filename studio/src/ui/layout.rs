//! Window layout: where each pane goes for a window size.
//!
//! `arrange` turns the window size, the page, and the saved pane sizes into
//! an [`Arrangement`]. Like btop, the layout follows the window as it is
//! resized, and below the minimum a message says what is needed. It also
//! returns the borders that the mouse can drag.

use ratatui::layout::{Constraint, Layout, Rect};

use crate::app::state::nav::Page;
use crate::config::settings::Sizes;
use crate::hit::{Border, BorderId};

/// Narrowest the fonts column, the colour column and the preview can be.
const FONTS_W: u16 = 46;
const COLOURS_W: u16 = 52;
const PREVIEW_MIN_W: u16 = 50;
/// Font roles: three roles of three lines, plus borders.
const ROLES_H: u16 = 3 * 3 + 2;
const FONT_LIST_MIN: u16 = 6;
const SAMPLE_MIN: u16 = 6;
/// The picker is at least this tall, and grows (bigger colour square) when
/// the colour column has room beyond the whole list.
const PICKER_MIN: u16 = 14;
const PICKER_MAX: u16 = 30;
/// Rows the colour list needs with every group open: 34 colours, 5 groups, borders.
const LIST_FULL: u16 = 34 + 5 + 2;
const LIST_MIN: u16 = 8;
/// The preview as a row under the columns (narrow windows).
const PREVIEW_ROW_MIN: u16 = 12;

/// The main page's panes. `wide`: three columns; otherwise fonts and colours
/// side by side with the preview as a full-width row underneath.
#[derive(Debug, PartialEq)]
pub struct MainLayout {
    pub roles: Rect,
    pub font_list: Rect,
    pub sample: Option<Rect>,
    pub colour_list: Rect,
    pub picker: Option<Rect>,
    pub preview: Rect,
    pub status: Rect,
    pub wide: bool,
    /// The borders that can be dragged to resize panes.
    pub borders: Vec<Border>,
}

/// Where everything goes for a window size. Like btop, the layout follows
/// the window as it's resized, and below the minimum a message says what's
/// needed.
#[derive(Debug, PartialEq)]
pub enum Arrangement {
    Main(MainLayout),
    /// Download fonts / Settings
    Page {
        body: Rect,
        status: Rect,
    },
    TooSmall {
        need_w: u16,
        need_h: u16,
    },
}

/// Where a vertical border (the first pane's right edge) can be grabbed.
fn v_border(id: BorderId, first: Rect, start: u16, total: u16) -> Border {
    let hit = Rect {
        x: first.x + first.width.saturating_sub(1),
        y: first.y,
        width: 1,
        height: first.height,
    };
    Border {
        id,
        hit,
        vertical: true,
        start,
        total,
        from_end: false,
    }
}

/// A horizontal border (the first pane's bottom edge); `from_end`: the share
/// is of what comes after it.
fn h_border(id: BorderId, first: Rect, start: u16, total: u16, from_end: bool) -> Border {
    let hit = Rect {
        x: first.x,
        y: first.y + first.height.saturating_sub(1),
        width: first.width,
        height: 1,
    };
    Border {
        id,
        hit,
        vertical: false,
        start,
        total,
        from_end,
    }
}

/// `show_preview`: the preview column (or row) is shown; hidden, the fonts
/// and colour columns share its space.
pub fn arrange(
    area: Rect,
    page: Page,
    picker: bool,
    sample: bool,
    show_preview: bool,
    sizes: &Sizes,
) -> Arrangement {
    if page != Page::Main {
        if area.width < 40 || area.height < 10 {
            return Arrangement::TooSmall {
                need_w: 40,
                need_h: 10,
            };
        }
        let [body, status] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
        return Arrangement::Page { body, status };
    }
    let wide = area.width >= FONTS_W + COLOURS_W + PREVIEW_MIN_W;
    let preview_w = if show_preview { PREVIEW_MIN_W } else { 0 };
    // the sample is too small to read under the narrow layout: left out there
    let sample = sample && wide;
    let fonts_need = ROLES_H + FONT_LIST_MIN + if sample { SAMPLE_MIN } else { 0 };
    let colours_need = LIST_MIN + if picker { PICKER_MIN } else { 0 };
    let columns_need = fonts_need.max(colours_need);
    let (need_w, need_h) = if wide {
        (FONTS_W + COLOURS_W + preview_w, columns_need + 1)
    } else {
        let row = if show_preview { PREVIEW_ROW_MIN } else { 0 };
        (FONTS_W + COLOURS_W, columns_need + row + 1)
    };
    if area.width < need_w || area.height < need_h {
        return Arrangement::TooSmall { need_w, need_h };
    }
    let [main, status] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
    // rounded, so a border put at a cell stays exactly there
    let share = |n: u16, f: f64| (n as f64 * f.clamp(0.0, 1.0)).round() as u16;
    let mut borders = Vec::new();
    let (fonts_col, colours_col, preview) = if !show_preview {
        // fonts and colours share the width, in the proportion they have
        // when the preview is shown
        let fonts_share = sizes.fonts_width / (sizes.fonts_width + sizes.colours_width).max(0.01);
        let fw = share(main.width, fonts_share)
            .max(FONTS_W)
            .min(main.width - COLOURS_W);
        let [f, c] = Layout::horizontal([Constraint::Length(fw), Constraint::Min(0)]).areas(main);
        borders.push(v_border(BorderId::FontsColours, f, main.x, main.width));
        (f, c, Rect::default())
    } else if wide {
        let fw = share(main.width, sizes.fonts_width)
            .max(FONTS_W)
            .min(main.width - COLOURS_W - PREVIEW_MIN_W);
        let cw = share(main.width, sizes.colours_width)
            .max(COLOURS_W)
            .min(main.width - fw - PREVIEW_MIN_W);
        let [f, c, p] = Layout::horizontal([
            Constraint::Length(fw),
            Constraint::Length(cw),
            Constraint::Min(0),
        ])
        .areas(main);
        borders.push(v_border(BorderId::FontsColours, f, main.x, main.width));
        borders.push(v_border(BorderId::ColoursPreview, c, c.x, main.width));
        (f, c, p)
    } else {
        let preview_h = share(main.height, sizes.narrow_preview_height)
            .clamp(PREVIEW_ROW_MIN, main.height - columns_need);
        let [top, p] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(preview_h)]).areas(main);
        let fw = share(top.width, sizes.narrow_fonts_width)
            .max(FONTS_W)
            .min(top.width - COLOURS_W);
        let [f, c] = Layout::horizontal([Constraint::Length(fw), Constraint::Min(0)]).areas(top);
        borders.push(h_border(
            BorderId::TopPreview,
            top,
            main.y + main.height,
            main.height,
            true,
        ));
        borders.push(v_border(BorderId::NarrowFontsColours, f, top.x, top.width));
        (f, c, p)
    };
    // fonts column: roles, list, sample
    let sample_h = if sample {
        share(fonts_col.height, sizes.sample_height)
            .clamp(SAMPLE_MIN, fonts_col.height - ROLES_H - FONT_LIST_MIN)
    } else {
        0
    };
    let [roles, font_list, sample_area] = Layout::vertical([
        Constraint::Length(ROLES_H),
        Constraint::Min(0),
        Constraint::Length(sample_h),
    ])
    .areas(fonts_col);
    if sample {
        borders.push(h_border(
            BorderId::ListSample,
            font_list,
            fonts_col.y + fonts_col.height,
            fonts_col.height,
            true,
        ));
    }
    // colour column: list, picker (automatic size unless resized)
    let picker_h = if !picker {
        0
    } else if sizes.picker_height > 0.0 {
        share(colours_col.height, sizes.picker_height)
            .clamp(PICKER_MIN, colours_col.height - LIST_MIN)
    } else {
        colours_col
            .height
            .saturating_sub(LIST_FULL)
            .clamp(PICKER_MIN, PICKER_MAX)
            .min(colours_col.height - LIST_MIN)
    };
    let [colour_list, picker_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(picker_h)]).areas(colours_col);
    if picker {
        borders.push(h_border(
            BorderId::ListPicker,
            colour_list,
            colours_col.y + colours_col.height,
            colours_col.height,
            true,
        ));
    }
    // between nvim and the shell (inside the preview's frame, above the bar)
    if show_preview {
        let desk = crate::ui::desktop_preview::desk_area(preview);
        let ((a, _), stacked) =
            crate::ui::desktop_preview::split_windows(desk, wide, sizes.preview_split);
        borders.push(if stacked {
            h_border(BorderId::PreviewSplit, a, desk.y, desk.height, false)
        } else {
            v_border(BorderId::PreviewSplit, a, desk.x, desk.width)
        });
    }
    Arrangement::Main(MainLayout {
        roles,
        font_list,
        sample: sample.then_some(sample_area),
        colour_list,
        picker: picker.then_some(picker_area),
        preview,
        status,
        wide,
        borders,
    })
}
