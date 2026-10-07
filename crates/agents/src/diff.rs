//! What a finished edit changed, from whatever the agent sent: Claude Code's `structuredPatch`
//! (real line numbers), Codex's patch, or the edit's own old and new text.

use serde_json::Value;
use vults_core::{Diff, FileDiff, Hunk};
use vults_protocol::MAX_FIELD_LEN;

/// Lines of a diff kept for the card; the hook already caps Claude Code's patch at the same.
const MAX_LINES: usize = 400;
/// Shared lines shown around an edit rebuilt from its old and new text.
const CONTEXT: usize = 3;

/// A Claude Code edit, from its `PostToolUse`.
pub(crate) fn claude(tool: &str, input: &Value, response: Option<&Value>) -> Option<Diff> {
    let path = input.get("file_path")?.as_str()?.to_string();
    let hunks = response
        .and_then(|r| r.get("structuredPatch"))
        .and_then(Value::as_array);
    match hunks {
        Some(hunks) if !hunks.is_empty() => {
            let hunks: Vec<Hunk> = hunks
                .iter()
                .map(|h| Hunk {
                    old_start: h.get("oldStart").and_then(Value::as_u64).map(|n| n as u32),
                    new_start: h.get("newStart").and_then(Value::as_u64).map(|n| n as u32),
                    lines: h
                        .get("lines")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        // "\ No newline at end of file" is noise on a card.
                        .filter(|l| !l.starts_with('\\'))
                        .map(str::to_string)
                        .collect(),
                })
                .collect();
            let cut = response.and_then(|r| r.get("cut")) == Some(&Value::Bool(true));
            Some(single(path, hunks, cut))
        }
        // A new file has no patch: all of it is added.
        Some(_) if tool == "Write" => {
            let content = input.get("content")?.as_str()?;
            let lines = content.lines().map(|l| format!("+{l}")).collect();
            Some(single(
                path,
                vec![Hunk {
                    old_start: None,
                    new_start: Some(1),
                    lines,
                }],
                was_cut(content),
            ))
        }
        Some(_) => None,
        // An older hook sent no patch: rebuild it from the edit itself.
        None => from_input(tool, input),
    }
}

/// An edit rebuilt from its old and new text (`Edit`, `MultiEdit`, Gemini's `replace`), without
/// line numbers. A `Write` over a file says nothing of what was there, so it has none.
pub(crate) fn from_input(tool: &str, input: &Value) -> Option<Diff> {
    let path = input.get("file_path")?.as_str()?.to_string();
    let side = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let edits: Vec<(String, String)> = match tool {
        "Edit" | "replace" => vec![(side(input, "old_string"), side(input, "new_string"))],
        "MultiEdit" => input
            .get("edits")?
            .as_array()?
            .iter()
            .map(|e| (side(e, "old_string"), side(e, "new_string")))
            .collect(),
        _ => return None,
    };
    let cut = edits.iter().any(|(old, new)| was_cut(old) || was_cut(new));
    let hunks = edits.iter().map(|(old, new)| hunk(old, new)).collect();
    Some(single(path, hunks, cut))
}

/// Codex's `apply_patch`: `*** Update File:` / `*** Add File:` / `*** Delete File:` sections,
/// each `@@` a hunk, without line numbers.
pub(crate) fn codex(patch: &str) -> Option<Diff> {
    let mut files: Vec<FileDiff> = Vec::new();
    for line in patch.lines() {
        if let Some(path) = ["*** Add File: ", "*** Update File: ", "*** Delete File: "]
            .iter()
            .find_map(|h| line.strip_prefix(h))
        {
            files.push(FileDiff {
                path: path.trim().to_string(),
                added: 0,
                removed: 0,
                hunks: Vec::new(),
            });
            continue;
        }
        let Some(file) = files.last_mut() else {
            continue;
        };
        if line.starts_with("@@") || file.hunks.is_empty() {
            file.hunks.push(Hunk {
                old_start: None,
                new_start: None,
                lines: Vec::new(),
            });
            if line.starts_with("@@") {
                continue;
            }
        }
        if line.starts_with(['+', '-', ' '])
            && let Some(hunk) = file.hunks.last_mut()
        {
            hunk.lines.push(line.to_string());
        }
    }
    for file in &mut files {
        file.hunks.retain(|h| !h.lines.is_empty());
    }
    if files.is_empty() {
        return None;
    }
    // The hook cuts the patch past its cap: then it never reaches its end marker.
    let cut = !patch.trim_end().ends_with("*** End Patch");
    Some(capped(files, cut))
}

/// Shared lines at both ends set aside, a few kept as context; the rest is what changed.
fn hunk(old: &str, new: &str) -> Hunk {
    let old: Vec<&str> = old.lines().collect();
    let new: Vec<&str> = new.lines().collect();
    let head = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let tail = old[head..]
        .iter()
        .rev()
        .zip(new[head..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let context = |lines: &[&str]| lines.iter().map(|l| format!(" {l}")).collect::<Vec<_>>();
    let mut lines = context(&old[head.saturating_sub(CONTEXT)..head]);
    lines.extend(old[head..old.len() - tail].iter().map(|l| format!("-{l}")));
    lines.extend(new[head..new.len() - tail].iter().map(|l| format!("+{l}")));
    lines.extend(context(&old[old.len() - tail..][..tail.min(CONTEXT)]));
    Hunk {
        old_start: None,
        new_start: None,
        lines,
    }
}

fn single(path: String, hunks: Vec<Hunk>, cut: bool) -> Diff {
    capped(
        vec![FileDiff {
            path,
            added: 0,
            removed: 0,
            hunks,
        }],
        cut,
    )
}

/// At most [`MAX_LINES`] lines in all, then each file's counts from what is left.
fn capped(mut files: Vec<FileDiff>, mut cut: bool) -> Diff {
    let mut left = MAX_LINES;
    for file in &mut files {
        for hunk in &mut file.hunks {
            if hunk.lines.len() > left {
                hunk.lines.truncate(left);
                cut = true;
            }
            left -= hunk.lines.len();
        }
        file.hunks.retain(|h| !h.lines.is_empty());
        let count = |mark: char| {
            file.hunks
                .iter()
                .flat_map(|h| &h.lines)
                .filter(|l| l.starts_with(mark))
                .count() as u32
        };
        file.added = count('+');
        file.removed = count('-');
    }
    Diff { files, cut }
}

/// The hook cut this string to its field cap.
fn was_cut(text: &str) -> bool {
    text.len() >= MAX_FIELD_LEN && text.ends_with('…')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_edit_rebuilt_from_its_text_keeps_a_little_context() {
        let d = from_input(
            "Edit",
            &json!({ "file_path": "/w/a.rs", "old_string": "1\n2\n3\n4\nold\n5", "new_string": "1\n2\n3\n4\nnew\nnewer\n5" }),
        )
        .unwrap();
        assert_eq!(
            d.files[0].hunks[0].lines,
            [" 2", " 3", " 4", "-old", "+new", "+newer", " 5"]
        );
        assert_eq!((d.added(), d.removed(), d.cut), (2, 1, false));
        assert!(from_input("Write", &json!({ "file_path": "/w/a.rs", "content": "x" })).is_none());
    }

    #[test]
    fn a_codex_patch_file_by_file() {
        let d = codex(
            "*** Begin Patch\n*** Update File: /w/a.rs\n@@ fn main\n ctx\n-old\n+new\n@@\n-gone\n*** Add File: /w/b.rs\n+one\n+two\n*** Delete File: /w/c.rs\n*** End Patch",
        )
        .unwrap();
        let shape: Vec<_> = d
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.hunks.len(), f.added, f.removed))
            .collect();
        assert_eq!(
            shape,
            [("/w/a.rs", 2, 1, 2), ("/w/b.rs", 1, 2, 0), ("/w/c.rs", 0, 0, 0)]
        );
        assert_eq!(d.files[0].hunks[0].lines, [" ctx", "-old", "+new"]);
        assert!(!d.cut);
        // Cut by the hook: no end marker.
        assert!(codex("*** Begin Patch\n*** Add File: a\n+x…").unwrap().cut);
        assert!(codex("not a patch").is_none());
    }

    #[test]
    fn a_long_diff_is_capped() {
        let content = (0..1000).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
        let d = claude(
            "Write",
            &json!({ "file_path": "/w/a", "content": content }),
            Some(&json!({ "structuredPatch": [] })),
        )
        .unwrap();
        assert_eq!(d.added(), MAX_LINES as u32);
        assert!(d.cut);
    }
}
