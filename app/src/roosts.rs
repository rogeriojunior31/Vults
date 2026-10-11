//! Roosts (`docs/dev/plan-zeca.md` M3): the sessions grouped by the repository they work in, each
//! with its branch, the GitHub card's pull requests and checks, and its project memory. A roost's
//! key is the repository (`owner/repo` from GitHub's remote, or the folder's name): the worktrees of
//! one repository are one roost, and its memory is `roosts/<key>.md`.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use tauri::AppHandle;
use vults_zeca::facts::File;
use vults_zeca::memory::Memory;

use crate::flock_tools::Source;

/// A folder's repository, read from its `.git` on this computer (no network). Everything in it is
/// the folder's own claim (an agent can write `.git/config`): good for grouping and display, never
/// proof of which repository it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// The working tree's top folder.
    pub root: PathBuf,
    /// The repository's shared git dir: the worktrees of one repository have the same.
    pub common: PathBuf,
    /// The `origin` remote as (host, path), lowercased host, path without `.git`.
    pub origin: Option<(String, String)>,
    /// The checked-out branch; none on a detached HEAD (or one we can't read).
    pub branch: Option<String>,
}

impl Repo {
    /// `owner/repo` when `origin` is on GitHub.
    pub fn github(&self) -> Option<String> {
        let (host, path) = self.origin.as_ref()?;
        let tame = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        let (owner, repo) = path.split_once('/')?;
        (matches!(host.as_str(), "github.com" | "www.github.com") && tame(owner) && tame(repo))
            .then(|| path.clone())
    }

    /// The roost's key, one namespace per kind so no two kinds meet: the GitHub repository, the
    /// remote elsewhere, or (with no remote) the repository's own git dir on this computer.
    pub fn key(&self) -> String {
        match (&self.github(), &self.origin) {
            (Some(gh), _) => format!("github:{}", gh.to_lowercase()),
            (None, Some((host, path))) => format!("git:{host}/{}", path.to_lowercase()),
            (None, None) => format!("local:{}", self.common.display()),
        }
    }

    /// What the Nest calls it.
    pub fn name(&self) -> String {
        if let Some(gh) = self.github() {
            return gh;
        }
        let last = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
        match &self.origin {
            Some((_, path)) => last(path),
            None => self
                .root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }

    /// The roost's memory file: its name, readable, and a hash of its key, so names that slug
    /// alike (`a-b/c`, `a/b-c`, a folder named `owner-repo`) never share a memory.
    pub fn memory(&self) -> Option<File> {
        let key = self.key();
        let hash = key.bytes().fold(0x811c_9dc5_u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        });
        let name: String = self.name().chars().take(40).collect();
        File::roost(&format!("{name} {hash:08x}")).ok()
    }
}

/// The repository `dir` is in, walking up to its `.git` (a folder, or a worktree's file naming
/// its git dir), never as far as the home folder: a dotfiles repository there is not a project, and
/// the chat's own folder lives under it.
pub fn repo_of(dir: &Path) -> Option<Repo> {
    repo_within(dir, &crate::paths::home())
}

fn repo_within(dir: &Path, stop: &Path) -> Option<Repo> {
    let mut at = Some(dir);
    while let Some(d) = at {
        if stop.starts_with(d) {
            return None;
        }
        if let Some(git) = git_dir(&d.join(".git")) {
            // A worktree's own HEAD; the remotes are in the shared dir (`commondir`).
            let common = small(&git.join("commondir"))
                .map(|c| git.join(c.trim()))
                .unwrap_or_else(|| git.clone());
            let common = common.canonicalize().unwrap_or(common);
            let branch = small(&git.join("HEAD"))
                .and_then(|h| h.trim().strip_prefix("ref: refs/heads/").map(str::to_string))
                // The reftable format leaves this stub in HEAD: the branch is elsewhere.
                .filter(|b| b != ".invalid");
            let origin = small(&common.join("config"))
                .and_then(|c| origin(&c))
                .and_then(|url| host_path(&url));
            return Some(Repo {
                root: d.to_path_buf(),
                common,
                origin,
                branch,
            });
        }
        at = d.parent();
    }
    None
}

/// Most a file under `.git` is read for: a HEAD, a pointer or a config is far smaller.
const SMALL: u64 = 64 * 1024;

/// A small regular file's text. A FIFO, a device (`/dev/zero` through a link) or anything large
/// planted in `.git` must not hang or flood the app.
fn small(path: &Path) -> Option<String> {
    use std::io::Read;
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > SMALL {
        return None;
    }
    let mut text = String::new();
    std::fs::File::open(path)
        .ok()?
        .take(SMALL)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}

fn git_dir(dot: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(dot).ok()?;
    if meta.is_dir() {
        return Some(dot.to_path_buf());
    }
    let text = small(dot)?;
    let path = PathBuf::from(text.trim().strip_prefix("gitdir:")?.trim());
    let path = if path.is_relative() {
        dot.parent()?.join(path)
    } else {
        path
    };
    path.is_dir().then_some(path)
}

/// A config line with its comment cut (`;` or `#` outside quotes) and its quotes taken off.
fn uncomment(line: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            ';' | '#' if !quoted => break,
            c => out.push(c),
        }
    }
    out.trim().to_string()
}

/// The `origin` remote's URL from a git config. Section and key names ignore case, as git's do;
/// the remote's name does not. (`insteadOf` and `include` are not followed.)
fn origin(config: &str) -> Option<String> {
    let mut inside = false;
    for raw in config.lines() {
        let trimmed = raw.trim();
        if let Some(head) = trimmed.strip_prefix('[') {
            let head = head.split(']').next().unwrap_or_default();
            let (section, sub) = head.split_once(char::is_whitespace).unwrap_or((head, ""));
            inside = section.eq_ignore_ascii_case("remote") && sub.trim() == "\"origin\"";
            continue;
        }
        let line = uncomment(trimmed);
        if inside
            && let Some((k, v)) = line.split_once('=')
            && k.trim().eq_ignore_ascii_case("url")
        {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// (host, path) from a remote URL: `scheme://[user@]host[:port]/path` or `[user@]host:path`. The
/// host is matched whole, so `notgithub.com` or a path holding `github.com/` is not GitHub.
fn host_path(url: &str) -> Option<(String, String)> {
    let (host, path) = match url.split_once("://") {
        Some((_, rest)) => {
            let (authority, path) = rest.split_once('/')?;
            let host = authority.rsplit('@').next()?;
            let host = host.split(':').next()?;
            (host, path)
        }
        None => {
            let (authority, path) = url.split_once(':')?;
            // A local path (`/srv/repo`, `C:\repo`) is not a host.
            if authority.contains('/') || authority.len() < 2 {
                return None;
            }
            (authority.rsplit('@').next()?, path)
        }
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    (!host.is_empty() && !path.is_empty()).then(|| (host.to_lowercase(), path.to_string()))
}

/// One roost, as the Nest shows it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Roost {
    pub key: String,
    /// What the Nest calls it.
    pub name: String,
    pub github: Option<String>,
    /// Its working trees with a session in them, each on its branch.
    pub trees: Vec<Tree>,
    pub sessions: Vec<RoostSession>,
    /// The user's open pull requests in it, from the GitHub card.
    pub prs: Vec<Pr>,
    /// The default branch's checks, from the GitHub card.
    pub checks: Option<String>,
    /// Its project memory's facts.
    pub memory: Vec<String>,
    /// Whether Claude Code reads the `AGENTS.md` Codex reads (M9): `shared`, `offer` (an
    /// `AGENTS.md` its `CLAUDE.md` does not import), or empty (no `AGENTS.md`).
    pub agents: String,
    /// What its agents cost, today and in all (W9); none before anything was spent here.
    pub cost: Option<Cost>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    pub today: f64,
    pub total: f64,
    /// Some of it was on a subscription: "at API prices".
    pub subscription: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tree {
    pub path: String,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoostSession {
    pub agent: String,
    pub status: String,
    pub step: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pr {
    pub name: String,
    pub title: String,
    pub checks: Option<String>,
    pub review: Option<String>,
    pub url: Option<String>,
}

/// The roosts of the view's sessions, each session's repository found by `repo` (the file system,
/// or a test's), and each roost's memory by `memory`. A session outside any repository has no
/// roost. Roosts keep the order their first session has on the island.
pub fn group(
    view: &Value,
    repo: impl Fn(&Path) -> Option<Repo>,
    memory: impl Fn(&File) -> Vec<String>,
) -> Vec<Roost> {
    let text = |v: &Value| v.as_str().map(str::to_string);
    let mut roosts: Vec<Roost> = Vec::new();
    for s in view["sessions"].as_array().into_iter().flatten() {
        let Some(cwd) = s["cwd"].as_str() else {
            continue;
        };
        let Some(r) = repo(Path::new(cwd)) else {
            continue;
        };
        let key = r.key();
        let at = match roosts.iter().position(|x| x.key == key) {
            Some(i) => i,
            None => {
                roosts.push(Roost {
                    key: key.clone(),
                    name: r.name(),
                    github: r.github(),
                    trees: Vec::new(),
                    sessions: Vec::new(),
                    prs: Vec::new(),
                    checks: None,
                    memory: r.memory().map(|f| memory(&f)).unwrap_or_default(),
                    agents: sharing(&r.root).into(),
                    cost: None,
                });
                roosts.len() - 1
            }
        };
        let roost = &mut roosts[at];
        let path = crate::paths::shown(&r.root);
        if !roost.trees.iter().any(|t| t.path == path) {
            roost.trees.push(Tree {
                path,
                branch: r.branch.clone(),
            });
        }
        roost.sessions.push(RoostSession {
            agent: text(&s["agent_name"])
                .or_else(|| text(&s["agent"]))
                .unwrap_or_default(),
            status: text(&s["status"]).unwrap_or_default(),
            step: s["step"].as_str().map(vults_core::redact::secrets),
        });
    }
    let rows = view["boards"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["connector"] == "github")
        .flat_map(|b| b["rows"].as_array().cloned().unwrap_or_default());
    for row in rows {
        let item = row["item"].as_str().unwrap_or_default();
        // `pr:owner/repo#12` and `branch:owner/repo`; reviews asked of the user are other people's
        // work, not this roost's.
        let (repo, pr) = if let Some(rest) = item.strip_prefix("pr:") {
            (rest.split_once('#').map_or(rest, |(r, _)| r), true)
        } else if let Some(rest) = item.strip_prefix("branch:") {
            (rest, false)
        } else {
            continue;
        };
        let Some(roost) = roosts
            .iter_mut()
            .find(|r| r.github.as_deref().is_some_and(|g| g.eq_ignore_ascii_case(repo)))
        else {
            continue;
        };
        if pr {
            roost.prs.push(Pr {
                name: text(&row["name"]).unwrap_or_default(),
                title: text(&row["title"]).unwrap_or_default(),
                checks: text(&row["checks"]),
                review: text(&row["review"]),
                url: text(&row["url"]),
            });
        } else {
            roost.checks = text(&row["checks"]);
        }
    }
    roosts
}

/// The two `CLAUDE.md` Claude Code reads in `root`, with the import each needs: the top one imports
/// `AGENTS.md` beside it, `.claude/CLAUDE.md` the one a folder up.
fn claude_files(root: &Path) -> [(PathBuf, &'static str); 2] {
    [
        (root.join("CLAUDE.md"), "AGENTS.md"),
        (root.join(".claude").join("CLAUDE.md"), "../AGENTS.md"),
    ]
}

/// The one to edit: the top one, unless only `.claude/CLAUDE.md` exists.
fn claude_md(root: &Path) -> (PathBuf, &'static str) {
    let [top, inner] = claude_files(root);
    if !top.0.exists() && inner.0.exists() {
        inner
    } else {
        top
    }
}

/// Most of a `CLAUDE.md` read or edited: past this it is not one a person wrote by hand.
const MAX_CLAUDE_MD: u64 = 1024 * 1024;

/// A `CLAUDE.md` Vults may read and edit, whole: missing (empty), or a regular file of a sane size
/// that is not a link out of the repository, not a hard link, and not reached through a linked
/// folder. A FIFO planted there must not hang the app; a link must not make the diff name one file
/// and the write change another.
fn editable(path: &Path, root: &Path) -> Result<String, String> {
    let refuse = || Err(format!("{} is not a file Vults edits", path.display()));
    let meta = match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // A new file: its folder must be the repository's own, not a link elsewhere.
            let dir = path.parent().unwrap_or(root);
            if dir.exists() && !inside(dir, root) {
                return refuse();
            }
            return Ok(String::new());
        }
        Err(e) => return Err(e.to_string()),
        Ok(m) => m,
    };
    #[cfg(unix)]
    let hard_linked = {
        use std::os::unix::fs::MetadataExt;
        meta.nlink() > 1
    };
    #[cfg(not(unix))]
    let hard_linked = false;
    if !meta.is_file() || meta.len() > MAX_CLAUDE_MD || hard_linked || !inside(path, root) {
        return refuse();
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

/// Does `path`, every link followed, stay under `root`?
fn inside(path: &Path, root: &Path) -> bool {
    match (std::fs::canonicalize(path), std::fs::canonicalize(root)) {
        (Ok(p), Ok(r)) => p.starts_with(r),
        _ => false,
    }
}

/// Does this `CLAUDE.md` import `target` (`@AGENTS.md`, or `@./AGENTS.md`, anywhere in a line, as
/// Claude Code reads it), outside a fenced code block, where it reads no import?
fn imports(text: &str, target: &str) -> bool {
    let wanted = [format!("@{target}"), format!("@./{target}")];
    let mut fenced = false;
    for line in text.trim_start_matches('\u{feff}').lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if !fenced
            && line.split_whitespace().any(|w| {
                wanted
                    .iter()
                    .any(|x| w.trim_end_matches(['.', ',', ';', ')']) == x)
            })
        {
            return true;
        }
    }
    false
}

/// Where the repository at `root` stands: `shared`, `offer` or empty (see [`Roost::agents`]).
fn sharing(root: &Path) -> &'static str {
    if !root.join("AGENTS.md").is_file() {
        return "";
    }
    // Claude Code reads both files: either one importing is enough.
    let shared = claude_files(root)
        .iter()
        .any(|(path, target)| editable(path, root).is_ok_and(|t| imports(&t, target)));
    if shared { "shared" } else { "offer" }
}

/// `text` with the import of `target` added at its end, after a blank line, in the file's own line
/// ends; the line alone for a new file.
fn with_import(text: &str, target: &str) -> String {
    let line = format!("@{target}");
    if text.trim().is_empty() {
        return format!("{line}\n");
    }
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = text.trim_end_matches(['\n', '\r']).to_string();
    out.push_str(nl);
    out.push_str(nl);
    out.push_str(&line);
    out.push_str(nl);
    out
}

/// The repository roots of the roost `key` among the sessions on the island now: what a window
/// names a roost by, never a path it could make up. Worktrees of one repository share a key.
fn roots_of(app: &AppHandle, key: &str) -> Result<Vec<PathBuf>, String> {
    let view = app.view().ok_or("The flock is not known yet.")?;
    let mut roots: Vec<PathBuf> = view["sessions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s["cwd"].as_str())
        .filter_map(|cwd| repo_of(Path::new(cwd)))
        .filter(|r| r.key() == key)
        .map(|r| r.root)
        .collect();
    roots.dedup();
    if roots.is_empty() {
        return Err("That roost has no session now.".into());
    }
    Ok(roots)
}

/// What sharing `AGENTS.md` with Claude Code would change in a roost: the file (as shown, and as it
/// is, for the write to name it again) and the diff, to see first.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharePreview {
    pub path: String,
    pub file: String,
    pub diff: String,
    pub fingerprint: String,
}

/// The change, for the Nest to show before any write: for the first of the roost's worktrees with
/// an `AGENTS.md` its `CLAUDE.md` does not import.
#[tauri::command]
pub async fn roost_share_preview(
    app: AppHandle,
    window: tauri::WebviewWindow,
    key: String,
) -> Result<SharePreview, String> {
    if window.label() != crate::NEST {
        return Err("only the Nest can do this".into());
    }
    let roots = roots_of(&app, &key)?;
    tauri::async_runtime::spawn_blocking(move || {
        let root = roots
            .into_iter()
            .find(|r| sharing(r) == "offer")
            .ok_or("Claude Code already reads this project's AGENTS.md.")?;
        let (path, target) = claude_md(&root);
        editable(&path, &root)?;
        let p = vults_agent_config::text::preview(&path, |t| with_import(t, target))
            .map_err(|e| e.to_string())?;
        Ok(SharePreview {
            path: crate::paths::shown(&path),
            file: path.to_string_lossy().into_owned(),
            diff: p.diff,
            fingerprint: p.fingerprint,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The change the user saw, written (rule 3): only to the very file previewed, if it is still the
/// one Claude Code reads in one of the roost's worktrees, only over the very bytes the diff came
/// from, after a dated backup in the app's folder, never in the repository.
#[tauri::command]
pub async fn roost_share_apply(
    app: AppHandle,
    window: tauri::WebviewWindow,
    key: String,
    file: String,
    fingerprint: String,
) -> Result<(), String> {
    if window.label() != crate::NEST {
        return Err("only the Nest can do this".into());
    }
    let roots = roots_of(&app, &key)?;
    tauri::async_runtime::spawn_blocking(move || {
        let file = PathBuf::from(file);
        let (root, target) = roots
            .iter()
            .find_map(|r| {
                let (path, target) = claude_md(r);
                (path == file).then(|| (r.clone(), target))
            })
            .ok_or("That file is no longer the one Claude Code reads here: look again.")?;
        editable(&file, &root)?;
        let backups = crate::paths::data_dir().join("backups");
        // The folder's name and a few hex digits of its path: two `site` repositories never mix.
        let hash = root.to_string_lossy().bytes().fold(0x811c_9dc5_u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        });
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".into());
        let backup = vults_agent_config::text::apply(
            &file,
            &fingerprint,
            |t| with_import(t, target),
            &backups,
            &format!("{name}-{hash:08x}"),
            std::time::SystemTime::now(),
        )
        .map_err(|e| e.to_string())?;
        tracing::info!(
            backup = backup.is_some(),
            "a project's CLAUDE.md now reads its AGENTS.md"
        );
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The memory, when it exists (reading never makes it).
fn memory_at(dir: &Path) -> Option<Memory> {
    dir.join(".git")
        .exists()
        .then(|| Memory::open(dir).ok())
        .flatten()
}

/// The roosts now, for the Nest's Roosts tab.
#[tauri::command]
pub async fn roosts(app: AppHandle) -> Vec<Roost> {
    let Some(view) = app.view() else {
        return Vec::new();
    };
    let on = crate::settings::memory(&app);
    tauri::async_runtime::spawn_blocking(move || {
        // With the memory off, none of it is shown: off means off.
        let memory = on.then(|| memory_at(&crate::paths::memory_dir())).flatten();
        let mut roosts = group(&view, repo_of, |f| {
            memory
                .as_ref()
                .and_then(|m| m.page(f).ok())
                .map(|p| p.facts().map(|f| f.text.clone()).collect())
                .unwrap_or_default()
        });
        let mut costs = crate::spend::roosts();
        for r in &mut roosts {
            r.cost = costs.remove(&r.key).map(|c| Cost {
                today: c.today,
                total: c.total,
                subscription: c.subscription,
            });
        }
        roosts
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use serde_json::json;

    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-roosts-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_folder_s_repository_branch_and_github_name() {
        let base = temp("repo");
        let root = base.join("vults");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/feat/m3\n").unwrap();
        std::fs::write(
            root.join(".git/config"),
            "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = git@github.com:x/y.git\n[Remote \"origin\"] ; ours\n\tURL = \"https://github.com/rogeriojunior31/Vults.git\" # the fork\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/zeca")).unwrap();
        let r = repo_within(&root.join("crates/zeca"), Path::new("/nowhere")).unwrap();
        assert_eq!(r.root, root);
        assert_eq!(r.github().as_deref(), Some("rogeriojunior31/Vults"));
        assert_eq!(r.key(), "github:rogeriojunior31/vults");
        assert_eq!(r.branch.as_deref(), Some("feat/m3"));
        assert!(
            r.memory()
                .unwrap()
                .path()
                .starts_with("roosts/rogeriojunior31-vults-")
        );

        // A worktree: `.git` is a file; its HEAD is its own, the remote the main repository's.
        let wt = base.join("vults-wt");
        std::fs::create_dir_all(&wt).unwrap();
        let gitdir = root.join(".git/worktrees/wt");
        std::fs::create_dir_all(&gitdir).unwrap();
        std::fs::write(gitdir.join("HEAD"), "ref: refs/heads/feat/other\n").unwrap();
        std::fs::write(gitdir.join("commondir"), "../..\n").unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();
        let w = repo_within(&wt, Path::new("/nowhere")).unwrap();
        assert_eq!(w.key(), r.key(), "one repository, one roost");
        assert_eq!(w.memory(), r.memory());
        assert_eq!(w.branch.as_deref(), Some("feat/other"));

        // No remote: keyed by its own git dir, named by its folder; a detached HEAD has no branch.
        let plain = base.join("plain");
        std::fs::create_dir_all(plain.join(".git")).unwrap();
        std::fs::write(plain.join(".git/HEAD"), "0123abcd\n").unwrap();
        let p = repo_within(&plain, Path::new("/nowhere")).unwrap();
        assert_eq!(p.name(), "plain");
        assert!(p.key().starts_with("local:"));
        assert_eq!((p.github(), p.branch.clone()), (None, None));
        // A folder named like a GitHub repository is not that repository.
        let lookalike = base.join("rogeriojunior31-vults");
        std::fs::create_dir_all(lookalike.join(".git")).unwrap();
        let l = repo_within(&lookalike, Path::new("/nowhere")).unwrap();
        assert_ne!(l.memory(), r.memory());

        // The walk stops before the home folder: a dotfiles repository there is no roost.
        let home = base.join("home");
        std::fs::create_dir_all(home.join(".git")).unwrap();
        std::fs::create_dir_all(home.join("notes/deep")).unwrap();
        assert_eq!(repo_within(&home.join("notes/deep"), &home), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_planted_fifo_or_device_never_hangs_the_read() {
        let root = temp("fifo").join("r");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let fifo = root.join(".git/HEAD");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        std::os::unix::fs::symlink("/dev/zero", root.join(".git/config")).unwrap();
        let r = repo_within(&root, Path::new("/nowhere")).unwrap();
        assert_eq!((r.branch, r.origin), (None, None));
        // And a `.git` file that is a FIFO is no repository at all.
        let other = temp("fifo2").join("r");
        std::fs::create_dir_all(&other).unwrap();
        std::process::Command::new("mkfifo")
            .arg(other.join(".git"))
            .status()
            .unwrap();
        assert_eq!(repo_within(&other, Path::new("/nowhere")), None);
    }

    #[test]
    fn hosts_are_matched_whole() {
        let gh = |url: &str| {
            Repo {
                root: "/r".into(),
                common: "/r/.git".into(),
                origin: host_path(url),
                branch: None,
            }
            .github()
        };
        for (url, want) in [
            ("git@github.com:o/r.git", Some("o/r")),
            ("https://github.com/o/r", Some("o/r")),
            ("https://github.com/o/r/", Some("o/r")),
            ("ssh://git@github.com:22/o/r.git", Some("o/r")),
            ("https://user@www.GitHub.com/o/r", Some("o/r")),
            ("https://gitlab.com/o/r.git", None),
            ("https://notgithub.com/o/r", None),
            ("https://gitlab.com/github.com/o/r", None),
            ("https://github.com/o", None),
            ("https://github.com/o/r/tree/main", None),
            ("https://github.com/o/r;rm -rf", None),
            ("/srv/git/r.git", None),
        ] {
            assert_eq!(gh(url).as_deref(), want, "{url}");
        }
        assert_eq!(
            host_path("git@gitlab.com:Team/API.git"),
            Some(("gitlab.com".into(), "Team/API".into()))
        );
        // Two repositories named alike elsewhere stay two roosts.
        let at = |url: &str| Repo {
            root: "/r".into(),
            common: "/r/.git".into(),
            origin: host_path(url),
            branch: None,
        };
        assert_ne!(
            at("https://gitlab.com/a/api").key(),
            at("https://gitlab.com/b/api").key()
        );
        assert_ne!(
            at("https://github.com/a-b/c").memory(),
            at("https://github.com/a/b-c").memory()
        );
        assert_eq!(
            at("https://github.com/Owner/Repo").key(),
            at("git@github.com:owner/repo.git").key(),
            "GitHub names ignore case"
        );
    }

    #[test]
    fn sessions_join_their_repository_s_roost_with_its_card_rows() {
        let view = json!({
            "sessions": [
                { "agent": "claude", "agent_name": "Claude Code", "status": "working", "cwd": "/w/vults", "step": "Running cargo test" },
                { "agent": "codex", "status": "idle", "cwd": "/w/vults-m3", "step": null },
                { "agent": "claude", "status": "working", "cwd": "/w/site", "step": "TOKEN=abc123 make" },
                { "agent": "claude", "status": "idle", "cwd": "/tmp/scratch" },
                { "agent": "claude", "status": "idle", "cwd": null },
            ],
            "boards": [{ "connector": "github", "rows": [
                { "item": "pr:rogeriojunior31/Vults#242", "name": "Vults#242", "title": "M2", "checks": "passing", "review": null, "url": "https://github.com/x" },
                { "item": "review:rogeriojunior31/Vults#7", "name": "Vults#7", "title": "Someone's", "checks": null, "review": null },
                { "item": "branch:rogeriojunior31/vults", "name": "Vults", "title": "main", "checks": "failing", "review": null },
                { "item": "pr:other/repo#1", "name": "repo#1", "title": "Elsewhere", "checks": null, "review": null },
            ]}],
        });
        let repo = |p: &Path| -> Option<Repo> {
            let s = p.to_str()?;
            let (root, github, branch) = match s {
                "/w/vults" => ("/w/vults", Some("rogeriojunior31/Vults"), Some("main")),
                "/w/vults-m3" => ("/w/vults-m3", Some("rogeriojunior31/Vults"), Some("feat/m3")),
                "/w/site" => ("/w/site", None, Some("main")),
                _ => return None,
            };
            Some(Repo {
                root: root.into(),
                common: format!("{root}/.git").into(),
                origin: github.map(|g| ("github.com".to_string(), g.to_string())),
                branch: branch.map(str::to_string),
            })
        };
        let vults_memory = repo(Path::new("/w/vults")).unwrap().memory().unwrap();
        let memory = |f: &File| {
            if *f == vults_memory {
                vec!["Merges with --merge.".to_string()]
            } else {
                Vec::new()
            }
        };
        let roosts = group(&view, repo, memory);
        let names: Vec<&str> = roosts.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["rogeriojunior31/Vults", "site"]);
        let v = &roosts[0];
        assert_eq!(v.trees.len(), 2);
        assert_eq!(v.trees[1].branch.as_deref(), Some("feat/m3"));
        assert_eq!(v.sessions.len(), 2);
        assert_eq!(v.sessions[0].agent, "Claude Code");
        assert_eq!(v.prs.len(), 1, "only the user's own PRs in this repository");
        assert_eq!(v.prs[0].checks.as_deref(), Some("passing"));
        assert_eq!(v.checks.as_deref(), Some("failing"), "GitHub names ignore case");
        assert_eq!(v.memory, ["Merges with --merge."]);
        let site = &roosts[1];
        assert_eq!(site.sessions[0].step.as_deref(), Some("TOKEN=[redacted] make"));
        assert!(site.prs.is_empty() && site.checks.is_none());
    }

    #[test]
    fn agents_md_is_offered_once_and_imported_where_claude_reads_it() {
        let root = temp("agents");
        assert_eq!(sharing(&root), "", "no AGENTS.md, nothing to share");
        std::fs::write(root.join("AGENTS.md"), "- Tests run with nextest.\n").unwrap();
        assert_eq!(sharing(&root), "offer");
        let (path, target) = claude_md(&root);
        assert_eq!(path, root.join("CLAUDE.md"));
        assert_eq!(
            with_import("", target),
            "@AGENTS.md\n",
            "a new CLAUDE.md is the line alone"
        );
        assert_eq!(
            with_import("# Rules\n\nBe brief.\n\n", target),
            "# Rules\n\nBe brief.\n\n@AGENTS.md\n"
        );
        assert_eq!(
            with_import("A\r\nB\r\n", target),
            "A\r\nB\r\n\r\n@AGENTS.md\r\n",
            "its own line ends"
        );
        for (text, shared) in [
            ("# Rules\n  @AGENTS.md  \n", true),
            ("See @AGENTS.md for the rules.\n", true),
            ("\u{feff}@AGENTS.md\n", true),
            ("```\n@AGENTS.md\n```\n", false),
            ("@../AGENTS.md\n", false),
            ("Mentions AGENTS.md only.\n", false),
        ] {
            std::fs::write(root.join("CLAUDE.md"), text).unwrap();
            assert_eq!(sharing(&root) == "shared", shared, "{text:?}");
        }
        // Both files: `.claude/CLAUDE.md` importing the one a folder up is enough.
        std::fs::write(root.join("CLAUDE.md"), "Rules.\n").unwrap();
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        std::fs::write(root.join(".claude/CLAUDE.md"), "@AGENTS.md\n").unwrap();
        assert_eq!(sharing(&root), "offer", ".claude/AGENTS.md is not the project's");
        std::fs::write(root.join(".claude/CLAUDE.md"), "@../AGENTS.md\n").unwrap();
        assert_eq!(sharing(&root), "shared");
        // Only `.claude/CLAUDE.md`: it is the one edited, importing from the folder above.
        let inner = temp("agents-inner");
        std::fs::write(inner.join("AGENTS.md"), "x\n").unwrap();
        std::fs::create_dir_all(inner.join(".claude")).unwrap();
        std::fs::write(inner.join(".claude/CLAUDE.md"), "Rules.\n").unwrap();
        let (path, target) = claude_md(&inner);
        assert!(path.ends_with(".claude/CLAUDE.md"));
        assert_eq!(with_import("Rules.\n", target), "Rules.\n\n@../AGENTS.md\n");
    }

    #[cfg(unix)]
    #[test]
    fn a_claude_md_that_leads_out_of_the_repository_is_never_edited() {
        let root = temp("agents-link");
        let outside = temp("agents-outside");
        std::fs::write(root.join("AGENTS.md"), "x\n").unwrap();
        std::fs::write(outside.join("gitconfig"), "[user]\n").unwrap();
        std::os::unix::fs::symlink(outside.join("gitconfig"), root.join("CLAUDE.md")).unwrap();
        assert!(editable(&root.join("CLAUDE.md"), &root).is_err());
        // A hard link, and a `.claude` folder that is a link elsewhere: refused too.
        let hard = temp("agents-hard");
        std::fs::write(hard.join("AGENTS.md"), "x\n").unwrap();
        std::fs::hard_link(outside.join("gitconfig"), hard.join("CLAUDE.md")).unwrap();
        assert!(editable(&hard.join("CLAUDE.md"), &hard).is_err());
        let linked = temp("agents-dir");
        std::os::unix::fs::symlink(&outside, linked.join(".claude")).unwrap();
        assert!(editable(&linked.join(".claude").join("CLAUDE.md"), &linked).is_err());
        // A FIFO: refused before any read.
        let fifo = temp("agents-fifo");
        std::process::Command::new("mkfifo")
            .arg(fifo.join("CLAUDE.md"))
            .status()
            .unwrap();
        assert!(editable(&fifo.join("CLAUDE.md"), &fifo).is_err());
        assert_eq!(
            std::fs::read_to_string(outside.join("gitconfig")).unwrap(),
            "[user]\n"
        );
    }
}
