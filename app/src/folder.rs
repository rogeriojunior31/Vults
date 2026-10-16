//! What changed in a session's folder during a turn, besides the edits its agent told of: its
//! commands write files too (`sed -i`, `mv`, a generator). Two looks at the folder's repository, at
//! the turn's start and at its end, compared by each file's size and modification time.
//!
//! Git only lists the files, from its index and its ignore rules (`ls-files`): it compares no
//! content, so a filter, fsmonitor or hook the agent planted in the repository's config never runs
//! (`diff-files` would run a clean filter on a racy entry), and the index is never written.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

/// Files a look lists, at most: past it the repository is too big to stat twice a turn, and the
/// turn gets nothing from it.
const MAX_FILES: usize = 100_000;

/// How long git may take: a folder on a hung share gives the look up instead of a thread.
const GIT_TIMEOUT: Duration = Duration::from_secs(10);

/// The repository's files at one moment, by their path in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Look {
    top: PathBuf,
    /// Size and modification time; none for a file that is gone (a deleted tracked one).
    files: HashMap<String, Option<(u64, SystemTime)>>,
}

/// A look at the repository `cwd` is in; none outside one, or when git can't tell.
pub fn look(cwd: &Path) -> Option<Look> {
    let top = top(cwd)?;
    let out = git(
        &top,
        &["ls-files", "-z", "--cached", "--others", "--exclude-standard"],
    )?;
    let names: Vec<&str> = out.split('\0').filter(|n| !n.is_empty()).collect();
    if names.len() > MAX_FILES {
        return None;
    }
    let files = names
        .into_iter()
        .map(|name| {
            let meta = std::fs::symlink_metadata(top.join(name))
                .ok()
                .and_then(|m| Some((m.len(), m.modified().ok()?)));
            (name.to_string(), meta)
        })
        .collect();
    Some(Look { top, files })
}

/// The repository's top, spelled from `cwd`: git resolves symlinks (`/home` on some systems), and
/// the agent names its files under the folder it was given.
fn top(cwd: &Path) -> Option<PathBuf> {
    let prefix = git(cwd, &["rev-parse", "--show-prefix"])?;
    let up = Path::new(prefix.trim_end()).components().count();
    let mut top = cwd.to_path_buf();
    for _ in 0..up {
        if !top.pop() {
            return None;
        }
    }
    Some(top)
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
/// program, and at most [`GIT_TIMEOUT`]. Its output, or none when it fails.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "core.fsmonitor=false", "-c", "core.hooksPath=/dev/null"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Read on another thread: a full pipe would stop git before it exits.
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        stdout.read_to_end(&mut out).map(|_| out)
    });
    let deadline = Instant::now() + GIT_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let out = reader.join().ok()?.ok()?;
    status
        .success()
        .then(|| String::from_utf8_lossy(&out).into_owned())
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
        let at = |name: &str| after.top.join(name).to_string_lossy().into_owned();
        assert_eq!(changed(&before, &after), [at("kept.rs"), at("new.rs")]);
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
    fn a_planted_clean_filter_never_runs() {
        let dir = repo("filter");
        let mark = dir.join("ran");
        std::fs::write(dir.join(".gitattributes"), "* filter=x\n").unwrap();
        let clean = format!("sh -c 'touch {}; cat'", mark.display());
        let ok = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["config", "filter.x.clean", &clean])
            .status()
            .unwrap();
        assert!(ok.success());
        // A racy entry: the file is no older than the index, so a stat compare reads it.
        let now = std::time::SystemTime::now();
        for f in [dir.join("kept.rs"), dir.join(".git/index")] {
            std::fs::File::options()
                .append(true)
                .open(&f)
                .unwrap()
                .set_modified(now)
                .unwrap();
        }
        assert!(look(&dir).is_some());
        assert!(!mark.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_reached_through_a_symlink_keeps_its_spelling() {
        let dir = repo("link");
        let link = dir.with_file_name(format!("{}-to", dir.file_name().unwrap().to_string_lossy()));
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&dir, &link).unwrap();
        let before = look(&link).unwrap();
        std::fs::write(dir.join("kept.rs"), "changed\n").unwrap();
        let after = look(&link).unwrap();
        assert_eq!(changed(&before, &after), [format!("{}/kept.rs", link.display())]);
        let _ = std::fs::remove_file(&link);
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
