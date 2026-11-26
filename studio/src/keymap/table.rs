//! The action table: one row for each action with its id, its label and its
//! group.
//!
//! To add an action: add the variant to `Action` (in `action.rs`) and add
//! ONE row here. `Action::id`, `from_id`, `describe`, `group` and
//! `all_actions` all read this table.
//!
//! Three kinds of action carry data and have no row of their own: `Panel(n)`,
//! `UseFor(role)` and `Filter(chip)`. Each has one small rule in `action.rs`
//! (an id and label built from the data, and a fixed group). `all_actions`
//! places panels and `UseFor` right after the `Redo` row, to keep the order of
//! the keymaps file.

use super::action::{Action, ActionGroup};

/// One row of the table: everything that describes a plain action.
pub(super) struct Row {
    /// The action that this row describes.
    pub action: Action,
    /// The name in keymaps.json.
    pub id: &'static str,
    /// Short description, for the help screen, hints and the leader popup.
    pub label: &'static str,
    /// Which part of the app handles the action.
    pub group: ActionGroup,
}

const fn row(action: Action, id: &'static str, label: &'static str, group: ActionGroup) -> Row {
    Row {
        action,
        id,
        label,
        group,
    }
}

/// All actions without data, in the order of the keymaps file and Settings page.
// Long function (reason verified, phase 6): one table row for each of the plain actions; nothing else is here.
pub(super) const ROWS: &[Row] = {
    use Action as A;
    use ActionGroup as G;
    &[
        row(A::Quit, "quit", "quit", G::Page),
        row(A::Help, "help", "all keys", G::Page),
        row(A::PageMain, "page-main", "main page", G::Page),
        row(
            A::PageDownloads,
            "page-downloads",
            "download fonts page",
            G::Page,
        ),
        row(A::PageSettings, "page-settings", "settings page", G::Page),
        row(
            A::TogglePicker,
            "toggle-picker",
            "show / hide the colour picker",
            G::Window,
        ),
        row(
            A::ToggleSample,
            "toggle-sample",
            "show / hide the font sample",
            G::Window,
        ),
        row(
            A::ToggleHints,
            "toggle-hints",
            "show / hide the key hints",
            G::Page,
        ),
        row(
            A::TogglePreview,
            "toggle-preview",
            "show / hide this page's preview",
            G::Page,
        ),
        row(A::PaneLeft, "pane-left", "pane to the left", G::Window),
        row(A::PaneRight, "pane-right", "pane to the right", G::Window),
        row(A::PaneUp, "pane-up", "pane above", G::Window),
        row(A::PaneDown, "pane-down", "pane below", G::Window),
        row(
            A::ResizeMode,
            "resize-mode",
            "resize panes (h/j/k/l move the edges)",
            G::Window,
        ),
        row(
            A::ResetSizes,
            "reset-sizes",
            "pane sizes back to the defaults",
            G::Column,
        ),
        row(
            A::UndoConfig,
            "undo-config-change",
            "undo the last config change (again: redo)",
            G::Column,
        ),
        row(
            A::PreviewFetch,
            "preview-fetch",
            "fetch the font for its preview",
            G::Column,
        ),
        row(
            A::PreviewText,
            "preview-text",
            "preview: the next text (code, icons, prose, README)",
            G::Column,
        ),
        row(
            A::PreviewLang,
            "preview-language",
            "preview: the next language",
            G::Column,
        ),
        row(
            A::PreviewNext,
            "preview-next-page",
            "preview: next page",
            G::Column,
        ),
        row(
            A::PreviewPrev,
            "preview-previous-page",
            "preview: previous page",
            G::Column,
        ),
        row(
            A::FillFamily,
            "fill-roles-from-family",
            "every role from this font family",
            G::Column,
        ),
        row(
            A::FixRoleFont,
            "fix-role-font",
            "a fitting face from the same family",
            G::Column,
        ),
        row(
            A::NextHeading,
            "next-heading",
            "README: the next heading",
            G::Column,
        ),
        row(
            A::PrevHeading,
            "previous-heading",
            "README: the previous heading",
            G::Column,
        ),
        row(
            A::TourNames,
            "tournament-names",
            "tournament: show / hide the fonts' names",
            G::Column,
        ),
        row(
            A::TourBracket,
            "tournament-bracket",
            "tournament: show the bracket",
            G::Column,
        ),
        row(
            A::TourUndo,
            "tournament-undo",
            "tournament: take the last pick back",
            G::Column,
        ),
        row(
            A::OpenPalette,
            "open-palette",
            "another palette for this theme",
            G::Theme,
        ),
        row(
            A::OpenFonts,
            "open-font-preset",
            "another font preset for this theme",
            G::Theme,
        ),
        row(
            A::NewPalette,
            "new-palette",
            "a new palette for this theme",
            G::Theme,
        ),
        row(
            A::NewFonts,
            "new-font-preset",
            "a new font preset for this theme",
            G::Theme,
        ),
        row(
            A::Discard,
            "discard-changes",
            "drop the unsaved settings",
            G::Column,
        ),
        row(A::TabNext, "tab-next", "next tab", G::Column),
        row(A::TabPrev, "tab-previous", "previous tab", G::Column),
        row(A::Add, "add", "add a key to the action", G::Column),
        row(A::Delete, "delete", "unbind / delete", G::Column),
        row(A::Restore, "restore", "restore this version", G::Column),
        row(A::SaveNamed, "save-named", "save under a name", G::Column),
        row(
            A::ResetPreset,
            "reset-preset",
            "reset the preset to the shipped keys",
            G::Column,
        ),
        row(
            A::ToggleAll,
            "toggle-all",
            "only differences / everything",
            G::Column,
        ),
        row(
            A::EditText,
            "edit-as-text",
            "type the keys (Vim notation)",
            G::Column,
        ),
        row(
            A::PreviewCycle,
            "preview-cycle",
            "type into the preview / next window / back",
            G::Window,
        ),
        row(A::SavePalette, "save-palette", "save palette", G::Theme),
        row(A::SaveFonts, "save-fonts", "save font preset", G::Theme),
        row(
            A::ApplyPalette,
            "apply-palette",
            "save and apply palette",
            G::Theme,
        ),
        row(
            A::ApplyFonts,
            "apply-fonts",
            "save and apply font preset",
            G::Theme,
        ),
        row(A::Save, "save", "save the theme", G::Theme),
        row(A::SaveAs, "save-as", "save the theme as", G::Theme),
        row(A::Apply, "apply", "save and apply the theme", G::Theme),
        row(A::Open, "open", "open a theme", G::Theme),
        row(A::New, "new", "new theme", G::Theme),
        row(A::Undo, "undo", "undo", G::Column),
        row(A::Redo, "redo", "redo", G::Column),
        row(A::FocusNext, "focus-next", "next control", G::Column),
        row(
            A::FocusPrev,
            "focus-previous",
            "previous control",
            G::Column,
        ),
        row(
            A::Back,
            "back",
            "back (in the picker: discard the change)",
            G::Column,
        ),
        row(A::Confirm, "confirm", "edit / use / open / keep", G::Column),
        row(A::Up, "up", "up", G::Column),
        row(A::Down, "down", "down", G::Column),
        row(A::Left, "left", "left / smaller / close", G::Column),
        row(A::Right, "right", "right / bigger / open", G::Column),
        row(A::UpBig, "up-big", "up (big step)", G::Column),
        row(A::DownBig, "down-big", "down (big step)", G::Column),
        row(A::LeftBig, "left-big", "left (big step)", G::Column),
        row(A::RightBig, "right-big", "right (big step)", G::Column),
        row(A::Top, "top", "first", G::Column),
        row(A::Bottom, "bottom", "last", G::Column),
        row(A::PageUp, "page-up", "page up", G::Column),
        row(A::PageDown, "page-down", "page down", G::Column),
        row(A::Pin, "pin", "pin / follow again", G::Column),
        row(A::CopyHex, "copy-hex", "copy hex", G::Column),
        row(
            A::CopyFrom,
            "copy-from",
            "copy another key's colour",
            G::Column,
        ),
        row(A::HexEntry, "hex-entry", "type a hex colour", G::Column),
        row(
            A::ToggleYazi,
            "toggle-yazi",
            "preview: shell / yazi",
            G::Column,
        ),
        row(
            A::ToggleRofi,
            "toggle-rofi",
            "preview: rofi menu",
            G::Column,
        ),
        row(
            A::ToggleView,
            "toggle-view",
            "preview: desktop / swatches",
            G::Column,
        ),
        row(A::SizeUp, "size-up", "font bigger", G::Column),
        row(A::SizeDown, "size-down", "font smaller", G::Column),
        row(
            A::PickFont,
            "pick-font",
            "pick a font from the list",
            G::Column,
        ),
        row(A::Search, "search", "search", G::Column),
    ]
};
