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

/// A folder's repository, read from its `.git` on this computer (no network).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// The working tree's top folder.
    pub root: PathBuf,
    /// `owner/repo` on GitHub, from the `origin` remote.
    pub github: Option<String>,
    /// The checked-out branch; none on a detached HEAD.
    pub branch: Option<String>,
}

impl Repo {
    /// The roost's key: the GitHub repository, else the folder's name.
    pub fn key(&self) -> String {
        self.github.clone().unwrap_or_else(|| {
            self.root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    }

    /// The roost's memory file.
    pub fn memory(&self) -> Option<File> {
        File::roost(&self.key()).ok()
    }
}

/// The repository `dir` is in, walking up to its `.git` (a folder, or a worktree's file naming
/// its git dir).
pub fn repo_of(dir: &Path) -> Option<Repo> {
    let mut at = Some(dir);
    while let Some(d) = at {
        let dot = d.join(".git");
        if let Some(git) = git_dir(&dot) {
            // A worktree's own HEAD; the remotes are in the shared dir (`commondir`).
            let common = std::fs::read_to_string(git.join("commondir"))
                .map(|c| git.join(c.trim()))
                .unwrap_or_else(|_| git.clone());
            let branch = std::fs::read_to_string(git.join("HEAD"))
                .ok()
                .and_then(|h| h.trim().strip_prefix("ref: refs/heads/").map(str::to_string));
            let github = std::fs::read_to_string(common.join("config"))
                .ok()
                .and_then(|c| origin(&c))
                .and_then(|url| github_repo(&url));
            return Some(Repo {
                root: d.to_path_buf(),
                github,
                branch,
            });
        }
        at = d.parent();
    }
    None
}

fn git_dir(dot: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(dot).ok()?;
    if meta.is_dir() {
        return Some(dot.to_path_buf());
    }
    let text = std::fs::read_to_string(dot).ok()?;
    let path = PathBuf::from(text.trim().strip_prefix("gitdir:")?.trim());
    let path = if path.is_relative() {
        dot.parent()?.join(path)
    } else {
        path
    };
    path.is_dir().then_some(path)
}

/// The `origin` remote's URL from a git config.
fn origin(config: &str) -> Option<String> {
    let mut inside = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line.replace(' ', "") == "[remote\"origin\"]";
        } else if inside
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == "url"
        {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// `owner/repo` from a GitHub URL (`git@github.com:o/r.git`, `https://github.com/o/r`,
/// `ssh://git@github.com/o/r.git`); none for another host.
fn github_repo(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.split_once("github.com/").map(|(_, r)| r))?;
    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let mut parts = rest.split('/');
    let (owner, repo) = (parts.next()?, parts.next()?);
    let tame = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    (parts.next().is_none() && tame(owner) && tame(repo)).then(|| format!("{owner}/{repo}"))
}

/// One roost, as the Nest shows it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Roost {
    pub key: String,
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
                    github: r.github.clone(),
                    trees: Vec::new(),
                    sessions: Vec::new(),
                    prs: Vec::new(),
                    checks: None,
                    memory: r.memory().map(|f| memory(&f)).unwrap_or_default(),
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

/// The facts of a roost's memory, when the memory exists (reading never makes it).
fn facts(dir: &Path, file: &File) -> Vec<String> {
    if !dir.join(".git").exists() {
        return Vec::new();
    }
    Memory::open(dir)
        .and_then(|m| m.page(file))
        .map(|p| p.facts().map(|f| f.text.clone()).collect())
        .unwrap_or_default()
}

/// The roosts now, for the Nest's Roosts tab.
#[tauri::command]
pub async fn roosts(app: AppHandle) -> Vec<Roost> {
    let Some(view) = app.view() else {
        return Vec::new();
    };
    let dir = crate::paths::memory_dir();
    tauri::async_runtime::spawn_blocking(move || group(&view, repo_of, |f| facts(&dir, f)))
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
        let root = temp("repo");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/feat/m3\n").unwrap();
        std::fs::write(
            root.join(".git/config"),
            "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = git@github.com:x/y.git\n[remote \"origin\"]\n\turl = https://github.com/rogeriojunior31/Vults.git\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/zeca")).unwrap();
        let r = repo_of(&root.join("crates/zeca")).unwrap();
        assert_eq!(r.root, root);
        assert_eq!(r.github.as_deref(), Some("rogeriojunior31/Vults"));
        assert_eq!(r.branch.as_deref(), Some("feat/m3"));
        assert_eq!(r.memory().unwrap().path(), "roosts/rogeriojunior31-vults.md");

        // A worktree: `.git` is a file; its HEAD is its own, the remote the main repository's.
        let wt = temp("worktree");
        let gitdir = root.join(".git/worktrees/wt");
        std::fs::create_dir_all(&gitdir).unwrap();
        std::fs::write(gitdir.join("HEAD"), "ref: refs/heads/feat/other\n").unwrap();
        std::fs::write(gitdir.join("commondir"), "../..\n").unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();
        let w = repo_of(&wt).unwrap();
        assert_eq!(w.key(), r.key(), "one repository, one roost");
        assert_eq!(w.branch.as_deref(), Some("feat/other"));

        // No remote on GitHub: the folder's name; a detached HEAD has no branch.
        let plain = temp("plain");
        std::fs::create_dir_all(plain.join(".git")).unwrap();
        std::fs::write(plain.join(".git/HEAD"), "0123abcd\n").unwrap();
        let p = repo_of(&plain).unwrap();
        assert_eq!(p.key(), plain.file_name().unwrap().to_string_lossy());
        assert_eq!((p.github, p.branch), (None, None));
    }

    #[test]
    fn github_names_only_from_github() {
        for (url, want) in [
            ("git@github.com:o/r.git", Some("o/r")),
            ("https://github.com/o/r", Some("o/r")),
            ("https://github.com/o/r/", Some("o/r")),
            ("ssh://git@github.com/o/r.git", Some("o/r")),
            ("https://gitlab.com/o/r.git", None),
            ("https://github.com/o", None),
            ("https://github.com/o/r/tree/main", None),
            ("https://github.com/o/r;rm -rf", None),
        ] {
            assert_eq!(github_repo(url).as_deref(), want, "{url}");
        }
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
                github: github.map(str::to_string),
                branch: branch.map(str::to_string),
            })
        };
        let memory = |f: &File| {
            if f.path() == "roosts/rogeriojunior31-vults.md" {
                vec!["Merges with --merge.".to_string()]
            } else {
                Vec::new()
            }
        };
        let roosts = group(&view, repo, memory);
        let keys: Vec<&str> = roosts.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["rogeriojunior31/Vults", "site"]);
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
}
