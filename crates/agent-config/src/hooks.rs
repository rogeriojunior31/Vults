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

/// `existing` with our entries (re)added, one per event. Everything keeps its place, so the diff
/// the user approves shows only what changed: a hook of ours is updated where it sits (in the
/// file's key order, inside whatever group holds it), a new one goes after everyone else's, a new
/// event after the file's own, and `hooks` stays where it is among the top-level keys.
pub fn with_ours(existing: &Value, entries: &[HookEntry], marker: &str) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    let mut hooks = root
        .get("hooks")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    // Ours under an event we no longer register go; a list that held only them goes too.
    hooks.retain(|event, value| {
        if entries.iter().any(|e| e.event == event) {
            return true;
        }
        let Some(list) = value.as_array_mut() else {
            return true;
        };
        let before = list.len();
        strip_ours(list, None, marker);
        !list.is_empty() || before == 0
    });
    for entry in entries {
        let want = hook_json(entry);
        let slot = hooks.entry(entry.event).or_insert_with(|| json!([]));
        // Someone else's non-list value: leave it, skip the event rather than clobber it.
        let Some(list) = slot.as_array_mut() else {
            continue;
        };
        match first_of_ours(list, marker) {
            // Only our hook changes: a group the user shares with another tool keeps the rest.
            Some((g, h)) => {
                let hook = &mut list[g]["hooks"][h];
                *hook = crate::in_order_of(hook, want);
                strip_ours(list, Some((g, h)), marker);
            }
            None => list.push(json!({ "hooks": [want] })),
        }
    }
    // Replacing an existing key keeps its place.
    root.insert("hooks".into(), Value::Object(hooks));
    Value::Object(root)
}

/// Where the hook of ours to keep sits: (group, hook) in an event's list. A group of our own with
/// no matcher first, so a duplicate under another tool's matcher never narrows what we see.
fn first_of_ours(list: &[Value], marker: &str) -> Option<(usize, usize)> {
    let ours = |g: usize, group: &Value| {
        let hooks = group.get("hooks")?.as_array()?;
        Some((g, hooks.iter().position(|h| is_our_hook(h, marker))?))
    };
    let own = |group: &Value| {
        group.get("matcher").is_none()
            && group["hooks"]
                .as_array()
                .is_some_and(|hooks| hooks.iter().all(|h| is_our_hook(h, marker)))
    };
    let mut groups = list.iter().enumerate();
    groups
        .clone()
        .filter(|(_, group)| own(group))
        .find_map(|(g, group)| ours(g, group))
        .or_else(|| groups.find_map(|(g, group)| ours(g, group)))
}

/// Takes our hooks (all but `keep`) out of their groups. A group goes only when nothing but ours
/// was in it: another tool's hook sharing a group with ours keeps that group, matcher and all.
fn strip_ours(list: &mut Vec<Value>, keep: Option<(usize, usize)>, marker: &str) {
    let mut g = 0;
    list.retain_mut(|group| {
        g += 1;
        let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
            return true;
        };
        let before = hooks.len();
        let mut h = 0;
        hooks.retain(|hook| {
            h += 1;
            keep == Some((g - 1, h - 1)) || !is_our_hook(hook, marker)
        });
        !hooks.is_empty() || before == 0
    });
}

/// The command of the first entry of ours: which hook binary the config runs.
pub fn our_command<'a>(existing: &'a Value, marker: &str) -> Option<&'a str> {
    existing
        .get("hooks")?
        .as_object()?
        .values()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|e| e.get("hooks")?.as_array())
        .flatten()
        .filter_map(|h| h.get("command")?.as_str())
        .find(|c| c.contains(marker))
}

/// `existing` without any of our hooks, and nothing else changed. A group goes only when it held
/// nothing but ours; an event list we empty is dropped, and so is a `hooks` object we empty (an
/// event list that was already empty before install cannot be told apart, so it goes too).
pub fn remove_ours(existing: &Value, marker: &str) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    let Some(hooks) = root.get("hooks").and_then(Value::as_object) else {
        return Value::Object(root);
    };
    let mut kept = Map::new();
    for (event, value) in hooks {
        match value.as_array() {
            Some(list) => {
                let mut others = list.clone();
                strip_ours(&mut others, None, marker);
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

fn hook_json(entry: &HookEntry) -> Value {
    let mut hook = json!({ "type": "command", "command": entry.command, "timeout": entry.timeout });
    if let Some(message) = entry.status_message {
        hook["statusMessage"] = message.into();
    }
    hook
}

/// Our hooks in `existing` are exactly `entries`, wherever they sit among the user's own (a group
/// the user shares with another tool included).
/// Compared as JSON values, not text: a file keeps its own key order (often alphabetical), and a
/// text comparison called an installed config outdated forever while reinstalling changed nothing.
pub fn ours_match(existing: &Value, entries: &[HookEntry], marker: &str) -> bool {
    let mut have: Vec<(&str, &Value)> = existing
        .get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .flat_map(|(event, list)| {
            list.as_array()
                .into_iter()
                .flatten()
                .filter_map(|g| g.get("hooks")?.as_array())
                .flatten()
                .filter(|h| is_our_hook(h, marker))
                .map(move |h| (event.as_str(), h))
        })
        .collect();
    if have.len() != entries.len() {
        return false;
    }
    entries.iter().all(|entry| {
        let want = hook_json(entry);
        match have
            .iter()
            .position(|(event, e)| *event == entry.event && **e == want)
        {
            Some(i) => {
                have.swap_remove(i);
                true
            }
            None => false,
        }
    })
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
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| hooks.iter().any(|h| is_our_hook(h, marker)))
}

fn is_our_hook(hook: &Value, marker: &str) -> bool {
    hook.get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| c.contains(marker))
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

    #[test]
    fn ours_match_ignores_the_files_key_order() {
        // Read back from disk, the keys come in the file's order (here alphabetical), not ours.
        let text = r#"{ "hooks": {
            "PreToolUse": [ { "hooks": [ { "command": "\"/opt/vultures-ai-hook\" --agent claude PreToolUse", "timeout": 10, "type": "command" } ] } ],
            "Stop": [ { "hooks": [ { "command": "\"/opt/vultures-ai-hook\" --agent claude Stop", "statusMessage": "Waiting", "timeout": 10, "type": "command" } ] } ]
        } }"#;
        let from_disk: Value = serde_json::from_str(text).unwrap();
        assert!(ours_match(&from_disk, &entries(), MARKER));
        // And reinstalling it is a no-op, as the status now says.
        assert_eq!(with_ours(&from_disk, &entries(), MARKER), from_disk);
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
    fn an_update_keeps_every_place() {
        let text = |v: &Value| serde_json::to_string(v).unwrap();
        let ours = |cmd: &str| json!({ "hooks": [ { "command": cmd, "timeout": 5, "type": "command" } ] });
        let old = |event: &str| ours(&format!("'/old/vultures-ai-hook' --agent claude {event}"));
        let theirs = json!({ "hooks": [ { "type": "command", "command": "other-tool" } ] });
        let before = json!({
            "env": {},
            "hooks": {
                "Gone": [ old("Gone") ],
                "Stop": [ old("Stop"), theirs.clone(), old("Stop") ],
                "PreToolUse": [ theirs.clone(), old("PreToolUse") ]
            },
            "model": "opus"
        });
        let after = with_ours(&before, &entries(), MARKER);
        // Our entry where the first one sat, in the file's key order; the duplicate and an
        // event we no longer register gone; `hooks` still between `env` and `model`.
        let stop = r#"{"hooks":[{"command":"\"/opt/vultures-ai-hook\" --agent claude Stop","timeout":10,"type":"command","statusMessage":"Waiting"}]}"#;
        let pre = r#"{"hooks":[{"command":"\"/opt/vultures-ai-hook\" --agent claude PreToolUse","timeout":10,"type":"command"}]}"#;
        let other = text(&theirs);
        assert_eq!(
            text(&after),
            format!(
                r#"{{"env":{{}},"hooks":{{"Stop":[{stop},{other}],"PreToolUse":[{other},{pre}]}},"model":"opus"}}"#
            )
        );
        assert_eq!(
            our_command(&before, MARKER),
            Some("'/old/vultures-ai-hook' --agent claude Gone")
        );
    }

    #[test]
    fn a_shared_group_loses_only_our_hook() {
        let theirs = json!({ "type": "command", "command": "other-tool", "timeout": 3 });
        let old = json!({ "type": "command", "command": "'/old/vultures-ai-hook' Stop", "timeout": 5 });
        let before = json!({ "hooks": {
            "Stop": [ { "matcher": "*", "hooks": [ old.clone(), theirs.clone(), old.clone() ] } ],
            "Gone": [ { "hooks": [ theirs.clone(), old.clone() ] }, { "hooks": [ old ] } ]
        } });
        let after = with_ours(&before, &entries(), MARKER);
        // Ours updated in place, its duplicate gone; the group and their hook untouched.
        assert_eq!(after["hooks"]["Stop"].as_array().unwrap().len(), 1);
        assert_eq!(after["hooks"]["Stop"][0]["matcher"], "*");
        let stop = after["hooks"]["Stop"][0]["hooks"].as_array().unwrap();
        assert_eq!(stop.len(), 2);
        assert_eq!(stop[1], theirs);
        assert!(stop[0]["command"].as_str().unwrap().contains("/opt/"));
        // Under an event we no longer register: the shared group stays, ours alone goes.
        assert_eq!(after["hooks"]["Gone"], json!([ { "hooks": [ theirs.clone() ] } ]));
        assert!(ours_match(&after, &entries(), MARKER));

        // A duplicate of ours under another tool's matcher goes, not our own group for all tools.
        let narrowed = json!({ "hooks": { "Stop": [
            { "matcher": "Bash", "hooks": [ theirs.clone(), stop[0].clone() ] },
            { "hooks": [ stop[0].clone() ] }
        ] } });
        assert_eq!(
            with_ours(&narrowed, &entries(), MARKER)["hooks"]["Stop"],
            json!([ { "matcher": "Bash", "hooks": [ theirs.clone() ] }, { "hooks": [ stop[0].clone() ] } ])
        );

        let removed = remove_ours(&after, MARKER);
        assert_eq!(
            removed,
            json!({ "hooks": {
                "Stop": [ { "matcher": "*", "hooks": [ theirs.clone() ] } ],
                "Gone": [ { "hooks": [ theirs ] } ]
            } })
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
