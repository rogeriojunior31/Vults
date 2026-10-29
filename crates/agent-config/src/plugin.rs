//! A whole file of ours in an agent's folder: a plugin the agent loads by itself (OpenCode's
//! `plugins/`). Same rules as a config edit: a diff the user saw, the fingerprint of the bytes
//! it came from, a dated backup, an atomic write. The file must carry `marker`: one without it
//! is the user's, and is never overwritten nor removed. Backups go in the folder above: the agent
//! loads every file in the plugin's.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::{Backups, Error, Preview, back_up, diff, fingerprint, read_bytes, replace};

/// Whether the file at `path` is ours: present and carrying `marker`.
pub fn installed(path: &Path, marker: &str) -> bool {
    std::fs::read(path).is_ok_and(|b| contains(&b, marker))
}

/// Ours, and exactly the text this version writes.
pub fn up_to_date(path: &Path, text: &str) -> bool {
    std::fs::read(path).is_ok_and(|b| b == text.as_bytes())
}

/// Why `path` can't be written: a file of the user's sits there.
pub fn blocked(path: &Path, marker: &str) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    (!bytes.is_empty() && !contains(&bytes, marker)).then(|| foreign(path).to_string())
}

/// The diff from the file as it is to `next` (`None`: the file removed).
pub fn preview(path: &Path, next: Option<&str>, marker: &str) -> Result<Preview, Error> {
    let bytes = checked(path, marker)?;
    let before = String::from_utf8_lossy(&bytes);
    Ok(Preview {
        diff: diff(&before, next.unwrap_or_default(), path),
        fingerprint: fingerprint(&bytes),
    })
}

/// Preview all owned files of one plugin before changing any of them.
pub fn preview_files(files: &[(PathBuf, String)], install: bool, marker: &str) -> Result<Preview, Error> {
    let plans: Vec<_> = files
        .iter()
        .map(|(path, text)| preview(path, install.then_some(text.as_str()), marker))
        .collect::<Result<_, _>>()?;
    Ok(bundle(files, &plans))
}

fn bundle(files: &[(PathBuf, String)], plans: &[Preview]) -> Preview {
    let mut bytes = Vec::new();
    for ((path, _), plan) in files.iter().zip(plans) {
        let path = path.as_os_str().as_encoded_bytes();
        bytes.extend_from_slice(&(path.len() as u64).to_le_bytes());
        bytes.extend_from_slice(path);
        bytes.extend_from_slice(plan.fingerprint.as_bytes());
    }
    Preview {
        diff: plans
            .iter()
            .map(|p| p.diff.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        fingerprint: fingerprint(&bytes),
    }
}

/// Preflight every file, then atomically replace each, retaining its original fingerprint check.
pub fn apply_files(
    files: &[(PathBuf, String)],
    seen: &str,
    install: bool,
    marker: &str,
    now: SystemTime,
) -> Result<Option<PathBuf>, Error> {
    let plans: Vec<_> = files
        .iter()
        .map(|(path, text)| preview(path, install.then_some(text.as_str()), marker))
        .collect::<Result<_, _>>()?;
    if bundle(files, &plans).fingerprint != seen {
        return Err(Error::Changed {
            path: files.first().map(|f| f.0.clone()).unwrap_or_default(),
        });
    }
    let mut backup = None;
    for ((path, text), plan) in files.iter().zip(plans) {
        let saved = apply(
            path,
            &plan.fingerprint,
            install.then_some(text.as_str()),
            marker,
            now,
        )?;
        if backup.is_none() {
            backup = saved;
        }
    }
    Ok(backup)
}

/// Writes `next` (or removes the file) if it still has the bytes `fingerprint_seen` was taken
/// from. Returns the backup path, or `None` when there was no file to back up.
pub fn apply(
    path: &Path,
    fingerprint_seen: &str,
    next: Option<&str>,
    marker: &str,
    now: SystemTime,
) -> Result<Option<PathBuf>, Error> {
    let bytes = checked(path, marker)?;
    if fingerprint(&bytes) != fingerprint_seen {
        return Err(Error::Changed {
            path: path.to_path_buf(),
        });
    }
    match next {
        Some(text) => replace(path, text.as_bytes(), now, Backups::AboveFolder),
        None if bytes.is_empty() && !path.exists() => Ok(None),
        None => {
            // Backed up like any write, then gone: removing the plugin is the agent's uninstall.
            let write_err = |source| Error::Write {
                path: path.to_path_buf(),
                source,
            };
            let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            let backup = back_up(&target, true, now, Backups::AboveFolder).map_err(write_err)?;
            std::fs::remove_file(path).map_err(write_err)?;
            Ok(backup)
        }
    }
}

/// The file's bytes, when it is ours or absent; a file of the user's is refused.
fn checked(path: &Path, marker: &str) -> Result<Vec<u8>, Error> {
    let bytes = read_bytes(path)?;
    if !bytes.is_empty() && !contains(&bytes, marker) {
        return Err(foreign(path));
    }
    Ok(bytes)
}

fn foreign(path: &Path) -> Error {
    Error::Foreign {
        path: path.to_path_buf(),
    }
}

fn contains(bytes: &[u8], marker: &str) -> bool {
    !marker.is_empty() && bytes.windows(marker.len()).any(|w| w == marker.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const MARK: &str = "// Generated by the app";

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-plugin-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join("plugins").join("ours.js")
    }

    fn text(v: u32) -> String {
        format!("{MARK}\nexport const X = {v};\n")
    }

    #[test]
    fn bundled_plugin_preflights_all_files_and_rejects_manifest_drift() {
        let root = temp("bundle").parent().unwrap().to_path_buf();
        let files = vec![
            (root.join("vults/__init__.py"), format!("{MARK}\ncode")),
            (root.join("vults/plugin.yaml"), format!("{MARK}\nname: vults")),
        ];
        std::fs::create_dir_all(files[0].0.parent().unwrap()).unwrap();
        std::fs::write(&files[1].0, "user-owned manifest").unwrap();
        assert!(preview_files(&files, true, MARK).is_err());
        assert!(!files[0].0.exists());
        std::fs::remove_file(&files[1].0).unwrap();
        let seen = preview_files(&files, true, MARK).unwrap();
        std::fs::write(&files[1].0, format!("{MARK}\nchanged")).unwrap();
        assert!(apply_files(&files, &seen.fingerprint, true, MARK, SystemTime::now()).is_err());
        assert!(!files[0].0.exists());
        let seen = preview_files(&files, true, MARK).unwrap();
        apply_files(&files, &seen.fingerprint, true, MARK, SystemTime::now()).unwrap();
        assert!(files.iter().all(|(p, t)| up_to_date(p, t)));
        let seen = preview_files(&files, false, MARK).unwrap();
        apply_files(&files, &seen.fingerprint, false, MARK, SystemTime::now()).unwrap();
        assert!(files.iter().all(|(p, _)| !p.exists()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn install_update_and_remove_with_a_backup_each_time() {
        let path = temp("cycle");
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);

        let p = preview(&path, Some(&text(1)), MARK).expect("preview");
        assert!(p.diff.contains("+export const X = 1;"));
        assert_eq!(
            apply(&path, &p.fingerprint, Some(&text(1)), MARK, now).expect("install"),
            None
        );
        assert!(installed(&path, MARK) && up_to_date(&path, &text(1)));
        assert!(!up_to_date(&path, &text(2)));

        let p = preview(&path, Some(&text(2)), MARK).expect("preview");
        assert!(p.diff.contains("-export const X = 1;") && p.diff.contains("+export const X = 2;"));
        let backup = apply(&path, &p.fingerprint, Some(&text(2)), MARK, now)
            .expect("update")
            .expect("a backup");
        assert_eq!(std::fs::read_to_string(&backup).expect("backup"), text(1));
        // Never in the plugin folder, where the agent would find it. Compared canonical: on
        // Windows the temp dir can be a short 8.3 name and the backup a `\\?\` long one.
        let folder = path.parent().expect("plugins");
        let above = |p: &Path| std::fs::canonicalize(p.parent().expect("parent")).expect("canonical");
        let outside = above(folder);
        assert_eq!(above(&backup), outside);
        assert_eq!(std::fs::read_dir(folder).expect("folder").count(), 1);

        let p = preview(&path, None, MARK).expect("preview");
        assert!(p.diff.contains("-export const X = 2;"));
        let backup = apply(&path, &p.fingerprint, None, MARK, now)
            .expect("remove")
            .expect("a backup");
        assert!(!path.exists() && !installed(&path, MARK));
        assert_eq!(above(&backup), outside);
        assert_eq!(std::fs::read_to_string(backup).expect("backup"), text(2));
    }

    #[test]
    fn a_file_of_the_users_is_never_written_nor_removed() {
        let path = temp("foreign");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, "export const Mine = 1;\n").expect("write");
        assert!(blocked(&path, MARK).is_some());
        assert!(matches!(
            preview(&path, Some(&text(1)), MARK),
            Err(Error::Foreign { .. })
        ));
        assert!(matches!(preview(&path, None, MARK), Err(Error::Foreign { .. })));
        let now = SystemTime::now();
        assert!(apply(&path, "x", None, MARK, now).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "export const Mine = 1;\n"
        );
        assert!(!installed(&path, MARK));
    }

    #[test]
    fn a_change_after_the_preview_writes_nothing() {
        let path = temp("changed");
        let p = preview(&path, Some(&text(1)), MARK).expect("preview");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, text(9)).expect("write");
        let now = SystemTime::now();
        assert!(matches!(
            apply(&path, &p.fingerprint, Some(&text(1)), MARK, now),
            Err(Error::Changed { .. })
        ));
        assert_eq!(std::fs::read_to_string(&path).expect("read"), text(9));
    }

    #[test]
    fn removing_what_is_not_there_is_a_no_op() {
        let path = temp("absent");
        let p = preview(&path, None, MARK).expect("preview");
        assert!(p.is_noop() && blocked(&path, MARK).is_none());
        assert_eq!(
            apply(&path, &p.fingerprint, None, MARK, SystemTime::now()).expect("no-op"),
            None
        );
    }
}
