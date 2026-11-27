//! The upgrade of an older keymaps.json to the current format.
//!
//! `migrate` runs one step for each format version that the file lacks.

use serde_json::Value;

/// Updates an older keymaps.json. Version 1 (no "version") had panes on the
/// plain digits; bindings still exactly like that move to the leader, so
/// your own changes are left alone.
pub(super) fn migrate(mut v: Value) -> Value {
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(1);
    if version < 2 {
        move_panes_to_leader(&mut v);
    }
    if version < 3 {
        rename_use_for_actions(&mut v);
    }
    v
}

/// Version 1 to 2: a pane binding that is still the plain digit moves to
/// `<leader>` plus the digit.
fn move_panes_to_leader(v: &mut Value) {
    if let Some(presets) = v.get_mut("presets").and_then(Value::as_object_mut) {
        for preset in presets.values_mut() {
            let Some(global) = preset
                .pointer_mut("/bindings/global")
                .and_then(Value::as_object_mut)
            else {
                continue;
            };
            for n in 1..=9 {
                let id = format!("panel-{n}");
                if global.get(&id) == Some(&serde_json::json!([n.to_string()])) {
                    global.insert(id, serde_json::json!([format!("<leader>{n}")]));
                }
            }
        }
    }
}

/// Version 2 to 3: the `use-for-<role>` actions are named after the font
/// roles now.
fn rename_use_for_actions(v: &mut Value) {
    if let Some(presets) = v.get_mut("presets").and_then(Value::as_object_mut) {
        for preset in presets.values_mut() {
            let Some(contexts) = preset.get_mut("bindings").and_then(Value::as_object_mut) else {
                continue;
            };
            for ctx in contexts.values_mut().filter_map(Value::as_object_mut) {
                let old = std::mem::take(ctx);
                *ctx = old
                    .into_iter()
                    .map(|(k, keys)| {
                        let role = k.strip_prefix("use-for-").and_then(|r| {
                            crate::fonts::presets::OLD_ROLES
                                .iter()
                                .position(|o| *o == r)
                        });
                        match role {
                            Some(r) => (format!("use-for-{}", crate::model::theme::ROLES[r]), keys),
                            None => (k, keys),
                        }
                    })
                    .collect();
            }
        }
    }
}
