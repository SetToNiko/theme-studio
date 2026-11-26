//! Every action a key can run: its identity, its name in keymaps.json, and
//! its group.
//!
//! The id, label and group of each action are in the table in `table.rs`.
//! The default keys are in `defaults.rs`.

use super::table::{ROWS, Row};

/// Everything a key can do. Movement actions mean the obvious thing in each
/// pane (Right in the colour list starts editing, in the font list it opens
/// a group, in the font roles it makes the font bigger).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    Quit,
    Help,
    PageMain,
    PageDownloads,
    PageSettings,
    TogglePicker,
    ToggleSample,
    ToggleHints,
    TogglePreview,
    PaneLeft,
    PaneRight,
    PaneUp,
    PaneDown,
    ResizeMode,
    ResetSizes,
    UndoConfig,
    PreviewFetch,
    PreviewText,
    PreviewLang,
    PreviewNext,
    PreviewPrev,
    FillFamily,
    /// A role.s font that doesn.t fit: the same family.s face that does.
    FixRoleFont,
    /// README tab: the next / previous heading.
    NextHeading,
    PrevHeading,
    /// Tournament: show the fonts' names, the bracket, take a pick back.
    TourNames,
    TourBracket,
    TourUndo,
    /// Tournament results: use the font for Monospace (0), Proportional (1)
    /// or Icons (2).
    UseFor(u8),
    OpenPalette,
    OpenFonts,
    NewPalette,
    NewFonts,
    Discard,
    TabNext,
    TabPrev,
    Add,
    Delete,
    Restore,
    SaveNamed,
    ResetPreset,
    ToggleAll,
    EditText,
    PreviewCycle,
    SavePalette,
    SaveFonts,
    ApplyPalette,
    ApplyFonts,
    Save,
    SaveAs,
    Apply,
    Open,
    New,
    Undo,
    Redo,
    Panel(u8),
    FocusNext,
    FocusPrev,
    Back,
    Confirm,
    Up,
    Down,
    Left,
    Right,
    UpBig,
    DownBig,
    LeftBig,
    RightBig,
    Top,
    Bottom,
    PageUp,
    PageDown,
    Pin,
    CopyHex,
    CopyFrom,
    HexEntry,
    ToggleYazi,
    ToggleRofi,
    ToggleView,
    SizeUp,
    SizeDown,
    PickFont,
    Search,
    Filter(char),
}

/// Every action, in the order the keymaps file and the Settings page list them.
pub fn all_actions() -> Vec<Action> {
    let mut v = Vec::new();
    for row in ROWS {
        v.push(row.action);
        // The actions with data have no row: they follow `Redo`.
        if row.action == Action::Redo {
            v.extend((1..=9).map(Action::Panel));
            v.extend((0..3).map(Action::UseFor));
        }
    }
    v.extend(
        crate::fonts::types::CHIPS
            .iter()
            .map(|(c, _)| Action::Filter(*c)),
    );
    v
}

/// Names for the filter chips in the keymaps file.
const FILTER_IDS: [(char, &str); 8] = [
    ('m', "filter-mono"),
    ('p', "filter-propo"),
    ('n', "filter-nerd-font"),
    ('M', "filter-nerd-font-mono"),
    ('P', "filter-nerd-font-propo"),
    ('x', "filter-no-nerd-font"),
    ('b', "show-bitmap"),
    ('c', "show-cjk-emoji-math"),
];

impl Action {
    /// True if the action has its own row in `ROWS`; false for the three
    /// actions with data (`Panel`, `UseFor`, `Filter`), which have a rule instead.
    ///
    /// This `match` has no `_` arm on purpose: a new `Action` variant does not
    /// compile until you list it here. Then add its row in `table.rs`
    /// (the unit test and the `debug_assert` in `row` check that the row exists).
    fn has_table_row(self) -> bool {
        match self {
            Action::Panel(_) | Action::UseFor(_) | Action::Filter(_) => false,
            Action::Quit
            | Action::Help
            | Action::PageMain
            | Action::PageDownloads
            | Action::PageSettings
            | Action::TogglePicker
            | Action::ToggleSample
            | Action::ToggleHints
            | Action::TogglePreview
            | Action::PaneLeft
            | Action::PaneRight
            | Action::PaneUp
            | Action::PaneDown
            | Action::ResizeMode
            | Action::ResetSizes
            | Action::UndoConfig
            | Action::PreviewFetch
            | Action::PreviewText
            | Action::PreviewLang
            | Action::PreviewNext
            | Action::PreviewPrev
            | Action::FillFamily
            | Action::FixRoleFont
            | Action::NextHeading
            | Action::PrevHeading
            | Action::TourNames
            | Action::TourBracket
            | Action::TourUndo
            | Action::OpenPalette
            | Action::OpenFonts
            | Action::NewPalette
            | Action::NewFonts
            | Action::Discard
            | Action::TabNext
            | Action::TabPrev
            | Action::Add
            | Action::Delete
            | Action::Restore
            | Action::SaveNamed
            | Action::ResetPreset
            | Action::ToggleAll
            | Action::EditText
            | Action::PreviewCycle
            | Action::SavePalette
            | Action::SaveFonts
            | Action::ApplyPalette
            | Action::ApplyFonts
            | Action::Save
            | Action::SaveAs
            | Action::Apply
            | Action::Open
            | Action::New
            | Action::Undo
            | Action::Redo
            | Action::FocusNext
            | Action::FocusPrev
            | Action::Back
            | Action::Confirm
            | Action::Up
            | Action::Down
            | Action::Left
            | Action::Right
            | Action::UpBig
            | Action::DownBig
            | Action::LeftBig
            | Action::RightBig
            | Action::Top
            | Action::Bottom
            | Action::PageUp
            | Action::PageDown
            | Action::Pin
            | Action::CopyHex
            | Action::CopyFrom
            | Action::HexEntry
            | Action::ToggleYazi
            | Action::ToggleRofi
            | Action::ToggleView
            | Action::SizeUp
            | Action::SizeDown
            | Action::PickFont
            | Action::Search => true,
        }
    }

    /// The action's row in the table; `None` for `Panel`, `UseFor` and `Filter`.
    ///
    /// A plain action without a row is a bug: debug builds and tests stop with
    /// a message that names the action.
    fn row(self) -> Option<&'static Row> {
        let row = ROWS.iter().find(|r| r.action == self);
        debug_assert!(
            row.is_some() == self.has_table_row(),
            "{self:?}: the row in table.rs and has_table_row disagree"
        );
        row
    }

    /// The action's name in keymaps.json.
    pub fn id(self) -> String {
        match self {
            Action::UseFor(r) => format!("use-for-{}", crate::model::theme::ROLES[r as usize % 3]),
            Action::Panel(n) => format!("panel-{n}"),
            Action::Filter(c) => FILTER_IDS
                .iter()
                .find(|(k, _)| *k == c)
                .map_or("filter", |(_, id)| *id)
                .to_string(),
            _ => self.row().map_or("", |r| r.id).to_string(),
        }
    }

    /// Short description, for the help screen, hints and the leader popup.
    pub fn describe(self) -> String {
        match self {
            Action::UseFor(r) => format!(
                "use the font for {}",
                crate::fonts::types::ROLE_NAMES[r as usize % 3]
            ),
            Action::Panel(n) => format!("panel {n}"),
            Action::Filter(c) => {
                let label = crate::fonts::types::CHIPS
                    .iter()
                    .find(|(k, _)| *k == c)
                    .map_or("?", |(_, l)| *l);
                format!("filter: {label}")
            }
            _ => self.row().map_or("", |r| r.label).to_string(),
        }
    }

    /// Looks up a name that an earlier version used.
    ///
    /// - `None`: `id` is not an old name.
    /// - `Some(LegacyId::Dropped)`: the old action was dropped (no warning is needed).
    /// - `Some(LegacyId::Renamed(a))`: the old action was renamed to `a`.
    pub fn legacy(id: &str) -> Option<LegacyId> {
        match id {
            "tab-colours" => Some(LegacyId::Renamed(Action::PageMain)),
            "tab-fonts" => Some(LegacyId::Dropped),
            _ => None,
        }
    }

    /// The action with this id (the name in keymap files); `None` if unknown.
    pub fn from_id(id: &str) -> Option<Action> {
        all_actions().into_iter().find(|a| a.id() == id)
    }

    /// The group this action belongs to; `run_action` uses it to route the action.
    pub fn group(self) -> ActionGroup {
        match self {
            Action::Panel(_) => ActionGroup::Window,
            Action::UseFor(_) | Action::Filter(_) => ActionGroup::Column,
            _ => self.row().map_or(ActionGroup::Column, |r| r.group),
        }
    }
}

/// What became of an action name that an earlier version used (see [`Action::legacy`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LegacyId {
    /// The old action now has another name: use this action instead.
    Renamed(Action),
    /// The old action no longer exists: ignore it without a warning.
    Dropped,
}

/// Which part of the app handles an action (see [`Action::group`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionGroup {
    /// Quit, help, page switch, hints and preview toggles: work on every page.
    Page,
    /// Windows and panes of the Main page: picker, sample, resize, pane focus.
    Window,
    /// The theme files of the Main page: save, apply, open, new.
    Theme,
    /// Everything else: the focused column (fonts or colours), or another page.
    Column,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_row_variant_has_exactly_one_row() {
        // `has_table_row` is an exhaustive `match`, so no variant is missed.
        for row in ROWS {
            let a = row.action;
            assert!(a.has_table_row(), "{a:?} has a row but is a data action");
            let rows = ROWS.iter().filter(|r| r.action == a).count();
            assert_eq!(rows, 1, "{a:?}");
        }
        for a in all_actions() {
            // `row` also runs the `debug_assert` that finds a missing row.
            assert_eq!(a.row().is_some(), a.has_table_row(), "{a:?}");
        }
    }

    #[test]
    fn table_rows_are_listed_and_filled_in() {
        let all = all_actions();
        for r in ROWS {
            assert!(all.contains(&r.action), "{:?} not in all_actions", r.action);
            assert!(!r.id.is_empty() && !r.label.is_empty(), "{:?}", r.action);
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<String> = all_actions().iter().map(|a| a.id()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn from_id_inverts_id_for_every_action() {
        for a in all_actions() {
            assert_eq!(Action::from_id(&a.id()), Some(a));
        }
    }
}
