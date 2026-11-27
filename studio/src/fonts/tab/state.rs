//! State of the Fonts tab: the struct, its construction, the installed
//! fonts, and the checks of the role fonts.
//!
//! The other files of this folder add methods to `FontsTab`: `list.rs` (the
//! font list), `presets.rs` (open and save), `keys.rs` (actions and clicks).

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::fonts::filters::Filters;
use crate::fonts::installed::installed_fonts;
use crate::fonts::presets::{Font, FontStore, Preset, load_active};
use crate::fonts::render::{SampleRenderer, screen_dpi};
use crate::fonts::tree::Node;
use crate::fonts::types::{FontFocus, Installed, ROLE_NEEDS};

/// State of the Fonts tab.
pub struct FontsTab {
    pub store: FontStore,
    pub name: String,
    pub is_new: bool,
    pub preset: Preset,
    pub saved: Preset,
    pub active: Option<String>,
    pub offsets: [i32; 3],
    pub slot: usize,
    pub focus: FontFocus,
    pub installed: Vec<Installed>,
    pub tree: Vec<Node>,
    /// Open groups, by path (`JetBrains Mono`, `JetBrains Mono/Nerd Font`).
    pub expanded: HashSet<String>,
    pub search: String,
    pub filters: Filters,
    /// Opening the list unfolds the groups of the role's current font (a setting).
    pub unfold_current: bool,
    pub list_sel: usize,
    pub list_scroll: usize,
    pub dpi: u32,
    pub sample: SampleRenderer,
    pub load_error: Option<String>,
    /// Picking a font for this role (the list opened from it, with its
    /// filter on).
    pub picking: Option<usize>,
    /// The user's filters while a role's filter is on.
    pub saved_filters: Option<Filters>,
    /// A font was just picked (the app saves and applies it).
    pub picked: bool,
    /// When a role's size last changed, until the app applies it.
    pub sized_at: Option<std::time::Instant>,
}

impl FontsTab {
    // Long function (reason verified, phase 6): one struct literal that sets every field of the tab.
    pub fn new(data: &Path, share: &Path, draft_dir: &Path, dirty: Arc<AtomicBool>) -> FontsTab {
        let store = FontStore {
            dir: data.to_path_buf(),
            share: share.to_path_buf(),
        };
        let active = FontStore::active_name();
        let (name, preset, is_new, load_error) = load_active(&store, active.as_deref());
        FontsTab {
            store,
            name,
            is_new,
            saved: preset.clone(),
            preset,
            active,
            offsets: FontStore::offsets(),
            slot: 0,
            focus: FontFocus::Slots,
            installed: Vec::new(),
            tree: Vec::new(),
            expanded: HashSet::new(),
            search: String::new(),
            filters: Filters::default(),
            unfold_current: true,
            list_sel: 0,
            list_scroll: 0,
            dpi: 96,
            sample: SampleRenderer::new(draft_dir, dirty),
            load_error,
            picking: None,
            saved_filters: None,
            picked: false,
            sized_at: None,
        }
    }

    /// Reads the installed fonts and screen DPI (the first time the tab opens).
    pub fn refresh_installed(&mut self) {
        let fonts = installed_fonts();
        self.set_installed(fonts);
        self.dpi = screen_dpi();
    }

    pub fn set_installed(&mut self, fonts: Vec<Installed>) {
        self.tree = crate::fonts::tree::build(&fonts);
        self.installed = fonts;
    }

    pub fn dirty(&self) -> bool {
        // sizes are the theme's: they don't make the preset unsaved
        self.is_new || !self.preset.same_fonts(&self.saved)
    }

    /// Size actually used on this machine (base + host offset).
    pub fn effective_size(&self, role: usize, base: u32) -> u32 {
        u32::try_from((i64::from(base) + i64::from(self.offsets[role])).max(1)).unwrap_or(u32::MAX)
    }

    /// The fonts the sample shows: while browsing the list, the highlighted
    /// font stands in for the selected role.
    pub fn shown_fonts(&self) -> [Font; 3] {
        let mut fonts = self.preset.fonts.clone();
        if matches!(self.focus, FontFocus::List | FontFocus::Search)
            && let Some(i) = self.highlighted()
        {
            fonts[self.slot]
                .family
                .clone_from(&self.installed[i].family);
        }
        fonts
    }

    /// The role.s font doesn.t meet its requirement (or isn.t installed):
    /// `r` can swap in a fitting face from its family.
    pub fn misfit(&self, role: usize) -> bool {
        !self.warnings(role).is_empty()
    }

    /// Problems with a role's font, for the slot and the status line.
    pub fn warnings(&self, role: usize) -> Vec<String> {
        let family = &self.preset.fonts[role].family;
        let Some(f) = self
            .installed
            .iter()
            .find(|f| f.family.eq_ignore_ascii_case(family))
        else {
            if self.installed.is_empty() {
                return Vec::new();
            }
            return vec![format!("\"{family}\" is not installed")];
        };
        if crate::fonts::types::fits(role, f) {
            Vec::new()
        } else {
            vec![format!("not {}", ROLE_NEEDS[role])]
        }
    }
}

#[cfg(test)]
// The helpers `font` and `tab_with` are shared by the tests of `list.rs` and `keys.rs`.
pub(super) mod tests {
    use super::*;

    pub(in crate::fonts::tab) fn font(family: &str, mono: bool, nerd: bool) -> Installed {
        Installed {
            family: family.into(),
            mono,
            nerd,
            bitmap: family == "Fixed",
            special: family.contains("CJK"),
        }
    }

    pub(in crate::fonts::tab) fn tab_with(fonts: Vec<Installed>) -> FontsTab {
        let dir = std::env::temp_dir();
        let mut t = FontsTab::new(
            &dir.join("nonexistent"),
            &dir.join("nonexistent"),
            &dir,
            Arc::new(AtomicBool::new(false)),
        );
        t.set_installed(fonts);
        t.preset = Preset::default_preset();
        t
    }

    #[test]
    fn roles_need_their_kind_of_font_and_r_swaps_in_a_fitting_face() {
        use crate::fonts::types::fits;
        let nf = font("GeistMono Nerd Font", true, true);
        let nf_mono = font("GeistMono Nerd Font Mono", true, true);
        let nf_propo = font("GeistMono Nerd Font Propo", false, true);
        let plain = font("Inter", false, false);
        assert_eq!(
            [fits(0, &nf), fits(0, &nf_mono), fits(0, &plain)],
            [false, true, false]
        );
        assert_eq!(
            [fits(1, &nf), fits(1, &nf_propo), fits(1, &plain)],
            [false, true, true]
        );
        assert_eq!(
            [fits(2, &nf), fits(2, &nf_propo), fits(2, &plain)],
            [false, true, false]
        );
        let mut t = tab_with(vec![nf, nf_mono, nf_propo, plain]);
        t.preset.fonts = [0, 1, 2].map(|_| Font {
            family: "GeistMono Nerd Font".into(),
            size: 10,
        });
        assert!((0..3).all(|r| t.misfit(r)));
        // r on each role: the same family.s face that fits it
        for (role, want) in [
            (0, "GeistMono Nerd Font Mono"),
            (1, "GeistMono Nerd Font Propo"),
            (2, "GeistMono Nerd Font Propo"),
        ] {
            t.slot = role;
            let fill = t.fix_role().unwrap();
            assert!(fill.changed && fill.missing.is_empty(), "{fill:?}");
            assert_eq!(t.preset.fonts[role].family, want);
            assert!(!t.misfit(role));
            assert!(t.fix_role().is_err(), "fits already");
        }
        // no fitting face: kept, and said why
        t.preset.fonts[0].family = "Inter".into();
        t.slot = 0;
        let fill = t.fix_role().unwrap();
        assert_eq!((fill.changed, fill.missing.clone()), (false, vec![0]));
        assert_eq!(
            fill.msg,
            "Inter has no Nerd Font Mono face (Monospace kept)"
        );
    }

    #[test]
    fn warnings_for_role_fonts() {
        let mut t = tab_with(vec![
            font("Beta Sans", false, false),
            font("Gamma Mono NF", true, true),
        ]);
        t.preset.fonts[0].family = "Beta Sans".into();
        assert_eq!(t.warnings(0), ["not a Nerd Font Mono"]);
        t.preset.fonts[1].family = "Beta Sans".into();
        assert!(t.warnings(1).is_empty(), "proportional");
        t.preset.fonts[2].family = "Gamma Mono NF".into();
        assert_eq!(t.warnings(2), ["not a Nerd Font Propo"]);
        t.preset.fonts[2].family = "Nope".into();
        assert!(t.warnings(2)[0].contains("not installed"));
    }
}
