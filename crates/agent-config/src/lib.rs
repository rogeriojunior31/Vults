//! Edits of files that belong to an agent (`~/.claude/settings.json`, `~/.codex/hooks.json`).
//!
//! The rule is strict: read, show the user a diff, and write only what they saw, after a
//! dated backup. Anything we cannot read or parse is refused, never treated as empty:
//! that is how a whole settings file gets replaced by our three lines.

mod hooks;
pub mod status_line;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::Value;

pub use hooks::{HookEntry, has_ours, remove_ours, with_ours};

#[derive(Debug)]
pub enum Error {
    /// The file exists but could not be read (lock, permissions, bad drive).
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Valid JSON, but not an object.
    NotAnObject {
        path: PathBuf,
    },
    Invalid {
        path: PathBuf,
        reason: String,
    },
    /// A symlink to a file that doesn't exist (dotfiles not checked out yet): writing would
    /// replace the link with a plain file.
    DanglingLink {
        path: PathBuf,
        target: PathBuf,
    },
    /// The file changed after the user saw the diff.
    Changed {
        path: PathBuf,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreadable { path, source } => write!(f, "can't read {}: {source}", path.display()),
            Error::NotAnObject { path } => {
                write!(
                    f,
                    "{} isn't a JSON object, so it was left untouched",
                    path.display()
                )
            }
            Error::Invalid { path, reason } => write!(
                f,
                "{} isn't valid JSON ({reason}); fix or move it, then try again. Nothing was written",
                path.display()
            ),
            Error::DanglingLink { path, target } => write!(
                f,
                "{} is a link to {}, which doesn't exist; create it or remove the link, then try again. Nothing was written",
                path.display(),
                target.display()
            ),
            Error::Changed { path } => {
                write!(
                    f,
                    "{} changed since the preview. Nothing was written: review the new diff",
                    path.display()
                )
            }
            Error::Write { path, source } => write!(f, "can't write {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

/// What a change would do, for the user to review before [`apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Unified diff from the file as it is now to exactly what a write puts there; empty when nothing changes.
    pub diff: String,
    /// Identifies the exact bytes the diff was computed from; hand it back to [`apply`].
    pub fingerprint: String,
}

impl Preview {
    pub fn is_noop(&self) -> bool {
        self.diff.is_empty()
    }
}

/// Reads a JSON object file. A missing or blank file is `{}`; anything else unusable is an error.
pub fn read_json(path: &Path) -> Result<Value, Error> {
    let bytes = read_bytes(path)?;
    parse_json(&bytes, path)
}

pub fn preview(path: &Path, change: impl FnOnce(&Value) -> Value) -> Result<Preview, Error> {
    let bytes = read_bytes(path)?;
    let current = parse_json(&bytes, path)?;
    let next = change(&current);
    // Against the bytes on disk, so the diff shows everything the write changes, formatting
    // included; nothing to change is an empty diff however the file is laid out.
    let diff = if next == current {
        String::new()
    } else {
        let before = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
        diff(&String::from_utf8_lossy(before), &rendered(&next), path)
    };
    Ok(Preview {
        diff,
        fingerprint: fingerprint(&bytes),
    })
}

/// Applies `change` if the file still has the bytes `fingerprint` was taken from.
/// Returns the backup path, or `None` when there was no file to back up.
pub fn apply(
    path: &Path,
    fingerprint_seen: &str,
    change: impl FnOnce(&Value) -> Value,
    now: SystemTime,
) -> Result<Option<PathBuf>, Error> {
    // Read before anything else: an unreadable file must stop us before we touch the disk.
    let bytes = read_bytes(path)?;
    let current = parse_json(&bytes, path)?;
    if fingerprint(&bytes) != fingerprint_seen {
        return Err(Error::Changed {
            path: path.to_path_buf(),
        });
    }
    let write_err = |source| Error::Write {
        path: path.to_path_buf(),
        source,
    };

    // A config kept in a dotfiles repo is often a symlink: renaming over the link would turn
    // it into a plain file and the repo copy would silently stop getting our changes.
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir).map_err(write_err)?;
    }
    let original = std::fs::metadata(&target).ok();
    let backup = if original.is_some() {
        let backup = backup_path(&target, now);
        std::fs::copy(&target, &backup).map_err(write_err)?;
        Some(backup)
    } else {
        None
    };

    let text = rendered(&change(&current));
    // Write beside the target and rename over it: a crash leaves the original intact.
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("config");
    let temp = target.with_file_name(format!(".{name}.tmp-{}", std::process::id()));
    let written = write_private(&temp, text.as_bytes(), original.as_ref())
        .and_then(|()| std::fs::rename(&temp, &target));
    if let Err(source) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(write_err(source));
    }
    Ok(backup)
}

/// Creates `temp` readable only by us, then gives it the original's mode: a settings file
/// the user locked to 0600 must not come back world-readable after an install.
fn write_private(temp: &Path, bytes: &[u8], original: Option<&std::fs::Metadata>) -> std::io::Result<()> {
    use std::io::Write;

    let _ = std::fs::remove_file(temp); // A leftover from a crash would keep its old mode.
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(temp)?;
    file.write_all(bytes)?;
    if let Some(original) = original {
        file.set_permissions(original.permissions())?;
    }
    file.sync_all()
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, Error> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => match std::fs::read_link(path) {
            Ok(target) => Err(Error::DanglingLink {
                path: path.to_path_buf(),
                target,
            }),
            // Truly missing: an empty config, created on write.
            Err(_) => Ok(Vec::new()),
        },
        Err(source) => Err(Error::Unreadable {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn parse_json(bytes: &[u8], path: &Path) -> Result<Value, Error> {
    // PowerShell's `Set-Content -Encoding utf8` writes a BOM, which serde_json refuses.
    let text = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if text.iter().all(u8::is_ascii_whitespace) {
        return Ok(Value::Object(Default::default()));
    }
    match serde_json::from_slice::<Value>(text) {
        Ok(v) if v.is_object() => Ok(v),
        Ok(_) => Err(Error::NotAnObject {
            path: path.to_path_buf(),
        }),
        Err(e) => Err(Error::Invalid {
            path: path.to_path_buf(),
            reason: e.to_string(),
        }),
    }
}

/// The exact text a write puts in the file. Keys keep their order (serde_json's
/// `preserve_order`): a config is the user's, not ours to sort.
fn rendered(v: &Value) -> String {
    let mut text = serde_json::to_string_pretty(v).unwrap_or_default();
    text.push('\n');
    text
}

fn diff(before: &str, after: &str, path: &Path) -> String {
    if before == after {
        return String::new();
    }
    let name = path.display().to_string();
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .context_radius(3)
        .header(&name, &name)
        .to_string()
}

/// FNV-1a: the only question is "is this still the file I showed the user?".
fn fingerprint(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// `settings.json.bak-20261001-191914Z`, down to the second so two edits in a minute keep both.
fn backup_path(path: &Path, now: SystemTime) -> PathBuf {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("config");
    path.with_file_name(format!("{name}.bak-{}", utc_stamp(now)))
}

fn utc_stamp(now: SystemTime) -> String {
    let secs = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Civil date from days since 1970-01-01 (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vultures-ai-config-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("settings.json")
    }

    fn add_model(v: &Value) -> Value {
        let mut v = v.clone();
        v["model"] = json!("opus");
        v
    }

    #[test]
    fn a_bom_is_stripped_and_blank_files_are_empty() {
        let p = Path::new("x.json");
        assert_eq!(
            parse_json(b"\xEF\xBB\xBF{\"a\":1}", p).unwrap(),
            json!({ "a": 1 })
        );
        assert_eq!(parse_json(b"  \n\t", p).unwrap(), json!({}));
    }

    #[test]
    fn unusable_content_is_an_error_never_empty() {
        let p = Path::new("x.json");
        assert!(matches!(parse_json(b"{ nope", p), Err(Error::Invalid { .. })));
        assert!(matches!(parse_json(b"[1]", p), Err(Error::NotAnObject { .. })));
    }

    #[test]
    fn utc_stamps() {
        assert_eq!(utc_stamp(SystemTime::UNIX_EPOCH), "19700101-000000Z");
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_790_883_554); // 2026-10-01 19:39:14 UTC
        assert_eq!(utc_stamp(t), "20261001-193914Z");
        let leap = SystemTime::UNIX_EPOCH + Duration::from_secs(951_782_400); // 2000-02-29
        assert_eq!(utc_stamp(leap), "20000229-000000Z");
    }

    #[test]
    fn preview_then_apply_backs_up_and_writes() {
        let path = temp("apply");
        let original = b"\xEF\xBB\xBF{\"theme\":\"dark\"}";
        std::fs::write(&path, original).unwrap();

        let plan = preview(&path, add_model).unwrap();
        assert!(plan.diff.contains("+  \"model\": \"opus\""), "{}", plan.diff);
        let backup = apply(&path, &plan.fingerprint, add_model, SystemTime::now())
            .unwrap()
            .unwrap();

        assert_eq!(std::fs::read(&backup).unwrap(), original);
        assert_eq!(
            read_json(&path).unwrap(),
            json!({ "theme": "dark", "model": "opus" })
        );
        // No temp file left behind.
        assert_eq!(std::fs::read_dir(path.parent().unwrap()).unwrap().count(), 2);
    }

    #[test]
    fn the_keys_keep_their_order() {
        let path = temp("order");
        let original = "{\n  \"theme\": \"dark\",\n  \"hooks\": {\n    \"Stop\": []\n  },\n  \"env\": {\n    \"Z\": \"1\",\n    \"A\": \"2\"\n  }\n}\n";
        std::fs::write(&path, original).unwrap();
        let plan = preview(&path, add_model).unwrap();
        apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap();
        // Everything where it was; the new key last.
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\n  \"theme\": \"dark\",\n  \"hooks\": {\n    \"Stop\": []\n  },\n  \"env\": {\n    \"Z\": \"1\",\n    \"A\": \"2\"\n  },\n  \"model\": \"opus\"\n}\n"
        );
    }

    #[test]
    fn the_diff_is_what_gets_written() {
        // Formatted its own way: writing reformats it, and the diff has to say so.
        let path = temp("format");
        let original = "{\"theme\": \"dark\",\n    \"tui\": \"fullscreen\"}";
        std::fs::write(&path, original).unwrap();
        let plan = preview(&path, add_model).unwrap();
        apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(plan.diff, diff(original, &written, &path));
        // Nothing to change stays an empty diff, however the file is formatted.
        assert!(preview(&path, Value::clone).unwrap().is_noop());
        std::fs::write(&path, original).unwrap();
        assert!(preview(&path, Value::clone).unwrap().is_noop());
    }

    #[test]
    fn a_missing_file_is_created_without_a_backup() {
        let path = temp("missing");
        let plan = preview(&path, add_model).unwrap();
        assert_eq!(
            apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap(),
            None
        );
        assert_eq!(read_json(&path).unwrap(), json!({ "model": "opus" }));
    }

    #[test]
    fn a_file_changed_after_the_preview_is_left_alone() {
        let path = temp("changed");
        std::fs::write(&path, b"{}").unwrap();
        let plan = preview(&path, add_model).unwrap();
        std::fs::write(&path, b"{\"edited\":true}").unwrap();
        let err = apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap_err();
        assert!(matches!(err, Error::Changed { .. }));
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"edited\":true}");
    }

    #[test]
    fn broken_json_is_never_written() {
        let path = temp("broken");
        std::fs::write(&path, b"{ broken").unwrap();
        assert!(preview(&path, add_model).is_err());
        assert!(apply(&path, &fingerprint(b"{ broken"), add_model, SystemTime::now()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{ broken");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_config_stays_a_link() {
        let link = temp("symlink");
        let dir = link.parent().unwrap();
        let real = dir.join("dotfiles").join("settings.json");
        std::fs::create_dir_all(real.parent().unwrap()).unwrap();
        std::fs::write(&real, b"{}").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let plan = preview(&link, add_model).unwrap();
        let backup = apply(&link, &plan.fingerprint, add_model, SystemTime::now())
            .unwrap()
            .unwrap();

        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(read_json(&real).unwrap(), json!({ "model": "opus" }));
        assert_eq!(backup.parent(), real.parent());
        assert_eq!(std::fs::read(&backup).unwrap(), b"{}");
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_nothing_is_refused_and_kept() {
        // Dotfiles not checked out yet: writing would turn the link into a plain file.
        let link = temp("dangling");
        let missing = link.parent().unwrap().join("dotfiles").join("settings.json");
        std::os::unix::fs::symlink(&missing, &link).unwrap();

        assert!(matches!(
            preview(&link, add_model),
            Err(Error::DanglingLink { .. })
        ));
        assert!(matches!(read_json(&link), Err(Error::DanglingLink { .. })));
        let fingerprint = fingerprint(b"");
        assert!(matches!(
            apply(&link, &fingerprint, add_model, SystemTime::now()),
            Err(Error::DanglingLink { .. })
        ));
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert!(!missing.exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_original_mode_is_kept() {
        use std::os::unix::fs::PermissionsExt;
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;

        for wanted in [0o600, 0o644] {
            let path = temp(&format!("mode-{wanted:o}"));
            std::fs::write(&path, b"{}").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(wanted)).unwrap();
            let plan = preview(&path, add_model).unwrap();
            apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap();
            assert_eq!(mode(&path), wanted);
        }

        // A file we create is ours alone.
        let path = temp("mode-new");
        let plan = preview(&path, add_model).unwrap();
        apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap();
        assert_eq!(mode(&path), 0o600);
    }

    #[test]
    fn no_change_means_an_empty_diff() {
        let path = temp("noop");
        std::fs::write(&path, b"{}").unwrap();
        assert!(preview(&path, Value::clone).unwrap().is_noop());
    }
}
