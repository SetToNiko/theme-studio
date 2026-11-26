//! The status line: the page tabs, a message, and the key hints.
//!
//! `draw_status` puts the pieces side by side: the tabs (unless they are on
//! top), then the message at the left, then the key hints at the right. The
//! hints depend on where the focus is, so most of this file is one small
//! function per screen that lists the actions to show.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::widgets::friendly;
use super::{draw_tabs, truncate};
use crate::app::App;
use crate::app::state::nav::{Focus, Page, Side};
use crate::fonts::download_page::DlTab;
use crate::fonts::types::{CHIPS, FontFocus};
use crate::keymap::Context;
use crate::keymap::action::Action;
use crate::keymap::action::Action as A;

/// Hint rows: the actions whose keys are shown, and the label for them.
type Rows<'a> = Vec<(&'a [Action], &'static str)>;

/// Draws the status line: tabs, then the message, then the key hints.
// Long function: one place that lays out the tabs, message and right-aligned hints in order.
pub(super) fn draw_status(f: &mut Frame, app: &mut App, area: Rect) {
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
    // the message at the left (after the tabs), the key hints at the right
    let message = status_message(app, area.width);
    // hidden (settings: layout.key-hints, or the toggle): none
    if !app.layout.hints_visible {
        return draw_message_only(f, area, message);
    }
    let msg_w = message.as_ref().map_or(0, |s| s.width());
    // a message keeps a space between it and the hints
    let room = (area.width as usize).saturating_sub(msg_w + usize::from(msg_w > 0));
    let last = hint(app, &[A::Help], "keys").unwrap_or_default();
    let hints = fit_hints(&context_hints(app), &general_hints(app), &last, room);
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

/// The message to show, cut to `width`: a status message, else a note about
/// what the mouse is over, else the preview fetch progress.
// Long function: one fallback chain of three message sources, each with its own style.
fn status_message(app: &App, width: u16) -> Option<Span<'static>> {
    app.messages
        .message
        .as_ref()
        .map(|(msg, err)| {
            Span::styled(
                truncate(&format!(" {msg} "), width as usize),
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
                    truncate(&format!(" {n} "), width as usize),
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
        })
}

/// One hint, such as `"j/k move"`: the keys bound to `actions`, then `label`.
/// `None` when none of the actions has a key.
fn hint(app: &App, actions: &[Action], label: &str) -> Option<String> {
    let ctx = app.contexts();
    let keys: Vec<String> = actions
        .iter()
        .filter_map(|a| app.keymap.key_for(&ctx, *a))
        .collect();
    let keys: Vec<String> = keys.iter().map(|k| friendly(k)).collect();
    (!keys.is_empty()).then(|| format!("{} {label}", keys.join("/")))
}

/// Turns rows into hints; rows without a key are left out.
fn row_hints(app: &App, rows: Rows) -> Vec<String> {
    rows.into_iter()
        .filter_map(|(a, l)| hint(app, a, l))
        .collect()
}

/// The hints for the current control, picked by where the focus is.
// Long function: one if/else chain over the 10 focus states, each branch only a short list.
fn context_hints(app: &App) -> Vec<String> {
    let fonts = app.side == Side::Fonts;
    if app.layout.resize_mode {
        resize_hints(app)
    } else if app.layout.page_tabs_focused {
        let back = if app.settings.layout.tabs_on_top() {
            A::Down
        } else {
            A::Up
        };
        [
            hint(app, &[A::Left, A::Right], "pages"),
            hint(app, &[back], "back to the page"),
        ]
        .into_iter()
        .flatten()
        .collect()
    } else if app.page == Page::Main && app.layout.palette_menu_focused && app.side == Side::Colours
    {
        [
            hint(app, &[A::Confirm], "palettes"),
            hint(app, &[A::Down], "colours"),
        ]
        .into_iter()
        .flatten()
        .collect()
    } else if app.page == Page::Downloads {
        downloads_hints(app)
    } else if app.page == Page::Settings {
        settings_hints(app)
    } else if fonts && app.fonts.focus == FontFocus::Search {
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
    } else if fonts {
        fonts_hints(app)
    } else {
        colours_hints(app)
    }
}

/// Hints for resize mode: the keys come from the resize context only.
fn resize_hints(app: &App) -> Vec<String> {
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
}

/// Hints on the font download page: search, tournament, or the list.
fn downloads_hints(app: &App) -> Vec<String> {
    if app.dl.searching {
        return vec!["type to search".into(), "Enter/Esc done".into()];
    }
    if app.dl.tab == DlTab::Tournament && !app.dl.tabs_focused {
        return row_hints(app, tournament_rows(app));
    }
    let mut v: Vec<String> = [
        hint(app, &[A::Down, A::Up], "move"),
        hint(app, &[A::Confirm], "install"),
        hint(app, &[A::Delete], "remove"),
        hint(app, &[A::Search], "search"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if app.dl.tab == DlTab::Google {
        v.extend(hint(app, &[A::Left, A::Right], "category"));
    }
    v.extend(hint(app, &[A::TabPrev, A::TabNext], "tabs"));
    v
}

/// The tournament rows: one list per stage, or the bracket view.
// Long function: one match over the 4 tournament stages plus the bracket view.
fn tournament_rows(app: &App) -> Rows<'static> {
    use crate::fonts::tournament_page::Stage;
    let stage = app.tour_page.as_ref().map_or(Stage::Setup, |t| t.stage);
    let bracket = app.tour_page.as_ref().is_some_and(|t| t.bracket);
    match stage {
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
    }
}

/// Hints on the settings page: the tab bar, or the current tab.
fn settings_hints(app: &App) -> Vec<String> {
    if app.settings_page.tabs_focused {
        let back = if app.settings.layout.inner_tabs_on_top() {
            A::Down
        } else {
            A::Up
        };
        return [
            hint(app, &[A::Left, A::Right], "tabs"),
            hint(app, &[back], "back to the list"),
        ]
        .into_iter()
        .flatten()
        .collect();
    }
    let mut v = row_hints(app, settings_rows(app));
    v.extend(hint(app, &[A::TabPrev, A::TabNext], "tabs"));
    v
}

/// The settings rows for the current tab.
// Long function: one match over the 5 settings tabs, each arm a row table.
fn settings_rows(app: &App) -> Rows<'static> {
    use crate::settings_page::Tab;
    match app.settings_page.tab {
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
    }
}

/// Hints for the fonts column, by the focused control.
// Long function: builds the chip list, then one match over 3 font focus states.
fn fonts_hints(app: &App) -> Vec<String> {
    // picking for a role: only the chips that aren't locked
    let chips: Vec<Action> = CHIPS
        .iter()
        .filter(|(c, _)| app.fonts.filters.chip_free(*c))
        .map(|(c, _)| A::Filter(*c))
        .collect();
    let rows: Rows = match app.fonts.focus {
        FontFocus::Sample => vec![
            (&[A::ToggleSample], "hide sample"),
            (&[A::FocusNext], "next pane"),
        ],
        FontFocus::Slots => {
            let mut v: Rows = vec![
                (&[A::Down, A::Up], "role"),
                (&[A::Left, A::Right], "size"),
                (&[A::PickFont], "pick font"),
            ];
            // the role's font doesn't fit it
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
    };
    row_hints(app, rows)
}

/// Hints for the colour column, by the focused control.
fn colours_hints(app: &App) -> Vec<String> {
    let rows: Rows = match app.focus {
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
    };
    row_hints(app, rows)
}

/// The general hints, after the control's own: next pane, panels, save,
/// apply, open. The main page's keys don't work on the other pages, so the
/// other pages have none.
fn general_hints(app: &App) -> Vec<String> {
    let main = app.page == Page::Main;
    [
        hint(app, &[A::FocusNext], "next").filter(|_| main),
        panels_hint(app).filter(|_| main),
        hint(app, &[A::Save], "save").filter(|_| main),
        hint(app, &[A::Apply], "apply").filter(|_| main),
        hint(app, &[A::Open], "open").filter(|_| main),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The panel keys as one range, such as `"Space 1-7 panels"`.
fn panels_hint(app: &App) -> Option<String> {
    match (hint(app, &[A::Panel(1)], ""), hint(app, &[A::Panel(7)], "")) {
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
    }
}

/// The status line without key hints: just the message, if any.
fn draw_message_only(f: &mut Frame, area: Rect, message: Option<Span<'static>>) {
    if let Some(m) = message {
        f.render_widget(Paragraph::new(Line::from(m)), area);
    }
}

/// As many hints as fit in `room` columns: the ones for the current control
/// first, then the general ones, and `last` (the help key) always at the end.
// Long function: one loop that fills the hints by the available width, then the fallbacks for no hints or no room.
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
