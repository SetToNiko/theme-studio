//! Every action a key can run: its identity, its name in keymaps.json, and
//! its group.
//!
//! The labels shown to the user are in `labels.rs`. The default keys are in
//! `defaults.rs`.

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
    use Action as A;
    let mut v = vec![
        A::Quit,
        A::Help,
        A::PageMain,
        A::PageDownloads,
        A::PageSettings,
        A::TogglePicker,
        A::ToggleSample,
        A::ToggleHints,
        A::TogglePreview,
        A::PaneLeft,
        A::PaneRight,
        A::PaneUp,
        A::PaneDown,
        A::ResizeMode,
        A::ResetSizes,
        A::UndoConfig,
        A::PreviewFetch,
        A::PreviewText,
        A::PreviewLang,
        A::PreviewNext,
        A::PreviewPrev,
        A::FillFamily,
        A::FixRoleFont,
        A::NextHeading,
        A::PrevHeading,
        A::TourNames,
        A::TourBracket,
        A::TourUndo,
        A::OpenPalette,
        A::OpenFonts,
        A::NewPalette,
        A::NewFonts,
        A::Discard,
        A::TabNext,
        A::TabPrev,
        A::Add,
        A::Delete,
        A::Restore,
        A::SaveNamed,
        A::ResetPreset,
        A::ToggleAll,
        A::EditText,
        A::PreviewCycle,
        A::SavePalette,
        A::SaveFonts,
        A::ApplyPalette,
        A::ApplyFonts,
        A::Save,
        A::SaveAs,
        A::Apply,
        A::Open,
        A::New,
        A::Undo,
        A::Redo,
    ];
    v.extend((1..=9).map(A::Panel));
    v.extend((0..3).map(A::UseFor));
    v.extend([
        A::FocusNext,
        A::FocusPrev,
        A::Back,
        A::Confirm,
        A::Up,
        A::Down,
        A::Left,
        A::Right,
        A::UpBig,
        A::DownBig,
        A::LeftBig,
        A::RightBig,
        A::Top,
        A::Bottom,
        A::PageUp,
        A::PageDown,
        A::Pin,
        A::CopyHex,
        A::CopyFrom,
        A::HexEntry,
        A::ToggleYazi,
        A::ToggleRofi,
        A::ToggleView,
        A::SizeUp,
        A::SizeDown,
        A::PickFont,
        A::Search,
    ]);
    v.extend(
        crate::fonts::types::CHIPS
            .iter()
            .map(|(c, _)| A::Filter(*c)),
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
    /// The action's name in keymaps.json.
    pub fn id(self) -> String {
        use Action as A;
        let s = match self {
            A::Quit => "quit",
            A::Help => "help",
            A::PageMain => "page-main",
            A::PageDownloads => "page-downloads",
            A::PageSettings => "page-settings",
            A::TogglePicker => "toggle-picker",
            A::ToggleSample => "toggle-sample",
            A::ToggleHints => "toggle-hints",
            A::TogglePreview => "toggle-preview",
            A::PaneLeft => "pane-left",
            A::PaneRight => "pane-right",
            A::PaneUp => "pane-up",
            A::PaneDown => "pane-down",
            A::ResizeMode => "resize-mode",
            A::ResetSizes => "reset-sizes",
            A::UndoConfig => "undo-config-change",
            A::PreviewFetch => "preview-fetch",
            A::PreviewText => "preview-text",
            A::PreviewLang => "preview-language",
            A::PreviewNext => "preview-next-page",
            A::PreviewPrev => "preview-previous-page",
            A::FillFamily => "fill-roles-from-family",
            A::FixRoleFont => "fix-role-font",
            A::NextHeading => "next-heading",
            A::PrevHeading => "previous-heading",
            A::TourNames => "tournament-names",
            A::TourBracket => "tournament-bracket",
            A::TourUndo => "tournament-undo",
            A::UseFor(r) => {
                return format!("use-for-{}", crate::fonts::types::ROLES[r as usize % 3]);
            }
            A::OpenPalette => "open-palette",
            A::OpenFonts => "open-font-preset",
            A::NewPalette => "new-palette",
            A::NewFonts => "new-font-preset",
            A::Discard => "discard-changes",
            A::TabNext => "tab-next",
            A::TabPrev => "tab-previous",
            A::Add => "add",
            A::Delete => "delete",
            A::Restore => "restore",
            A::SaveNamed => "save-named",
            A::ResetPreset => "reset-preset",
            A::ToggleAll => "toggle-all",
            A::EditText => "edit-as-text",
            A::PreviewCycle => "preview-cycle",
            A::SavePalette => "save-palette",
            A::SaveFonts => "save-fonts",
            A::ApplyPalette => "apply-palette",
            A::ApplyFonts => "apply-fonts",
            A::Save => "save",
            A::SaveAs => "save-as",
            A::Apply => "apply",
            A::Open => "open",
            A::New => "new",
            A::Undo => "undo",
            A::Redo => "redo",
            A::Panel(n) => return format!("panel-{n}"),
            A::FocusNext => "focus-next",
            A::FocusPrev => "focus-previous",
            A::Back => "back",
            A::Confirm => "confirm",
            A::Up => "up",
            A::Down => "down",
            A::Left => "left",
            A::Right => "right",
            A::UpBig => "up-big",
            A::DownBig => "down-big",
            A::LeftBig => "left-big",
            A::RightBig => "right-big",
            A::Top => "top",
            A::Bottom => "bottom",
            A::PageUp => "page-up",
            A::PageDown => "page-down",
            A::Pin => "pin",
            A::CopyHex => "copy-hex",
            A::CopyFrom => "copy-from",
            A::HexEntry => "hex-entry",
            A::ToggleYazi => "toggle-yazi",
            A::ToggleRofi => "toggle-rofi",
            A::ToggleView => "toggle-view",
            A::SizeUp => "size-up",
            A::SizeDown => "size-down",
            A::PickFont => "pick-font",
            A::Search => "search",
            A::Filter(c) => FILTER_IDS
                .iter()
                .find(|(k, _)| *k == c)
                .map(|(_, id)| *id)
                .unwrap_or("filter"),
        };
        s.to_string()
    }

    /// Names from earlier versions: renamed, or (None) dropped without a warning.
    pub fn legacy(id: &str) -> Option<Option<Action>> {
        match id {
            "tab-colours" => Some(Some(Action::PageMain)),
            "tab-fonts" => Some(None),
            _ => None,
        }
    }

    pub fn from_id(id: &str) -> Option<Action> {
        all_actions().into_iter().find(|a| a.id() == id)
    }

    /// The group this action belongs to; `run_action` uses it to route the action.
    pub fn group(self) -> ActionGroup {
        use Action as A;
        match self {
            A::Quit
            | A::Help
            | A::PageMain
            | A::PageDownloads
            | A::PageSettings
            | A::ToggleHints
            | A::TogglePreview => ActionGroup::Page,
            A::PreviewCycle
            | A::TogglePicker
            | A::ToggleSample
            | A::ResizeMode
            | A::PaneLeft
            | A::PaneRight
            | A::PaneUp
            | A::PaneDown
            | A::Panel(_) => ActionGroup::Window,
            A::SavePalette
            | A::SaveFonts
            | A::ApplyPalette
            | A::ApplyFonts
            | A::Save
            | A::SaveAs
            | A::Apply
            | A::Open
            | A::New
            | A::OpenPalette
            | A::OpenFonts
            | A::NewPalette
            | A::NewFonts => ActionGroup::Theme,
            _ => ActionGroup::Column,
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
