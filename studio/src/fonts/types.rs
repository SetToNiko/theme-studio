//! Plain data types and constants of the fonts feature; a leaf, so keymap and the font helpers can use them without a cycle.

/// The three font roles' keys in fonts.json (version 3) and the host
/// offset files. Defined in `model::theme` (a theme stores one size per role);
/// re-exported here for the fonts feature.

/// The roles on screen.
pub const ROLE_NAMES: [&str; 3] = ["Monospace", "Proportional", "Icons"];
/// What each role's font has to be (`fits()` below checks it).
pub const ROLE_NEEDS: [&str; 3] = ["a Nerd Font Mono", "proportional", "a Nerd Font Propo"];

/// An installed font family, as fontconfig reports it.
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent yes/no facts that fontconfig reports about one family"
)]
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

/// Which kind of font a family is, decided by its name and flags (see `kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FontKind {
    /// A Nerd Font family whose name ends in "Nerd Font Mono".
    NerdMono,
    /// A Nerd Font family whose name ends in "Nerd Font Propo".
    NerdPropo,
    /// Any other family that has Nerd Font icons.
    Nerd,
    /// A plain monospaced family without Nerd Font icons.
    Mono,
    /// A plain proportional family without Nerd Font icons.
    Propo,
}

/// Which kind of font a family is, by its name and flags.
fn kind(f: &Installed) -> FontKind {
    if f.family.ends_with("Nerd Font Mono") {
        FontKind::NerdMono
    } else if f.family.ends_with("Nerd Font Propo") {
        FontKind::NerdPropo
    } else if f.nerd {
        FontKind::Nerd
    } else if f.mono {
        FontKind::Mono
    } else {
        FontKind::Propo
    }
}

/// Whether a font meets a role's requirement.
pub fn fits(role: usize, f: &Installed) -> bool {
    match role {
        0 => kind(f) == FontKind::NerdMono,
        1 => !f.mono,
        _ => kind(f) == FontKind::NerdPropo,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn font(family: &str, mono: bool, nerd: bool) -> Installed {
        Installed {
            family: family.into(),
            mono,
            nerd,
            bitmap: false,
            special: false,
        }
    }

    #[test]
    fn kinds() {
        assert_eq!(
            kind(&font("JetBrainsMono Nerd Font Mono", true, true)),
            FontKind::NerdMono
        );
        assert_eq!(
            kind(&font("JetBrainsMono Nerd Font Propo", false, true)),
            FontKind::NerdPropo
        );
        assert_eq!(
            kind(&font("JetBrainsMono Nerd Font", true, true)),
            FontKind::Nerd
        );
        assert_eq!(kind(&font("JetBrains Mono", true, false)), FontKind::Mono);
        assert_eq!(kind(&font("Inter", false, false)), FontKind::Propo);
    }
}
