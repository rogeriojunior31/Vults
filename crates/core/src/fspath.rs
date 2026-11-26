//! Folders as the agents report them, made comparable. The core does no IO and runs on any OS, so
//! the path's own shape says which rules hold: a Windows one (`C:\x`, `c:/x/`, `\\?\C:\x`,
//! `\\server\share`) ignores case and takes either slash; a Unix one is kept as written.

/// The comparable form of an absolute path: `/`-separated, no trailing slash (`/` stays) and a
/// Windows one lowercased. None for a relative path or one that climbs (`..`), which then reaches
/// nothing.
pub fn key(path: &str) -> Option<String> {
    let path = match path.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => path.strip_prefix(r"\\?\").unwrap_or(path).to_owned(),
    };
    let key = if is_windows(&path) {
        path.replace('\\', "/").to_lowercase()
    } else if path.starts_with('/') {
        path
    } else {
        return None;
    };
    if key.split('/').any(|c| c == "..") {
        return None;
    }
    let trimmed = key.trim_end_matches('/');
    Some(if trimmed.is_empty() {
        "/".into()
    } else {
        trimmed.into()
    })
}

/// A drive path (`C:\`, `C:/`) or a UNC one (`\\server\share`, `//server/share` once keyed).
pub fn is_windows(path: &str) -> bool {
    let b = path.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/'))
        || (b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
        || path.starts_with(r"\\")
        || path.starts_with("//")
}

/// Is a keyed path the top of its disk (`/`, `c:`, `//server/share`): a folder no allow reaches?
pub fn is_top(key: &str) -> bool {
    if key == "/" || (key.len() == 2 && key.ends_with(':')) {
        return true;
    }
    key.strip_prefix("//")
        .is_some_and(|rest| rest.split('/').filter(|c| !c.is_empty()).count() <= 2)
}

/// Is `path` `dir` or inside it, both as agents report them?
pub fn inside(path: &str, dir: &str) -> bool {
    let (Some(path), Some(dir)) = (key(path), key(dir)) else {
        return false;
    };
    dir == "/" || path == dir || path.strip_prefix(&dir).is_some_and(|r| r.starts_with('/'))
}

/// Do two absolute folders name the same place? A relative one matches only itself.
pub fn same(a: &str, b: &str) -> bool {
    match (key(a), key(b)) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_paths_ignore_case_slashes_and_prefixes_unix_ones_stay() {
        for spelling in [r"C:\Proj", r"c:\proj\", "C:/proj", r"\\?\C:\PROJ"] {
            assert_eq!(key(spelling).as_deref(), Some("c:/proj"), "{spelling}");
        }
        assert_eq!(
            key(r"\\?\UNC\Server\Share\x").as_deref(),
            Some("//server/share/x")
        );
        assert_eq!(key("/home/U/site/").as_deref(), Some("/home/U/site"));
        assert_eq!(key("/").as_deref(), Some("/"));
        // Keying a key gives it back: `C:\` keys to `c:`, the top of the disk.
        assert_eq!(key(r"C:\").as_deref(), Some("c:"));
        assert_eq!(key("c:").as_deref(), Some("c:"));
        assert_eq!(key("//server/share/x").as_deref(), Some("//server/share/x"));
        for no in ["site", r"C:proj", "", r"C:\a\..\b", "/a/../b", "~/x"] {
            assert_eq!(key(no), None, "{no}");
        }
    }

    #[test]
    fn inside_and_same_hold_across_spellings() {
        assert!(inside(r"C:\Proj\src\main.rs", "c:/proj"));
        assert!(inside(r"c:\proj", r"C:\Proj\"));
        assert!(!inside(r"C:\proj-old\x", r"C:\proj"));
        assert!(!inside("/home/u/Site", "/home/u/site"), "Unix keeps case");
        assert!(inside("/anything", "/"));
        assert!(!inside("relative", "/"));
        assert!(same(r"C:\Proj\", "c:/proj") && !same("/a", "/A") && same("x", "x"));
        assert!(is_top("/") && is_top("c:") && is_top("//server/share") && !is_top("c:/users"));
    }
}
