//! Claude Code's `statusLine`: `{ "statusLine": { "type": "command", "command": "…" } }`. A config
//! has only one, so a statusLine someone else set is never touched: ours goes in only where
//! there is none (or ours), and comes out only if it is ours.

use serde_json::{Value, json};

/// Whose statusLine the config has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    None,
    Ours,
    Theirs,
}

pub fn owner(existing: &Value, marker: &str) -> Owner {
    match existing.get("statusLine") {
        None | Some(Value::Null) => Owner::None,
        Some(v) if v["command"].as_str().is_some_and(|c| c.contains(marker)) => Owner::Ours,
        Some(_) => Owner::Theirs,
    }
}

/// `existing` with our statusLine, unless the user has their own.
pub fn with_ours(existing: &Value, command: &str, marker: &str) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    if owner(existing, marker) != Owner::Theirs {
        root.insert(
            "statusLine".into(),
            json!({ "type": "command", "command": command }),
        );
    }
    Value::Object(root)
}

/// `existing` without our statusLine; anyone else's stays.
pub fn remove_ours(existing: &Value, marker: &str) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    if owner(existing, marker) == Owner::Ours {
        root.remove("statusLine");
    }
    Value::Object(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: &str = "vultures-ai-hook";

    #[test]
    fn ours_goes_in_where_there_is_none_and_comes_back_out() {
        let empty = json!({ "model": "opus" });
        let with = with_ours(&empty, "'/x/vultures-ai-hook' --statusline", M);
        assert_eq!(owner(&with, M), Owner::Ours);
        assert_eq!(with["model"], "opus");
        assert_eq!(remove_ours(&with, M), empty);
        // Again is the same.
        assert_eq!(with_ours(&with, "'/x/vultures-ai-hook' --statusline", M), with);
    }

    #[test]
    fn someone_elses_status_line_is_never_touched() {
        let theirs =
            json!({ "statusLine": { "type": "command", "command": "~/bin/my-line.sh", "padding": 1 } });
        assert_eq!(owner(&theirs, M), Owner::Theirs);
        assert_eq!(
            with_ours(&theirs, "'/x/vultures-ai-hook' --statusline", M),
            theirs
        );
        assert_eq!(remove_ours(&theirs, M), theirs);
        assert_eq!(owner(&json!({}), M), Owner::None);
        assert_eq!(owner(&json!({ "statusLine": null }), M), Owner::None);
    }
}
