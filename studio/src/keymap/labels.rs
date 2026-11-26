//! The short descriptions of actions, for the help screen, the key hints and
//! the leader popup.

use super::action::Action;

impl Action {
    /// Short description, for the help screen, hints and the leader popup.
    pub fn describe(self) -> String {
        match self {
            Action::Quit => "quit".into(),
            Action::Help => "all keys".into(),
            Action::PageMain => "main page".into(),
            Action::PageDownloads => "download fonts page".into(),
            Action::PageSettings => "settings page".into(),
            Action::TogglePicker => "show / hide the colour picker".into(),
            Action::ToggleSample => "show / hide the font sample".into(),
            Action::ToggleHints => "show / hide the key hints".into(),
            Action::TogglePreview => "show / hide this page's preview".into(),
            Action::PaneLeft => "pane to the left".into(),
            Action::PaneRight => "pane to the right".into(),
            Action::PaneUp => "pane above".into(),
            Action::PaneDown => "pane below".into(),
            Action::ResizeMode => "resize panes (h/j/k/l move the edges)".into(),
            Action::ResetSizes => "pane sizes back to the defaults".into(),
            Action::UndoConfig => "undo the last config change (again: redo)".into(),
            Action::PreviewFetch => "fetch the font for its preview".into(),
            Action::PreviewText => "preview: the next text (code, icons, prose, README)".into(),
            Action::PreviewLang => "preview: the next language".into(),
            Action::PreviewNext => "preview: next page".into(),
            Action::PreviewPrev => "preview: previous page".into(),
            Action::FillFamily => "every role from this font family".into(),
            Action::FixRoleFont => "a fitting face from the same family".into(),
            Action::NextHeading => "README: the next heading".into(),
            Action::PrevHeading => "README: the previous heading".into(),
            Action::TourNames => "tournament: show / hide the fonts' names".into(),
            Action::TourBracket => "tournament: show the bracket".into(),
            Action::TourUndo => "tournament: take the last pick back".into(),
            Action::UseFor(r) => {
                format!(
                    "use the font for {}",
                    crate::fonts::types::ROLE_NAMES[r as usize % 3]
                )
            }
            Action::OpenPalette => "another palette for this theme".into(),
            Action::OpenFonts => "another font preset for this theme".into(),
            Action::NewPalette => "a new palette for this theme".into(),
            Action::NewFonts => "a new font preset for this theme".into(),
            Action::Discard => "drop the unsaved settings".into(),
            Action::TabNext => "next tab".into(),
            Action::TabPrev => "previous tab".into(),
            Action::Add => "add a key to the action".into(),
            Action::Delete => "unbind / delete".into(),
            Action::Restore => "restore this version".into(),
            Action::SaveNamed => "save under a name".into(),
            Action::ResetPreset => "reset the preset to the shipped keys".into(),
            Action::ToggleAll => "only differences / everything".into(),
            Action::EditText => "type the keys (Vim notation)".into(),
            Action::PreviewCycle => "type into the preview / next window / back".into(),
            Action::SavePalette => "save palette".into(),
            Action::SaveFonts => "save font preset".into(),
            Action::ApplyPalette => "save and apply palette".into(),
            Action::ApplyFonts => "save and apply font preset".into(),
            Action::Save => "save the theme".into(),
            Action::SaveAs => "save the theme as".into(),
            Action::Apply => "save and apply the theme".into(),
            Action::Open => "open a theme".into(),
            Action::New => "new theme".into(),
            Action::Undo => "undo".into(),
            Action::Redo => "redo".into(),
            Action::Panel(n) => format!("panel {n}"),
            Action::FocusNext => "next control".into(),
            Action::FocusPrev => "previous control".into(),
            Action::Back => "back (in the picker: discard the change)".into(),
            Action::Confirm => "edit / use / open / keep".into(),
            Action::Up => "up".into(),
            Action::Down => "down".into(),
            Action::Left => "left / smaller / close".into(),
            Action::Right => "right / bigger / open".into(),
            Action::UpBig => "up (big step)".into(),
            Action::DownBig => "down (big step)".into(),
            Action::LeftBig => "left (big step)".into(),
            Action::RightBig => "right (big step)".into(),
            Action::Top => "first".into(),
            Action::Bottom => "last".into(),
            Action::PageUp => "page up".into(),
            Action::PageDown => "page down".into(),
            Action::Pin => "pin / follow again".into(),
            Action::CopyHex => "copy hex".into(),
            Action::CopyFrom => "copy another key's colour".into(),
            Action::HexEntry => "type a hex colour".into(),
            Action::ToggleYazi => "preview: shell / yazi".into(),
            Action::ToggleRofi => "preview: rofi menu".into(),
            Action::ToggleView => "preview: desktop / swatches".into(),
            Action::SizeUp => "font bigger".into(),
            Action::SizeDown => "font smaller".into(),
            Action::PickFont => "pick a font from the list".into(),
            Action::Search => "search".into(),
            Action::Filter(c) => {
                let label = crate::fonts::types::CHIPS
                    .iter()
                    .find(|(k, _)| *k == c)
                    .map(|(_, l)| *l)
                    .unwrap_or("?");
                format!("filter: {label}")
            }
        }
    }
}
