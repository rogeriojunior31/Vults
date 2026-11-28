//! Files and folders of a conversation's folder for the chat's `@` menu: their relative paths only,
//! found by a bounded walk that leaves out version control, dependencies and build output.

use std::path::Path;

/// One match: its path relative to the folder, with `/` separators, and whether it is a folder.
#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub dir: bool,
}

/// Never walked into: what no one means by "this file", and what makes a walk slow.
const SKIP: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
    ".idea",
    ".vs",
    ".gradle",
    "vendor",
    "coverage",
];
const MAX_SEEN: usize = 20_000;
const MAX_DEPTH: usize = 8;
const MAX_HITS: usize = 12;

/// The best matches for `query` (case-insensitive, on the relative path) in `folder`. A name that
/// starts with the query comes first, then a name that holds it, then a path that holds it; the
/// shorter path first within each.
pub fn find(folder: &Path, query: &str) -> Vec<Hit> {
    let query = query.trim().to_lowercase();
    if query.is_empty() || query.len() > 200 {
        return Vec::new();
    }
    let mut scored: Vec<(u8, usize, Hit)> = Vec::new();
    let mut seen = 0usize;
    let mut stack = vec![(folder.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > MAX_SEEN {
                break;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            // A symlink is never followed: it may lead out of the folder.
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            let Ok(rel) = path.strip_prefix(folder) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            let dir = kind.is_dir();
            if dir && (SKIP.contains(&name.as_str()) || depth >= MAX_DEPTH) {
                continue;
            }
            let lower_name = name.to_lowercase();
            let rank = if lower_name.starts_with(&query) {
                Some(0)
            } else if lower_name.contains(&query) {
                Some(1)
            } else if rel.to_lowercase().contains(&query) {
                Some(2)
            } else {
                None
            };
            if let Some(rank) = rank {
                scored.push((
                    rank,
                    rel.len(),
                    Hit {
                        path: rel.clone(),
                        dir,
                    },
                ));
            }
            if dir {
                stack.push((path, depth + 1));
            }
        }
        if seen > MAX_SEEN {
            break;
        }
    }
    scored.sort_by(|a, b| (a.0, a.1, &a.2.path).cmp(&(b.0, b.1, &b.2.path)));
    scored.into_iter().take(MAX_HITS).map(|(_, _, hit)| hit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_that_start_with_the_query_come_first_and_dependencies_stay_out() {
        let root = std::env::temp_dir().join(format!("vults-chat-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["src/auth", "node_modules/auth", ".git", "docs"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in [
            "src/auth/token.rs",
            "src/main.rs",
            "docs/auth-guide.md",
            "node_modules/auth/index.js",
            "README.md",
        ] {
            std::fs::write(root.join(file), "x").unwrap();
        }
        let hits = find(&root, "auth");
        let paths: Vec<&str> = hits.iter().map(|h| h.path.as_str()).collect();
        assert_eq!(paths, ["src/auth", "docs/auth-guide.md", "src/auth/token.rs"]);
        assert!(hits[0].dir && !hits[1].dir);
        assert!(find(&root, "index").is_empty(), "node_modules is never walked");
        assert!(find(&root, "  ").is_empty());
        assert_eq!(find(&root, "MAIN")[0].path, "src/main.rs", "case does not matter");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
