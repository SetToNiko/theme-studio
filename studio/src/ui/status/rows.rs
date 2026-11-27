//! Key hints of the pages and columns: which actions the status line shows.
//!
//! One small function per screen lists the actions. `super::hint::hint` and
//! `super::hint::row_hints` turn them into text with the keys of the active keymap.

use super::hint::{Rows, hint, row_hints};
use crate::app::App;
use crate::app::state::nav::Focus;
use crate::fonts::pages::download::DlTab;
use crate::fonts::types::{CHIPS, FontFocus};
use crate::keymap::action::Action;
use crate::keymap::action::Action as A;

/// Hints on the font download page: search, tournament, or the list.
pub(super) fn downloads_hints(app: &App) -> Vec<String> {
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
// Long function (reason verified, phase 6): a pure data table: one key-hint list per tournament stage, plus the bracket view.
pub(super) fn tournament_rows(app: &App) -> Rows<'static> {
    use crate::fonts::pages::tournament::Stage;
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
pub(super) fn settings_hints(app: &App) -> Vec<String> {
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
// Long function (reason verified, phase 6): a pure data table: one key-hint list per settings tab (the Keymaps tab has two).
pub(super) fn settings_rows(app: &App) -> Rows<'static> {
    use crate::settings_page::state::Tab;
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
pub(super) fn fonts_hints(app: &App) -> Vec<String> {
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
        FontFocus::Slots => slots_rows(app),
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

/// The hints for the role slots: the fitting-face hint shows only when the role's font does not fit.
fn slots_rows(app: &App) -> Rows<'static> {
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

/// Hints for the colour column, by the focused control.
pub(super) fn colours_hints(app: &App) -> Vec<String> {
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
