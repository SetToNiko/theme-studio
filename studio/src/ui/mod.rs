//! Drawing. The studio's own chrome uses named terminal colours (so it
//! follows the system theme); only swatches and the preview use the palette
//! being edited.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::app::state::nav::{Focus, Overlay, Page, Side};
use crate::app::{App, open_matches};
use crate::color::Rgb;
use crate::config::settings::Sizes;
use crate::hit::{Border, BorderId, Target, hover};
use crate::keymap::Context;
use crate::keymap::action::Action;
mod pages;
pub mod pane_chrome;
pub mod widgets;
use pages::{draw_body_page, draw_main_page, draw_too_small};
use pane_chrome::pane_border;
use widgets::friendly;
mod panes;

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
        let desk = crate::preview::desk_area(preview);
        let ((a, _), stacked) = crate::preview::split_windows(desk, wide, sizes.preview_split);
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
    draw_overlay(f, app, full);
    draw_pending(f, app, full);
    draw_notice(f, app, full);
}

/// A save confirmation in a box (settings.messages.saved = "popup").
fn draw_notice(f: &mut Frame, app: &App, area: Rect) {
    let Some(text) = &app.messages.notice else {
        return;
    };
    let hint = "press any key to dismiss";
    let w = text.chars().count().max(hint.len()) as u16 + 4;
    let lines = vec![
        Line::styled(format!(" {text}"), Style::new().green().bold()),
        Line::raw(""),
        Line::styled(format!(" {hint}"), Style::new().dark_gray()),
    ];
    let r = centered(area, w, 5);
    f.render_widget(Clear, r);
    let block = Block::bordered().border_style(Style::new().green());
    f.render_widget(Paragraph::new(lines).block(block), r);
}

/// The page tabs (F1 Main, F2 Fonts, F3 Settings), clickable; returns the
/// width used.
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

/// A swatch: `width` cells of background colour.
fn swatch(c: Rgb, width: usize) -> Span<'static> {
    Span::styled(" ".repeat(width), Style::new().bg(c.color()))
}

// ----- status line --------------------------------------------------------

fn draw_status(f: &mut Frame, app: &mut App, area: Rect) {
    // the page tabs start the line unless they're on top (settings: layout.tabs)
    let used = if app.settings.layout.tabs_on_top() {
        0
    } else {
        draw_tabs(f, app, area)
    };
    let area = Rect {
        x: area.x + used,
        width: area.width.saturating_sub(used),
        ..area
    };
    let app = &*app;
    let fonts = app.side == Side::Fonts;
    // the message at the left (after the tabs), the key hints at the right
    let message = app
        .messages
        .message
        .as_ref()
        .map(|(msg, err)| {
            Span::styled(
                truncate(&format!(" {msg} "), area.width as usize),
                if *err {
                    Style::new().red().bold()
                } else {
                    Style::new().green()
                },
            )
        })
        // otherwise, a note about what the mouse is over
        .or_else(|| {
            app.messages.hover_note.as_ref().map(|n| {
                Span::styled(
                    truncate(&format!(" {n} "), area.width as usize),
                    Style::new().dark_gray(),
                )
            })
        })
        // otherwise, previews being fetched ahead of time
        .or_else(|| {
            let p = &app.prefetch;
            p.active().then(|| {
                Span::styled(
                    format!(" fetching previews {}/{} ", p.done, p.total),
                    Style::new().dark_gray(),
                )
            })
        });
    let msg_w = message.as_ref().map_or(0, |s| s.width());
    let hints = {
        use Action as A;
        let hint = |actions: &[Action], label: &str| -> Option<String> {
            let ctx = app.contexts();
            let keys: Vec<String> = actions
                .iter()
                .filter_map(|a| app.keymap.key_for(&ctx, *a))
                .collect();
            let keys: Vec<String> = keys.iter().map(|k| friendly(k)).collect();
            (!keys.is_empty()).then(|| format!("{} {label}", keys.join("/")))
        };
        // picking for a role: only the chips that aren.t locked
        let chips: Vec<Action> = crate::fonts::types::CHIPS
            .iter()
            .filter(|(c, _)| app.fonts.filters.chip_free(*c))
            .map(|(c, _)| A::Filter(*c))
            .collect();
        let context: Vec<String> = if app.layout.resize_mode {
            let k = |a| {
                app.keymap
                    .key_for(&[Context::Resize], a)
                    .map(|s| friendly(&s))
                    .unwrap_or_default()
            };
            vec![
                "RESIZE".into(),
                format!("{}/{} side edge", k(A::Left), k(A::Right)),
                format!("{}/{} lower edge", k(A::Down), k(A::Up)),
                "capitals by 5".into(),
                format!("{} reset", k(A::ResetSizes)),
                format!("{} done", k(A::Back)),
            ]
        } else if app.layout.page_tabs_focused {
            let back = if app.settings.layout.tabs_on_top() {
                A::Down
            } else {
                A::Up
            };
            [
                hint(&[A::Left, A::Right], "pages"),
                hint(&[back], "back to the page"),
            ]
            .into_iter()
            .flatten()
            .collect()
        } else if app.page == Page::Main
            && app.layout.palette_menu_focused
            && app.side == Side::Colours
        {
            [hint(&[A::Confirm], "palettes"), hint(&[A::Down], "colours")]
                .into_iter()
                .flatten()
                .collect()
        } else if app.page == Page::Downloads && app.dl.searching {
            vec!["type to search".into(), "Enter/Esc done".into()]
        } else if app.page == Page::Downloads
            && app.dl.tab == crate::fonts::download_page::DlTab::Tournament
            && !app.dl.tabs_focused
        {
            use crate::tour_page::Stage;
            let stage = app.tour_page.as_ref().map_or(Stage::Setup, |t| t.stage);
            let bracket = app.tour_page.as_ref().is_some_and(|t| t.bracket);
            let rows: Vec<(&[Action], &str)> = match stage {
                _ if bracket => vec![(&[A::Down, A::Up], "scroll"), (&[A::Back], "close")],
                Stage::Setup => vec![
                    (&[A::Down, A::Up], "move"),
                    (&[A::Confirm], "start / open"),
                    (&[A::Left, A::Right], "pool size"),
                    (&[A::Delete], "delete"),
                ],
                Stage::Fetching => vec![(&[A::Back], "stop")],
                Stage::Playing => vec![
                    (&[A::Left, A::Right], "pick"),
                    (&[A::TourUndo], "take back"),
                    (&[A::TourNames], "names"),
                    (&[A::TourBracket], "bracket"),
                    (&[A::PreviewText], "code/prose"),
                    (&[A::PreviewLang], "language"),
                    (&[A::SizeUp, A::SizeDown], "size"),
                    (&[A::Back], "leave"),
                ],
                Stage::Results => vec![
                    (&[A::Down, A::Up], "move"),
                    (&[A::Confirm], "install"),
                    (
                        &[A::UseFor(0), A::UseFor(1), A::UseFor(2)],
                        "use for a role",
                    ),
                    (&[A::FillFamily], "every role"),
                    (&[A::TourBracket], "bracket"),
                    (&[A::Back], "leave"),
                ],
            };
            rows.into_iter().filter_map(|(a, l)| hint(a, l)).collect()
        } else if app.page == Page::Downloads {
            let mut v: Vec<String> = [
                hint(&[A::Down, A::Up], "move"),
                hint(&[A::Confirm], "install"),
                hint(&[A::Delete], "remove"),
                hint(&[A::Search], "search"),
            ]
            .into_iter()
            .flatten()
            .collect();
            if app.dl.tab == crate::fonts::download_page::DlTab::Google {
                v.extend(hint(&[A::Left, A::Right], "category"));
            }
            v.extend(hint(&[A::TabPrev, A::TabNext], "tabs"));
            v
        } else if app.page == Page::Settings && app.settings_page.tabs_focused {
            let back = if app.settings.layout.inner_tabs_on_top() {
                A::Down
            } else {
                A::Up
            };
            [
                hint(&[A::Left, A::Right], "tabs"),
                hint(&[back], "back to the list"),
            ]
            .into_iter()
            .flatten()
            .collect()
        } else if app.page == Page::Settings {
            use crate::settings::Tab;
            let rows: Vec<(&[Action], &str)> = match app.settings_page.tab {
                Tab::Keymaps if app.settings_page.browse.is_some() => vec![
                    (&[A::Down, A::Up], "presets"),
                    (&[A::Confirm], "load"),
                    (&[A::SaveNamed], "new from it"),
                    (&[A::Delete], "delete"),
                    (&[A::ToggleAll], "everything"),
                    (&[A::Back], "back"),
                ],
                Tab::Keymaps => vec![
                    (&[A::Down, A::Up], "move"),
                    (&[A::Confirm], "rebind"),
                    (&[A::Add], "add keys"),
                    (&[A::Delete], "unbind"),
                    (&[A::SaveNamed], "save preset"),
                ],
                Tab::Behaviour => vec![
                    (&[A::Down, A::Up], "move"),
                    (&[A::Left, A::Right], "change"),
                ],
                Tab::History => vec![
                    (&[A::Left, A::Right], "files/versions"),
                    (&[A::Restore], "restore"),
                    (&[A::SaveNamed], "name"),
                    (&[A::ToggleAll], "everything"),
                    (&[A::Delete], "delete named"),
                ],
                Tab::Readme => vec![
                    (&[A::Down, A::Up], "scroll"),
                    (&[A::PageDown, A::PageUp], "page"),
                    (&[A::NextHeading, A::PrevHeading], "headings"),
                    (&[A::Top, A::Bottom], "top/end"),
                ],
            };
            let mut v: Vec<String> = rows.into_iter().filter_map(|(a, l)| hint(a, l)).collect();
            v.extend(hint(&[A::TabPrev, A::TabNext], "tabs"));
            v
        } else if fonts && app.fonts.focus == crate::fonts::types::FontFocus::Search {
            vec!["type to search".into(), "Enter/Esc done".into()]
        } else if app.preview.focus.is_some() {
            let out = app
                .keymap
                .key_for(&[Context::Global], A::PreviewCycle)
                .unwrap_or_default();
            vec![
                "keys go to the preview window (Ctrl+C too)".into(),
                format!("{} next window / back", friendly(&out)),
            ]
        } else if !fonts && app.focus == Focus::Hex {
            vec![
                "type #rrggbb".into(),
                "Enter set".into(),
                "Esc cancel".into(),
            ]
        } else {
            let rows: Vec<(&[Action], &str)> = if fonts {
                match app.fonts.focus {
                    crate::fonts::types::FontFocus::Sample => vec![
                        (&[A::ToggleSample], "hide sample"),
                        (&[A::FocusNext], "next pane"),
                    ],
                    crate::fonts::types::FontFocus::Slots => {
                        let mut v: Vec<(&[Action], &str)> = vec![
                            (&[A::Down, A::Up], "role"),
                            (&[A::Left, A::Right], "size"),
                            (&[A::PickFont], "pick font"),
                        ];
                        // the role.s font doesn.t fit it
                        if app.fonts.misfit(app.fonts.slot) {
                            v.push((&[A::FixRoleFont], "fitting face"));
                        }
                        v
                    }
                    _ => vec![
                        (&[A::Down, A::Up], "browse"),
                        (&[A::Right, A::Left], "open/close"),
                        (&[A::Confirm], "use"),
                        (&[A::FillFamily], "whole family"),
                        (&[A::Search], "search"),
                        (&chips, "filters"),
                        (&[A::Back], "back"),
                    ],
                }
            } else {
                match app.focus {
                    Focus::List => vec![
                        (&[A::Down, A::Up], "move"),
                        (&[A::Confirm], "edit"),
                        (&[A::Pin], "pin"),
                        (&[A::CopyFrom], "copy"),
                        (&[A::HexEntry], "hex"),
                        (&[A::TogglePicker], "picker"),
                    ],
                    Focus::Square => vec![
                        (&[A::Left, A::Down, A::Up, A::Right], "saturation/value"),
                        (&[A::LeftBig, A::DownBig, A::UpBig, A::RightBig], "faster"),
                        (&[A::Back], "list"),
                    ],
                    Focus::Hue => vec![
                        (&[A::Down, A::Up], "hue"),
                        (&[A::DownBig, A::UpBig], "faster"),
                        (&[A::Back], "list"),
                    ],
                    _ => vec![
                        (&[A::Left, A::Right], "adjust"),
                        (&[A::LeftBig, A::RightBig], "faster"),
                        (&[A::Down, A::Up], "next slider"),
                    ],
                }
            };
            rows.into_iter().filter_map(|(a, l)| hint(a, l)).collect()
        };
        let panels = match (hint(&[A::Panel(1)], ""), hint(&[A::Panel(7)], "")) {
            // "Space 1-7 panels" rather than "Space 1-Space 7 panels"
            (Some(a), Some(b)) => {
                let (a, b) = (a.trim(), b.trim());
                let shared = a.len().saturating_sub(1);
                Some(if a.len() == b.len() && a[..shared] == b[..shared] {
                    format!("{a}-{} panels", &b[shared..])
                } else {
                    format!("{a}-{b} panels")
                })
            }
            _ => None,
        };
        // the main page's keys don't work on the other pages
        let main = app.page == Page::Main;
        let general: Vec<String> = [
            hint(&[A::FocusNext], "next").filter(|_| main),
            panels.filter(|_| main),
            hint(&[A::Save], "save").filter(|_| main),
            hint(&[A::Apply], "apply").filter(|_| main),
            hint(&[A::Open], "open").filter(|_| main),
        ]
        .into_iter()
        .flatten()
        .collect();
        // hidden (settings: layout.key-hints, or the toggle): none
        if !app.layout.hints_visible {
            return draw_message_only(f, area, message);
        }
        let last = hint(&[A::Help], "keys").unwrap_or_default();
        // a message keeps a space between it and the hints
        let room = (area.width as usize).saturating_sub(msg_w + usize::from(msg_w > 0));
        fit_hints(&context, &general, &last, room)
    };
    if let Some(m) = message {
        f.render_widget(Paragraph::new(Line::from(m)), area);
    }
    let hints_w = (hints.chars().count() as u16).min(area.width);
    let right = Rect {
        x: area.right() - hints_w,
        width: hints_w,
        ..area
    };
    f.render_widget(
        Paragraph::new(Span::styled(hints, Style::new().dark_gray())),
        right,
    );
}

/// As many hints as fit in `room` columns: the ones for the current control
/// first, then the general ones, and `last` (the help key) always at the end.
/// The status line without key hints: just the message, if any.
fn draw_message_only(f: &mut Frame, area: Rect, message: Option<Span<'static>>) {
    if let Some(m) = message {
        f.render_widget(Paragraph::new(Line::from(m)), area);
    }
}

fn fit_hints(context: &[String], general: &[String], last: &str, room: usize) -> String {
    let mut out = String::from(" ");
    let budget = room.saturating_sub(last.chars().count() + 3);
    let mut general_started = false;
    for (i, hint) in context.iter().chain(general.iter()).enumerate() {
        let sep = if i == 0 {
            ""
        } else if i == context.len() && !general_started {
            general_started = true;
            " │ "
        } else {
            "  "
        };
        if out.chars().count() + sep.chars().count() + hint.chars().count() > budget {
            break;
        }
        out.push_str(sep);
        out.push_str(hint);
    }
    if out.trim().is_empty() {
        out = format!(" {last} ");
    } else {
        out.push_str("  ");
        out.push_str(last);
        out.push(' ');
    }
    if out.chars().count() > room {
        String::new()
    } else {
        out
    }
}

// ----- pop-ups ------------------------------------------------------------

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

fn popup(f: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
    f.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(" {title} "))
        .border_style(Style::new().yellow());
    f.render_widget(Paragraph::new(lines).block(block), area);
}

/// While a sequence is half typed (e.g. after the leader): what can follow.
fn draw_pending(f: &mut Frame, app: &App, area: Rect) {
    if app.pending_keys.is_empty() {
        return;
    }
    let ctx = app.contexts();
    let next = app.keymap.continuations(&ctx, &app.pending_keys);
    let typed = crate::keymap::key::display_seq(&app.pending_keys, app.keymap.leader);
    let mut lines: Vec<Line> = next
        .iter()
        .map(|(k, what)| {
            Line::from(vec![
                Span::styled(format!(" {k:<6}"), Style::new().yellow()),
                Span::raw(what.clone()),
            ])
        })
        .collect();
    lines.push(Line::styled(" Esc cancels", Style::new().dark_gray()));
    let w = lines.iter().map(|l| l.width()).max().unwrap_or(10) as u16 + 4;
    let h = lines.len() as u16 + 2;
    let r = Rect {
        x: area.x + area.width.saturating_sub(w + 1),
        y: area.y + area.height.saturating_sub(h + 1),
        width: w.min(area.width),
        height: h.min(area.height),
    };
    f.render_widget(Clear, r);
    let block = Block::bordered()
        .title(format!(" {typed} "))
        .border_style(Style::new().yellow());
    f.render_widget(Paragraph::new(lines).block(block), r);
}
fn draw_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    match &app.overlay {
        Overlay::None | Overlay::CopyFrom { .. } => {
            if let Overlay::CopyFrom { target } = app.overlay {
                let r = Rect {
                    x: area.x,
                    y: area.y + area.height - 1,
                    width: area.width,
                    height: 1,
                };
                f.render_widget(Clear, r);
                f.render_widget(
                    Paragraph::new(format!(" Copy into {target} from: choose a key with j/k, Enter to copy, Esc to cancel ")).yellow().reversed(),
                    r,
                );
            }
        }
        Overlay::Help { page } => {
            // Generated from the keymap, so it shows what the keys really are.
            let mut lines: Vec<Line> = Vec::new();
            for ctx in Context::ALL {
                // one row per action, all its keys together
                let mut rows: Vec<(Action, Vec<String>)> = Vec::new();
                for b in app.keymap.bindings.iter().filter(|b| b.context == ctx) {
                    let k = crate::keymap::key::display_seq(&b.keys, app.keymap.leader);
                    match rows.iter_mut().find(|(a, _)| *a == b.action) {
                        Some((_, keys)) => keys.push(k),
                        None => rows.push((b.action, vec![k])),
                    }
                }
                if rows.is_empty() {
                    continue;
                }
                lines.push(Line::styled(
                    format!(" {}", ctx.title()),
                    Style::new().bold(),
                ));
                for (a, keys) in rows {
                    lines.push(Line::from(vec![
                        Span::styled(format!("   {:<20}", keys.join(" ")), Style::new().yellow()),
                        Span::raw(a.describe()),
                    ]));
                }
                // what each role's font has to be (the list shows only those)
                if ctx == Context::FontRoles {
                    for (name, need) in crate::fonts::types::ROLE_NAMES
                        .iter()
                        .zip(crate::fonts::types::ROLE_NEEDS)
                    {
                        lines.push(Line::from(vec![
                            Span::styled(format!("   {name:<20}"), Style::new().dark_gray()),
                            Span::raw(format!("needs {need}")),
                        ]));
                    }
                }
            }
            lines.push(Line::styled(
                format!(
                    " Leader: {}   Text fields take keys as typing.   ←/→ more columns; any other key closes.",
                    crate::keymap::key::display_seq(&[app.keymap.leader], crate::keymap::key::Key::new(ratatui::crossterm::event::KeyCode::Null, ratatui::crossterm::event::KeyModifiers::NONE))
                ),
                Style::new().dark_gray(),
            ));
            // columns wide enough to read; when they don't all fit, !90/!92 page
            const COL_W: u16 = 52;
            let r = centered(
                area,
                area.width.saturating_sub(4).min(4 * COL_W + 2),
                area.height.saturating_sub(2),
            );
            f.render_widget(Clear, r);
            let inner_h = r.height.saturating_sub(2) as usize;
            let per_col = inner_h.max(1);
            let cols: Vec<&[Line]> = lines.chunks(per_col).collect();
            let fit = (r.width.saturating_sub(2) / COL_W).max(1) as usize;
            let page = (*page).min(cols.len().saturating_sub(fit));
            let more = if cols.len() > fit {
                format!(
                    " columns {} 13{} of {} (!90/!92 for more) ",
                    page + 1,
                    (page + fit).min(cols.len()),
                    cols.len()
                )
            } else {
                String::new()
            };
            let block = Block::bordered()
                .title(format!(" Keys: {} preset ", app.keymap.name))
                .title_bottom(Line::styled(more, Style::new().yellow()))
                .border_style(Style::new().yellow());
            let inner = block.inner(r);
            f.render_widget(block, r);
            for (i, chunk) in cols.iter().skip(page).take(fit).enumerate() {
                let col = Rect {
                    x: inner.x + i as u16 * COL_W,
                    width: COL_W.min(inner.right().saturating_sub(inner.x + i as u16 * COL_W)),
                    ..inner
                };
                f.render_widget(Paragraph::new(chunk.to_vec()), col);
            }
        }
        Overlay::Confirm(p) => {
            // what's unsaved: the palette, the font preset, the settings
            let quit = matches!(p, crate::app::state::nav::Pending::Quit);
            let fonts = matches!(
                p,
                crate::app::state::nav::Pending::NewFonts
                    | crate::app::state::nav::Pending::OpenFonts(_)
            );
            let mut what = Vec::new();
            if (quit || !fonts) && app.dirty() {
                what.push(format!("palette \"{}\"", app.name));
            }
            if (quit || fonts) && app.fonts.dirty() {
                what.push(format!("font preset \"{}\"", app.fonts.name));
            }
            if (quit
                || matches!(
                    p,
                    crate::app::state::nav::Pending::NewTheme
                        | crate::app::state::nav::Pending::OpenTheme(_)
                ))
                && app.theme_dirty()
                && what.is_empty()
            {
                // only the parts swapped (another palette or preset)
                what.push(format!("theme \"{}\"", app.theme));
            }
            if quit && app.settings_page.draft.is_some() {
                what.push("settings".to_string());
            }
            let what = format!(" Unsaved: {}.", what.join(", "));
            let r = centered(area, (what.chars().count() as u16 + 4).max(56), 5);
            let y = r.y + 3;
            // the choices are clickable, like their keys
            let mut button = |text: &'static str, x: u16, choice: char, base: Style| {
                let w = text.chars().count() as u16;
                let hovered = app.hits.add(
                    Rect {
                        x,
                        y,
                        width: w,
                        height: 1,
                    },
                    Target::ConfirmChoice(choice),
                );
                (
                    Span::styled(text, if hovered { hover(base) } else { base }),
                    x + w,
                )
            };
            let (discard, x) = button(" y  discard them ", r.x + 1, 'y', Style::new().yellow());
            let mut spans = vec![discard];
            let mut x = x;
            if !app.is_new {
                spans.push("  ".into());
                let (save, nx) = button(" s  save first ", x + 2, 's', Style::new());
                spans.push(save);
                x = nx;
            }
            spans.push("  ".into());
            spans.push(button(" Esc  cancel ", x + 2, 'n', Style::new()).0);
            let lines = vec![Line::raw(what), Line::raw(""), Line::from(spans)];
            popup(f, r, "Unsaved changes", lines);
        }
        Overlay::SaveAs {
            input, overwrite, ..
        } => {
            let mut lines = vec![
                Line::raw(match app.overlay_for {
                    crate::app::state::nav::For::Fonts => " Name in fonts.json:",
                    crate::app::state::nav::For::Palette => " Name in palettes.json:",
                    crate::app::state::nav::For::Theme => " Theme name (new parts get it too):",
                }),
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(format!("{input}▏"), Style::new().yellow()),
                ]),
            ];
            if *overwrite {
                lines.push(Line::styled(
                    " exists: Enter again to overwrite",
                    Style::new().red(),
                ));
            }
            popup(f, centered(area, 50, 5), "Save as", lines);
        }
        Overlay::Capture { what, keys } => {
            let typed = crate::keymap::key::display_seq(keys, app.keymap.leader);
            let lines = match what {
                crate::app::state::nav::Capture::Leader => vec![
                    Line::raw(" Press the new leader key."),
                    Line::styled(" Esc cancels.", Style::new().dark_gray()),
                ],
                crate::app::state::nav::Capture::Binding { action, add, .. } => vec![
                    Line::raw(format!(
                        " {} keys for \"{}\":",
                        if *add { "Extra" } else { "New" },
                        action.describe()
                    )),
                    Line::from(vec![
                        Span::raw(" "),
                        Span::styled(format!("{typed}▏"), Style::new().yellow().bold()),
                    ]),
                    Line::styled(
                        " Press keys (a sequence is fine). Enter keeps them, Backspace takes one back, Esc cancels.",
                        Style::new().dark_gray(),
                    ),
                ],
            };
            popup(
                f,
                centered(area, 64, lines.len() as u16 + 2),
                "Record keys",
                lines,
            );
        }
        Overlay::ChooseKey { sel, original, .. } => {
            let keys = crate::behaviour::theme_keys();
            // at the right edge, so the rows it previews stay in view
            let mut r = centered(
                area,
                40,
                area.height.saturating_sub(4).min(keys.len() as u16 + 3),
            );
            r.x = area.right().saturating_sub(r.width + 2).max(area.x);
            let list_h = r.height.saturating_sub(3) as usize;
            let start = sel.saturating_sub(list_h.saturating_sub(1));
            let current = original.clone();
            let mut lines = vec![Line::styled(
                " j/k move, Enter choose, Esc cancel",
                Style::new().dark_gray(),
            )];
            for (i, key) in keys.iter().enumerate().skip(start).take(list_h) {
                let rr = Rect {
                    x: r.x + 1,
                    y: r.y + 1 + lines.len() as u16,
                    width: r.width.saturating_sub(2),
                    height: 1,
                };
                let mut line = Line::from(vec![
                    Span::styled(" ██ ", Style::new().fg(app.theme_colour(key))),
                    Span::styled(format!("{key:<20}"), Style::new()),
                    Span::styled(
                        if *key == current { " current" } else { "" },
                        Style::new().dark_gray(),
                    ),
                ]);
                if i == *sel {
                    // the cursor on the text, not the swatch or padding
                    line = crate::hit::cursor_line(line, Style::new().reversed());
                }
                if app.hits.add(rr, Target::OverlayRow(i)) && i != *sel {
                    line = crate::hit::hover_line(line, 1);
                }
                lines.push(line);
            }
            popup(f, r, "Colour from the active theme", lines);
        }
        Overlay::Prompt { title, input, .. } => {
            let lines = vec![
                Line::raw(format!(" {title}:")),
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(format!("{input}▏"), Style::new().yellow()),
                ]),
                Line::styled(" Enter keeps it, Esc cancels.", Style::new().dark_gray()),
            ];
            popup(f, centered(area, 64, 5), "Name", lines);
        }
        Overlay::Ask { question, .. } => {
            let r = centered(area, 64, 6);
            let y = r.y + 4;
            let mut button = |text: &'static str, x: u16, choice: char, base: Style| {
                let w = text.chars().count() as u16;
                let hovered = app.hits.add(
                    Rect {
                        x,
                        y,
                        width: w,
                        height: 1,
                    },
                    Target::ConfirmChoice(choice),
                );
                (
                    Span::styled(text, if hovered { hover(base) } else { base }),
                    x + w,
                )
            };
            let (yes, x) = button(" y  yes ", r.x + 1, 'y', Style::new().yellow());
            let (no, _) = button(" Esc  no ", x + 2, 'n', Style::new());
            let lines = vec![
                Line::raw(format!(" {}", truncate(question, 60))),
                Line::raw(if question.chars().count() > 60 {
                    format!(" {}", question.chars().skip(60).collect::<String>())
                } else {
                    String::new()
                }),
                Line::raw(""),
                Line::from(vec![yes, "  ".into(), no]),
            ];
            popup(f, r, "Sure?", lines);
        }
        Overlay::Menu { menu, sel, at } => {
            // hangs under the title text that opened it
            let items = crate::fonts::preview_page::menu_items(*menu);
            let w = (items.iter().map(|s| s.chars().count()).max().unwrap_or(4) as u16 + 4)
                .min(area.width);
            let h = (items.len() as u16 + 2).min(area.bottom().saturating_sub(at.y + 1));
            let r = Rect {
                x: at.x.saturating_sub(1).min(area.right().saturating_sub(w)),
                y: at.y + 1,
                width: w,
                height: h,
            };
            let mut lines = Vec::new();
            for (i, item) in items.iter().enumerate().take(h.saturating_sub(2) as usize) {
                let rr = Rect::new(r.x + 1, r.y + 1 + i as u16, r.width.saturating_sub(2), 1);
                let mut line = Line::raw(format!(" {item:<width$} ", width = w as usize - 4));
                if i == *sel {
                    line = crate::hit::cursor_line(line, Style::new().reversed());
                }
                lines.push(if app.hits.add(rr, Target::OverlayRow(i)) {
                    crate::hit::hover_line(line, 0)
                } else {
                    line
                });
            }
            f.render_widget(Clear, r);
            f.render_widget(
                Paragraph::new(lines).block(Block::bordered().border_style(Style::new().yellow())),
                r,
            );
        }
        Overlay::Open {
            filter,
            sel,
            entries,
        } => {
            // the palette list hangs under the Palette pane's dropdown row; the
            // other lists (and this one on other pages) are centred
            let hanging = match app.layout.palette_pane {
                Some(p) if app.overlay_for == crate::app::state::nav::For::Palette => {
                    let w = 70.min(area.width);
                    let h = (entries.len() as u16 + 4)
                        .min(30)
                        .min(area.bottom().saturating_sub(p.y + 2));
                    Some(Rect {
                        x: p.x.min(area.right().saturating_sub(w)),
                        y: p.y + 2,
                        width: w,
                        height: h,
                    })
                }
                _ => None,
            };
            let r = hanging
                .unwrap_or_else(|| centered(area, 70, area.height.saturating_sub(6).min(30)));
            let matches = open_matches(entries, filter);
            let list_h = r.height.saturating_sub(4) as usize;
            let start = sel.saturating_sub(list_h.saturating_sub(1));
            let mut lines = vec![
                Line::from(vec![
                    " Search: ".into(),
                    Span::styled(format!("{filter}▏"), Style::new().yellow()),
                ]),
                Line::styled(
                    format!(
                        " {} of {}{}",
                        matches.len(),
                        entries.len(),
                        if app.dirty() {
                            "   (unsaved changes: you'll be asked)"
                        } else {
                            ""
                        }
                    ),
                    Style::new().dark_gray(),
                ),
            ];
            for (row, &i) in matches.iter().enumerate().skip(start).take(list_h) {
                let (name, sw) = &entries[i];
                let mut spans = vec![Span::raw(format!(" {:<34}", truncate(name, 33)))];
                for c in sw {
                    spans.push(swatch(*c, 2));
                }
                if app.active.as_deref() == Some(name) {
                    spans.push(Span::styled(" active", Style::new().dark_gray()));
                }
                // rows are clickable (a second click opens), hover-highlighted
                let rr = Rect {
                    x: r.x + 1,
                    y: r.y + 1 + lines.len() as u16,
                    width: r.width.saturating_sub(2),
                    height: 1,
                };
                let mut line = Line::from(spans);
                if row == *sel {
                    // the cursor on the text, not the padding or swatches
                    line = crate::hit::cursor_line(line, Style::new().reversed());
                }
                lines.push(if app.hits.add(rr, Target::OverlayRow(row)) {
                    crate::hit::hover_line(line, 0)
                } else {
                    line
                });
            }
            popup(
                f,
                r,
                match app.overlay_for {
                    crate::app::state::nav::For::Fonts => "Open a font preset for this theme",
                    crate::app::state::nav::For::Palette => "Open a palette for this theme",
                    crate::app::state::nav::For::Theme => "Open theme",
                },
                lines,
            );
        }
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::fit_hints;

    #[test]
    fn hints_shorten_to_fit_and_keep_help_last() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let ctx = s(&["j/k move", "Enter edit", "p pin"]);
        let general = s(&["Tab next", "1-4 panels", "s save", "a apply", "o open"]);
        let wide = fit_hints(&ctx, &general, "? keys", 200);
        assert_eq!(
            wide,
            " j/k move  Enter edit  p pin │ Tab next  1-4 panels  s save  a apply  o open  ? keys "
        );
        let narrow = fit_hints(&ctx, &general, "? keys", 40);
        assert!(narrow.chars().count() <= 40, "{narrow}");
        assert!(
            narrow.starts_with(" j/k move  Enter edit") && narrow.ends_with("? keys "),
            "{narrow}"
        );
        assert_eq!(fit_hints(&ctx, &general, "? keys", 10), " ? keys ");
        assert_eq!(fit_hints(&ctx, &general, "? keys", 5), "");
        for room in 0..120 {
            assert!(fit_hints(&ctx, &general, "? keys", room).chars().count() <= room);
        }
    }
}
