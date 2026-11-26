//! Where the user is: page, side, focus, the open overlay, and what is pending.
//!
//! Plain data types, with no imports from the app or feature modules. The only method is `Pending::is_fonts`. The `App` struct in
//! the parent module holds one value of most of them.

use std::path::PathBuf;

use ratatui::layout::Rect;

use crate::color::Rgb;
use crate::keymap::{Action, Context, Key};

/// Which control the keyboard drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    List,
    Square,
    Hue,
    SliderH,
    SliderS,
    SliderL,
    Hex,
}

/// Something that would throw away unsaved changes, waiting for confirmation.
#[derive(Clone, Debug)]
pub enum Pending {
    Quit,
    New,
    Open(String),
    NewFonts,
    OpenFonts(String),
    NewTheme,
    OpenTheme(String),
}

impl Pending {
    /// True for the two font-preset variants: they check the fonts, not the palette.
    pub(in crate::app) fn is_fonts(&self) -> bool {
        matches!(self, Pending::NewFonts | Pending::OpenFonts(_))
    }
}

/// What the Open / Save as pop-up is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum For {
    Palette,
    Fonts,
    Theme,
}

/// Which column has the keyboard (both are always on screen).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Colours,
    Fonts,
}

/// F1 / F2 / F3.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Main,
    Downloads,
    Settings,
}

/// A row of the colour list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColourRow {
    Group { index: usize, expanded: bool },
    Key(usize),
}

/// What a key press is being recorded for (the Keymaps / Behaviour tabs).
#[derive(Clone, Debug, PartialEq)]
pub enum Capture {
    /// New keys for an action (`add`: next to the ones it has).
    Binding {
        ctx: Context,
        action: Action,
        add: bool,
    },
    /// The leader key.
    Leader,
}

/// Something waiting for a name typed into the prompt.
#[derive(Clone, Debug, PartialEq)]
pub enum Naming {
    /// Save the active keymap as a new preset.
    Preset,
    /// A new preset, a copy of the named one (the preset list).
    PresetFrom(String),
    /// Save a file's current state as a named version.
    Version(&'static str),
    /// Keys typed as text (Vim notation).
    BindingText {
        ctx: Context,
        action: Action,
        add: bool,
    },
}

/// Something asked about first (y / n).
#[derive(Clone, Debug, PartialEq)]
pub enum Deferred {
    Restore {
        file: &'static str,
        version: PathBuf,
    },
    DeleteVersion(PathBuf),
    DeletePreset(String),
    ResetPreset(String),
    /// Deletes the preview fonts (Behaviour tab).
    ClearPreviews,
    /// Starts a tournament of a pool (Fonts page, Tournament tab).
    StartTournament(usize),
    /// Downloads a family.s Nerd Font (catalogue id), then fills these
    /// roles from it.
    NerdForRoles {
        id: String,
        group: String,
        roles: Vec<usize>,
    },
}

/// Which highlight look a row sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookId {
    /// Clickable things under the mouse.
    Hover,
    /// The Palette pane's derivation relatives of the selected colour.
    Derived,
}

/// A row of the Behaviour tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BhRow {
    /// A blank line before a group (not selectable).
    Gap,
    /// A group's title (not selectable).
    Header(&'static str),
    /// A look as it's set (not selectable).
    Sample(LookId),
    Leader,
    Tabs,
    InnerTabs,
    KeyHints,
    MainPreview,
    FontsPreview,
    /// How long messages stay in the status line.
    MessageSeconds,
    /// Save confirmations: status line or a box.
    SavedAs,
    SaveSizes,
    ResetSizes,
    Highlight(LookId),
    HighlightColour(LookId),
    Underline(LookId),
    UnderlineColour(LookId),
    Text(LookId),
    TextColour(LookId),
    TextWeight(LookId),
    PaneBorder,
    PreviewBorder,
    GroupOpen(usize),
    PickerAtStart,
    PickerAfterEditing,
    SampleAtStart,
    ListOpensToCurrent,
    HideBitmap,
    HideSpecial,
    Spacing,
    NerdFont,
    /// Fetch Fonts page previews ahead of time.
    Prefetch,
    /// Whether a category (config::settings::PREFETCH_CATEGORIES) is prefetched.
    PrefetchCategory(usize),
    /// Delete the preview fonts.
    ClearPreviews,
}

/// The pop-up that is open over the page, if any.
pub enum Overlay {
    None,
    Open {
        filter: String,
        sel: usize,
        entries: Vec<(String, Vec<Rgb>)>,
    },
    SaveAs {
        input: String,
        overwrite: bool,
        then_apply: bool,
    },
    Confirm(Pending),
    /// Picking the key whose colour gets copied into `target`.
    CopyFrom {
        target: &'static str,
    },
    /// The keys, in columns; `page`: the first column shown when they don't
    /// all fit.
    Help {
        page: usize,
    },
    /// Settings page: recording keys (a binding or the leader).
    Capture {
        what: Capture,
        keys: Vec<Key>,
    },
    /// Settings page: typing a name or keys.
    Prompt {
        title: String,
        input: String,
        purpose: Naming,
    },
    /// Settings page: choosing a palette key for a hover colour.
    ChooseKey {
        row: BhRow,
        sel: usize,
        /// The key before the list opened (Esc puts it back).
        original: String,
    },
    /// Settings page: a yes / no question.
    Ask {
        question: String,
        then: Deferred,
    },
    /// A dropdown list hanging under `at`: the preview's text kind (0) or
    /// language (1).
    Menu {
        menu: u8,
        sel: usize,
        at: Rect,
    },
}
