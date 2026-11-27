//! The font list of the Fonts tab: the rows, the highlighted row, opening
//! and closing groups, picking a font, and mouse clicks on the list.

use crate::fonts::tab::state::FontsTab;
use crate::fonts::tree;
use crate::fonts::tree::Node;
use crate::fonts::types::{FontFocus, Row};

impl FontsTab {
    /// The rows the list shows (see [`tree::rows`]).
    pub fn rows(&self) -> Vec<Row> {
        let ctx = tree::RowCtx {
            installed: &self.installed,
            filters: &self.filters,
            expanded: &self.expanded,
        };
        tree::rows(&ctx, &self.tree, &self.search)
    }

    /// The font on the highlighted row, if it is a font.
    pub fn highlighted(&self) -> Option<usize> {
        match self.rows().get(self.list_sel) {
            Some(Row::Font { index, .. }) => Some(*index),
            _ => None,
        }
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
    pub(super) fn set_expanded(&mut self, open: bool) {
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
    pub(super) fn activate(&mut self) {
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
    pub(super) fn go_up(&mut self) {
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
        let current = i64::try_from(self.list_sel).unwrap_or(0);
        let last = i64::try_from(n.saturating_sub(1)).unwrap_or(0);
        self.list_sel = usize::try_from((current + i64::from(d)).clamp(0, last)).unwrap_or(0);
        self.focus = FontFocus::List;
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
            self.preset.fonts[self.slot]
                .family
                .clone_from(&self.installed[i].family);
            self.focus = FontFocus::Slots;
            // the app saves and applies it (fonts/pick.rs)
            self.end_pick();
            self.picked = true;
        }
    }
}

#[cfg(test)]
// The helper `labels` is shared with the tests of `keys.rs`.
pub(super) mod tests {
    use super::*;
    use crate::fonts::tab::state::tests::{font, tab_with};

    pub(in crate::fonts::tab) fn labels(t: &FontsTab) -> Vec<String> {
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
