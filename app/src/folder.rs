//! What changed in a session's folder during a turn, besides the edits its agent told of: its
//! commands write files too (`sed -i`, `mv`, a generator). Two looks at the folder's repository, at
//! the turn's start and at its end, compared by each file's size and modification time.
//!
//! Git only lists the candidates, through commands that read no file's content (`diff-files`
//! compares the index's stat data, `ls-files` the ignore rules): a filter, fsmonitor or hook the
//! agent planted in the repository's config never runs, and the index is never written.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

/// Files a look lists, at most: past it the folder is too busy to tell (a build tree no ignore
/// rule covers), and the turn gets nothing from it.
const MAX_FILES: usize = 5_000;

/// The repository's changed and untracked files at one moment, by their path in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Look {
    top: PathBuf,
    /// Size and modification time; none for a file that is gone (a deleted tracked one).
    files: HashMap<String, Option<(u64, SystemTime)>>,
}

/// A look at the repository `cwd` is in; none outside one, or when git can't tell.
pub fn look(cwd: &Path) -> Option<Look> {
    let top = PathBuf::from(git(cwd, &["rev-parse", "--show-toplevel"])?.trim_end());
    let mut names = Vec::new();
    for args in [
        &["diff-files", "--name-only", "-z"][..],
        &["ls-files", "--others", "--exclude-standard", "-z"][..],
    ] {
        let out = git(&top, args)?;
        names.extend(out.split('\0').filter(|n| !n.is_empty()).map(str::to_string));
        if names.len() > MAX_FILES {
            return None;
        }
    }
    let files = names
        .into_iter()
        .map(|name| {
            let meta = std::fs::symlink_metadata(top.join(&name))
                .ok()
                .and_then(|m| Some((m.len(), m.modified().ok()?)));
            (name, meta)
        })
        .collect();
    Some(Look { top, files })
}

/// The absolute paths of the files that showed up or moved between the two looks, sorted; none
/// when the folder is in another repository since. A file left as it was, however dirty, is not
/// the turn's.
pub fn changed(before: &Look, after: &Look) -> Vec<String> {
    if before.top != after.top {
        return Vec::new();
    }
    let mut out: Vec<String> = after
        .files
        .iter()
        .filter(|(name, meta)| before.files.get(*name) != Some(meta))
        .map(|(name, _)| after.top.join(name).to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// Git in `dir`, reading only: no prompt, no lock, nothing of the repository's config that runs a
/// program. Its output, or none when it fails.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "core.fsmonitor=false", "-c", "core.hooksPath=/dev/null"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn repo(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-folder-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            let ok = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .unwrap();
            assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stderr));
        };
        run(&["init", "-q"]);
        std::fs::write(dir.join("kept.rs"), "a\n").unwrap();
        std::fs::write(dir.join("dirty.rs"), "a\n").unwrap();
        std::fs::write(dir.join(".gitignore"), "target/\n").unwrap();
        run(&["add", "."]);
        run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "init",
        ]);
        dir
    }

    #[test]
    fn a_turn_sees_what_its_commands_wrote_and_not_what_was_dirty() {
        let dir = repo("turn");
        std::fs::write(dir.join("dirty.rs"), "before the turn\n").unwrap();
        let before = look(&dir.join(".")).unwrap();
        std::fs::write(dir.join("kept.rs"), "a longer line\n").unwrap();
        std::fs::write(dir.join("new.rs"), "x\n").unwrap();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::write(dir.join("target/out.o"), "x\n").unwrap();
        let after = look(&dir).unwrap();
        let top = before.top.to_string_lossy().into_owned();
        assert_eq!(
            changed(&before, &after),
            [format!("{top}/kept.rs"), format!("{top}/new.rs")]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_planted_fsmonitor_never_runs() {
        let dir = repo("planted");
        let mark = dir.join("ran");
        let hook = format!("touch {}", mark.display());
        let ok = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["config", "core.fsmonitor", &hook])
            .status()
            .unwrap();
        assert!(ok.success());
        std::fs::write(dir.join("kept.rs"), "b\n").unwrap();
        assert!(look(&dir).is_some());
        assert!(!mark.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn outside_a_repository_there_is_no_look() {
        let dir = std::env::temp_dir().join(format!("vults-folder-{}-none", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(look(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
