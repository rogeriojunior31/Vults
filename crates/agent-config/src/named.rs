//! A hooks file whose top-level keys name hooks, each holding its own events (Antigravity's
//! `hooks.json`): `{ "<name>": { "<Event>": [ … ] }, "<other-tool>": { … } }`.
//! Ours is one key; every other key is someone else's and is never touched.

use serde_json::Value;

/// `existing` with our hook set to `ours`, in the key's place when it was already there.
pub fn with_ours(existing: &Value, name: &str, ours: Value) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    let next = match root.get(name) {
        Some(old) => crate::in_order_of(old, ours),
        None => ours,
    };
    // Replacing an existing key keeps its place.
    root.insert(name.into(), next);
    Value::Object(root)
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
                Some(c) if c.contains(marker) => Some(c),
                _ => m.values().find_map(|v| find(v, marker)),
            },
            Value::Array(a) => a.iter().find_map(|v| find(v, marker)),
            _ => None,
        }
    }
    find(existing.get(name)?, marker)
}

/// Our hook is exactly `ours`, compared as JSON values (a file keeps its own key order).
pub fn ours_match(existing: &Value, name: &str, ours: &Value) -> bool {
    existing.get(name) == Some(ours)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MARKER: &str = "vultures-ai-hook";

    fn ours(exe: &str) -> Value {
        json!({ "Stop": [ { "type": "command", "command": format!("'{exe}/vultures-ai-hook' Stop"), "timeout": 5 } ] })
    }

    #[test]
    fn other_hooks_keep_their_place_and_content() {
        let file = json!({
            "lint": { "PostToolUse": [ { "matcher": "run_command", "hooks": [ { "command": "./lint.sh" } ] } ] },
            "vultures-ai": ours("/old"),
            "guard": { "enabled": false, "Stop": [ { "command": "./guard.sh" } ] }
        });
        let updated = with_ours(&file, "vultures-ai", ours("/new"));
        let keys: Vec<&String> = updated.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["lint", "vultures-ai", "guard"]);
        assert_eq!(updated["lint"], file["lint"]);
        assert_eq!(updated["guard"], file["guard"]);
        assert!(ours_match(&updated, "vultures-ai", &ours("/new")));
        assert!(!ours_match(&file, "vultures-ai", &ours("/new")));
        assert_eq!(
            our_command(&file, "vultures-ai", MARKER),
            Some("'/old/vultures-ai-hook' Stop")
        );

        let removed = remove_ours(&updated, "vultures-ai", MARKER);
        assert_eq!(removed, json!({ "lint": file["lint"], "guard": file["guard"] }));
        assert!(!has_ours(&removed, "vultures-ai", MARKER));
    }

    #[test]
    fn a_key_that_does_not_run_our_hook_is_not_ours() {
        let file = json!({ "vultures-ai": { "Stop": [ { "command": "./mine.sh" } ] } });
        assert!(!has_ours(&file, "vultures-ai", MARKER));
        assert_eq!(remove_ours(&file, "vultures-ai", MARKER), file);
        // Another key running our hook is the user's copy, not ours to manage.
        let copied = json!({ "mine": ours("/x") });
        assert!(!has_ours(&copied, "vultures-ai", MARKER));
    }

    #[test]
    fn a_new_file_gets_only_our_key() {
        assert_eq!(
            with_ours(&json!({}), "vultures-ai", ours("/x")),
            json!({ "vultures-ai": ours("/x") })
        );
    }
}
