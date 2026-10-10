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
        let repo = match gix::open(dir) {
            Ok(repo) => repo,
            Err(_) => gix::init(dir).map_err(err)?,
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
        let full = self.dir.join(&path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        std::fs::write(&full, text).map_err(err)?;
        let blob = self.repo.write_blob(text.as_bytes()).map_err(err)?.detach();
        let parent = self.repo.head_id().ok().map(|id| id.detach());
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
        // The index follows the commit: `git status` in the folder shows it clean, and a revert
        // by hand starts from the right place.
        let mut index = self.repo.index_from_tree(&tree).map_err(err)?;
        index.write(gix::index::write::Options::default()).map_err(err)?;
        Ok(())
    }

    /// The file at `path` as last committed; none when there is no such file.
    pub fn read(&self, path: &str) -> Result<Option<String>, Error> {
        let path = relative(path)?;
        match std::fs::read_to_string(self.dir.join(path)) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(err(e)),
        }
    }

    /// The history, newest first: each commit's message.
    pub fn log(&self) -> Result<Vec<String>, Error> {
        let Ok(head) = self.repo.head_id() else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for info in head.ancestors().all().map_err(err)? {
            let commit = info.map_err(err)?.object().map_err(err)?;
            let message: BString = commit.message_raw().map_err(err)?.into();
            out.push(message.to_string().trim_end().to_owned());
        }
        Ok(out)
    }
}

/// A path inside the memory: relative, with no `..` and no hidden part (`.git` is git's).
fn relative(path: &str) -> Result<String, Error> {
    let ok = !path.is_empty()
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != ".." && !part.starts_with('.'));
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
        for bad in ["/etc/passwd", "../x.md", "a/../../x", ".git/config", "", "a//b"] {
            assert!(m.write(bad, "x", "bad").is_err(), "{bad}");
        }
    }
}
