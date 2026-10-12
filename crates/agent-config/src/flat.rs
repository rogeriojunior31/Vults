//! A hooks file of flat lists per event, shared with other tools (Cursor's `hooks.json`):
//! `{ "version": 1, "hooks": { "<event>": [ { "command": …, "timeout": … }, … ] } }`.
//! Ours are the handlers whose command runs our hook; every other handler, event and key is
//! someone else's and is never touched.

use serde_json::{Map, Value};

/// `existing` with our handlers set to `ours` (`(event, handler)`, in order): ours from before
/// are taken out first, each new one goes at the end of its event's list. A file without a
/// `version` gets `version`.
pub fn with_ours(existing: &Value, ours: &[(&str, Value)], marker: &str, version: u64) -> Value {
    let mut root = remove_ours(existing, marker)
        .as_object()
        .cloned()
        .unwrap_or_default();
    root.entry("version").or_insert(version.into());
    let mut hooks = root
        .get("hooks")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (event, handler) in ours {
        match hooks.get_mut(*event).and_then(Value::as_array_mut) {
            Some(list) => list.push(handler.clone()),
            None => {
                hooks.insert((*event).into(), Value::Array(vec![handler.clone()]));
            }
        }
    }
    root.insert("hooks".into(), Value::Object(hooks));
    Value::Object(root)
}

/// `existing` without our handlers; an event they leave empty goes too, one that was already
/// empty stays.
pub fn remove_ours(existing: &Value, marker: &str) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    if let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) {
        let mut next = Map::new();
        for (event, list) in std::mem::take(hooks) {
            match list {
                Value::Array(items) if items.iter().any(|h| is_ours(h, marker)) => {
                    let kept: Vec<Value> = items.into_iter().filter(|h| !is_ours(h, marker)).collect();
                    if !kept.is_empty() {
                        next.insert(event, Value::Array(kept));
                    }
                }
                other => {
                    next.insert(event, other);
                }
            }
        }
        *hooks = next;
    }
    Value::Object(root)
}

pub fn has_ours(existing: &Value, marker: &str) -> bool {
    our_command(existing, marker).is_some()
}

/// The first command of ours: which hook binary the file runs.
pub fn our_command<'a>(existing: &'a Value, marker: &str) -> Option<&'a str> {
    existing
        .get("hooks")?
        .as_object()?
        .values()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|h| h.get("command").and_then(Value::as_str))
        .find(|c| crate::runs_ours(c, marker))
}

/// Our handlers are exactly `ours`, each under its event (another tool's around them aside).
pub fn ours_match(existing: &Value, ours: &[(&str, Value)], marker: &str) -> bool {
    let Some(hooks) = existing.get("hooks").and_then(Value::as_object) else {
        return false;
    };
    let mut found: Vec<(&str, &Value)> = Vec::new();
    for (event, list) in hooks {
        for h in list
            .as_array()
            .into_iter()
            .flatten()
            .filter(|h| is_ours(h, marker))
        {
            found.push((event.as_str(), h));
        }
    }
    found.len() == ours.len() && ours.iter().all(|(e, h)| found.contains(&(*e, h)))
}

fn is_ours(handler: &Value, marker: &str) -> bool {
    handler
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| crate::runs_ours(c, marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const M: &str = "vults-hook";

    fn ours(cmd: &str) -> Vec<(&'static str, Value)> {
        vec![
            (
                "sessionStart",
                json!({ "command": format!("{cmd} sessionStart"), "timeout": 5 }),
            ),
            ("stop", json!({ "command": format!("{cmd} stop"), "timeout": 5 })),
        ]
    }

    #[test]
    fn another_tools_handlers_stay_around_ours() {
        let theirs = json!({ "command": "audit.sh", "timeout": 3 });
        let file = json!({ "version": 1, "hooks": { "stop": [theirs], "afterFileEdit": [{ "command": "fmt.sh" }] } });
        let next = with_ours(&file, &ours("/a/vults-hook"), M, 1);
        assert_eq!(next["hooks"]["stop"][0], theirs);
        assert_eq!(next["hooks"]["stop"][1]["command"], "/a/vults-hook stop");
        assert_eq!(next["hooks"]["afterFileEdit"][0]["command"], "fmt.sh");
        assert!(ours_match(&next, &ours("/a/vults-hook"), M));
        assert!(!ours_match(&next, &ours("/b/vults-hook"), M));
        assert_eq!(our_command(&next, M), Some("/a/vults-hook stop"));

        // A reinstall replaces ours, never adds a second set.
        let again = with_ours(&next, &ours("/b/vults-hook"), M, 1);
        assert_eq!(again["hooks"]["stop"].as_array().unwrap().len(), 2);
        assert!(ours_match(&again, &ours("/b/vults-hook"), M));

        // Removing gives the file back as it was.
        assert_eq!(remove_ours(&again, M), file);
    }

    #[test]
    fn an_empty_file_gets_a_version() {
        let next = with_ours(&json!({}), &ours("/a/vults-hook"), M, 1);
        assert_eq!(next["version"], 1);
        assert!(has_ours(&next, M));
        assert_eq!(remove_ours(&next, M), json!({ "version": 1, "hooks": {} }));
        // A version the user set is theirs.
        assert_eq!(with_ours(&json!({ "version": 2 }), &[], M, 1)["version"], 2);
    }
}
