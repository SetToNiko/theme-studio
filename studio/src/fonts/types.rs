//! Plain data types and constants of the fonts feature; a leaf, so keymap and the font helpers can use them without a cycle.

/// The three font roles' keys in fonts.json (version 3) and the host
/// offset files. Each is named after what its font has to be.
pub const ROLES: [&str; 3] = ["monospace", "proportional", "icons"];

/// The roles on screen.
pub const ROLE_NAMES: [&str; 3] = ["Monospace", "Proportional", "Icons"];
/// What each role's font has to be (fonts_pick.rs checks it).
pub const ROLE_NEEDS: [&str; 3] = ["a Nerd Font Mono", "proportional", "a Nerd Font Propo"];

/// An installed font family, as fontconfig reports it.
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub family: String,
    /// Monospaced (fontconfig reports a spacing, as font-switch.sh checks).
    pub mono: bool,
    /// Has Nerd Font icons (checked with one icon, U+F115).
    pub nerd: bool,
    /// Only bitmap files (PCF/BDF): render badly in Ghostty and GTK.
    pub bitmap: bool,
    /// Made for other scripts or symbols (CJK, emoji, math, Unifont), by name.
    pub special: bool,
}

/// Chip key and label, in display order.
pub const CHIPS: [(char, &str); 8] = [
    ('m', "mono"),
    ('p', "propo"),
    ('n', "NF"),
    ('M', "NF Mono"),
    ('P', "NF Propo"),
    ('x', "no NF"),
    ('b', "bitmap"),
    ('c', "CJK·emoji·math"),
];

/// A visible row of the font list.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// A design or variant group; `count` is how many fonts it shows.
    Group {
        depth: usize,
        path: String,
        name: String,
        expanded: bool,
        count: usize,
    },
    Font {
        depth: usize,
        index: usize,
    },
}

impl Row {
    pub fn depth(&self) -> usize {
        match self {
            Row::Group { depth, .. } | Row::Font { depth, .. } => *depth,
        }
    }
}

/// Which part of the Fonts tab has the keyboard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FontFocus {
    /// ¹ the three role slots
    Slots,
    /// ² the installed fonts list
    List,
    /// ³ the sample (nothing to type; focusable so it can be resized)
    Sample,
    /// typing into the list's search box
    Search,
}
