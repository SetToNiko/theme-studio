//! The fonts column of the Main page: font presets (fonts.json, the data folder;
//! applied by font-switch) with their Monospace, Proportional and Icons
//! roles, the installed fonts (fontconfig), and the sample image
//! (ImageMagick + Pango, shown with the kitty graphics protocol).
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::keymap::Action;

use filters::Filters;
use installed::installed_fonts;
use presets::{Font, FontStore, Preset};
use render::{SampleRenderer, screen_dpi};
use tree::Node;
use types::{FontFocus, Installed, ROLE_NEEDS, Row};

pub mod download_page;
pub mod filters;
pub mod installed;
pub mod pick;
pub mod presets;
pub mod preview_page;
pub mod render;
pub mod tree;
pub mod types;

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
    /// Open groups, by path ("JetBrains Mono", "JetBrains Mono/Nerd Font").
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
    pub fn new(data: &Path, share: &Path, draft_dir: &Path, dirty: Arc<AtomicBool>) -> FontsTab {
        let store = FontStore {
            dir: data.to_path_buf(),
            share: share.to_path_buf(),
        };
        let active = store.active_name();
        let (name, preset, is_new, load_error) = match active.as_deref().map(|n| (n, store.load(n)))
        {
            Some((n, Ok(p))) => (n.to_string(), p, false, None),
            Some((n, Err(e))) => (
                n.to_string(),
                Preset::default_preset(),
                true,
                Some(format!("{e:#}")),
            ),
            None => ("untitled".to_string(), Preset::default_preset(), true, None),
        };
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
        (base as i32 + self.offsets[role]).max(1) as u32
    }

    /// The rows the list shows: the grouped tree (open groups expanded) when
    /// not searching, a flat list of matching fonts while searching. Groups
    /// with no font passing the filters are left out.
    pub fn rows(&self) -> Vec<Row> {
        let allowed = |i: usize| self.filters.allows(&self.installed[i]);
        if !self.search.is_empty() {
            let q = self.search.to_lowercase();
            return (0..self.installed.len())
                .filter(|&i| allowed(i) && self.installed[i].family.to_lowercase().contains(&q))
                .map(|index| Row::Font { depth: 0, index })
                .collect();
        }
        fn walk(t: &FontsTab, nodes: &[Node], depth: usize, parent: &str, out: &mut Vec<Row>) {
            for n in nodes {
                match n {
                    Node::Leaf(i) => {
                        if t.filters.allows(&t.installed[*i]) {
                            out.push(Row::Font { depth, index: *i });
                        }
                    }
                    Node::Group { name, children } => {
                        let count = n
                            .leaves()
                            .into_iter()
                            .filter(|&i| t.filters.allows(&t.installed[i]))
                            .count();
                        if count == 0 {
                            continue;
                        }
                        // one font left by the filters: the font, not a group
                        if count == 1
                            && let Some(i) = n
                                .leaves()
                                .into_iter()
                                .find(|&i| t.filters.allows(&t.installed[i]))
                        {
                            out.push(Row::Font { depth, index: i });
                            continue;
                        }
                        let path = if parent.is_empty() {
                            name.clone()
                        } else {
                            format!("{parent}/{name}")
                        };
                        let expanded = t.expanded.contains(&path);
                        out.push(Row::Group {
                            depth,
                            path: path.clone(),
                            name: name.clone(),
                            expanded,
                            count,
                        });
                        if expanded {
                            walk(t, children, depth + 1, &path, out);
                        }
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(self, &self.tree, 0, "", &mut out);
        out
    }

    /// The font on the highlighted row, if it is a font.
    pub fn highlighted(&self) -> Option<usize> {
        match self.rows().get(self.list_sel) {
            Some(Row::Font { index, .. }) => Some(*index),
            _ => None,
        }
    }

    /// The fonts the sample shows: while browsing the list, the highlighted
    /// font stands in for the selected role.
    pub fn shown_fonts(&self) -> [Font; 3] {
        let mut fonts = self.preset.fonts.clone();
        if matches!(self.focus, FontFocus::List | FontFocus::Search)
            && let Some(i) = self.highlighted()
        {
            fonts[self.slot].family = self.installed[i].family.clone();
        }
        fonts
    }

    /// Group paths leading to a font in the tree.
    pub(crate) fn path_to(&self, index: usize) -> Vec<String> {
        fn find(nodes: &[Node], index: usize, parent: &str, out: &mut Vec<String>) -> bool {
            for n in nodes {
                match n {
                    Node::Leaf(i) if *i == index => return true,
                    Node::Leaf(_) => {}
                    Node::Group { name, children } => {
                        let path = if parent.is_empty() {
                            name.clone()
                        } else {
                            format!("{parent}/{name}")
                        };
                        out.push(path.clone());
                        if find(children, index, &path, out) {
                            return true;
                        }
                        out.pop();
                    }
                }
            }
            false
        }
        let mut out = Vec::new();
        find(&self.tree, index, "", &mut out);
        out
    }

    pub(crate) fn clamp_sel(&mut self) {
        self.list_sel = self.list_sel.min(self.rows().len().saturating_sub(1));
    }

    /// Opens or closes the group on the highlighted row.
    fn set_expanded(&mut self, open: bool) {
        if let Some(Row::Group { path, .. }) = self.rows().get(self.list_sel) {
            if open {
                self.expanded.insert(path.clone());
            } else {
                self.expanded.remove(path);
            }
        }
    }

    /// Enter / click on the highlighted row: a group opens or closes, a font
    /// is used for the selected role.
    fn activate(&mut self) {
        match self.rows().get(self.list_sel) {
            Some(Row::Group { expanded, .. }) => {
                let open = !expanded;
                self.set_expanded(open);
            }
            Some(Row::Font { .. }) => self.pick(),
            None => {}
        }
    }

    /// Left / h: close the highlighted group, or go up to the group a font is in.
    fn go_up(&mut self) {
        let rows = self.rows();
        match rows.get(self.list_sel) {
            Some(Row::Group { expanded: true, .. }) => self.set_expanded(false),
            Some(r) if r.depth() > 0 => {
                let d = r.depth();
                if let Some(i) = (0..self.list_sel).rev().find(|&i| rows[i].depth() < d) {
                    self.list_sel = i;
                }
            }
            _ => {}
        }
    }

    /// Mouse: a click on one of the three roles.
    pub fn click_role(&mut self, role: usize) {
        if role < 3 {
            self.slot = role;
            self.focus = FontFocus::Slots;
        }
    }

    /// Mouse: a click on a filter chip.
    pub fn click_chip(&mut self, c: char) {
        self.filters.toggle(c);
        self.focus = FontFocus::List;
        self.clamp_sel();
    }

    /// Mouse: a click on a list row; a second click on the highlighted row
    /// opens the group or uses the font.
    pub fn click_row(&mut self, i: usize) {
        if i >= self.rows().len() {
            return;
        }
        if i == self.list_sel && self.focus == FontFocus::List {
            self.activate();
        } else {
            self.list_sel = i;
            self.focus = FontFocus::List;
        }
    }

    pub fn scroll(&mut self, d: i32) {
        let n = self.rows().len();
        self.list_sel = (self.list_sel as i32 + d).clamp(0, n.saturating_sub(1) as i32) as usize;
        self.focus = FontFocus::List;
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

    /// Selects the list row of the selected role's current family.
    /// Opens the groups the role's current font is in and highlights it.
    pub fn open_list(&mut self) {
        self.search.clear();
        let family = self.preset.fonts[self.slot].family.to_lowercase();
        if let Some(index) = self
            .installed
            .iter()
            .position(|f| f.family.to_lowercase() == family)
        {
            for p in self
                .path_to(index)
                .into_iter()
                .filter(|_| self.unfold_current)
            {
                self.expanded.insert(p);
            }
            if let Some(i) = self.rows().iter().position(|r| {
                *r == Row::Font {
                    depth: r.depth(),
                    index,
                }
            }) {
                self.list_sel = i;
            }
        }
        self.clamp_sel();
        self.focus = FontFocus::List;
    }

    fn pick(&mut self) {
        if let Some(i) = self.highlighted() {
            self.preset.fonts[self.slot].family = self.installed[i].family.clone();
            self.focus = FontFocus::Slots;
            // the app saves and applies it (fonts/pick.rs)
            self.end_pick();
            self.picked = true;
        }
    }

    /// h / l on a role: the theme's size for it. The app saves and applies
    /// it once the keys have been still for a moment (`sized_at`).
    pub fn resize(&mut self, d: i32) {
        let f = &mut self.preset.fonts[self.slot];
        f.size = (f.size as i32 + d).clamp(4, 72) as u32;
        self.sized_at = Some(std::time::Instant::now());
    }

    /// Opens a preset: its fonts, at the sizes in use (they're the theme's).
    pub fn open(&mut self, name: &str) -> Result<()> {
        let mut p = self.store.load(name)?;
        p.set_sizes(self.preset.sizes());
        self.name = name.to_string();
        self.is_new = false;
        self.saved = p.clone();
        self.preset = p;
        self.focus = FontFocus::Slots;
        Ok(())
    }

    pub fn new_preset(&mut self) {
        self.name = "untitled".to_string();
        self.is_new = true;
        self.saved = self.preset.clone();
        self.focus = FontFocus::Slots;
    }

    pub fn save_as(&mut self, name: &str) -> Result<()> {
        self.store.save(name, &self.preset)?;
        self.name = name.to_string();
        self.is_new = false;
        self.saved = self.preset.clone();
        Ok(())
    }

    /// Typing into the search box (the keymap doesn't apply while typing).
    pub fn on_search_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Down => self.focus = FontFocus::List,
            KeyCode::Backspace => {
                self.search.pop();
                self.list_sel = 0;
            }
            KeyCode::Char(c) => {
                self.search.push(c);
                self.list_sel = 0;
            }
            _ => {}
        }
    }

    /// Runs an action in the roles or the font list; returns false if it
    /// means nothing there.
    pub fn act(&mut self, a: Action) -> bool {
        let n = self.rows().len();
        let last = n.saturating_sub(1);
        match self.focus {
            FontFocus::Search | FontFocus::Sample => return false,
            FontFocus::List => match a {
                Action::Down => self.list_sel = (self.list_sel + 1).min(last),
                Action::Up => self.list_sel = self.list_sel.saturating_sub(1),
                Action::Top => self.list_sel = 0,
                Action::Bottom => self.list_sel = last,
                Action::PageDown => self.list_sel = (self.list_sel + 10).min(last),
                Action::PageUp => self.list_sel = self.list_sel.saturating_sub(10),
                Action::Search => self.focus = FontFocus::Search,
                Action::Right => match self.rows().get(self.list_sel) {
                    Some(Row::Group {
                        expanded: false, ..
                    }) => self.set_expanded(true),
                    // already open: step into it
                    Some(Row::Group { expanded: true, .. }) => {
                        self.list_sel = (self.list_sel + 1).min(last)
                    }
                    _ => {}
                },
                Action::Left => self.go_up(),
                Action::Confirm => self.activate(),
                Action::Back => {
                    self.end_pick();
                    self.focus = FontFocus::Slots
                }
                Action::Filter(c) => {
                    self.filters.toggle(c);
                    self.clamp_sel();
                }
                _ => return false,
            },
            FontFocus::Slots => match a {
                Action::Down => self.slot = (self.slot + 1).min(2),
                Action::Up => self.slot = self.slot.saturating_sub(1),
                Action::Right | Action::SizeUp => self.resize(1),
                Action::Left | Action::SizeDown => self.resize(-1),
                Action::RightBig => self.resize(4),
                Action::LeftBig => self.resize(-4),
                Action::Confirm | Action::PickFont => self.start_pick(),
                _ => return false,
            },
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font(family: &str, mono: bool, nerd: bool) -> Installed {
        Installed {
            family: family.into(),
            mono,
            nerd,
            bitmap: family == "Fixed",
            special: family.contains("CJK"),
        }
    }

    fn tab_with(fonts: Vec<Installed>) -> FontsTab {
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

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(
            KeyCode::Char(c),
            ratatui::crossterm::event::KeyModifiers::NONE,
        )
    }

    fn special(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, ratatui::crossterm::event::KeyModifiers::NONE)
    }

    /// A key as the app routes it: typing while searching, otherwise the
    /// Vim keymap's action for the font list or roles.
    fn press(t: &mut FontsTab, k: KeyEvent) {
        use crate::keymap::{Context, Key, Keymap, Lookup};
        if t.focus == FontFocus::Search {
            return t.on_search_key(k);
        }
        let pane = if t.focus == FontFocus::Slots {
            Context::FontRoles
        } else {
            Context::FontList
        };
        if let Lookup::Run(a) = Keymap::vim().lookup(
            &[pane, Context::Fonts, Context::Global],
            &[Key::from_event(k)],
        ) {
            t.act(a);
        }
    }

    fn labels(t: &FontsTab) -> Vec<String> {
        t.rows()
            .iter()
            .map(|r| match r {
                Row::Group {
                    depth,
                    name,
                    expanded,
                    count,
                    ..
                } => {
                    format!(
                        "{}{}{name} {count}",
                        "  ".repeat(*depth),
                        if *expanded { "▾" } else { "▸" }
                    )
                }
                Row::Font { depth, index } => {
                    format!("{}{}", "  ".repeat(*depth), t.installed[*index].family)
                }
            })
            .collect()
    }

    #[test]
    fn grouped_list_opens_filters_searches_and_picks() {
        let mut t = tab_with(vec![
            font("DejaVu Sans", false, false),
            font("DejaVu Sans Mono", true, false),
            font("Fixed", true, false),
            font("JetBrainsMono Nerd Font", true, true),
            font("JetBrainsMono Nerd Font Mono", true, true),
            font("JetBrainsMono Nerd Font Propo", false, true),
            font("Noto Sans CJK JP", false, false),
            font("Noto Sans CJK KR", false, false),
        ]);
        // bitmap and CJK fonts are hidden at first; groups start closed
        assert_eq!(labels(&t), ["▸DejaVu 2", "▸JetBrains Mono 3"]);
        // opening the list shows where the role's font is
        t.open_list();
        assert_eq!(
            labels(&t),
            [
                "▸DejaVu 2",
                "▾JetBrains Mono 3",
                "  JetBrainsMono Nerd Font",
                "  JetBrainsMono Nerd Font Mono",
                "  JetBrainsMono Nerd Font Propo",
            ]
        );
        assert_eq!(t.list_sel, 2);
        // browsing previews the highlighted font without setting it
        press(&mut t, key('j'));
        assert_eq!(t.shown_fonts()[0].family, "JetBrainsMono Nerd Font Mono");
        assert_eq!(t.preset.fonts[0].family, "JetBrainsMono Nerd Font");
        // h goes up to the group, again closes it
        press(&mut t, key('h'));
        assert_eq!(t.list_sel, 1);
        press(&mut t, key('h'));
        assert_eq!(labels(&t), ["▸DejaVu 2", "▸JetBrains Mono 3"]);
        // filters: NF Mono only, then show the hidden kinds
        press(&mut t, key('M'));
        // a group left with one font shows the font itself
        assert_eq!(labels(&t), ["JetBrainsMono Nerd Font Mono"]);
        press(&mut t, key('M'));
        press(&mut t, key('b'));
        press(&mut t, key('c'));
        press(&mut t, key('p'));
        // (only Noto Sans CJK fonts here, so that is the shared name)
        assert_eq!(
            labels(&t),
            [
                "DejaVu Sans",
                "JetBrainsMono Nerd Font Propo",
                "▸Noto Sans CJK 2"
            ]
        );
        press(&mut t, key('p'));
        assert!(labels(&t).contains(&"Fixed".to_string()));
        // l opens a group, l again steps in, Enter uses the font
        t.list_sel = 0;
        press(&mut t, key('l'));
        press(&mut t, key('l'));
        press(&mut t, special(KeyCode::Enter));
        assert_eq!(t.preset.fonts[t.slot].family, "DejaVu Sans");
        assert_eq!(t.focus, FontFocus::Slots);
        // searching shows a flat list of matches
        t.open_list();
        press(&mut t, key('/'));
        for c in "font mono".chars() {
            press(&mut t, key(c));
        }
        assert_eq!(labels(&t), ["JetBrainsMono Nerd Font Mono"]);
        press(&mut t, special(KeyCode::Enter));
        press(&mut t, special(KeyCode::Enter));
        assert_eq!(
            t.preset.fonts[t.slot].family,
            "JetBrainsMono Nerd Font Mono"
        );
        assert!(t.dirty());
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

    #[test]
    fn mouse_clicks_select_open_and_toggle() {
        let mut t = tab_with(vec![
            font("JetBrainsMono Nerd Font", true, true),
            font("JetBrainsMono Nerd Font Mono", true, true),
            font("Zeta", false, false),
        ]);
        t.click_role(2); // bar
        assert_eq!((t.slot, t.focus), (2, FontFocus::Slots));
        t.click_row(0); // the group: select it
        assert_eq!((t.list_sel, t.focus), (0, FontFocus::List));
        t.click_row(0); // again: open it
        assert_eq!(t.rows().len(), 4);
        t.click_row(2); // second font: select, then use it
        t.click_row(2);
        assert_eq!(t.preset.fonts[2].family, "JetBrainsMono Nerd Font Mono");
        t.click_chip('m');
        assert!(t.filters.is_on('m'));
        assert!(!labels(&t).contains(&"Zeta".to_string()));
        t.click_row(99); // past the end: nothing
    }
}
