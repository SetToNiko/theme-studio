//! Fitting the hints in the room of the status line.

/// The hints that fit in `budget` columns, as one string: the context hints,
/// then the general ones after a `│`.
fn fill_hints(context: &[String], general: &[String], budget: usize) -> String {
    let mut out = String::from(" ");
    let mut general_started = false;
    for (i, hint) in context.iter().chain(general.iter()).enumerate() {
        let sep = if i == 0 {
            ""
        } else if i == context.len() && !general_started {
            general_started = true;
            " │ "
        } else {
            "  "
        };
        if out.chars().count() + sep.chars().count() + hint.chars().count() > budget {
            break;
        }
        out.push_str(sep);
        out.push_str(hint);
    }
    out
}

/// As many hints as fit in `room` columns: the ones for the current control
/// first, then the general ones, and `last` (the help key) always at the end.
pub(super) fn fit_hints(context: &[String], general: &[String], last: &str, room: usize) -> String {
    let budget = room.saturating_sub(last.chars().count() + 3);
    let mut out = fill_hints(context, general, budget);
    if out.trim().is_empty() {
        out = format!(" {last} ");
    } else {
        out.push_str("  ");
        out.push_str(last);
        out.push(' ');
    }
    // not even the help key fits: no hints at all
    if out.chars().count() > room {
        String::new()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::fit_hints;

    #[test]
    fn hints_shorten_to_fit_and_keep_help_last() {
        let s = |v: &[&str]| v.iter().map(ToString::to_string).collect::<Vec<_>>();
        let ctx = s(&["j/k move", "Enter edit", "p pin"]);
        let general = s(&["Tab next", "1-4 panels", "s save", "a apply", "o open"]);
        let wide = fit_hints(&ctx, &general, "? keys", 200);
        assert_eq!(
            wide,
            " j/k move  Enter edit  p pin │ Tab next  1-4 panels  s save  a apply  o open  ? keys "
        );
        let narrow = fit_hints(&ctx, &general, "? keys", 40);
        assert!(narrow.chars().count() <= 40, "{narrow}");
        assert!(
            narrow.starts_with(" j/k move  Enter edit") && narrow.ends_with("? keys "),
            "{narrow}"
        );
        assert_eq!(fit_hints(&ctx, &general, "? keys", 10), " ? keys ");
        assert_eq!(fit_hints(&ctx, &general, "? keys", 5), "");
        for room in 0..120 {
            assert!(fit_hints(&ctx, &general, "? keys", room).chars().count() <= room);
        }
    }
}
