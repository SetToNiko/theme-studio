//! The Settings page (F3): Keymaps, Behaviour and History tabs. Keymaps and
//! Behaviour changes wait in a draft until Save (see draft.rs); History acts
//! at once. Every write goes through the history, so it can be undone.

use std::path::PathBuf;

use serde_json::Value;

use crate::keymap::Context;
use crate::keymap::action::Action;
use crate::model::color::Rgb;
use crate::model::history::SnapshotInfo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Keymaps,
    Behaviour,
    History,
    /// docs/theme-studio.md, rendered (docs.rs).
    Readme,
}

pub const TABS: [(Tab, &str); 4] = [
    (Tab::Keymaps, "Keymaps"),
    (Tab::Behaviour, "Behaviour"),
    (Tab::History, "History"),
    (Tab::Readme, "README"),
];

/// A row of the Keymaps tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KmRow {
    /// The preset button (opens the preset list).
    Preset,
    /// A blank line before a group (not selectable).
    Gap,
    /// A group's title (not selectable).
    Header(Context),
    Binding(Context, Action),
}

/// The files with a history, as the History tab lists them.
pub const FILES: [&str; 5] = [
    "themes.json",
    "palettes.json",
    "fonts.json",
    "keymaps.json",
    "settings.json",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistFocus {
    Files,
    Versions,
}

pub struct SettingsPage {
    pub tab: Tab,
    pub km_sel: usize,
    pub km_scroll: usize,
    pub bh_sel: usize,
    pub bh_scroll: usize,
    pub hist_file: usize,
    pub hist_focus: HistFocus,
    pub hist_sel: usize,
    pub hist_scroll: usize,
    /// The keyboard is on the tab strip (left/right change tabs).
    pub tabs_focused: bool,
    /// Unsaved Keymaps / Behaviour changes.
    pub draft: Option<crate::draft::Draft>,
    /// A config file was reloaded during a draft edit (see with_draft).
    pub reloaded: bool,
    /// Keymaps tab: closed groups.
    pub km_closed: std::collections::HashSet<Context>,
    /// Behaviour tab: closed groups, by title.
    pub bh_closed: std::collections::HashSet<&'static str>,
    /// Keymaps tab: browsing presets, with this one highlighted.
    pub browse: Option<usize>,
    pub browse_scroll: usize,
    /// The preset list's difference shows every action.
    pub browse_all: bool,
    /// The difference view shows everything, not just what differs.
    pub show_all: bool,
    pub diff_scroll: usize,
    /// README tab: the guide's text with the file's modified time (read
    /// again when that changes), its render for the last width, and the
    /// first line shown.
    pub readme: Option<(
        Option<std::time::SystemTime>,
        std::result::Result<String, String>,
    )>,
    pub readme_doc: Option<(u16, crate::docs::Doc)>,
    pub readme_scroll: usize,
    /// Cached until something changes (see history::generation).
    versions: Option<(VersionsKey, Vec<(PathBuf, SnapshotInfo)>)>,
    view: Option<(ViewKey, DiffView)>,
}

/// (history generation, file)
type VersionsKey = (u64, usize);
/// (history generation, file, selected version, show everything, palette)
type ViewKey = (u64, usize, Option<PathBuf>, bool, String);
/// A palette's colours by key.
pub type Colours = Vec<(String, Rgb)>;

/// The History tab's preview of the selected version.
#[derive(Clone, Debug, Default)]
pub struct DiffView {
    pub lines: Vec<DiffLine>,
    /// palettes.json: the palette being edited, now and in the selected
    /// version (key, colour).
    pub palette: Option<(Colours, Colours)>,
}

impl Default for SettingsPage {
    fn default() -> Self {
        SettingsPage {
            tab: Tab::Keymaps,
            km_sel: 0,
            km_scroll: 0,
            bh_sel: 0,
            bh_scroll: 0,
            hist_file: 0,
            hist_focus: HistFocus::Versions,
            hist_sel: 0,
            hist_scroll: 0,
            tabs_focused: false,
            draft: None,
            reloaded: false,
            km_closed: Default::default(),
            bh_closed: Default::default(),
            browse: None,
            browse_scroll: 0,
            browse_all: false,
            show_all: false,
            diff_scroll: 0,
            readme: None,
            readme_doc: None,
            readme_scroll: 0,
            versions: None,
            view: None,
        }
    }
}

impl SettingsPage {
    /// The cached versions of a file, if they were read at this history
    /// generation (see `history::generation`).
    pub(crate) fn cached_versions(
        &self,
        generation: u64,
        file: usize,
    ) -> Option<Vec<(PathBuf, SnapshotInfo)>> {
        match &self.versions {
            Some((k, v)) if *k == (generation, file) => Some(v.clone()),
            _ => None,
        }
    }

    /// Keeps a file's versions until the history generation changes.
    pub(crate) fn store_versions(
        &mut self,
        generation: u64,
        file: usize,
        v: Vec<(PathBuf, SnapshotInfo)>,
    ) {
        self.versions = Some(((generation, file), v));
    }

    /// The cached difference view, if it was built for exactly this
    /// generation, file, selected version, "show everything" flag and palette.
    pub(crate) fn cached_view(
        &self,
        generation: u64,
        file: usize,
        version: Option<&std::path::Path>,
        show_all: bool,
        palette: &str,
    ) -> Option<DiffView> {
        match &self.view {
            Some((k, v))
                if k.0 == generation
                    && k.1 == file
                    && k.2.as_deref() == version
                    && k.3 == show_all
                    && k.4 == palette =>
            {
                Some(v.clone())
            }
            _ => None,
        }
    }

    /// Keeps the difference view for the same five values.
    pub(crate) fn store_view(
        &mut self,
        generation: u64,
        file: usize,
        version: Option<PathBuf>,
        show_all: bool,
        palette: String,
        view: DiffView,
    ) {
        self.view = Some(((generation, file, version, show_all, palette), view));
    }
}

/// A line of the History tab's difference view.
#[derive(Clone, Debug, PartialEq)]
pub enum DiffLine {
    Head {
        depth: usize,
        key: String,
    },
    Same {
        depth: usize,
        key: String,
        value: String,
    },
    Changed {
        depth: usize,
        key: String,
        now: String,
        then: String,
    },
    /// Only in the selected version (restoring adds it).
    Added {
        depth: usize,
        key: String,
        value: String,
    },
    /// Only now (restoring removes it).
    Removed {
        depth: usize,
        key: String,
        value: String,
    },
}

fn show(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(show).collect::<Vec<_>>().join("  "),
        Value::Object(o) => format!("{{{} entries}}", o.len()),
        other => other.to_string(),
    }
}

/// The selected version against the file now: every entry, with what
/// differs marked (`all`), or only the differences and the groups they're in.
pub fn diff_lines(now: &Value, then: &Value, all: bool) -> Vec<DiffLine> {
    fn walk(
        now: Option<&Value>,
        then: Option<&Value>,
        depth: usize,
        all: bool,
        out: &mut Vec<DiffLine>,
    ) {
        let (Some(Value::Object(n)), Some(Value::Object(t))) = (now, then) else {
            return;
        };
        let mut keys: Vec<&String> = t.keys().collect();
        keys.extend(n.keys().filter(|k| !t.contains_key(*k)));
        for k in keys {
            let (a, b) = (n.get(k), t.get(k));
            match (a, b) {
                (Some(Value::Object(_)), Some(Value::Object(_))) => {
                    let mut inner = Vec::new();
                    walk(a, b, depth + 1, all, &mut inner);
                    if all || !inner.is_empty() {
                        out.push(DiffLine::Head {
                            depth,
                            key: k.clone(),
                        });
                        out.extend(inner);
                    }
                }
                (Some(x), Some(y)) if x == y => {
                    if all {
                        out.push(DiffLine::Same {
                            depth,
                            key: k.clone(),
                            value: show(x),
                        })
                    }
                }
                (Some(x), Some(y)) => out.push(DiffLine::Changed {
                    depth,
                    key: k.clone(),
                    now: show(x),
                    then: show(y),
                }),
                (None, Some(y)) => out.push(DiffLine::Added {
                    depth,
                    key: k.clone(),
                    value: show(y),
                }),
                (Some(x), None) => out.push(DiffLine::Removed {
                    depth,
                    key: k.clone(),
                    value: show(x),
                }),
                (None, None) => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(Some(now), Some(then), 0, all, &mut out);
    out
}

/// The first and last rows the cursor can rest on.
pub(crate) fn ends(ok: &[bool]) -> (usize, usize) {
    (
        ok.iter().position(|o| *o).unwrap_or(0),
        ok.iter().rposition(|o| *o).unwrap_or(0),
    )
}

/// Moves `sel` over the rows where `ok` is true (skipping titles and
/// blank lines); false if `a` isn't a move.
pub(crate) fn move_over(sel: &mut usize, ok: &[bool], a: Action) -> bool {
    let next = |from: usize, up: bool| -> usize {
        let mut i = from;
        loop {
            if up {
                if i == 0 {
                    return from;
                }
                i -= 1;
            } else {
                if i + 1 >= ok.len() {
                    return from;
                }
                i += 1;
            }
            if ok[i] {
                return i;
            }
        }
    };
    let (first, last) = ends(ok);
    let s = (*sel).min(ok.len().saturating_sub(1));
    *sel = match a {
        Action::Up => next(s, true),
        Action::Down => next(s, false),
        Action::PageUp => (0..10).fold(s, |s, _| next(s, true)),
        Action::PageDown => (0..10).fold(s, |s, _| next(s, false)),
        Action::Top => first,
        Action::Bottom => last,
        _ => return false,
    };
    true
}

pub(crate) fn wrap(i: usize, d: i32, n: usize) -> usize {
    if n == 0 {
        0
    } else {
        (i as i32 + d).rem_euclid(n as i32) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn difference_view_shows_changes_in_place() {
        let now = json!({"version": 2, "palettes": {"a": {"red": "#111111", "bg0": "#000000"}, "gone": {"x": 1}}});
        let then = json!({"version": 2, "palettes": {"a": {"red": "#222222", "bg0": "#000000"}, "new": {"x": 2}}});
        let only = diff_lines(&now, &then, false);
        assert_eq!(
            only,
            vec![
                DiffLine::Head {
                    depth: 0,
                    key: "palettes".into()
                },
                DiffLine::Head {
                    depth: 1,
                    key: "a".into()
                },
                DiffLine::Changed {
                    depth: 2,
                    key: "red".into(),
                    now: "#111111".into(),
                    then: "#222222".into()
                },
                DiffLine::Added {
                    depth: 1,
                    key: "new".into(),
                    value: "{1 entries}".into()
                },
                DiffLine::Removed {
                    depth: 1,
                    key: "gone".into(),
                    value: "{1 entries}".into()
                },
            ]
        );
        let all = diff_lines(&now, &then, true);
        assert!(all.contains(&DiffLine::Same {
            depth: 2,
            key: "bg0".into(),
            value: "#000000".into()
        }));
        assert!(all.contains(&DiffLine::Same {
            depth: 0,
            key: "version".into(),
            value: "2".into()
        }));
        assert!(diff_lines(&now, &now, false).is_empty());
    }
}
