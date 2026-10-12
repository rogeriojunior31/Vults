//! A TOML config of the agent's (Kimi Code's `config.toml`), edited by the same rules as a JSON
//! one: a strict read, the diff the user saw, the fingerprint of the bytes it came from, a dated
//! backup beside it and an atomic write. The document is edited in place, so the user's
//! comments, order and layout stay as they were; only what `change` touches is new.
//!
//! Hooks kept as an array of tables (`[[hooks]]`) are shared with other tools: ours are the
//! tables whose `command` runs our hook, and only those are added, replaced or taken out.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table};

use crate::{Backups, Error, Preview, diff, fingerprint, read_bytes, replace};

/// Reads a TOML file. A missing or blank file is an empty document; anything that does not parse
/// is an error, never an empty document: writing over it would lose the user's file.
pub fn read(path: &Path) -> Result<DocumentMut, Error> {
    parse(&read_bytes(path)?, path)
}

pub fn preview(path: &Path, change: impl FnOnce(&mut DocumentMut)) -> Result<Preview, Error> {
    let bytes = read_bytes(path)?;
    let mut doc = parse(&bytes, path)?;
    let before = doc.to_string();
    change(&mut doc);
    let after = doc.to_string();
    Ok(Preview {
        diff: if after == before {
            String::new()
        } else {
            diff(&text(&bytes), &after, path)
        },
        fingerprint: fingerprint(&bytes),
    })
}

/// Applies `change` if the file still has the bytes `fingerprint_seen` was taken from. Returns the
/// backup, or `None` when there was no file.
pub fn apply(
    path: &Path,
    fingerprint_seen: &str,
    change: impl FnOnce(&mut DocumentMut),
    now: SystemTime,
) -> Result<Option<PathBuf>, Error> {
    let bytes = read_bytes(path)?;
    let mut doc = parse(&bytes, path)?;
    if fingerprint(&bytes) != fingerprint_seen {
        return Err(Error::Changed {
            path: path.to_path_buf(),
        });
    }
    change(&mut doc);
    replace(path, doc.to_string().as_bytes(), now, Backups::Beside)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes)).into_owned()
}

fn parse(bytes: &[u8], path: &Path) -> Result<DocumentMut, Error> {
    let invalid = |reason: String| Error::InvalidToml {
        path: path.to_path_buf(),
        reason,
    };
    let text = std::str::from_utf8(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes))
        .map_err(|e| invalid(e.to_string()))?;
    text.parse::<DocumentMut>()
        .map_err(|e| invalid(e.message().to_string()))
}

/// Our tables in `key`'s array of tables set to `ours`, in order, after every other tool's.
/// A `key` that is something else than an array of tables is the user's: see [`taken`].
pub fn with_ours(doc: &mut DocumentMut, key: &str, ours: &[Table], marker: &str) {
    if taken(doc, key) {
        return;
    }
    remove_ours(doc, key, marker);
    let list = doc
        .entry(key)
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
    if let Some(list) = list.as_array_of_tables_mut() {
        for t in ours {
            list.push(t.clone());
        }
    }
}

/// `key` is there but not an array of tables (`hooks = "…"`, or a `[hooks]` table): installing
/// would overwrite it.
pub fn taken(doc: &DocumentMut, key: &str) -> bool {
    doc.get(key).is_some_and(|i| !i.is_array_of_tables())
}

/// Our tables taken out of `key`; the key goes too when none are left.
pub fn remove_ours(doc: &mut DocumentMut, key: &str, marker: &str) {
    let Some(list) = doc.get_mut(key).and_then(Item::as_array_of_tables_mut) else {
        return;
    };
    if !list.iter().any(|t| is_ours(t, marker)) {
        return;
    }
    list.retain(|t| !is_ours(t, marker));
    if list.is_empty() {
        doc.remove(key);
    }
}

/// The first command of ours: which hook binary the file runs.
pub fn our_command<'a>(doc: &'a DocumentMut, key: &str, marker: &str) -> Option<&'a str> {
    doc.get(key)?
        .as_array_of_tables()?
        .iter()
        .filter_map(|t| t.get("command").and_then(Item::as_str))
        .find(|c| crate::runs_ours(c, marker))
}

/// Our tables are exactly `ours`, in order (compared as TOML, not as text).
pub fn ours_match(doc: &DocumentMut, key: &str, ours: &[Table], marker: &str) -> bool {
    let Some(list) = doc.get(key).and_then(Item::as_array_of_tables) else {
        return ours.is_empty();
    };
    let found: Vec<String> = list
        .iter()
        .filter(|t| is_ours(t, marker))
        .map(canonical)
        .collect();
    let wanted: Vec<String> = ours.iter().map(canonical).collect();
    found == wanted
}

fn is_ours(t: &Table, marker: &str) -> bool {
    t.get("command")
        .and_then(Item::as_str)
        .is_some_and(|c| crate::runs_ours(c, marker))
}

/// A table's keys and values, without the layout around them.
fn canonical(t: &Table) -> String {
    let mut pairs: Vec<String> = t
        .iter()
        .map(|(k, v)| format!("{k}={}", v.to_string().trim()))
        .collect();
    pairs.sort();
    pairs.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const M: &str = "vults-hook";

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-toml-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("config.toml")
    }

    fn hook(exe: &str, event: &str) -> Table {
        let mut t = Table::new();
        t.insert("event", toml_edit::value(event));
        t.insert(
            "command",
            toml_edit::value(format!("'{exe}' --agent kimi {event}")),
        );
        t.insert("timeout", toml_edit::value(5));
        t
    }

    const FILE: &str = "# my settings\nmodel = \"k2\" # the fast one\n\n[[hooks]]\nevent = \"Stop\"\ncommand = \"notify-send done\"\n\n[ui]\ntheme = \"dark\"\n";

    #[test]
    fn comments_and_other_tools_hooks_stay() {
        let mut doc: DocumentMut = FILE.parse().unwrap();
        let ours = [
            hook("/a/vults-hook", "SessionStart"),
            hook("/a/vults-hook", "Stop"),
        ];
        with_ours(&mut doc, "hooks", &ours, M);
        let text = doc.to_string();
        assert!(
            text.starts_with("# my settings\nmodel = \"k2\" # the fast one\n"),
            "{text}"
        );
        assert!(text.contains("notify-send done"));
        assert!(text.contains("theme = \"dark\""));
        assert!(ours_match(&doc, "hooks", &ours, M));
        assert_eq!(
            our_command(&doc, "hooks", M),
            Some("'/a/vults-hook' --agent kimi SessionStart")
        );

        // A reinstall replaces ours; a new path is not up to date.
        let moved = [
            hook("/b/vults-hook", "SessionStart"),
            hook("/b/vults-hook", "Stop"),
        ];
        assert!(!ours_match(&doc, "hooks", &moved, M));
        with_ours(&mut doc, "hooks", &moved, M);
        assert_eq!(doc["hooks"].as_array_of_tables().unwrap().len(), 3);

        // Removing gives the user's file back.
        remove_ours(&mut doc, "hooks", M);
        assert_eq!(doc.to_string(), FILE);
    }

    #[test]
    fn a_hooks_key_of_another_shape_is_left_alone() {
        let mut doc: DocumentMut = "[hooks]\nenabled = true\n".parse().unwrap();
        assert!(taken(&doc, "hooks"));
        with_ours(&mut doc, "hooks", &[hook("/a/vults-hook", "Stop")], M);
        assert_eq!(doc.to_string(), "[hooks]\nenabled = true\n");
    }

    #[test]
    fn the_write_is_the_diff_the_user_saw_after_a_backup() {
        let path = temp("write");
        std::fs::write(&path, FILE).unwrap();
        let ours = [hook("/a/vults-hook", "Stop")];
        let change = |d: &mut DocumentMut| with_ours(d, "hooks", &ours, M);
        let p = preview(&path, change).unwrap();
        assert!(
            p.diff
                .contains("+command = \"'/a/vults-hook' --agent kimi Stop\""),
            "{}",
            p.diff
        );
        assert!(!p.diff.contains("-# my settings"), "{}", p.diff);

        // The file changed after the preview: nothing is written.
        std::fs::write(&path, format!("{FILE}\n")).unwrap();
        assert!(matches!(
            apply(&path, &p.fingerprint, change, SystemTime::now()),
            Err(Error::Changed { .. })
        ));
        std::fs::write(&path, FILE).unwrap();
        let backup = apply(&path, &p.fingerprint, change, SystemTime::now())
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), FILE);
        assert!(ours_match(&read(&path).unwrap(), "hooks", &ours, M));
        // Nothing left to change: an empty diff.
        assert!(preview(&path, change).unwrap().is_noop());
    }

    #[test]
    fn a_file_that_does_not_parse_is_refused_and_a_missing_one_is_empty() {
        let path = temp("bad");
        std::fs::write(&path, "model = \n").unwrap();
        assert!(matches!(read(&path), Err(Error::InvalidToml { .. })));
        let change = |d: &mut DocumentMut| with_ours(d, "hooks", &[hook("/a/vults-hook", "Stop")], M);
        assert!(preview(&path, change).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "model = \n");

        let missing = temp("missing");
        assert_eq!(read(&missing).unwrap().to_string(), "");
        assert_eq!(
            apply(
                &missing,
                &preview(&missing, change).unwrap().fingerprint,
                change,
                SystemTime::now()
            )
            .unwrap(),
            None
        );
        assert!(std::fs::read_to_string(&missing).unwrap().contains("[[hooks]]"));
    }
}
