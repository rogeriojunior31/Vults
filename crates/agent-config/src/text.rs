//! A text file of the user's, edited by the rules of a config edit (rule 3): the diff the user saw,
//! the fingerprint of the bytes it came from, a dated backup, an atomic write. For a file inside a
//! repository (a project's `CLAUDE.md`), the backup goes to a folder of the app's, never beside it:
//! a backup in the working tree would be one more file for `git status` and for the agents.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::{Error, Preview, backup_path, diff, fingerprint, read_bytes, write_file};

/// The diff from the file as it is to what `change` makes of it, and the fingerprint of the bytes
/// it came from.
pub fn preview(path: &Path, change: impl FnOnce(&str) -> String) -> Result<Preview, Error> {
    let bytes = read_bytes(path)?;
    let before = String::from_utf8_lossy(&bytes);
    Ok(Preview {
        diff: diff(&before, &change(&before), path),
        fingerprint: fingerprint(&bytes),
    })
}

/// Writes what `change` makes of the file, if it still has the bytes `fingerprint_seen` was taken
/// from: the text written comes from the very bytes checked, so it is the diff the user saw. A
/// dated backup goes to `backups` first (named after `label` and the file). Returns the backup, or
/// none when there was no file.
pub fn apply(
    path: &Path,
    fingerprint_seen: &str,
    change: impl FnOnce(&str) -> String,
    backups: &Path,
    label: &str,
    now: SystemTime,
) -> Result<Option<PathBuf>, Error> {
    let bytes = read_bytes(path)?;
    if fingerprint(&bytes) != fingerprint_seen {
        return Err(Error::Changed {
            path: path.to_path_buf(),
        });
    }
    let next = change(&String::from_utf8_lossy(&bytes));
    let write_err = |source| Error::Write {
        path: path.to_path_buf(),
        source,
    };
    let original = std::fs::metadata(path).ok();
    let backup = match &original {
        Some(_) => {
            std::fs::create_dir_all(backups).map_err(write_err)?;
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            let backup = backup_path(&backups.join(format!("{label}-{name}")), now);
            std::fs::copy(path, &backup).map_err(write_err)?;
            Some(backup)
        }
        None => None,
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(write_err)?;
    }
    write_file(path, next.as_bytes(), original.as_ref()).map_err(write_err)?;
    // A new project file is the project's: readable like its others, not private to us.
    #[cfg(unix)]
    if original.is_none() {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).map_err(write_err)?;
    }
    Ok(backup)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-text-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_seen_diff_a_backup_elsewhere_and_a_refusal_when_it_changed() {
        let dir = temp("edit");
        let repo = dir.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let file = repo.join("CLAUDE.md");
        std::fs::write(&file, "# Rules\n").unwrap();
        let next = "# Rules\n\n@AGENTS.md\n";
        let add = |t: &str| format!("{t}\n@AGENTS.md\n");
        let p = preview(&file, add).unwrap();
        assert!(p.diff.contains("+@AGENTS.md"), "{}", p.diff);
        let backups = dir.join("backups");
        let backup = apply(&file, &p.fingerprint, add, &backups, "vults", SystemTime::now())
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), next);
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "# Rules\n");
        assert!(backup.starts_with(&backups), "never in the repository");
        assert_eq!(
            std::fs::read_dir(&repo).unwrap().count(),
            1,
            "nothing left beside it"
        );
        // The file changed after the diff was seen: refused, untouched.
        assert!(matches!(
            apply(
                &file,
                &p.fingerprint,
                |_| "other".to_string(),
                &backups,
                "vults",
                SystemTime::now()
            ),
            Err(Error::Changed { .. })
        ));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), next);
        // A file that isn't there is made, with no backup.
        let new = repo.join("NEW.md");
        let line = |_: &str| "@AGENTS.md\n".to_string();
        let p = preview(&new, line).unwrap();
        assert_eq!(
            apply(&new, &p.fingerprint, line, &backups, "vults", SystemTime::now()).unwrap(),
            None
        );
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "@AGENTS.md\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&new).unwrap().permissions().mode() & 0o777,
                0o644
            );
        }
    }
}
