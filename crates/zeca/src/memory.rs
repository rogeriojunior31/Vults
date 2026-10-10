//! Zeca's memory: a git repository of Markdown files (plan-zeca S9; M1 fills it). Every write is a
//! commit, so what he learned can be read back in history and undone by a revert.

use std::path::{Path, PathBuf};

use gix::bstr::BString;

/// Who writes in the memory's history.
const NAME: &str = "Zeca";
const EMAIL: &str = "zeca@localhost";

#[derive(Debug)]
pub struct Memory {
    repo: gix::Repository,
    dir: PathBuf,
}

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Zeca's memory: {}", self.0)
    }
}

impl std::error::Error for Error {}

fn err(e: impl std::fmt::Display) -> Error {
    Error(e.to_string())
}

impl Memory {
    /// Opens the memory in `dir`, making it the first time; opening it again changes nothing.
    pub fn open(dir: &Path) -> Result<Self, Error> {
        std::fs::create_dir_all(dir).map_err(err)?;
        // Made only where there is none: a repository that fails to open says why.
        let repo = if dir.join(".git").exists() {
            gix::open(dir).map_err(err)?
        } else {
            gix::init(dir).map_err(err)?
        };
        Ok(Self {
            repo,
            dir: dir.to_path_buf(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Writes `text` to `path` (relative, inside the memory) and commits it with `message`.
    pub fn write(&mut self, path: &str, text: &str, message: &str) -> Result<(), Error> {
        let path = relative(path)?;
        let full = self.inside(&path)?;
        let parent = self.parent()?;
        let blob = self.repo.write_blob(text.as_bytes()).map_err(err)?.detach();
        let base = match parent {
            Some(id) => self
                .repo
                .find_commit(id)
                .map_err(err)?
                .tree_id()
                .map_err(err)?
                .detach(),
            None => self.repo.empty_tree().id,
        };
        let mut editor = self.repo.edit_tree(base).map_err(err)?;
        editor
            .upsert(path.as_str(), gix::object::tree::EntryKind::Blob, blob)
            .map_err(err)?;
        let tree = editor.write().map_err(err)?.detach();
        let now = gix::date::Time::now_local_or_utc();
        let mut time = gix::date::parse::TimeBuf::default();
        let me = gix::actor::SignatureRef {
            name: NAME.into(),
            email: EMAIL.into(),
            time: now.to_str(&mut time),
        };
        self.repo
            .commit_as(me, me, "HEAD", message, tree, parent)
            .map_err(err)?;
        // The file follows the commit, then the index: a crash before leaves the folder behind
        // HEAD (a checkout restores it), never ahead of it; `git status` shows it clean and a
        // revert by hand starts from the right place.
        if let Some(dir) = full.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        std::fs::write(&full, text).map_err(err)?;
        let mut index = self.repo.index_from_tree(&tree).map_err(err)?;
        index.write(gix::index::write::Options::default()).map_err(err)?;
        Ok(())
    }

    /// The last commit, or none in a memory with no commit yet. Any other trouble with HEAD is an
    /// error: taken for "no commit yet", it would start the history over and drop it.
    fn parent(&self) -> Result<Option<gix::ObjectId>, Error> {
        let head = self.repo.head().map_err(err)?;
        if head.is_unborn() {
            return Ok(None);
        }
        Ok(Some(self.repo.head_id().map_err(err)?.detach()))
    }

    /// `path` under the memory's folder, refusing any step of it that is a symbolic link: a write
    /// must never land outside the memory, whatever a model asked for.
    fn inside(&self, path: &str) -> Result<PathBuf, Error> {
        let mut at = self.dir.clone();
        for part in path.split('/') {
            at.push(part);
            match std::fs::symlink_metadata(&at) {
                Ok(m) if m.file_type().is_symlink() => {
                    return Err(Error(format!("{path:?} goes through a link")));
                }
                _ => {}
            }
        }
        Ok(at)
    }

    /// The file at `path` as last committed (not as the folder may hold it); none when there is
    /// no such file.
    pub fn read(&self, path: &str) -> Result<Option<String>, Error> {
        let path = relative(path)?;
        let Some(head) = self.parent()? else {
            return Ok(None);
        };
        let tree = self.repo.find_commit(head).map_err(err)?.tree().map_err(err)?;
        let Some(entry) = tree.lookup_entry_by_path(path.as_str()).map_err(err)? else {
            return Ok(None);
        };
        let blob = entry.object().map_err(err)?;
        String::from_utf8(blob.data.clone()).map(Some).map_err(err)
    }

    /// The history, newest first: each commit's message.
    pub fn log(&self) -> Result<Vec<String>, Error> {
        let Some(head) = self.parent()? else {
            return Ok(Vec::new());
        };
        let head = self.repo.find_commit(head).map_err(err)?.id();
        let mut out = Vec::new();
        for info in head.ancestors().all().map_err(err)? {
            let commit = info.map_err(err)?.object().map_err(err)?;
            let message: BString = commit.message_raw().map_err(err)?.into();
            out.push(message.to_string().trim_end().to_owned());
        }
        Ok(out)
    }
}

/// A path inside the memory: relative parts of letters, digits, `.`, `_` and `-`, none hidden
/// (`.git` is git's) or `..`. Nothing a shell, Windows (`\`, `:`) or a link could read otherwise.
fn relative(path: &str) -> Result<String, Error> {
    let tame = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    let ok = !path.is_empty()
        && path
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.') && part.chars().all(tame));
    if ok {
        Ok(path.to_owned())
    } else {
        Err(Error(format!("not a memory path: {path:?}")))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-zeca-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn opening_twice_is_the_same_memory() {
        let dir = temp("open");
        let mut m = Memory::open(&dir).unwrap();
        m.write("system/USER.md", "Works on Vults.", "remember the project")
            .unwrap();
        drop(m);
        let m = Memory::open(&dir).unwrap();
        assert_eq!(
            m.read("system/USER.md").unwrap().as_deref(),
            Some("Works on Vults.")
        );
        assert_eq!(m.log().unwrap(), ["remember the project"]);
    }

    #[test]
    fn every_write_is_a_commit() {
        let dir = temp("commits");
        let mut m = Memory::open(&dir).unwrap();
        m.write("system/USER.md", "one", "first").unwrap();
        m.write("roosts/vults.md", "two", "second").unwrap();
        m.write("system/USER.md", "three", "third").unwrap();
        assert_eq!(m.log().unwrap(), ["third", "second", "first"]);
        assert_eq!(m.read("roosts/vults.md").unwrap().as_deref(), Some("two"));
        assert_eq!(m.read("system/USER.md").unwrap().as_deref(), Some("three"));
        assert_eq!(m.read("nope.md").unwrap(), None);
        // git itself sees a clean tree, every file tracked.
        let status = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&dir)
            .output();
        if let Ok(out) = status {
            assert_eq!(String::from_utf8_lossy(&out.stdout), "", "git status");
        }
    }

    #[test]
    fn a_path_outside_the_memory_is_refused() {
        let mut m = Memory::open(&temp("paths")).unwrap();
        for bad in [
            "/etc/passwd",
            "../x.md",
            "a/../../x",
            ".git/config",
            "",
            "a//b",
            "a\\..\\x.md",
            "C:x.md",
            "a b.md",
        ] {
            assert!(m.write(bad, "x", "bad").is_err(), "{bad}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_link_inside_the_memory_never_leads_a_write_out() {
        let dir = temp("link");
        let outside = temp("link-outside");
        std::fs::create_dir_all(&outside).unwrap();
        let mut m = Memory::open(&dir).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("notes")).unwrap();
        assert!(m.write("notes/x.md", "x", "escape").is_err());
        assert!(!outside.join("x.md").exists());
        std::os::unix::fs::symlink(outside.join("y.md"), dir.join("y.md")).unwrap();
        assert!(m.write("y.md", "y", "escape").is_err());
        assert!(!outside.join("y.md").exists());
    }

    #[test]
    fn a_broken_head_is_an_error_never_a_new_history() {
        let dir = temp("head");
        let mut m = Memory::open(&dir).unwrap();
        m.write("a.md", "one", "first").unwrap();
        std::fs::write(dir.join(".git/HEAD"), "garbage").unwrap();
        let m2 = Memory::open(&dir);
        if let Ok(mut m2) = m2 {
            assert!(m2.write("b.md", "two", "after").is_err());
            assert!(m2.log().is_err());
        }
        drop(m);
    }

    #[test]
    fn read_is_what_was_committed_not_what_the_folder_holds() {
        let dir = temp("read");
        let mut m = Memory::open(&dir).unwrap();
        m.write("a.md", "kept", "first").unwrap();
        std::fs::write(dir.join("a.md"), "edited behind its back").unwrap();
        assert_eq!(m.read("a.md").unwrap().as_deref(), Some("kept"));
    }
}
