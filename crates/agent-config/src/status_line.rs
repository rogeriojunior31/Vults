//! Claude Code's `statusLine`: `{ "statusLine": { "type": "command", "command": "…" } }`. A config
//! has only one, so a status line the user already has is kept running: on install its object
//! is saved in a sidecar beside our hook and only its `command` becomes ours (its `padding` and
//! the rest stay); our hook runs the saved command and prints its output. Uninstall puts the
//! saved object back exactly, and leaves alone a statusLine that is no longer ours.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Value, json};

use crate::{Error, Preview};

/// The sidecar beside the hook binary, holding the user's old statusLine object. The hook reads
/// it by this name next to itself.
pub const PREVIOUS_FILE: &str = "statusline-previous.json";

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

/// `existing` with our command in its statusLine. Whatever else the object has (padding, a
/// refresh interval) stays, in place. Someone else's command is replaced too: save it first
/// ([`install`] does).
pub fn with_ours(existing: &Value, command: &str, marker: &str) -> Value {
    let mut root = existing.as_object().cloned().unwrap_or_default();
    let line = match root.get("statusLine") {
        Some(Value::Object(old)) if owner(existing, marker) != Owner::None => {
            let mut line = old.clone();
            // Replacing an existing key keeps its place.
            line.insert("type".into(), json!("command"));
            line.insert("command".into(), json!(command));
            Value::Object(line)
        }
        _ => json!({ "type": "command", "command": command }),
    };
    root.insert("statusLine".into(), line);
    Value::Object(root)
}

/// Install: the config with our statusLine, and what the sidecar holds then. Someone else's
/// statusLine is saved there; ours keeps what was saved; none leaves nothing saved (an old
/// sidecar would run a command the user has since removed).
pub fn install(
    existing: &Value,
    saved: Option<&Value>,
    command: &str,
    marker: &str,
) -> (Value, Option<Value>) {
    let saved = match owner(existing, marker) {
        Owner::Theirs => existing.get("statusLine").cloned(),
        Owner::Ours => saved.cloned(),
        Owner::None => None,
    };
    (with_ours(existing, command, marker), saved)
}

/// Uninstall: ours goes and the saved one comes back as it was, or the key goes if nothing was
/// saved. A statusLine that is not ours is left alone, and so is the sidecar.
pub fn uninstall(existing: &Value, saved: Option<&Value>, marker: &str) -> (Value, Option<Value>) {
    if owner(existing, marker) != Owner::Ours {
        return (existing.clone(), saved.cloned());
    }
    let mut root = existing.as_object().cloned().unwrap_or_default();
    match saved {
        Some(old) => {
            root.insert("statusLine".into(), old.clone());
        }
        None => {
            root.shift_remove("statusLine");
        }
    }
    (Value::Object(root), None)
}

/// `existing` without our statusLine; anyone else's stays. Nothing saved is restored: see
/// [`uninstall`].
pub fn remove_ours(existing: &Value, marker: &str) -> Value {
    uninstall(existing, None, marker).0
}

/// [`crate::preview`] of a change to the config and the sidecar together: one diff with both
/// files, and a fingerprint of both.
pub fn preview(
    config: &Path,
    sidecar: &Path,
    change: impl FnOnce(&Value, Option<&Value>) -> (Value, Option<Value>),
) -> Result<Preview, Error> {
    let (side_bytes, saved) = read_sidecar(sidecar)?;
    let mut next_saved = None;
    let plan = crate::preview(config, |v| {
        let (next, s) = change(v, saved.as_ref());
        next_saved = s;
        next
    })?;
    let mut diff = plan.diff;
    if next_saved != saved {
        let before = saved.as_ref().map(crate::rendered).unwrap_or_default();
        let after = next_saved.as_ref().map(crate::rendered).unwrap_or_default();
        diff.push_str(&crate::diff(&before, &after, sidecar));
    }
    Ok(Preview {
        diff,
        fingerprint: format!("{}:{}", plan.fingerprint, crate::fingerprint(&side_bytes)),
    })
}

/// [`crate::apply`] of what [`preview`] showed. A sidecar to keep is written before the config
/// and one to drop is removed after it: whatever fails, the hook never runs our own command
/// while the user's old one is lost.
pub fn apply(
    config: &Path,
    sidecar: &Path,
    fingerprint_seen: &str,
    change: impl FnOnce(&Value, Option<&Value>) -> (Value, Option<Value>),
    now: SystemTime,
) -> Result<Option<PathBuf>, Error> {
    let changed = |path: &Path| Error::Changed {
        path: path.to_path_buf(),
    };
    let (config_seen, side_seen) = fingerprint_seen.split_once(':').ok_or_else(|| changed(config))?;
    let (side_bytes, saved) = read_sidecar(sidecar)?;
    if crate::fingerprint(&side_bytes) != side_seen {
        return Err(changed(sidecar));
    }
    let bytes = crate::read_bytes(config)?;
    if crate::fingerprint(&bytes) != config_seen {
        return Err(changed(config));
    }
    let current = crate::parse_json(&bytes, config)?;
    let (next, next_saved) = change(&current, saved.as_ref());

    let write_err = |source| Error::Write {
        path: sidecar.to_path_buf(),
        source,
    };
    let wrote_sidecar = match &next_saved {
        Some(v) if Some(v) != saved.as_ref() => {
            write_sidecar(sidecar, crate::rendered(v).as_bytes()).map_err(write_err)?;
            true
        }
        _ => false,
    };
    let result = crate::apply(config, config_seen, |_| next, now);
    match (&result, wrote_sidecar) {
        (Err(_), true) => {
            // Put the sidecar back as it was.
            let _ = if side_bytes.is_empty() {
                std::fs::remove_file(sidecar)
            } else {
                write_sidecar(sidecar, &side_bytes)
            };
        }
        (Ok(_), _) if next_saved.is_none() && saved.is_some() => {
            let _ = std::fs::remove_file(sidecar);
        }
        _ => {}
    }
    result
}

/// The sidecar's bytes and the object they hold. Missing is nothing saved; unreadable is an
/// error, never "nothing": that would drop the user's status line on uninstall.
fn read_sidecar(path: &Path) -> Result<(Vec<u8>, Option<Value>), Error> {
    let bytes = crate::read_bytes(path)?;
    let saved = if bytes.iter().all(u8::is_ascii_whitespace) {
        None
    } else {
        Some(crate::parse_json(&bytes, path)?)
    };
    Ok((bytes, saved))
}

fn write_sidecar(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("sidecar");
    let temp = path.with_file_name(format!(".{name}.tmp-{}", std::process::id()));
    let written = crate::write_private(&temp, bytes, None).and_then(|()| std::fs::rename(&temp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: &str = "vultures-ai-hook";
    const OURS: &str = "'/x/vultures-ai-hook' --agent claude --statusline";

    #[test]
    fn ours_goes_in_where_there_is_none_and_comes_back_out() {
        let empty = json!({ "model": "opus" });
        let with = with_ours(&empty, OURS, M);
        assert_eq!(owner(&with, M), Owner::Ours);
        assert_eq!(with["model"], "opus");
        assert_eq!(remove_ours(&with, M), empty);
        // Again is the same.
        assert_eq!(with_ours(&with, OURS, M), with);
    }

    #[test]
    fn taking_ours_out_keeps_the_order() {
        let ours = json!({ "a": 1, "statusLine": { "type": "command", "command": OURS }, "b": 2, "c": 3 });
        let keys: Vec<String> = remove_ours(&ours, M)
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys, ["a", "b", "c"]);
    }

    #[test]
    fn their_status_line_is_saved_and_only_its_command_changes() {
        let line = json!({ "type": "command", "command": "~/bin/my-line.sh", "padding": 1 });
        let theirs = json!({ "a": 1, "statusLine": line, "b": 2 });
        assert_eq!(owner(&theirs, M), Owner::Theirs);
        let (next, saved) = install(&theirs, None, OURS, M);
        assert_eq!(saved.as_ref(), Some(&line));
        assert_eq!(
            next,
            json!({ "a": 1, "statusLine": { "type": "command", "command": OURS, "padding": 1 }, "b": 2 })
        );
        // Reinstalling keeps what was saved, and the padding.
        let (again, kept) = install(&next, saved.as_ref(), OURS, M);
        assert_eq!((&again, &kept), (&next, &saved));
        // Uninstalling puts it back exactly.
        let (back, left) = uninstall(&next, saved.as_ref(), M);
        assert_eq!(back, theirs);
        assert_eq!(left, None);
        assert_eq!(owner(&json!({}), M), Owner::None);
        assert_eq!(owner(&json!({ "statusLine": null }), M), Owner::None);
    }

    #[test]
    fn a_status_line_that_is_no_longer_ours_is_left_alone() {
        let saved = json!({ "type": "command", "command": "old.sh" });
        let changed = json!({ "statusLine": { "type": "command", "command": "new.sh" } });
        assert_eq!(
            uninstall(&changed, Some(&saved), M),
            (changed.clone(), Some(saved))
        );
        // Installing over none drops a stale sidecar.
        assert_eq!(
            install(&json!({}), Some(&json!({ "command": "x" })), OURS, M).1,
            None
        );
    }

    fn temp(name: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("vultures-ai-statusline-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data")).unwrap();
        (dir.join("settings.json"), dir.join("data").join(PREVIOUS_FILE))
    }

    fn run(config: &Path, sidecar: &Path, install_it: bool) -> Preview {
        let change = |v: &Value, s: Option<&Value>| {
            if install_it {
                install(v, s, OURS, M)
            } else {
                uninstall(v, s, M)
            }
        };
        let plan = preview(config, sidecar, change).unwrap();
        apply(config, sidecar, &plan.fingerprint, change, SystemTime::now()).unwrap();
        plan
    }

    #[test]
    fn install_keeps_a_sidecar_and_uninstall_restores_the_file_byte_for_byte() {
        let (config, sidecar) = temp("roundtrip");
        let original = "{\n  \"model\": \"opus\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"~/bin/my-line.sh\",\n    \"padding\": 0\n  },\n  \"theme\": \"dark\"\n}\n";
        std::fs::write(&config, original).unwrap();

        let plan = run(&config, &sidecar, true);
        // The diff shows both files: our command, and the old line saved beside the hook.
        assert!(
            plan.diff.contains(&format!("+++ {}", sidecar.display())),
            "{}",
            plan.diff
        );
        assert!(
            plan.diff.contains("+  \"command\": \"~/bin/my-line.sh\""),
            "{}",
            plan.diff
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(&sidecar).unwrap()).unwrap(),
            json!({ "type": "command", "command": "~/bin/my-line.sh", "padding": 0 })
        );
        let installed = crate::read_json(&config).unwrap();
        assert_eq!(installed["statusLine"]["command"], OURS);
        assert_eq!(installed["statusLine"]["padding"], 0);

        // A second install changes nothing.
        assert!(run(&config, &sidecar, true).is_noop());

        run(&config, &sidecar, false);
        assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
        assert!(!sidecar.exists());
    }

    #[test]
    fn a_foreign_status_line_after_install_is_untouched() {
        let (config, sidecar) = temp("foreign");
        std::fs::write(&config, r#"{"statusLine":{"type":"command","command":"old.sh"}}"#).unwrap();
        run(&config, &sidecar, true);
        // The user sets another one by hand.
        let theirs =
            "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"new.sh\"\n  }\n}\n";
        std::fs::write(&config, theirs).unwrap();
        let plan = run(&config, &sidecar, false);
        assert!(plan.is_noop(), "{}", plan.diff);
        assert_eq!(std::fs::read_to_string(&config).unwrap(), theirs);
    }

    #[test]
    fn a_sidecar_changed_after_the_preview_stops_the_write() {
        let (config, sidecar) = temp("changed");
        std::fs::write(&config, r#"{"statusLine":{"type":"command","command":"old.sh"}}"#).unwrap();
        let change = |v: &Value, s: Option<&Value>| install(v, s, OURS, M);
        let plan = preview(&config, &sidecar, change).unwrap();
        std::fs::write(&sidecar, "{}").unwrap();
        assert!(matches!(
            apply(&config, &sidecar, &plan.fingerprint, change, SystemTime::now()),
            Err(Error::Changed { .. })
        ));
        // And a broken sidecar is refused, never taken for "nothing saved".
        std::fs::write(&sidecar, "{oops").unwrap();
        assert!(preview(&config, &sidecar, change).is_err());
    }
}
