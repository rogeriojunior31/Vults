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
    let path = path_of(input)?;
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

/// The file an edit names; a notebook's cell edit names its notebook.
fn path_of(input: &Value) -> Option<String> {
    let path = input.get("file_path").or_else(|| input.get("notebook_path"))?;
    Some(path.as_str()?.to_string())
}

/// An edit rebuilt from its old and new text (`Edit`, `MultiEdit`, Gemini's `replace`), without
/// line numbers. A `Write` over a file says nothing of what was there, so it has none; nor does a
/// notebook cell's edit, which sends only its new source.
pub(crate) fn from_input(tool: &str, input: &Value) -> Option<Diff> {
    let path = path_of(input)?;
    let side = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let edits: Vec<(String, String)> = match tool {
        "Edit" | "replace" => vec![(side(input, "old_string"), side(input, "new_string"))],
        "MultiEdit" => input
            .get("edits")?
            .as_array()?
            .iter()
            .map(|e| (side(e, "old_string"), side(e, "new_string")))
            .collect(),
        "NotebookEdit" if input.get("edit_mode").and_then(Value::as_str) == Some("delete") => {
            return Some(single(path, Vec::new(), false));
        }
        "NotebookEdit" => vec![(String::new(), side(input, "new_source"))],
        _ => return None,
    };
    Some(rebuilt(path, &edits))
}

/// Antigravity's edits, from its own argument names: `replace_file_content` (one chunk),
/// `multi_replace_file_content` (its `ReplacementChunks`) and `write_to_file` (a whole file),
/// without line numbers. The names are read off agy 1.3.6 itself: no recorded session edits yet.
pub(crate) fn antigravity(tool: &str, args: &Value) -> Option<Diff> {
    let path = args.get("TargetFile")?.as_str()?.to_string();
    let side = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let chunk = |v: &Value| (side(v, "TargetContent"), side(v, "ReplacementContent"));
    let edits: Vec<(String, String)> = match tool {
        "replace_file_content" => vec![chunk(args)],
        "multi_replace_file_content" => {
            // The model may send the list as JSON text.
            let chunks = match args.get("ReplacementChunks")? {
                Value::String(text) => serde_json::from_str(text).ok()?,
                v => v.clone(),
            };
            chunks.as_array()?.iter().map(chunk).collect()
        }
        "write_to_file" => {
            let content = args.get("CodeContent")?.as_str()?;
            let lines = content.lines().map(|l| format!("+{l}")).collect();
            let hunk = Hunk {
                old_start: None,
                new_start: Some(1),
                lines,
            };
            return Some(single(path, vec![hunk], was_cut(content)));
        }
        _ => return None,
    };
    Some(rebuilt(path, &edits))
}

/// One file's edits, each rebuilt from its old and new text.
fn rebuilt(path: String, edits: &[(String, String)]) -> Diff {
    let cut = edits.iter().any(|(old, new)| was_cut(old) || was_cut(new));
    let hunks = edits.iter().map(|(old, new)| hunk(old, new)).collect();
    single(path, hunks, cut)
}

/// Codex's `apply_patch`: `*** Update File:` / `*** Add File:` / `*** Delete File:` sections (a
/// `*** Move to:` renames the one before), each `@@` a hunk, without line numbers.
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
        // The file's changes land on its new path.
        if let Some(to) = line.strip_prefix("*** Move to: ") {
            file.path = to.trim().to_string();
            continue;
        }
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
    fn a_notebook_cell_edit_names_its_notebook() {
        let input = json!({ "notebook_path": "/w/a.ipynb", "cell_id": "c1", "new_source": "x = 1\ny = 2" });
        let d = claude("NotebookEdit", &input, None).unwrap();
        assert_eq!(
            (d.files[0].path.as_str(), d.added(), d.removed()),
            ("/w/a.ipynb", 2, 0)
        );
        let gone = json!({ "notebook_path": "/w/a.ipynb", "cell_id": "c1", "edit_mode": "delete" });
        assert_eq!(
            claude("NotebookEdit", &gone, None).unwrap().files[0].path,
            "/w/a.ipynb"
        );
    }

    #[test]
    fn antigravity_edits_read_its_own_arguments() {
        let one = json!({ "TargetFile": "/w/a.rs", "TargetContent": "x", "ReplacementContent": "y\nz" });
        let d = antigravity("replace_file_content", &one).unwrap();
        assert_eq!(
            (d.files[0].path.as_str(), d.added(), d.removed()),
            ("/w/a.rs", 2, 1)
        );
        let chunks = json!([{ "TargetContent": "a", "ReplacementContent": "b" }, { "TargetContent": "c", "ReplacementContent": "" }]);
        for chunks in [chunks.clone(), Value::String(chunks.to_string())] {
            let many = json!({ "TargetFile": "/w/a.rs", "ReplacementChunks": chunks });
            let d = antigravity("multi_replace_file_content", &many).unwrap();
            assert_eq!((d.added(), d.removed()), (1, 2));
        }
        let new = json!({ "TargetFile": "/w/b.rs", "CodeContent": "one\ntwo", "Overwrite": false });
        assert_eq!(antigravity("write_to_file", &new).unwrap().added(), 2);
        assert!(antigravity("view_file", &json!({ "TargetFile": "/w/b.rs" })).is_none());
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
