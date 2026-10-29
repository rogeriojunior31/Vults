//! What changed in a session's folder during a turn, besides the edits its agent told of: its
//! commands write files too (`sed -i`, `mv`, a generator). Two looks at the folder's repository, at
//! the turn's start and at its end, compared by each file's size and modification time.
//!
//! Turn metadata only lists files from the index/ignore rules (`ls-files`). The on-demand
//! working-tree preview additionally reads raw Git objects and compares local text in-process.
//! Neither path runs worktree filters, fsmonitor or hooks, and the index is never written.

use std::collections::HashMap;
use std::io::{Read, Write};
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
    git_raw(dir, args, None, 32 * 1024 * 1024).map(|out| String::from_utf8_lossy(&out).into_owned())
}

fn git_raw(dir: &Path, args: &[&str], input: Option<Vec<u8>>, cap: usize) -> Option<Vec<u8>> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "core.fsmonitor=false", "-c", "core.hooksPath=/dev/null"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(input) = input {
        let mut stdin = child.stdin.take()?;
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    // Read on another thread: a full pipe would stop git before it exits.
    let stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        stdout.take(cap as u64 + 1).read_to_end(&mut out).map(|_| out)
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
    (status.success() && out.len() <= cap).then_some(out)
}

/// Current working-tree content against HEAD, including command/script edits and new files.
/// Raw Git objects only: never diff the worktree through Git (which can execute clean filters).
pub fn workspace_diff(cwd: &Path) -> Option<vults_core::Diff> {
    use std::collections::{BTreeMap, BTreeSet};
    use vults_core::{Diff, FileDiff, Hunk};
    const FILE_BYTES: usize = 256 * 1024;
    const TOTAL_BYTES: usize = 16 * 1024 * 1024;
    const LINES: usize = 16_000;
    let top = top(cwd)?.canonicalize().ok()?;
    let listed = git(
        &top,
        &["ls-files", "-z", "--cached", "--others", "--exclude-standard"],
    )?;
    let head = if git(&top, &["rev-parse", "--verify", "HEAD"]).is_some() {
        git(&top, &["ls-tree", "-rz", "--full-tree", "HEAD"])?
    } else {
        String::new()
    };
    let mut objects = BTreeMap::new();
    for entry in head.split('\0').filter(|e| !e.is_empty()) {
        let (meta, path) = entry.split_once('\t')?;
        let mut parts = meta.split_whitespace();
        let mode = parts.next()?;
        let kind = parts.next()?;
        let oid = parts.next()?;
        if kind == "blob" && mode != "120000" && oid.bytes().all(|b| b.is_ascii_hexdigit()) {
            objects.insert(path.to_string(), oid.to_string());
        }
    }
    let mut paths: BTreeSet<String> = listed
        .split('\0')
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .collect();
    paths.extend(objects.keys().cloned());
    if paths.len() > MAX_FILES {
        return None;
    }
    let ids: BTreeSet<_> = objects.values().cloned().collect();
    let mut eligible = BTreeSet::new();
    let mut budget = 0usize;
    if !ids.is_empty() {
        let input = ids
            .iter()
            .map(|s| format!("{s}\n"))
            .collect::<String>()
            .into_bytes();
        let sizes = git_raw(
            &top,
            &["cat-file", "--batch-check"],
            Some(input),
            16 * 1024 * 1024,
        )?;
        for line in std::str::from_utf8(&sizes).ok()?.lines() {
            let p: Vec<_> = line.split_whitespace().collect();
            if p.len() != 3 || p[1] != "blob" {
                return None;
            }
            let size: usize = p[2].parse().ok()?;
            if size <= FILE_BYTES && budget.saturating_add(size) <= TOTAL_BYTES {
                budget += size;
                eligible.insert(p[0].to_string());
            }
        }
    }
    let mut blobs = HashMap::new();
    if !eligible.is_empty() {
        let input = eligible
            .iter()
            .map(|s| format!("{s}\n"))
            .collect::<String>()
            .into_bytes();
        let raw = git_raw(
            &top,
            &["cat-file", "--batch"],
            Some(input),
            TOTAL_BYTES + 16 * 1024 * 1024,
        )?;
        let mut at = 0;
        while at < raw.len() {
            let end = at + raw[at..].iter().position(|b| *b == b'\n')?;
            let meta: Vec<_> = std::str::from_utf8(&raw[at..end])
                .ok()?
                .split_whitespace()
                .collect();
            if meta.len() != 3 || meta[1] != "blob" {
                return None;
            }
            let size: usize = meta[2].parse().ok()?;
            at = end + 1;
            let bytes = raw.get(at..at.checked_add(size)?)?;
            if !bytes.contains(&0)
                && let Ok(text) = std::str::from_utf8(bytes)
            {
                blobs.insert(meta[0].to_string(), text.to_string());
            }
            at = at.checked_add(size + 1)?;
        }
    }
    let mut result = Diff::default();
    let mut kept = 0usize;
    let mut read = 0usize;
    for path in paths {
        if Path::new(&path)
            .components()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
        {
            result.cut = true;
            continue;
        }
        let before = match objects.get(&path) {
            Some(oid) => match blobs.get(oid) {
                Some(s) => s.as_str(),
                None => {
                    result.cut = true;
                    continue;
                }
            },
            None => "",
        };
        let file = top.join(&path);
        let after = match std::fs::symlink_metadata(&file) {
            Ok(meta) => {
                if !meta.is_file()
                    || meta.len() > FILE_BYTES as u64
                    || !file.canonicalize().ok()?.starts_with(&top)
                {
                    result.cut = true;
                    continue;
                }
                read = read.saturating_add(meta.len() as usize);
                if read > TOTAL_BYTES {
                    result.cut = true;
                    break;
                }
                let raw = std::fs::read(&file).ok()?;
                if raw.contains(&0) {
                    result.cut = true;
                    continue;
                }
                match String::from_utf8(raw) {
                    Ok(s) => s,
                    Err(_) => {
                        result.cut = true;
                        continue;
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(_) => return None,
        };
        if before == after {
            continue;
        }
        let patch = vults_agent_config::unified(before, &after, "file");
        let mut f = FileDiff {
            path,
            added: 0,
            removed: 0,
            hunks: Vec::new(),
        };
        for line in patch.lines().skip(2) {
            if let Some(header) = line.strip_prefix("@@ -") {
                let (old, rest) = header.split_once(" +")?;
                let (new, _) = rest.split_once(" @@")?;
                let start = |s: &str| s.split(',').next()?.parse::<u32>().ok();
                f.hunks.push(Hunk {
                    old_start: start(old),
                    new_start: start(new),
                    lines: Vec::new(),
                });
            } else if !line.starts_with('\\')
                && let Some(h) = f.hunks.last_mut()
            {
                if line.starts_with('+') {
                    f.added += 1;
                }
                if line.starts_with('-') {
                    f.removed += 1;
                }
                if kept < LINES {
                    if line.chars().count() > 5000 {
                        result.cut = true;
                    }
                    h.lines.push(line.chars().take(5000).collect());
                    kept += 1;
                } else {
                    result.cut = true;
                }
            }
        }
        result.files.push(f);
        if kept >= LINES {
            result.cut = true;
            break;
        }
    }
    Some(result)
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
    fn workspace_preview_finds_script_edits_adds_and_deletes_without_running_filters() {
        let dir = repo("workspace-preview");
        let marker = dir.join("filter-ran");
        let filter = format!("sh -c 'touch {}; cat'", marker.display());
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["config", "filter.x.clean", &filter])
                .status()
                .unwrap()
                .success()
        );
        std::fs::write(dir.join(".gitattributes"), "* filter=x\n").unwrap();
        std::fs::write(dir.join("kept.rs"), "changed by a script\n").unwrap();
        std::fs::remove_file(dir.join("dirty.rs")).unwrap();
        std::fs::write(dir.join("new.rs"), "new file\n").unwrap();
        std::fs::write(dir.join("image.bin"), [0, 1, 2]).unwrap();
        let diff = workspace_diff(&dir).unwrap();
        assert!(!marker.exists(), "Raw object reads never invoke a clean filter");
        let kept = diff.files.iter().find(|f| f.path == "kept.rs").unwrap();
        assert_eq!((kept.added, kept.removed), (1, 1));
        assert!(kept.hunks[0].lines.iter().any(|l| l == "+changed by a script"));
        assert_eq!(
            diff.files.iter().find(|f| f.path == "dirty.rs").unwrap().removed,
            1
        );
        assert_eq!(diff.files.iter().find(|f| f.path == "new.rs").unwrap().added, 1);
        assert!(!diff.files.iter().any(|f| f.path == "image.bin"));
        assert!(diff.cut);
        let clean = repo("workspace-clean");
        assert!(workspace_diff(&clean).unwrap().files.is_empty());
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(clean);
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
