//! Index arithmetic for list cursors and scroll positions.
//!
//! Cursor moves mix `usize` positions with `i32` steps (a step can be
//! negative). These helpers do the conversion in one place, without `as` casts.

/// A list position as `i32`. Lists are far shorter than `i32::MAX` rows; the
/// fallback only keeps the function total.
pub(in crate::app) fn to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// `index` moved by `delta` (negative moves back); never below 0.
pub(in crate::app) fn offset(index: usize, delta: i32) -> usize {
    usize::try_from(to_i32(index).saturating_add(delta)).unwrap_or(0)
}

/// `index` moved by `delta` in a ring of `len` entries (wraps around at both ends).
pub(in crate::app) fn wrapped(index: usize, delta: i32, len: usize) -> usize {
    let moved = to_i32(index).saturating_add(delta).rem_euclid(to_i32(len));
    usize::try_from(moved).unwrap_or(0)
}
