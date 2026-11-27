//! Cursor moves over the rows of a tab: skip rows that you cannot select, and wrap.

use crate::keymap::action::Action;

/// The first and last rows the cursor can rest on.
pub(crate) fn ends(ok: &[bool]) -> (usize, usize) {
    (
        ok.iter().position(|o| *o).unwrap_or(0),
        ok.iter().rposition(|o| *o).unwrap_or(0),
    )
}

/// The next row after `from` (up or down) where `ok` is true; `from`
/// itself if there is none.
fn next_row(ok: &[bool], from: usize, up: bool) -> usize {
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
}

/// Moves `sel` over the rows where `ok` is true (skipping titles and
/// blank lines); false if `a` isn't a move.
pub(crate) fn move_over(sel: &mut usize, ok: &[bool], a: Action) -> bool {
    let next = |from: usize, up: bool| next_row(ok, from, up);
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
        let len = i64::try_from(n).unwrap_or(i64::MAX);
        let from = i64::try_from(i).unwrap_or(0);
        usize::try_from((from + i64::from(d)).rem_euclid(len)).unwrap_or(0)
    }
}
