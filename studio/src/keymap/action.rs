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
    /// The action's row in the table; `None` for `Panel`, `UseFor` and `Filter`.
    fn row(self) -> Option<&'static Row> {
        ROWS.iter().find(|r| r.action == self)
    }

    /// The action's name in keymaps.json.
    pub fn id(self) -> String {
        match self {
            Action::UseFor(r) => format!("use-for-{}", crate::fonts::types::ROLES[r as usize % 3]),
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
    /// - `Some(None)`: the old action was dropped (no warning is needed).
    /// - `Some(Some(a))`: the old action was renamed to `a`.
    pub fn legacy(id: &str) -> Option<Option<Action>> {
        match id {
            "tab-colours" => Some(Some(Action::PageMain)),
            "tab-fonts" => Some(None),
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
    fn every_action_has_exactly_one_table_row_or_a_data_rule() {
        for a in all_actions() {
            let rows = ROWS.iter().filter(|r| r.action == a).count();
            let has_data = matches!(a, Action::Panel(_) | Action::UseFor(_) | Action::Filter(_));
            assert_eq!(rows, usize::from(!has_data), "{a:?}");
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
