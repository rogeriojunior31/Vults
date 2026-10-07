//! A hooks file whose top-level keys name hooks, each holding its own events (Antigravity's
//! `hooks.json`): `{ "<name>": { "<Event>": [ … ] }, "<other-tool>": { … } }`.
//! Ours is one key; every other key is someone else's and is never touched.

use serde_json::Value;

/// `existing` with our hook set to `ours`, in the key's place when it was already there. A key of
/// that name that does not run our hook is the user's: it stays as it is (see [`taken`]).
/// `enabled` is the user's switch: it is kept.
pub fn with_ours(existing: &Value, name: &str, marker: &str, ours: Value) -> Value {
    if taken(existing, name, marker) {
        return existing.clone();
    }
    let mut root = existing.as_object().cloned().unwrap_or_default();
    let next = match root.get(name) {
        Some(old) => {
            let mut next = crate::in_order_of(old, ours);
            if let (Some(enabled), Some(m)) = (old.get("enabled"), next.as_object_mut()) {
                m.insert("enabled".into(), enabled.clone());
            }
            next
        }
        None => ours,
    };
    // Replacing an existing key keeps its place.
    root.insert(name.into(), next);
    Value::Object(root)
}

/// The key is there but does not run our hook: installing would overwrite the user's own.
pub fn taken(existing: &Value, name: &str, marker: &str) -> bool {
    existing.get(name).is_some() && !has_ours(existing, name, marker)
}

/// `existing` without our hook. A key of that name that does not run our hook is not ours: it stays.
pub fn remove_ours(existing: &Value, name: &str, marker: &str) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    if has_ours(existing, name, marker) {
        root.shift_remove(name);
    }
    Value::Object(root)
}

pub fn has_ours(existing: &Value, name: &str, marker: &str) -> bool {
    our_command(existing, name, marker).is_some()
}

/// The first command of ours under our key: which hook binary the file runs.
pub fn our_command<'a>(existing: &'a Value, name: &str, marker: &str) -> Option<&'a str> {
    fn find<'a>(v: &'a Value, marker: &str) -> Option<&'a str> {
        match v {
            Value::Object(m) => match m.get("command").and_then(Value::as_str) {
                Some(c) if crate::runs_ours(c, marker) => Some(c),
                _ => m.values().find_map(|v| find(v, marker)),
            },
            Value::Array(a) => a.iter().find_map(|v| find(v, marker)),
            _ => None,
        }
    }
    find(existing.get(name)?, marker)
}

/// Our hook is exactly `ours`, compared as JSON values (a file keeps its own key order).
/// `enabled` is left out: turning our hook off in the tool is not an outdated install.
pub fn ours_match(existing: &Value, name: &str, ours: &Value) -> bool {
    let mut current = match existing.get(name) {
        Some(v) => v.clone(),
        None => return false,
    };
    if let Some(m) = current.as_object_mut() {
        m.shift_remove("enabled");
    }
    current == *ours
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MARKER: &str = "vults-hook";

    fn ours(exe: &str) -> Value {
        json!({ "Stop": [ { "type": "command", "command": format!("'{exe}/vults-hook' Stop"), "timeout": 5 } ] })
    }

    #[test]
    fn other_hooks_keep_their_place_and_content() {
        let file = json!({
            "lint": { "PostToolUse": [ { "matcher": "run_command", "hooks": [ { "command": "./lint.sh" } ] } ] },
            "vults": ours("/old"),
            "guard": { "enabled": false, "Stop": [ { "command": "./guard.sh" } ] }
        });
        let updated = with_ours(&file, "vults", MARKER, ours("/new"));
        let keys: Vec<&String> = updated.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["lint", "vults", "guard"]);
        assert_eq!(updated["lint"], file["lint"]);
        assert_eq!(updated["guard"], file["guard"]);
        assert!(ours_match(&updated, "vults", &ours("/new")));
        assert!(!ours_match(&file, "vults", &ours("/new")));
        assert_eq!(
            our_command(&file, "vults", MARKER),
            Some("'/old/vults-hook' Stop")
        );

        let removed = remove_ours(&updated, "vults", MARKER);
        assert_eq!(removed, json!({ "lint": file["lint"], "guard": file["guard"] }));
        assert!(!has_ours(&removed, "vults", MARKER));
    }

    #[test]
    fn a_key_that_does_not_run_our_hook_is_not_ours() {
        let file = json!({ "vults": { "Stop": [ { "command": "./mine.sh" } ] } });
        assert!(!has_ours(&file, "vults", MARKER));
        assert_eq!(remove_ours(&file, "vults", MARKER), file);
        // Nor ours to overwrite.
        assert!(taken(&file, "vults", MARKER));
        assert_eq!(with_ours(&file, "vults", MARKER, ours("/x")), file);
        // Another key running our hook is the user's copy, not ours to manage.
        let copied = json!({ "mine": ours("/x") });
        assert!(!has_ours(&copied, "vults", MARKER));
    }

    #[test]
    fn our_hook_turned_off_stays_off_and_up_to_date() {
        let mut off = ours("/x");
        off["enabled"] = json!(false);
        let file = json!({ "vults": off });
        assert!(ours_match(&file, "vults", &ours("/x")));
        let updated = with_ours(&file, "vults", MARKER, ours("/y"));
        assert_eq!(updated["vults"]["enabled"], json!(false));
        assert!(ours_match(&updated, "vults", &ours("/y")));
    }

    #[test]
    fn a_new_file_gets_only_our_key() {
        assert_eq!(
            with_ours(&json!({}), "vults", MARKER, ours("/x")),
            json!({ "vults": ours("/x") })
        );
    }
}
