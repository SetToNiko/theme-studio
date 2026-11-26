//! The font list's filter chips (spacing, Nerd Font, bitmap, special).

use super::types::Installed;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Spacing {
    #[default]
    Any,
    Mono,
    Propo,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NerdKind {
    #[default]
    Any,
    Nerd,
    NerdMono,
    NerdPropo,
    NotNerd,
}

/// The font list's filter chips. A font has to pass all of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    pub spacing: Spacing,
    pub nerd: NerdKind,
    /// Bitmap fonts are hidden unless this is on.
    pub show_bitmap: bool,
    /// CJK / emoji / math fonts are hidden unless this is on.
    pub show_special: bool,
    /// Picking for this role: only fonts that meet its requirement, and the
    /// spacing and Nerd Font chips are locked (fonts/pick.rs).
    pub need: Option<usize>,
}

impl Filters {
    pub fn allows(&self, f: &Installed) -> bool {
        let spacing_ok = match self.spacing {
            Spacing::Any => true,
            Spacing::Mono => f.mono,
            Spacing::Propo => !f.mono,
        };
        let nerd_ok = match self.nerd {
            NerdKind::Any => true,
            NerdKind::Nerd => f.nerd,
            NerdKind::NerdMono => f.family.ends_with("Nerd Font Mono"),
            NerdKind::NerdPropo => f.family.ends_with("Nerd Font Propo"),
            NerdKind::NotNerd => !f.nerd,
        };
        let fits_role = self.need.is_none_or(|r| crate::fonts::types::fits(r, f));
        fits_role
            && spacing_ok
            && nerd_ok
            && (self.show_bitmap || !f.bitmap)
            && (self.show_special || !f.special)
    }

    /// The chips that can be used now: while picking for a role, the
    /// spacing and Nerd Font chips are locked (the requirement decides).
    pub fn chip_free(&self, chip: char) -> bool {
        self.need.is_none() || matches!(chip, 'b' | 'c')
    }

    pub fn is_on(&self, chip: char) -> bool {
        match chip {
            'm' => self.spacing == Spacing::Mono,
            'p' => self.spacing == Spacing::Propo,
            'n' => self.nerd == NerdKind::Nerd,
            'M' => self.nerd == NerdKind::NerdMono,
            'P' => self.nerd == NerdKind::NerdPropo,
            'x' => self.nerd == NerdKind::NotNerd,
            'b' => self.show_bitmap,
            'c' => self.show_special,
            _ => false,
        }
    }

    /// Turns a chip on or off. Chips in the same row of choices (mono/propo,
    /// the Nerd Font kinds) replace each other.
    pub fn toggle(&mut self, chip: char) -> bool {
        if !self.chip_free(chip) {
            return false;
        }
        let flip_spacing =
            |cur: Spacing, want: Spacing| if cur == want { Spacing::Any } else { want };
        let flip_nerd =
            |cur: NerdKind, want: NerdKind| if cur == want { NerdKind::Any } else { want };
        match chip {
            'm' => self.spacing = flip_spacing(self.spacing, Spacing::Mono),
            'p' => self.spacing = flip_spacing(self.spacing, Spacing::Propo),
            'n' => self.nerd = flip_nerd(self.nerd, NerdKind::Nerd),
            'M' => self.nerd = flip_nerd(self.nerd, NerdKind::NerdMono),
            'P' => self.nerd = flip_nerd(self.nerd, NerdKind::NerdPropo),
            'x' => self.nerd = flip_nerd(self.nerd, NerdKind::NotNerd),
            'b' => self.show_bitmap = !self.show_bitmap,
            'c' => self.show_special = !self.show_special,
            _ => return false,
        }
        true
    }
}
