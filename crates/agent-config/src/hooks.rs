//! The `hooks` object shared by Claude Code's `settings.json` and Codex's `hooks.json`:
//! `{ "hooks": { "<Event>": [ { "hooks": [ { "type": "command", "command": "…", "timeout": 10 } ] } ] } }`.
//! Our entries are recognized by a marker in the command; everyone else's are kept as they are.

use serde_json::{Map, Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookEntry {
    pub event: &'static str,
    pub command: String,
    /// In the unit the agent reads: seconds for Claude Code and Codex, milliseconds for Gemini.
    pub timeout: u64,
    /// Shown by the agent while the hook runs, in place of its generic spinner text.
    pub status_message: Option<&'static str>,
}

/// `existing` with our entries (re)added, one per event, after everyone else's.
pub fn with_ours(existing: &Value, entries: &[HookEntry], marker: &str) -> Value {
    let mut root = remove_ours(existing, marker)
        .as_object()
        .cloned()
        .unwrap_or_default();
    let mut hooks = root
        .get("hooks")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for entry in entries {
        let slot = hooks.entry(entry.event).or_insert_with(|| json!([]));
        if !slot.is_array() {
            // Someone else's non-list value: leave it, skip the event rather than clobber it.
            continue;
        }
        if let Some(list) = slot.as_array_mut() {
            list.push(entry_json(entry));
        }
    }
    root.insert("hooks".into(), Value::Object(hooks));
    Value::Object(root)
}

/// `existing` without any of our entries, and nothing else changed. An event list we empty is
/// dropped, and so is a `hooks` object we empty (an event list that was already empty before
/// install cannot be told apart, so it goes too).
pub fn remove_ours(existing: &Value, marker: &str) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    let Some(hooks) = root.get("hooks").and_then(Value::as_object) else {
        return Value::Object(root);
    };
    let mut kept = Map::new();
    for (event, value) in hooks {
        match value.as_array() {
            Some(list) => {
                let others: Vec<Value> = list.iter().filter(|e| !is_ours(e, marker)).cloned().collect();
                if !others.is_empty() || list.is_empty() {
                    kept.insert(event.clone(), Value::Array(others));
                }
            }
            None => {
                kept.insert(event.clone(), value.clone());
            }
        }
    }
    if kept.is_empty() && !hooks.is_empty() {
        root.shift_remove("hooks");
    } else {
        root.insert("hooks".into(), Value::Object(kept));
    }
    Value::Object(root)
}

fn entry_json(entry: &HookEntry) -> Value {
    let mut hook = json!({ "type": "command", "command": entry.command, "timeout": entry.timeout });
    if let Some(message) = entry.status_message {
        hook["statusMessage"] = message.into();
    }
    json!({ "hooks": [hook] })
}

/// Our entries in `existing` are exactly `entries`, wherever they sit among the user's own.
pub fn ours_match(existing: &Value, entries: &[HookEntry], marker: &str) -> bool {
    let mut have: Vec<(String, String)> = existing
        .get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .flat_map(|(event, list)| {
            list.as_array()
                .into_iter()
                .flatten()
                .filter(|e| is_ours(e, marker))
                .map(move |e| (event.clone(), e.to_string()))
        })
        .collect();
    let mut want: Vec<(String, String)> = entries
        .iter()
        .map(|e| (e.event.to_string(), entry_json(e).to_string()))
        .collect();
    have.sort();
    want.sort();
    have == want
}

pub fn has_ours(existing: &Value, marker: &str) -> bool {
    existing
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|hooks| {
            hooks
                .values()
                .filter_map(Value::as_array)
                .flatten()
                .any(|e| is_ours(e, marker))
        })
}

fn is_ours(entry: &Value, marker: &str) -> bool {
    entry.get("hooks").and_then(Value::as_array).is_some_and(|hooks| {
        hooks.iter().any(|h| {
            h.get("command")
                .and_then(Value::as_str)
                .is_some_and(|c| c.contains(marker))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ours_match_ignores_where_they_sit_but_not_what_they_say() {
        let mine = json!({ "hooks": [ { "type": "command", "command": "mine" } ] });
        let installed = with_ours(&json!({}), &entries(), MARKER);
        assert!(ours_match(&installed, &entries(), MARKER));
        // The user's own hook added after ours changes nothing.
        let mut later = installed.clone();
        later["hooks"]["Stop"].as_array_mut().unwrap().push(mine);
        assert!(ours_match(&later, &entries(), MARKER));
        // An older timeout, or an entry gone, is out of date.
        let mut newer = entries();
        newer[0].timeout = 120;
        assert!(!ours_match(&installed, &newer, MARKER));
        assert!(!ours_match(&installed, &entries()[..1], MARKER));
    }

    const MARKER: &str = "vultures-ai-hook";

    fn entries() -> Vec<HookEntry> {
        ["PreToolUse", "Stop"]
            .into_iter()
            .map(|event| HookEntry {
                event,
                command: format!("\"/opt/vultures-ai-hook\" --agent claude {event}"),
                timeout: 10,
                status_message: (event == "Stop").then_some("Waiting"),
            })
            .collect()
    }

    fn settings() -> Value {
        json!({
            "model": "opus",
            "permissions": { "allow": ["Bash(ls)"] },
            "hooks": {
                "PreToolUse": [ { "matcher": "Bash", "hooks": [ { "type": "command", "command": "other-tool" } ] } ],
                "Notification": [ { "hooks": [ { "type": "command", "command": "keep-me" } ] } ]
            }
        })
    }

    #[test]
    fn install_keeps_everything_and_uninstall_restores_it_exactly() {
        let after = with_ours(&settings(), &entries(), MARKER);
        assert_eq!(after["model"], "opus");
        assert_eq!(after["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        assert_eq!(
            after["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
            "other-tool"
        );
        assert_eq!(
            after["hooks"]["Notification"][0]["hooks"][0]["command"],
            "keep-me"
        );
        assert!(has_ours(&after, MARKER));
        assert_eq!(remove_ours(&after, MARKER), settings());
    }

    #[test]
    fn a_status_message_is_written_only_where_set() {
        let after = with_ours(&json!({}), &entries(), MARKER);
        assert_eq!(after["hooks"]["Stop"][0]["hooks"][0]["statusMessage"], "Waiting");
        assert!(
            after["hooks"]["PreToolUse"][0]["hooks"][0]
                .get("statusMessage")
                .is_none()
        );
    }

    #[test]
    fn reinstalling_does_not_duplicate() {
        let once = with_ours(&settings(), &entries(), MARKER);
        assert_eq!(with_ours(&once, &entries(), MARKER), once);
    }

    #[test]
    fn a_fresh_file_round_trips_to_nothing() {
        let after = with_ours(&json!({}), &entries(), MARKER);
        assert!(has_ours(&after, MARKER));
        assert_eq!(remove_ours(&after, MARKER), json!({}));
    }

    #[test]
    fn foreign_shapes_are_left_alone() {
        let odd = json!({ "hooks": { "PreToolUse": "not a list" } });
        let after = with_ours(&odd, &entries(), MARKER);
        assert_eq!(after["hooks"]["PreToolUse"], "not a list");
        assert_eq!(remove_ours(&after, MARKER), odd);
    }
}
