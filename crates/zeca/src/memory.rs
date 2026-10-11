//! Zeca's memory: a git repository of Markdown files (plan-zeca S9; M1 fills it). Every write is a
//! commit, so what he learned can be read back in history and undone by a revert. Each commit says
//! who (S15): the author wrote the text, the committer confirmed it, and `Proposed-by` /
//! `Accepted-by` trailers say it in words. [`Memory::log`] reads who from the identities, never
//! from the message, which a model may have written.

use std::path::{Path, PathBuf};

use gix::bstr::BString;

/// Who writes in the memory's history.
const ZECA: (&str, &str) = ("Zeca", "zeca@localhost");
const USER: (&str, &str) = ("User", "user@localhost");
const SLEEP: (&str, &str) = ("Zeca asleep", "sleep@localhost");

const PROPOSED: &str = "Proposed-by:";
const ACCEPTED: &str = "Accepted-by:";

/// Who a write comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    /// The user, editing by hand (in the Nest): never the folder directly, since `read` reads the
    /// commit.
    User,
    /// Zeca, on his own, or with the user's yes (then the user is the committer).
    Zeca { accepted_by_user: bool },
    /// Zeca's sleep, consolidating on its own.
    Sleep,
}

impl By {
    /// Author and committer.
    fn signatures(self) -> ((&'static str, &'static str), (&'static str, &'static str)) {
        match self {
            Self::User => (USER, USER),
            Self::Zeca {
                accepted_by_user: true,
            } => (ZECA, USER),
            Self::Zeca {
                accepted_by_user: false,
            } => (ZECA, ZECA),
            Self::Sleep => (SLEEP, SLEEP),
        }
    }

    /// Back from a commit's identities (name and email); none for one we did not write. A commit
    /// made by hand in the folder (by anyone who can write there, an agent included) could copy
    /// them: nothing short of signing tells them apart, and the app never commits as anyone else.
    fn from_signatures(author: (&str, &str), committer: (&str, &str)) -> Option<Self> {
        [
            Self::User,
            Self::Zeca {
                accepted_by_user: true,
            },
            Self::Zeca {
                accepted_by_user: false,
            },
            Self::Sleep,
        ]
        .into_iter()
        .find(|by| by.signatures() == (author, committer))
    }
}

/// One commit of the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The commit, in hex: what an undo names.
    pub id: String,
    /// When, in seconds since 1970.
    pub seconds: i64,
    /// The message, without its trailers.
    pub message: String,
    pub author: String,
    pub committer: String,
    /// Who, read from author and committer; none for a commit we did not write. The one field
    /// to trust: the trailers below are words.
    pub by: Option<By>,
    /// The trailers, only on a commit we wrote (none otherwise, whatever its message says).
    pub proposed_by: Option<String>,
    pub accepted_by: Option<String>,
}

impl Entry {
    /// Written without the user's yes through the app: Zeca on his own, his sleep, or a commit
    /// we did not write. The sleep's debrief (M8) lists these.
    pub fn unaccepted(&self) -> bool {
        !matches!(
            self.by,
            Some(
                By::User
                    | By::Zeca {
                        accepted_by_user: true
                    }
            )
        )
    }
}

#[derive(Debug)]
pub struct Memory {
    repo: gix::Repository,
    dir: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// The repository or the folder failed.
    Repo(String),
    /// The write was refused, for a reason the model can act on (M1).
    Refused(crate::facts::Refused),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Repo(e) => write!(f, "Zeca's memory: {e}"),
            Self::Refused(r) => write!(f, "Zeca's memory: {r}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<crate::facts::Refused> for Error {
    fn from(r: crate::facts::Refused) -> Self {
        Self::Refused(r)
    }
}

fn err(e: impl std::fmt::Display) -> Error {
    Error::Repo(e.to_string())
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

    /// Writes `text` to `path` (relative, inside the memory) and commits it with `message`, as
    /// `by`.
    pub fn write(&mut self, path: &str, text: &str, message: &str, by: By) -> Result<(), Error> {
        self.write_all(&[(path, text)], message, by)
    }

    /// Writes every `(path, text)` in one commit: a replaced fact and its past land together.
    pub fn write_all(&mut self, files: &[(&str, &str)], message: &str, by: By) -> Result<(), Error> {
        let files: Vec<(&str, Option<&str>)> = files.iter().map(|(p, t)| (*p, Some(*t))).collect();
        self.commit(&files, message, by)
    }

    /// One commit of these files: a text writes one, none removes it.
    fn commit(&mut self, files: &[(&str, Option<&str>)], message: &str, by: By) -> Result<(), Error> {
        if files.is_empty() {
            return Ok(());
        }
        let mut checked = Vec::with_capacity(files.len());
        for (path, text) in files {
            let path = relative(path)?;
            let full = self.inside(&path)?;
            checked.push((path, full, *text));
        }
        let parent = self.parent()?;
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
        for (path, _, text) in &checked {
            match text {
                Some(text) => {
                    let blob = self.repo.write_blob(text.as_bytes()).map_err(err)?.detach();
                    editor
                        .upsert(path.as_str(), gix::object::tree::EntryKind::Blob, blob)
                        .map_err(err)?;
                }
                None => {
                    editor.remove(path.as_str()).map_err(err)?;
                }
            }
        }
        let tree = editor.write().map_err(err)?.detach();
        let now = gix::date::Time::now_local_or_utc();
        let mut time = gix::date::parse::TimeBuf::default();
        let time = now.to_str(&mut time);
        let ((an, ae), (cn, ce)) = by.signatures();
        let author = gix::actor::SignatureRef {
            name: an.into(),
            email: ae.into(),
            time,
        };
        let committer = gix::actor::SignatureRef {
            name: cn.into(),
            email: ce.into(),
            time,
        };
        self.repo
            .commit_as(
                committer,
                author,
                "HEAD",
                with_trailers(message, by),
                tree,
                parent,
            )
            .map_err(err)?;
        // The file follows the commit, then the index: a crash before leaves the folder behind
        // HEAD (a checkout restores it), never ahead of it; `git status` shows it clean and a
        // revert by hand starts from the right place.
        for (_, full, text) in &checked {
            match text {
                Some(text) => {
                    if let Some(dir) = full.parent() {
                        std::fs::create_dir_all(dir).map_err(err)?;
                    }
                    std::fs::write(full, text).map_err(err)?;
                }
                None => match std::fs::remove_file(full) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(err(e)),
                    _ => {}
                },
            }
        }
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
                    return Err(crate::facts::Refused::Path(format!("{path:?} goes through a link")).into());
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

    /// The files directly in `dir` as last committed, by name; none when there is no such folder.
    pub fn list(&self, dir: &str) -> Result<Vec<String>, Error> {
        let dir = relative(dir)?;
        let Some(head) = self.parent()? else {
            return Ok(Vec::new());
        };
        let tree = self.repo.find_commit(head).map_err(err)?.tree().map_err(err)?;
        let Some(entry) = tree.lookup_entry_by_path(dir.as_str()).map_err(err)? else {
            return Ok(Vec::new());
        };
        if !entry.mode().is_tree() {
            return Ok(Vec::new());
        }
        let sub = entry.object().map_err(err)?.into_tree();
        let mut names = Vec::new();
        for e in sub.iter() {
            let e = e.map_err(err)?;
            if e.mode().is_blob() {
                names.push(e.filename().to_string());
            }
        }
        Ok(names)
    }

    /// Every file of a commit's tree, by path, with its blob.
    fn files_of(
        &self,
        tree: gix::Tree<'_>,
    ) -> Result<std::collections::BTreeMap<String, gix::ObjectId>, Error> {
        let mut recorder = gix::traverse::tree::Recorder::default();
        tree.traverse().breadthfirst(&mut recorder).map_err(err)?;
        Ok(recorder
            .records
            .into_iter()
            .filter(|e| e.mode.is_blob())
            .map(|e| (e.filepath.to_string(), e.oid))
            .collect())
    }

    /// Undoes the commit `id`: each file it changed goes back to what it was before, in a new
    /// commit by `by`. Refused when a later commit changed one of those files again: undoing it
    /// would throw that later change away too.
    pub fn revert(&mut self, id: &str, by: By) -> Result<(), Error> {
        let oid = gix::ObjectId::from_hex(id.as_bytes())
            .map_err(|_| Error::Refused(crate::facts::Refused::Path(format!("not a commit: {id:?}"))))?;
        let commit = self.repo.find_commit(oid).map_err(err)?;
        let message = commit.message_raw().map_err(err)?.to_string();
        let subject = message.lines().next().unwrap_or_default().to_string();
        let after = self.files_of(commit.tree().map_err(err)?)?;
        let before = match commit.parent_ids().next() {
            Some(p) => self.files_of(
                self.repo
                    .find_commit(p.detach())
                    .map_err(err)?
                    .tree()
                    .map_err(err)?,
            )?,
            None => Default::default(),
        };
        drop(commit);
        let Some(head) = self.parent()? else {
            return Ok(());
        };
        let now = self.files_of(self.repo.find_commit(head).map_err(err)?.tree().map_err(err)?)?;
        let mut paths: Vec<&String> = after.keys().chain(before.keys()).collect();
        paths.sort();
        paths.dedup();
        let mut changes: Vec<(String, Option<String>)> = Vec::new();
        for path in paths {
            if after.get(path) == before.get(path) {
                continue;
            }
            if now.get(path) != after.get(path) {
                return Err(Error::Refused(crate::facts::Refused::Path(format!(
                    "{path} changed after that, undo the later change first"
                ))));
            }
            let text = match before.get(path) {
                Some(blob) => {
                    let data = self.repo.find_object(*blob).map_err(err)?.data.clone();
                    Some(String::from_utf8(data).map_err(err)?)
                }
                None => None,
            };
            changes.push((path.clone(), text));
        }
        let files: Vec<(&str, Option<&str>)> =
            changes.iter().map(|(p, t)| (p.as_str(), t.as_deref())).collect();
        self.commit(&files, &format!("Undo: {subject}"), by)
    }

    /// The history, newest first.
    pub fn log(&self) -> Result<Vec<Entry>, Error> {
        let Some(head) = self.parent()? else {
            return Ok(Vec::new());
        };
        let head = self.repo.find_commit(head).map_err(err)?.id();
        let mut out = Vec::new();
        for info in head.ancestors().all().map_err(err)? {
            let commit = info.map_err(err)?.object().map_err(err)?;
            let raw: BString = commit.message_raw().map_err(err)?.into();
            let who = |s: gix::actor::SignatureRef<'_>| (s.name.to_string(), s.email.to_string());
            let author = who(commit.author().map_err(err)?);
            let committer = who(commit.committer().map_err(err)?);
            let mut e = entry(&raw.to_string(), author, committer);
            e.id = commit.id.to_string();
            e.seconds = commit.time().map(|t| t.seconds).unwrap_or(0);
            out.push(e);
        }
        Ok(out)
    }
}

/// The message, its trailer lines taken out wherever they were (a model writing the message must
/// not claim a yes; a user's line that starts the same goes too), and its `---` lines (after one,
/// git reads the rest as a patch and hides the true trailers), then the trailers that are true.
/// An empty message gets a subject: git takes a lone first paragraph for one, never for trailers.
fn with_trailers(message: &str, by: By) -> String {
    let body: Vec<&str> = message
        .trim_end()
        .lines()
        .filter(|l| !is_trailer(l) && !hides_trailers(l))
        .collect();
    let mut out = body.join("\n").trim().to_owned();
    if out.is_empty() {
        out = "Memory updated".to_owned();
    }
    let sig = |(name, email): (&str, &str)| format!("{name} <{email}>");
    let trailers: Vec<String> = match by {
        By::User => Vec::new(),
        By::Zeca { accepted_by_user } => {
            let mut t = vec![format!("{PROPOSED} {}", sig(ZECA))];
            if accepted_by_user {
                t.push(format!("{ACCEPTED} {}", sig(USER)));
            }
            t
        }
        By::Sleep => vec![format!("{PROPOSED} {}", sig(SLEEP))],
    };
    if !trailers.is_empty() {
        out.push_str("\n\n");
        out.push_str(&trailers.join("\n"));
    }
    out
}

/// A line after which git stops reading trailers: a patch's `---`, or a scissors line (`# >8`).
fn hides_trailers(line: &str) -> bool {
    let l = line.trim_start();
    l.starts_with("---") || (l.starts_with('#') && l.contains(">8"))
}

/// A line git could read as one of our trailers: the key before the first colon, trimmed, in any
/// case (`Accepted-by :` counts, as it does for git).
fn is_trailer(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        let key = key.trim();
        [PROPOSED, ACCEPTED]
            .iter()
            .any(|k| key.eq_ignore_ascii_case(k.trim_end_matches(':')))
    })
}

fn entry(raw: &str, author: (String, String), committer: (String, String)) -> Entry {
    let by = By::from_signatures((&author.0, &author.1), (&committer.0, &committer.1));
    let value = |key: &str| {
        by.and_then(|_| {
            raw.lines()
                .rev()
                .find_map(|l| l.strip_prefix(key).map(|v| v.trim().to_owned()))
        })
    };
    let message = raw
        .lines()
        .filter(|l| !is_trailer(l))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_owned();
    Entry {
        id: String::new(),
        seconds: 0,
        message,
        by,
        proposed_by: value(PROPOSED),
        accepted_by: value(ACCEPTED),
        author: author.0,
        committer: committer.0,
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
        Err(crate::facts::Refused::Path(format!("not a memory path: {path:?}")).into())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn messages(m: &Memory) -> Vec<String> {
        m.log().unwrap().into_iter().map(|e| e.message).collect()
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-zeca-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn opening_twice_is_the_same_memory() {
        let dir = temp("open");
        let mut m = Memory::open(&dir).unwrap();
        m.write(
            "system/USER.md",
            "Works on Vults.",
            "remember the project",
            By::User,
        )
        .unwrap();
        drop(m);
        let m = Memory::open(&dir).unwrap();
        assert_eq!(
            m.read("system/USER.md").unwrap().as_deref(),
            Some("Works on Vults.")
        );
        assert_eq!(messages(&m), ["remember the project"]);
    }

    #[test]
    fn every_write_is_a_commit() {
        let dir = temp("commits");
        let mut m = Memory::open(&dir).unwrap();
        m.write("system/USER.md", "one", "first", By::User).unwrap();
        m.write("roosts/vults.md", "two", "second", By::User).unwrap();
        m.write("system/USER.md", "three", "third", By::User).unwrap();
        assert_eq!(messages(&m), ["third", "second", "first"]);
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
            assert!(m.write(bad, "x", "bad", By::User).is_err(), "{bad}");
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
        assert!(m.write("notes/x.md", "x", "escape", By::User).is_err());
        assert!(!outside.join("x.md").exists());
        std::os::unix::fs::symlink(outside.join("y.md"), dir.join("y.md")).unwrap();
        assert!(m.write("y.md", "y", "escape", By::User).is_err());
        assert!(!outside.join("y.md").exists());
    }

    #[test]
    fn a_broken_head_is_an_error_never_a_new_history() {
        let dir = temp("head");
        let mut m = Memory::open(&dir).unwrap();
        m.write("a.md", "one", "first", By::User).unwrap();
        std::fs::write(dir.join(".git/HEAD"), "garbage").unwrap();
        let m2 = Memory::open(&dir);
        if let Ok(mut m2) = m2 {
            assert!(m2.write("b.md", "two", "after", By::User).is_err());
            assert!(m2.log().is_err());
        }
        drop(m);
    }

    #[test]
    fn read_is_what_was_committed_not_what_the_folder_holds() {
        let dir = temp("read");
        let mut m = Memory::open(&dir).unwrap();
        m.write("a.md", "kept", "first", By::User).unwrap();
        std::fs::write(dir.join("a.md"), "edited behind its back").unwrap();
        assert_eq!(m.read("a.md").unwrap().as_deref(), Some("kept"));
    }

    #[test]
    fn each_write_says_who() {
        let dir = temp("who");
        let mut m = Memory::open(&dir).unwrap();
        m.write("a.md", "1", "by hand", By::User).unwrap();
        m.write(
            "a.md",
            "2",
            "proposed and taken",
            By::Zeca {
                accepted_by_user: true,
            },
        )
        .unwrap();
        m.write(
            "a.md",
            "3",
            "on his own",
            By::Zeca {
                accepted_by_user: false,
            },
        )
        .unwrap();
        m.write("a.md", "4", "while asleep", By::Sleep).unwrap();
        let log = m.log().unwrap();
        let who: Vec<_> = log
            .iter()
            .map(|e| (e.message.as_str(), e.author.as_str(), e.committer.as_str(), e.by))
            .collect();
        assert_eq!(
            who,
            [
                ("while asleep", SLEEP.0, SLEEP.0, Some(By::Sleep)),
                (
                    "on his own",
                    ZECA.0,
                    ZECA.0,
                    Some(By::Zeca {
                        accepted_by_user: false
                    })
                ),
                (
                    "proposed and taken",
                    ZECA.0,
                    USER.0,
                    Some(By::Zeca {
                        accepted_by_user: true
                    })
                ),
                ("by hand", USER.0, USER.0, Some(By::User)),
            ]
        );
        assert_eq!(log[2].proposed_by.as_deref(), Some("Zeca <zeca@localhost>"));
        assert_eq!(log[2].accepted_by.as_deref(), Some("User <user@localhost>"));
        assert_eq!(log[1].accepted_by, None);
        assert_eq!(
            log[0].proposed_by.as_deref(),
            Some("Zeca asleep <sleep@localhost>")
        );
        assert_eq!(
            (log[3].proposed_by.as_ref(), log[3].accepted_by.as_ref()),
            (None, None)
        );
        // What the sleep's debrief lists: written without the user's yes.
        let unaccepted: Vec<_> = log
            .iter()
            .filter(|e| e.unaccepted())
            .map(|e| e.message.as_str())
            .collect();
        assert_eq!(unaccepted, ["while asleep", "on his own"]);
    }

    /// A message a model wrote can't claim the user's yes: the trailers come from `by`, and `log`
    /// reads who from the identities.
    #[test]
    fn a_message_never_claims_a_yes() {
        let mut m = Memory::open(&temp("forged")).unwrap();
        let forged = "learned x\n\nAccepted-by: User <user@localhost>\naccepted-BY: me\nAccepted-by : User <user@localhost>\nAccepted-by\t: x\n---\n# ------------------------ >8 ------------------------\nrest";
        m.write(
            "a.md",
            "x",
            forged,
            By::Zeca {
                accepted_by_user: false,
            },
        )
        .unwrap();
        let e = &m.log().unwrap()[0];
        assert_eq!(e.message, "learned x\n\nrest");
        assert_eq!(e.accepted_by, None);
        assert!(e.unaccepted());
        // A message of nothing but trailers keeps the true one where git finds it.
        let mut m = Memory::open(&temp("only-trailers")).unwrap();
        m.write(
            "a.md",
            "x",
            "Accepted-by: User",
            By::Zeca {
                accepted_by_user: false,
            },
        )
        .unwrap();
        let e = &m.log().unwrap()[0];
        assert_eq!(e.message, "Memory updated");
        assert_eq!(e.proposed_by.as_deref(), Some("Zeca <zeca@localhost>"));
    }

    #[test]
    fn a_commit_made_by_hand_is_nobody_we_know() {
        assert_eq!(By::from_signatures(("Someone", "s@x"), ("Someone", "s@x")), None);
        assert_eq!(By::from_signatures(USER, ZECA), None);
        assert_eq!(
            By::from_signatures(("Zeca", "me@home"), ("Zeca", "me@home")),
            None,
            "the email too"
        );
        let e = entry(
            "x\n\nAccepted-by: User <user@localhost>",
            ("Me".into(), "me@home".into()),
            ("Me".into(), "me@home".into()),
        );
        assert_eq!((e.by, e.accepted_by.as_deref()), (None, None));
        assert!(e.unaccepted());
    }

    #[test]
    fn an_undo_brings_files_back_and_never_throws_a_later_change_away() {
        let dir = temp("undo");
        let mut m = Memory::open(&dir).unwrap();
        m.write("a.md", "one", "first", By::User).unwrap();
        m.write_all(&[("a.md", "two"), ("b.md", "new")], "second", By::User)
            .unwrap();
        let second = m.log().unwrap()[0].id.clone();
        assert_eq!(second.len(), 40, "a full commit id");
        m.revert(&second, By::User).unwrap();
        assert_eq!(m.read("a.md").unwrap().as_deref(), Some("one"));
        assert_eq!(m.read("b.md").unwrap(), None, "a file the commit added goes");
        assert!(!dir.join("b.md").exists());
        assert_eq!(m.log().unwrap()[0].message, "Undo: second");
        // A later change to the same file: the old commit can't be undone over it.
        m.write("a.md", "three", "third", By::User).unwrap();
        let first = m.log().unwrap().last().unwrap().id.clone();
        assert!(m.revert(&first, By::User).is_err());
        assert_eq!(m.read("a.md").unwrap().as_deref(), Some("three"));
        assert!(m.revert("nothex", By::User).is_err());
        // git sees a clean tree after it all.
        if let Ok(out) = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&dir)
            .output()
        {
            assert_eq!(String::from_utf8_lossy(&out.stdout), "");
        }
    }
}
