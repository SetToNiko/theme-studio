//! Where the user is: page, side, focus, the open overlay, and what is pending.
//!
//! Plain data types. The only method is `Pending::is_fonts`. The `App` struct in
//! the parent module holds one value of most of them.

use ratatui::layout::Rect;

use crate::color::Rgb;
use crate::keymap::Key;

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
    pub(super) fn is_fonts(&self) -> bool {
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
        what: crate::settings::Capture,
        keys: Vec<Key>,
    },
    /// Settings page: typing a name or keys.
    Prompt {
        title: String,
        input: String,
        purpose: crate::settings::Naming,
    },
    /// Settings page: choosing a palette key for a hover colour.
    ChooseKey {
        row: crate::behaviour::BhRow,
        sel: usize,
        /// The key before the list opened (Esc puts it back).
        original: String,
    },
    /// Settings page: a yes / no question.
    Ask {
        question: String,
        then: crate::settings::Deferred,
    },
    /// A dropdown list hanging under `at`: the preview's text kind (0) or
    /// language (1).
    Menu {
        menu: u8,
        sel: usize,
        at: Rect,
    },
}
