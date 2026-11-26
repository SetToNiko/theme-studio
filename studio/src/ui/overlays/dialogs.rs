//! Dialog overlays: help, the unsaved-changes box, save as, prompts, key lists and dropdowns.
//!
//! [`draw_overlay`] picks the drawing function for the overlay that is open.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::{centered, popup};
use crate::app::state::nav::{Capture, Overlay, Pending};
use crate::app::{App, open_matches};
use crate::color::Rgb;
use crate::hit::{Target, hover};
use crate::keymap::Context;
use crate::keymap::action::Action;
use crate::keymap::key::Key;
use crate::ui::{swatch, truncate};

/// Draws the open overlay (if any), on top of the page.
pub fn draw_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    // Take the overlay out while drawing: the drawing functions need `app`
    // mutably (click areas), and the overlay data must not borrow from it.
    let overlay = std::mem::replace(&mut app.overlay, Overlay::None);
    match &overlay {
        Overlay::None => {}
        Overlay::CopyFrom { target } => draw_copy_from(f, area, target),
        Overlay::Help { page } => draw_help(f, app, area, *page),
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

/// The key list, in columns, generated from the keymap (`!90`/`!92` page through them).
// Long function: rows from the keymap, the column layout and the frame use the same lines and sizes.
fn draw_help(f: &mut Frame, app: &App, area: Rect, page: usize) {
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
    let page = page.min(cols.len().saturating_sub(fit));
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

/// The "Unsaved changes" box with its clickable choices.
// Long function: what is unsaved and the buttons share the same variables.
fn draw_confirm(f: &mut Frame, app: &mut App, area: Rect, p: &Pending) {
    // what's unsaved: the palette, the font preset, the settings
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
        centered(area, 64, lines.len() as u16 + 2),
        "Record keys",
        lines,
    );
}

/// The list of theme keys for a hover colour, at the right edge of the screen.
// Long function: the box size, the scroll window and the clickable rows use the same values.
fn draw_choose_key(f: &mut Frame, app: &mut App, area: Rect, sel: usize, original: &str) {
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
    let current = original;
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
        if i == sel {
            // the cursor on the text, not the swatch or padding
            line = crate::hit::cursor_line(line, Style::new().reversed());
        }
        if app.hits.add(rr, Target::OverlayRow(i)) && i != sel {
            line = crate::hit::hover_line(line, 1);
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
// Long function: the box, the two clickable buttons and the question lines share the same positions.
fn draw_ask(f: &mut Frame, app: &mut App, area: Rect, question: &str) {
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

/// The dropdown list that hangs under the preview title text that opened it.
fn draw_menu(f: &mut Frame, app: &mut App, area: Rect, menu: u8, sel: usize, at: Rect) {
    // hangs under the title text that opened it
    let items = crate::fonts::preview_page::menu_items(menu);
    let w = (items.iter().map(|s| s.chars().count()).max().unwrap_or(4) as u16 + 4).min(area.width);
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
        if i == sel {
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

/// The "Open ..." list with a search field (centred, or hanging under the Palette pane).
// Long function: finding the box, filtering and the clickable rows share the same positions and counts.
fn draw_open(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    filter: &str,
    sel: usize,
    entries: &[(String, Vec<Rgb>)],
) {
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
    let r = hanging.unwrap_or_else(|| centered(area, 70, area.height.saturating_sub(6).min(30)));
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
        if row == sel {
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
