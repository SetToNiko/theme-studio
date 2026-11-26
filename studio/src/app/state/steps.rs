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
///
/// # Panics
/// Panics when `len` is 0 (a ring needs at least one entry): `len` must be > 0.
pub(in crate::app) fn wrapped(index: usize, delta: i32, len: usize) -> usize {
    let moved = to_i32(index).saturating_add(delta).rem_euclid(to_i32(len));
    usize::try_from(moved).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{offset, to_i32, wrapped};

    #[test]
    fn to_i32_keeps_small_values_and_caps_huge_ones() {
        assert_eq!(to_i32(0), 0);
        assert_eq!(to_i32(42), 42);
        assert_eq!(to_i32(usize::MAX), i32::MAX);
    }

    #[test]
    fn offset_moves_both_ways_and_stops_at_zero() {
        assert_eq!(offset(5, 3), 8);
        assert_eq!(offset(5, -3), 2);
        assert_eq!(offset(2, -10), 0);
        assert_eq!(offset(0, -1), 0);
    }

    #[test]
    fn offset_survives_the_limits_of_usize_and_i32() {
        assert_eq!(offset(usize::MAX, 1), i32::MAX as usize);
        assert_eq!(offset(0, i32::MAX), i32::MAX as usize);
        assert_eq!(offset(0, i32::MIN), 0);
        assert_eq!(offset(usize::MAX, i32::MIN), 0);
    }

    #[test]
    fn wrapped_goes_around_at_both_ends() {
        assert_eq!(wrapped(0, -1, 5), 4);
        assert_eq!(wrapped(4, 1, 5), 0);
        assert_eq!(wrapped(2, 1, 5), 3);
        assert_eq!(wrapped(1, -3, 5), 3);
        assert_eq!(wrapped(0, 12, 5), 2);
    }

    #[test]
    fn wrapped_with_one_entry_stays_on_it() {
        assert_eq!(wrapped(0, 7, 1), 0);
        assert_eq!(wrapped(0, -7, 1), 0);
    }

    #[test]
    fn wrapped_handles_the_limits_of_i32() {
        assert_eq!(wrapped(0, i32::MIN, 3), 1);
        assert_eq!(wrapped(usize::MAX, i32::MAX, 3), 1);
    }

    #[test]
    #[should_panic(expected = "divisor of zero")]
    fn wrapped_panics_on_an_empty_ring() {
        let _ = wrapped(0, 1, 0);
    }
}
