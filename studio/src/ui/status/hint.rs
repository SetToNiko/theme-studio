//! The hint text: one hint from the keys of the active keymap, and the
//! general hints that follow the hints of the control.

use crate::app::app_state::App;
use crate::app::state::nav::Page;
use crate::keymap::action::Action;
use crate::keymap::action::Action as A;
use crate::ui::widgets::friendly;

/// Hint rows: the actions whose keys are shown, and the label for them.
pub(super) type Rows<'a> = Vec<(&'a [Action], &'static str)>;

/// One hint, such as `"j/k move"`: the keys bound to `actions`, then `label`.
/// `None` when none of the actions has a key.
pub(super) fn hint(app: &App, actions: &[Action], label: &str) -> Option<String> {
    let ctx = app.contexts();
    let keys: Vec<String> = actions
        .iter()
        .filter_map(|a| app.keymap.key_for(&ctx, *a))
        .collect();
    let keys: Vec<String> = keys.iter().map(|k| friendly(k)).collect();
    (!keys.is_empty()).then(|| format!("{} {label}", keys.join("/")))
}

/// Turns rows into hints; rows without a key are left out.
pub(super) fn row_hints(app: &App, rows: Rows) -> Vec<String> {
    rows.into_iter()
        .filter_map(|(actions, label)| hint(app, actions, label))
        .collect()
}

/// The general hints, after the control's own: next pane, panels, save,
/// apply, open. The main page's keys don't work on the other pages, so the
/// other pages have none.
pub(super) fn general_hints(app: &App) -> Vec<String> {
    let main = app.page == Page::Main;
    [
        hint(app, &[A::FocusNext], "next").filter(|_| main),
        panels_hint(app).filter(|_| main),
        hint(app, &[A::Save], "save").filter(|_| main),
        hint(app, &[A::Apply], "apply").filter(|_| main),
        hint(app, &[A::Open], "open").filter(|_| main),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The panel keys as one range, such as `"Space 1-7 panels"`.
fn panels_hint(app: &App) -> Option<String> {
    match (hint(app, &[A::Panel(1)], ""), hint(app, &[A::Panel(7)], "")) {
        // "Space 1-7 panels" rather than "Space 1-Space 7 panels"
        (Some(a), Some(b)) => {
            let (a, b) = (a.trim(), b.trim());
            let shared = a.len().saturating_sub(1);
            Some(if a.len() == b.len() && a[..shared] == b[..shared] {
                format!("{a}-{} panels", &b[shared..])
            } else {
                format!("{a}-{b} panels")
            })
        }
        _ => None,
    }
}
