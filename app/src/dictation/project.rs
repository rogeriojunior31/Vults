//! Bounded, local project names for Whisper. Repository text is context, never an instruction.

use std::collections::{HashSet, VecDeque};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

const WORDS: usize = 256;
const PROJECTS: usize = 128;
const FILE_BYTES: u64 = 64 * 1024;

struct Entry {
    roost: String,
    root: PathBuf,
    words: Vec<String>,
}

#[derive(Default)]
pub(super) struct Cache(Mutex<VecDeque<Entry>>);

impl Cache {
    fn insert(&self, entry: Entry) {
        let mut entries = self.0.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|e| e.root != entry.root);
        if entries.len() == PROJECTS {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    pub(super) fn words(&self, window: &vults_platform::focus::Window, cwd: Option<&str>) -> Vec<String> {
        let entries = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cwd) = cwd {
            return entries
                .iter()
                .filter(|e| Path::new(cwd).starts_with(&e.root))
                .max_by_key(|e| e.root.components().count())
                .map(|e| e.words.clone())
                .unwrap_or_default();
        }
        if !window.is_terminal() && !editor(&window.class) {
            return Vec::new();
        }
        let title = window.title.to_lowercase();
        let exact: Vec<_> = entries
            .iter()
            .filter(|e| named(&title, &e.root.to_string_lossy().to_lowercase()))
            .collect();
        let matches = if exact.is_empty() {
            entries
                .iter()
                .filter(|e| {
                    e.root
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| named(&title, &n.to_lowercase()))
                        || named(&title, &e.roost.to_lowercase())
                })
                .collect()
        } else {
            exact
        };
        if matches.len() == 1 {
            matches[0].words.clone()
        } else {
            Vec::new()
        }
    }
}

fn editor(class: &str) -> bool {
    let class = class.to_ascii_lowercase();
    let name = class
        .strip_suffix(".desktop")
        .unwrap_or(&class)
        .rsplit('.')
        .next()
        .unwrap_or(&class);
    matches!(
        name,
        "code"
            | "code-oss"
            | "code-insiders"
            | "cursor"
            | "zed"
            | "kate"
            | "emacs"
            | "vim"
            | "nvim"
            | "sublime_text"
            | "geany"
    ) || name.starts_with("jetbrains-")
}

fn named(title: &str, name: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '.');
    !name.is_empty()
        && title.match_indices(name).any(|(at, _)| {
            title[..at].chars().next_back().is_none_or(|c| !word(c))
                && title[at + name.len()..].chars().next().is_none_or(|c| !word(c))
        })
}

pub(crate) fn refresh(app: &AppHandle, repo: &crate::roosts::Repo) {
    let enabled = app
        .try_state::<crate::settings::SettingsState>()
        .is_some_and(|s| s.0.lock().is_ok_and(|s| s.dictation_project_vocabulary));
    if enabled && let Some(cache) = app.try_state::<Cache>() {
        cache.insert(Entry {
            roost: repo.name(),
            root: repo.root.clone(),
            words: extract(repo),
        });
    }
}

fn small(path: &Path) -> Option<String> {
    use std::os::unix::fs::OpenOptionsExt;
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > FILE_BYTES {
        return None;
    }
    // A substituted FIFO or final symlink must not block a reader or expose another file.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(0o4000 | 0o400000)
        .open(path)
        .ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() > FILE_BYTES {
        return None;
    }
    let mut text = String::new();
    file.take(FILE_BYTES + 1).read_to_string(&mut text).ok()?;
    (text.len() as u64 <= FILE_BYTES).then_some(text)
}

fn children(dir: &Path) -> Vec<PathBuf> {
    // ponytail: inspect at most 512 entries per directory; larger trees need an indexed inventory.
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .take(512)
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| !t.is_symlink()))
        .map(|e| e.path())
        .collect();
    paths.sort();
    paths
}

fn inside(root: &Path, relative: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        if part == Component::CurDir {
            continue;
        }
        let Component::Normal(part) = part else {
            return None;
        };
        path.push(part);
        if !std::fs::symlink_metadata(&path).ok()?.is_dir() {
            return None;
        }
    }
    Some(path)
}

fn extract(repo: &crate::roosts::Repo) -> Vec<String> {
    let mut words = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |name: &str| {
        if words.len() < WORDS
            && !name.is_empty()
            && name.len() <= 128
            && !name.starts_with('.')
            && name.chars().any(char::is_alphanumeric)
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '@' | '+'))
            && seen.insert(name.to_lowercase().to_uppercase())
        {
            words.push(name.to_owned());
        }
    };
    if let Some(name) = repo.root.file_name().and_then(|s| s.to_str()) {
        add(name);
    }
    if let Some(branch) = &repo.branch {
        add(branch);
    }
    let mut dirs = vec![repo.root.clone()];
    let mut members = Vec::new();
    if let Some(cargo) =
        small(&repo.root.join("Cargo.toml")).and_then(|s| s.parse::<toml_edit::DocumentMut>().ok())
        && let Some(paths) = cargo
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
    {
        members.extend(
            paths
                .iter()
                .filter_map(|m| m.as_str())
                .take(64)
                .map(str::to_owned),
        );
    }
    if let Some(package) = small(&repo.root.join("package.json"))
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        && let Some(paths) = package.get("workspaces")
    {
        let paths = paths
            .as_array()
            .or_else(|| paths.get("packages").and_then(|p| p.as_array()));
        if let Some(paths) = paths {
            members.extend(
                paths
                    .iter()
                    .filter_map(|p| p.as_str())
                    .take(64)
                    .map(str::to_owned),
            );
        }
    }
    for member in members.iter().take(64) {
        if let Some(parent) = member.strip_suffix("/*") {
            if let Some(parent) = inside(&repo.root, parent) {
                dirs.extend(children(&parent).into_iter().filter(|p| p.is_dir()).take(64));
            }
        } else if let Some(dir) = inside(&repo.root, member) {
            dirs.push(dir);
        }
        if dirs.len() >= 65 {
            break;
        }
    }
    dirs.truncate(65);
    dirs.sort();
    dirs.dedup();
    for dir in dirs {
        if let Some(cargo) =
            small(&dir.join("Cargo.toml")).and_then(|s| s.parse::<toml_edit::DocumentMut>().ok())
        {
            for table in ["package", "lib"] {
                if let Some(name) = cargo
                    .get(table)
                    .and_then(|t| t.get("name"))
                    .and_then(|n| n.as_str())
                {
                    add(name);
                }
            }
            if let Some(bins) = cargo.get("bin").and_then(|t| t.as_array_of_tables()) {
                for bin in bins {
                    if let Some(name) = bin.get("name").and_then(|n| n.as_str()) {
                        add(name);
                    }
                }
            }
            for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(deps) = cargo.get(table).and_then(|t| t.as_table_like()) {
                    for (name, _) in deps.iter() {
                        add(name);
                    }
                }
            }
        }
        if let Some(package) =
            small(&dir.join("package.json")).and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        {
            if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
                add(name);
            }
            for table in ["dependencies", "devDependencies", "peerDependencies"] {
                if let Some(deps) = package.get(table).and_then(|t| t.as_object()) {
                    for name in deps.keys() {
                        add(name);
                    }
                }
            }
        }
        let src = dir.join("src");
        if std::fs::symlink_metadata(&src).is_ok_and(|m| m.is_dir()) {
            for path in children(&src) {
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    add(name);
                }
            }
        }
    }
    for path in children(&repo.root) {
        if path.is_file()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            add(name);
        }
    }
    words
}

#[cfg(test)]
mod tests;
