//! The Keymaps tab's preset browser (`settings_page/`; not the font presets of `fonts/presets.rs`): the preset list in the right column,
//! and on the left the highlighted preset against the current keymap
//! (only what differs; `A` for everything). Enter loads a preset, `n` makes
//! a new one from it, `d` deletes one of yours, `R` resets a shipped one.
//! Like every Keymaps change, all of it waits for Save.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::state::hit::{HitMap, Target, hover_line};
use crate::keymap::action::{Action, all_actions};
use crate::keymap::key::display_seq;
use crate::keymap::{context::Context, map::Keymap};
use crate::settings_page::SettingsPage;

/// A line of the difference between two keymaps.
#[derive(Clone, Debug, PartialEq)]
pub enum KmDiff {
    Leader {
        now: String,
        then: String,
    },
    Context(Context),
    /// An action's keys now and in the preset (equal when shown for "all").
    Row {
        action: Action,
        now: String,
        then: String,
    },
}

fn keys_text(km: &Keymap, ctx: Context, a: Action) -> String {
    let keys: Vec<String> = km
        .keys_of(ctx, a)
        .iter()
        .map(|k| display_seq(k, km.leader))
        .collect();
    keys.join("  ")
}

/// `then` against `now`: the actions whose keys differ, by context (or every
/// action, with `all`).
pub fn keymap_diff(now: &Keymap, then: &Keymap, all: bool) -> Vec<KmDiff> {
    let none = crate::keymap::key::Key::new(
        ratatui::crossterm::event::KeyCode::Null,
        ratatui::crossterm::event::KeyModifiers::NONE,
    );
    let mut out = Vec::new();
    let (ln, lt) = (
        display_seq(&[now.leader], none),
        display_seq(&[then.leader], none),
    );
    if all || ln != lt {
        out.push(KmDiff::Leader { now: ln, then: lt });
    }
    for ctx in Context::ALL {
        let rows: Vec<KmDiff> = all_actions()
            .into_iter()
            .filter_map(|a| {
                let (n, t) = (keys_text(now, ctx, a), keys_text(then, ctx, a));
                ((n != t || all) && !(n.is_empty() && t.is_empty())).then_some(KmDiff::Row {
                    action: a,
                    now: n,
                    then: t,
                })
            })
            .collect();
        if !rows.is_empty() {
            out.push(KmDiff::Context(ctx));
            out.extend(rows);
        }
    }
    out
}

pub(crate) fn is_shipped(name: &str) -> bool {
    Keymap::builtin().iter().any(|b| b.name == name)
}

// ----- drawing ---------------------------------------------------------------

/// What the preset browser draws from, and where it records click areas.
pub struct BrowseParts<'a> {
    /// Where the rows' click areas are added.
    pub hits: &'a mut HitMap,
    /// All presets, in list order.
    pub presets: &'a [Keymap],
    /// The active keymap (its preset is marked "active").
    pub keymap: &'a Keymap,
}

/// The browser: the difference on the left (`diff`), the list on the right
/// (`list`).
pub fn draw_browse(
    f: &mut Frame,
    BrowseParts {
        hits,
        presets,
        keymap,
    }: BrowseParts,
    page: &mut SettingsPage,
    diff: Rect,
    list: Rect,
) {
    let Some(sel) = page.browse else {
        return;
    };
    let Some(then) = presets.get(sel).cloned() else {
        return;
    };
    draw_preset_list(f, hits, presets, keymap, sel, list);
    draw_difference(f, hits, keymap, &then, page, diff);
}

/// The keys of the preset browser, one line each.
const BROWSE_HELP: [&str; 7] = [
    "Enter  load it (then Save)",
    "n      new preset from it",
    "d      delete (yours only)",
    "R      reset (shipped only)",
    "A      everything / differences",
    "PgUp/PgDn  scroll the keys",
    "Esc    back",
];

/// One row of the preset list: the name (highlighted when `selected`), then
/// whether the preset is shipped or yours, and whether it is active.
fn preset_line(p: &Keymap, active: bool, selected: bool) -> Line<'static> {
    let style = if selected {
        Style::new().reversed()
    } else {
        Style::new()
    };
    // the cursor on the name, not the padding around it
    let mut spans =
        crate::app::state::hit::cursor_spans(format!(" {:<16}", p.name), Style::new(), style);
    spans.push(Span::styled(
        format!(
            " {}{}",
            if is_shipped(&p.name) {
                "shipped"
            } else {
                "yours"
            },
            if active { ", active" } else { "" }
        ),
        Style::new().dark_gray(),
    ));
    Line::from(spans)
}

/// The preset list (the right column), with its key help.
fn draw_preset_list(
    f: &mut Frame,
    hits: &mut HitMap,
    presets: &[Keymap],
    keymap: &Keymap,
    sel: usize,
    list: Rect,
) {
    let lines = preset_list_lines(hits, presets, keymap, sel, list);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::new()
                .borders(Borders::LEFT)
                .border_style(Style::new().dark_gray()),
        ),
        list,
    );
}

/// The lines of the preset list: a title, a row for each preset that fits
/// in `list` (each adds a click area to `hits`), and the key help.
fn preset_list_lines(
    hits: &mut HitMap,
    presets: &[Keymap],
    keymap: &Keymap,
    sel: usize,
    list: Rect,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::styled("Presets", Style::new().bold()), Line::raw("")];
    let top = list.y + 2;
    for (i, p) in presets.iter().enumerate() {
        let y = top.saturating_add(u16::try_from(i).unwrap_or(u16::MAX));
        if y >= list.bottom() {
            break;
        }
        let r = Rect {
            x: list.x,
            y,
            width: list.width,
            height: 1,
        };
        let hovered = hits.add(r, Target::PresetRow(i)) && i != sel;
        let line = preset_line(p, p.name == keymap.name, i == sel);
        // only the name is underlined
        lines.push(if hovered { hover_line(line, 0) } else { line });
    }
    lines.push(Line::raw(""));
    for h in BROWSE_HELP {
        lines.push(Line::styled(h, Style::new().dark_gray()));
    }
    lines
}

/// The title above the difference: `changed` is how many actions differ.
fn diff_title(keymap: &Keymap, then: &Keymap, changed: usize) -> String {
    if then.name == keymap.name {
        format!(" {} (the current keymap)", then.name)
    } else if changed == 0 {
        format!(" {} → {}: the same keys", keymap.name, then.name)
    } else {
        format!(" {} → {}: {changed} differ", keymap.name, then.name)
    }
}

/// The two lines above the difference: the title and what the marks mean.
fn diff_head(title: String) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::styled(title, Style::new().bold()),
        Line::from(vec![
            Span::styled(" changed ", Style::new().black().on_yellow()),
            Span::styled(" current → preset", Style::new().dark_gray()),
        ]),
    ])
}

/// The difference (the left column): a title, then the keys that differ.
fn draw_difference(
    f: &mut Frame,
    hits: &mut HitMap,
    keymap: &Keymap,
    then: &Keymap,
    page: &mut SettingsPage,
    diff: Rect,
) {
    let rows = keymap_diff(keymap, then, page.browse_all);
    let changed = keymap_diff(keymap, then, false)
        .iter()
        .filter(|d| !matches!(d, KmDiff::Context(_)))
        .count();
    let [head, body] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(diff);
    let title = diff_title(keymap, then, changed);
    f.render_widget(diff_head(title), head);
    hits.add(body, Target::SettingsDiff);
    let h = usize::from(body.height);
    let max = rows.len().saturating_sub(h);
    page.browse_scroll = page.browse_scroll.min(max);
    let w = usize::from(body.width).saturating_sub(4).min(46);
    let out: Vec<Line> = rows
        .iter()
        .skip(page.browse_scroll)
        .take(h)
        .map(|d| diff_line(d, w))
        .collect();
    f.render_widget(Paragraph::new(out), body);
}

/// The keys, or "no keys" when there are none.
fn keys_or_none(s: &str) -> String {
    if s.is_empty() {
        "no keys".to_string()
    } else {
        s.to_string()
    }
}

/// One line of the difference: a context title, or a label (padded to `w`
/// columns) with its keys; changed rows show the old keys crossed out.
fn diff_line(d: &KmDiff, w: usize) -> Line<'static> {
    let (now, then) = match d {
        KmDiff::Context(c) => {
            return Line::styled(format!(" {}", c.title()), Style::new().bold().cyan());
        }
        KmDiff::Leader { now, then } | KmDiff::Row { now, then, .. } => (now, then),
    };
    let label = match d {
        KmDiff::Row { action, .. } => action.describe(),
        _ => "leader".into(),
    };
    let label = format!("   {:<w$} ", label.chars().take(w).collect::<String>());
    let none = keys_or_none;
    if now == then {
        Line::from(vec![
            Span::raw(label),
            Span::styled(none(now), Style::new().yellow()),
        ])
    } else {
        Line::from(vec![
            Span::styled(label, Style::new().black().on_yellow()),
            Span::styled(
                none(now),
                Style::new().dark_gray().add_modifier(Modifier::CROSSED_OUT),
            ),
            Span::raw(" → "),
            Span::styled(none(then), Style::new().yellow().bold()),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn differences_between_presets() {
        let (vim, emacs) = (Keymap::vim(), Keymap::emacs());
        let d = keymap_diff(&vim, &emacs, false);
        assert!(d.contains(&KmDiff::Leader {
            now: "<Space>".into(),
            then: "<C-c>".into()
        }));
        assert!(d.contains(&KmDiff::Context(Context::Global)));
        assert!(d.contains(&KmDiff::Row {
            action: Action::Quit,
            now: "q".into(),
            then: "<C-x><C-c>".into()
        }));
        // the same keys aren't listed, unless everything is asked for
        let same = KmDiff::Row {
            action: Action::PageMain,
            now: "<F1>".into(),
            then: "<F1>".into(),
        };
        assert!(!d.contains(&same));
        assert!(keymap_diff(&vim, &emacs, true).contains(&same));
        assert_eq!(keymap_diff(&vim, &vim, false), Vec::new());
    }
}
