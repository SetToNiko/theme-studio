//! The difference view of the History tab: what a saved version changes in a file.
//!
//! `diff_lines` compares two JSON values and returns one `DiffLine` for each entry.

use serde_json::Value;

use crate::model::color::Rgb;

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
    let mut out = Vec::new();
    diff_walk(Some(now), Some(then), 0, all, &mut out);
    out
}

/// Adds the lines for the entries of two objects (the keys of `then`, then
/// the keys only `now` has); anything that is not an object adds nothing.
fn diff_walk(
    now: Option<&Value>,
    then: Option<&Value>,
    depth: usize,
    all: bool,
    out: &mut Vec<DiffLine>,
) {
    let (Some(Value::Object(now_map)), Some(Value::Object(then_map))) = (now, then) else {
        return;
    };
    let mut keys: Vec<&String> = then_map.keys().collect();
    keys.extend(now_map.keys().filter(|key| !then_map.contains_key(*key)));
    for key in keys {
        diff_entry(key, now_map.get(key), then_map.get(key), depth, all, out);
    }
}

/// Adds the line (or, for an object, the lines) for one key.
// Long function (reason verified, phase 6): one match over the cases of a key (two objects, same, changed, added, removed).
fn diff_entry(
    key: &str,
    old: Option<&Value>,
    new: Option<&Value>,
    depth: usize,
    all: bool,
    out: &mut Vec<DiffLine>,
) {
    let key = key.to_string();
    match (old, new) {
        (Some(Value::Object(_)), Some(Value::Object(_))) => {
            let mut inner = Vec::new();
            diff_walk(old, new, depth + 1, all, &mut inner);
            if all || !inner.is_empty() {
                out.push(DiffLine::Head { depth, key });
                out.extend(inner);
            }
        }
        (Some(x), Some(y)) if x == y => {
            if all {
                out.push(DiffLine::Same {
                    depth,
                    key,
                    value: show(x),
                });
            }
        }
        (Some(x), Some(y)) => out.push(DiffLine::Changed {
            depth,
            key,
            now: show(x),
            then: show(y),
        }),
        (None, Some(y)) => out.push(DiffLine::Added {
            depth,
            key,
            value: show(y),
        }),
        (Some(x), None) => out.push(DiffLine::Removed {
            depth,
            key,
            value: show(x),
        }),
        (None, None) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    // Long function (reason verified, phase 6): two JSON documents and the lines expected for them, checked in one table.
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
        assert_eq!(diff_lines(&now, &now, false), Vec::new());
    }
}
