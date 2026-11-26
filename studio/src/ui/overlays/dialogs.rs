//! Dialog overlays: help, the unsaved-changes box, save as, prompts, key lists and dropdowns.
//!
//! [`draw_overlay`] picks the drawing function for the overlay that is open.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::{centered, popup};
use crate::app::state::hit::{Target, hover};
use crate::app::state::nav::{Capture, Overlay, Pending};
use crate::app::{App, open_matches};
use crate::keymap::key::Key;
use crate::model::color::Rgb;
use crate::ui::widgets::to_u16;
use crate::ui::{swatch, truncate};

/// Draws the open overlay (if any), on top of the page.
pub fn draw_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    // Take the overlay out while drawing: the drawing functions need `app`
    // mutably (click areas), and the overlay data must not borrow from it.
    let overlay = std::mem::replace(&mut app.overlay, Overlay::None);
    match &overlay {
        Overlay::None => {}
        Overlay::CopyFrom { target } => draw_copy_from(f, area, target),
        Overlay::Help { page } => super::help::draw_help(f, app, area, *page),
        Overlay::Confirm(p) => draw_confirm(f, app, area, p),
        Overlay::SaveAs {
            input, overwrite, ..
        } => draw_save_as(f, app, area, input, *overwrite),
        Overlay::Capture { what, keys } => draw_capture(f, app, area, what, keys),
        Overlay::ChooseKey { sel, original, .. } => {
            draw_choose_key(f, app, area, *sel, original);
        }
        Overlay::Prompt { title, input, .. } => draw_prompt(f, area, title, input),
        Overlay::Ask { question, .. } => draw_ask(f, app, area, question),
        Overlay::Menu { menu, sel, at } => draw_menu(f, app, area, *menu, *sel, *at),
        Overlay::Open {
            filter,
            sel,
            entries,
        } => draw_open(f, app, area, filter, *sel, entries),
    }
    app.overlay = overlay;
}

/// The one-line prompt at the bottom while a colour is copied from another key.
fn draw_copy_from(f: &mut Frame, area: Rect, target: &str) {
    let r = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        width: area.width,
        height: 1,
    };
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(format!(
            " Copy into {target} from: choose a key with j/k, Enter to copy, Esc to cancel "
        ))
        .yellow()
        .reversed(),
        r,
    );
}

/// What is unsaved for the "Unsaved changes" box: the palette, the font
/// preset, the theme or the settings, depending on what is pending.
fn unsaved_text(app: &App, p: &Pending) -> String {
    let quit = matches!(p, Pending::Quit);
    let fonts = matches!(p, Pending::NewFonts | Pending::OpenFonts(_));
    let mut what = Vec::new();
    if (quit || !fonts) && app.dirty() {
        what.push(format!("palette \"{}\"", app.name));
    }
    if (quit || fonts) && app.fonts.dirty() {
        what.push(format!("font preset \"{}\"", app.fonts.name));
    }
    if (quit || matches!(p, Pending::NewTheme | Pending::OpenTheme(_)))
        && app.theme_dirty()
        && what.is_empty()
    {
        // only the parts swapped (another palette or preset)
        what.push(format!("theme \"{}\"", app.theme));
    }
    if quit && app.settings_page.draft.is_some() {
        what.push("settings".to_string());
    }
    format!(" Unsaved: {}.", what.join(", "))
}

/// A clickable choice button at column `x` on row `y`: the span to draw and
/// the column after it. The choice is clickable, like its key.
fn choice_button(
    app: &mut App,
    y: u16,
    (text, x, choice): (&'static str, u16, char),
    base: Style,
) -> (Span<'static>, u16) {
    let width = to_u16(text.chars().count());
    let hovered = app.hits.add(
        Rect {
            x,
            y,
            width,
            height: 1,
        },
        Target::ConfirmChoice(choice),
    );
    (
        Span::styled(text, if hovered { hover(base) } else { base }),
        x + width,
    )
}

/// The "Unsaved changes" box with its clickable choices.
fn draw_confirm(f: &mut Frame, app: &mut App, area: Rect, pending: &Pending) {
    let what = unsaved_text(app, pending);
    let r = centered(area, (to_u16(what.chars().count()) + 4).max(56), 5);
    let y = r.y + 3;
    let (discard, mut x) = choice_button(
        app,
        y,
        (" y  discard them ", r.x + 1, 'y'),
        Style::new().yellow(),
    );
    let mut spans = vec![discard];
    if !app.is_new {
        spans.push("  ".into());
        let (save, next) = choice_button(app, y, (" s  save first ", x + 2, 's'), Style::new());
        spans.push(save);
        x = next;
    }
    spans.push("  ".into());
    spans.push(choice_button(app, y, (" Esc  cancel ", x + 2, 'n'), Style::new()).0);
    let lines = vec![Line::raw(what), Line::raw(""), Line::from(spans)];
    popup(f, r, "Unsaved changes", lines);
}

/// The "Save as" box: a name field for a palette, a font preset or a theme.
fn draw_save_as(f: &mut Frame, app: &App, area: Rect, input: &str, overwrite: bool) {
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
    if overwrite {
        lines.push(Line::styled(
            " exists: Enter again to overwrite",
            Style::new().red(),
        ));
    }
    popup(f, centered(area, 50, 5), "Save as", lines);
}

/// The "Record keys" box on the settings page.
fn draw_capture(f: &mut Frame, app: &App, area: Rect, what: &Capture, keys: &[Key]) {
    let typed = crate::keymap::key::display_seq(keys, app.keymap.leader);
    let lines = match what {
        Capture::Leader => vec![
            Line::raw(" Press the new leader key."),
            Line::styled(" Esc cancels.", Style::new().dark_gray()),
        ],
        Capture::Binding { action, add, .. } => vec![
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
        centered(area, 64, to_u16(lines.len()) + 2),
        "Record keys",
        lines,
    );
}

/// The list of theme keys for a hover colour, at the right edge of the screen.
fn draw_choose_key(f: &mut Frame, app: &mut App, area: Rect, sel: usize, original: &str) {
    let keys = crate::settings_page::behaviour::theme_keys();
    // at the right edge, so the rows it previews stay in view
    let mut r = centered(
        area,
        40,
        area.height.saturating_sub(4).min(to_u16(keys.len()) + 3),
    );
    r.x = area.right().saturating_sub(r.width + 2).max(area.x);
    let list_h = r.height.saturating_sub(3) as usize;
    let start = sel.saturating_sub(list_h.saturating_sub(1));
    let mut lines = vec![Line::styled(
        " j/k move, Enter choose, Esc cancel",
        Style::new().dark_gray(),
    )];
    for (i, key) in keys.iter().enumerate().skip(start).take(list_h) {
        let row = Rect {
            x: r.x + 1,
            y: r.y + 1 + to_u16(lines.len()),
            width: r.width.saturating_sub(2),
            height: 1,
        };
        let mut line = Line::from(vec![
            Span::styled(" ██ ", Style::new().fg(app.theme_colour(key))),
            Span::styled(format!("{key:<20}"), Style::new()),
            Span::styled(
                if *key == original { " current" } else { "" },
                Style::new().dark_gray(),
            ),
        ]);
        if i == sel {
            // the cursor on the text, not the swatch or padding
            line = crate::app::state::hit::cursor_line(line, Style::new().reversed());
        }
        if app.hits.add(row, Target::OverlayRow(i)) && i != sel {
            line = crate::app::state::hit::hover_line(line, 1);
        }
        lines.push(line);
    }
    popup(f, r, "Colour from the active theme", lines);
}

/// The "Name" box on the settings page.
fn draw_prompt(f: &mut Frame, area: Rect, title: &str, input: &str) {
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

/// The "Sure?" box with its yes and no buttons.
fn draw_ask(f: &mut Frame, app: &mut App, area: Rect, question: &str) {
    let r = centered(area, 64, 6);
    let y = r.y + 4;
    let (yes, x) = choice_button(app, y, (" y  yes ", r.x + 1, 'y'), Style::new().yellow());
    let (no, _) = choice_button(app, y, (" Esc  no ", x + 2, 'n'), Style::new());
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

/// The dropdown list that hangs under the preview title text that opened it.
fn draw_menu(f: &mut Frame, app: &mut App, area: Rect, menu: u8, sel: usize, at: Rect) {
    // hangs under the title text that opened it
    let items = crate::fonts::preview_page::menu_items(menu);
    let widest = items.iter().map(|s| s.chars().count()).max().unwrap_or(4);
    let w = (to_u16(widest) + 4).min(area.width);
    let h = (to_u16(items.len()) + 2).min(area.bottom().saturating_sub(at.y + 1));
    let r = Rect {
        x: at.x.saturating_sub(1).min(area.right().saturating_sub(w)),
        y: at.y + 1,
        width: w,
        height: h,
    };
    let mut lines = Vec::new();
    for (i, item) in items.iter().enumerate().take(h.saturating_sub(2) as usize) {
        let row = Rect::new(r.x + 1, r.y + 1 + to_u16(i), r.width.saturating_sub(2), 1);
        let mut line = Line::raw(format!(" {item:<width$} ", width = usize::from(w) - 4));
        if i == sel {
            line = crate::app::state::hit::cursor_line(line, Style::new().reversed());
        }
        lines.push(if app.hits.add(row, Target::OverlayRow(i)) {
            crate::app::state::hit::hover_line(line, 0)
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

/// The box of the "Open ..." list: under the Palette pane's dropdown row for
/// the palette list, else centred.
fn open_box(app: &App, area: Rect, entry_count: usize) -> Rect {
    match app.layout.palette_pane {
        Some(p) if app.overlay_for == crate::app::state::nav::For::Palette => {
            let w = 70.min(area.width);
            let h = (to_u16(entry_count) + 4)
                .min(30)
                .min(area.bottom().saturating_sub(p.y + 2));
            Rect {
                x: p.x.min(area.right().saturating_sub(w)),
                y: p.y + 2,
                width: w,
                height: h,
            }
        }
        _ => centered(area, 70, area.height.saturating_sub(6).min(30)),
    }
}

/// The first two lines of the "Open ..." box: the search field and the counts.
fn open_header(app: &App, filter: &str, shown: usize, total: usize) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            " Search: ".into(),
            Span::styled(format!("{filter}▏"), Style::new().yellow()),
        ]),
        Line::styled(
            format!(
                " {shown} of {total}{}",
                if app.dirty() {
                    "   (unsaved changes: you'll be asked)"
                } else {
                    ""
                }
            ),
            Style::new().dark_gray(),
        ),
    ]
}

/// One row of the "Open ..." list: the name, its swatches and an "active" tag.
fn open_row(app: &App, name: &str, swatches: &[Rgb]) -> Line<'static> {
    let mut spans = vec![Span::raw(format!(" {:<34}", truncate(name, 33)))];
    for c in swatches {
        spans.push(swatch(*c, 2));
    }
    if app.active.as_deref() == Some(name) {
        spans.push(Span::styled(" active", Style::new().dark_gray()));
    }
    Line::from(spans)
}

/// The "Open ..." list with a search field (centred, or hanging under the Palette pane).
// Long function: the row loop needs the box rectangle, the scroll window and the line count at once, and registers each row's click area in line order.
fn draw_open(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    filter: &str,
    sel: usize,
    entries: &[(String, Vec<Rgb>)],
) {
    let r = open_box(app, area, entries.len());
    let matches = open_matches(entries, filter);
    let list_h = r.height.saturating_sub(4) as usize;
    let start = sel.saturating_sub(list_h.saturating_sub(1));
    let mut lines = open_header(app, filter, matches.len(), entries.len());
    for (row, &i) in matches.iter().enumerate().skip(start).take(list_h) {
        let (name, swatches) = &entries[i];
        let mut line = open_row(app, name, swatches);
        // rows are clickable (a second click opens), hover-highlighted
        let hit = Rect {
            x: r.x + 1,
            y: r.y + 1 + to_u16(lines.len()),
            width: r.width.saturating_sub(2),
            height: 1,
        };
        if row == sel {
            // the cursor on the text, not the padding or swatches
            line = crate::app::state::hit::cursor_line(line, Style::new().reversed());
        }
        lines.push(if app.hits.add(hit, Target::OverlayRow(row)) {
            crate::app::state::hit::hover_line(line, 0)
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
