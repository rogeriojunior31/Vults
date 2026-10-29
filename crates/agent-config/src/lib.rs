//! Edits of files that belong to an agent (`~/.claude/settings.json`, `~/.codex/hooks.json`,
//! `~/.gemini/config/hooks.json`, `~/.cursor/hooks.json`, a TOML `config.toml`).
//!
//! The rule is strict: read, show the user a diff, and write only what they saw, after a
//! dated backup. Anything we cannot read or parse is refused, never treated as empty:
//! that is how a whole settings file gets replaced by our three lines.

pub mod flat;
mod hooks;
pub mod json_bundle;
pub mod named;
pub mod plugin;
pub mod status_line;
pub mod text;
pub mod toml;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::Value;

pub use hooks::{HookEntry, has_ours, hooks_taken, our_command, ours_match, remove_ours, with_ours};

/// A hook command runs ours: it names `marker`, or the hook binary from before the rename,
/// so an update replaces those entries instead of adding a second set.
pub fn runs_ours(command: &str, marker: &str) -> bool {
    command.contains(marker) || command.contains(vults_brand::LEGACY_HOOK_BIN)
}

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
    /// A TOML file that does not parse.
    InvalidToml {
        path: PathBuf,
        reason: String,
    },
    /// A symlink to a file that doesn't exist (dotfiles not checked out yet): writing would
    /// replace the link with a plain file.
    DanglingLink {
        path: PathBuf,
        target: PathBuf,
    },
    /// A file we would own whole (a plugin) is there without our mark: it is the user's.
    Foreign {
        path: PathBuf,
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
            Error::InvalidToml { path, reason } => write!(
                f,
                "{} isn't valid TOML ({reason}); fix or move it, then try again. Nothing was written",
                path.display()
            ),
            Error::DanglingLink { path, target } => write!(
                f,
                "{} is a link to {}, which doesn't exist; create it or remove the link, then try again. Nothing was written",
                path.display(),
                target.display()
            ),
            Error::Foreign { path } => write!(
                f,
                "{} is not one of ours, so it was left untouched; move it away to install",
                path.display()
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
        // The file's BOM and CRLF are kept on write (`styled_as`): not a change, so not shown.
        let before = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
        let before = String::from_utf8_lossy(before).replace("\r\n", "\n");
        diff(&before, &rendered(&next), path)
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
    replace(
        path,
        &styled_as(&bytes, &rendered(&change(&current))),
        now,
        Backups::Beside,
    )
}

/// `text` written the way `original` was: with its byte-order mark and its CRLF line ends, so a
/// file saved by a Windows editor comes back in its own style and the diff shows only our keys.
/// A raw newline only ever ends a line in JSON (one in a string is `\n`), so every one is a line end.
fn styled_as(original: &[u8], text: &str) -> Vec<u8> {
    const BOM: &[u8] = b"\xEF\xBB\xBF";
    let bom = original.starts_with(BOM);
    // The first line end says the file's style.
    let crlf = original
        .iter()
        .position(|&b| b == b'\n')
        .is_some_and(|i| i > 0 && original[i - 1] == b'\r');
    let mut out = Vec::with_capacity(text.len() + BOM.len());
    if bom {
        out.extend_from_slice(BOM);
    }
    if crlf {
        out.extend_from_slice(text.replace('\n', "\r\n").as_bytes());
    } else {
        out.extend_from_slice(text.as_bytes());
    }
    out
}

/// Where a dated backup goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backups {
    /// Beside the file: `settings.json.bak-…`.
    Beside,
    /// In the folder above the file's: for a folder the agent loads every file of (a plugin
    /// folder), where a backup beside it would be one more thing to load or to trip over.
    AboveFolder,
}

/// Writes `bytes` over `path` after a dated backup of what is there: beside the target and
/// renamed over it, so a crash leaves the original intact. Returns the backup path.
fn replace(path: &Path, bytes: &[u8], now: SystemTime, backups: Backups) -> Result<Option<PathBuf>, Error> {
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
    let backup = back_up(&target, original.is_some(), now, backups).map_err(write_err)?;
    write_file(&target, bytes, original.as_ref()).map_err(write_err)?;
    Ok(backup)
}

/// A temp file's name beside `target`, never the same twice: two writes at once, from this
/// process or another, each get their own.
fn temp_beside(target: &Path) -> PathBuf {
    static COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("config");
    target.with_file_name(format!(".{name}.tmp-{}-{n}-{nanos}", std::process::id()))
}

/// Puts `bytes` in `target` so a crash leaves either the old text or the new one: written to a
/// temp file beside it, synced, and renamed over it. A file with other names (hard links: a
/// dotfiles manager's, a backup tool's `cp -al`) is written in place instead, since a rename
/// would split this name from the others, each left with its own text.
pub(crate) fn write_file(
    target: &Path,
    bytes: &[u8],
    original: Option<&std::fs::Metadata>,
) -> std::io::Result<()> {
    let temp = temp_beside(target);
    let written = write_private(&temp, bytes, original);
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    if original.is_some_and(hard_linked) {
        match std::fs::OpenOptions::new().write(true).open(target) {
            Ok(file) => return write_in_place(file, bytes, &temp),
            // Read-only (0444): renamed over, as before, which splits it from its other names.
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
            Err(e) => {
                let _ = std::fs::remove_file(&temp);
                return Err(e);
            }
        }
    }
    std::fs::rename(&temp, target).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

#[cfg(unix)]
fn hard_linked(meta: &std::fs::Metadata) -> bool {
    std::os::unix::fs::MetadataExt::nlink(meta) > 1
}

/// Windows' link count is not in std yet: every file is renamed over there.
#[cfg(not(unix))]
fn hard_linked(_: &std::fs::Metadata) -> bool {
    false
}

/// Writes over `file` and cuts it to length. Not atomic, so `temp` already holds the whole new
/// text, synced: a crash part way leaves it there, and a failed write names it in the error.
/// Written over, never emptied first: a reader part way sees a file that fails to parse, not an
/// empty config it would load as "nothing set".
fn write_in_place(mut file: std::fs::File, bytes: &[u8], temp: &Path) -> std::io::Result<()> {
    use std::io::Write;

    let written = file
        .write_all(bytes)
        .and_then(|()| file.set_len(bytes.len() as u64))
        .and_then(|()| file.sync_all());
    match written {
        Ok(()) => {
            let _ = std::fs::remove_file(temp);
            Ok(())
        }
        Err(e) => Err(std::io::Error::new(
            e.kind(),
            format!("{e}; the new text is kept in {}", temp.display()),
        )),
    }
}

/// A dated copy of `target`, where `backups` says, when there is one.
fn back_up(
    target: &Path,
    exists: bool,
    now: SystemTime,
    backups: Backups,
) -> std::io::Result<Option<PathBuf>> {
    if !exists {
        return Ok(None);
    }
    let above = (backups == Backups::AboveFolder)
        .then(|| target.parent()?.parent())
        .flatten();
    let backup = match (above, target.file_name()) {
        (Some(dir), Some(name)) => backup_path(&dir.join(name), now),
        _ => backup_path(target, now),
    };
    std::fs::copy(target, &backup)?;
    Ok(Some(backup))
}

/// Creates `temp` readable only by us, then gives it the original's mode: a settings file
/// the user locked to 0600 must not come back world-readable after an install.
fn write_private(temp: &Path, bytes: &[u8], original: Option<&std::fs::Metadata>) -> std::io::Result<()> {
    use std::io::Write;

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

/// `new` with its keys in `old`'s order wherever both have them: an entry of ours that the user
/// or a tool re-sorted keeps its key order (keys only they added go), and only the values that
/// changed show in the diff.
pub(crate) fn in_order_of(old: &Value, new: Value) -> Value {
    match (old, new) {
        (Value::Object(old), Value::Object(mut new)) => {
            let mut out = serde_json::Map::new();
            for (key, o) in old {
                if let Some(n) = new.shift_remove(key) {
                    out.insert(key.clone(), in_order_of(o, n));
                }
            }
            out.extend(new);
            Value::Object(out)
        }
        (Value::Array(old), Value::Array(new)) => Value::Array(
            new.into_iter()
                .enumerate()
                .map(|(i, n)| match old.get(i) {
                    Some(o) => in_order_of(o, n),
                    None => n,
                })
                .collect(),
        ),
        (_, new) => new,
    }
}

fn diff(before: &str, after: &str, path: &Path) -> String {
    unified(before, after, &path.display().to_string())
}

/// The unified diff the user sees before a click, headed `name`; empty when nothing changes. Also
/// what a policy's consent is given to (`app/src/policies.rs`), so it must stay deterministic.
pub fn unified(before: &str, after: &str, name: &str) -> String {
    if before == after {
        return String::new();
    }
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .context_radius(3)
        .header(name, name)
        .to_string()
}

/// FNV-1a: the only question is "is this still the file I showed the user?".
fn fingerprint(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// `settings.json.bak-20261001-191914Z`, and `…Z-2` on for more edits in the same second: a
/// backup is never overwritten, or the second edit's would replace the true original.
fn backup_path(path: &Path, now: SystemTime) -> PathBuf {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("config");
    let base = format!("{name}.bak-{}", utc_stamp(now));
    (1..)
        .map(|n| match n {
            1 => path.with_file_name(&base),
            n => path.with_file_name(format!("{base}-{n}")),
        })
        .find(|p| !p.exists())
        .expect("an unused backup name")
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
        let dir = std::env::temp_dir().join(format!("vults-config-{}-{name}", std::process::id()));
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
    fn two_edits_in_one_second_keep_the_true_original() {
        let path = temp("twice");
        std::fs::write(&path, b"{}").unwrap();
        let now = SystemTime::now();
        let plan = preview(&path, add_model).unwrap();
        let first = apply(&path, &plan.fingerprint, add_model, now).unwrap().unwrap();
        let plan = preview(&path, |v| v.clone()).unwrap();
        let second = apply(&path, &plan.fingerprint, |v| v.clone(), now)
            .unwrap()
            .unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(&first).unwrap(), b"{}");
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

    /// Only the temp files of a write still under way would be left: none once it returned.
    fn temps_beside(path: &Path) -> Vec<String> {
        std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp-"))
            .collect()
    }

    #[test]
    fn a_temp_name_is_never_used_twice() {
        let path = temp("temps");
        let names: std::collections::HashSet<PathBuf> = (0..1000).map(|_| temp_beside(&path)).collect();
        assert_eq!(names.len(), 1000);
        for _ in 0..3 {
            let plan = preview(&path, add_model).unwrap();
            apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap();
            std::fs::write(&path, b"{}").unwrap();
        }
        assert_eq!(temps_beside(&path), Vec::<String>::new());
    }

    /// A rename would leave the other name on the old text, a file of its own.
    #[cfg(unix)]
    #[test]
    fn a_hard_linked_config_keeps_all_its_names() {
        use std::os::unix::fs::MetadataExt;
        let path = temp("hardlink");
        let other = path.with_file_name("dotfiles-settings.json");
        std::fs::write(&path, b"{}").unwrap();
        std::fs::hard_link(&path, &other).unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();

        let plan = preview(&path, add_model).unwrap();
        apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap();

        assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
        assert_eq!(std::fs::metadata(&other).unwrap().ino(), inode);
        assert_eq!(read_json(&other).unwrap(), json!({ "model": "opus" }));
        assert_eq!(temps_beside(&path), Vec::<String>::new());

        // Shorter than before: cut to its own length, no old tail left.
        let plan = preview(&path, |_| json!({})).unwrap();
        apply(&path, &plan.fingerprint, |_| json!({}), SystemTime::now()).unwrap();
        assert_eq!(std::fs::read(&other).unwrap(), b"{}\n");
    }

    /// A Windows editor's file: its BOM and CRLF come back, and the diff shows only our key.
    #[test]
    fn a_files_bom_and_line_ends_are_kept() {
        let path = temp("crlf");
        std::fs::write(&path, "\u{feff}{\r\n  \"theme\": \"dark\"\r\n}\r\n").unwrap();
        let plan = preview(&path, add_model).unwrap();
        assert_eq!(
            plan.diff
                .lines()
                .filter(|l| l.starts_with(['+', '-']) && !l.starts_with("+++") && !l.starts_with("---"))
                .collect::<Vec<_>>(),
            [
                "-  \"theme\": \"dark\"",
                "+  \"theme\": \"dark\",",
                "+  \"model\": \"opus\""
            ]
        );
        assert!(!plan.diff.contains('\r'));
        apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "\u{feff}{\r\n  \"theme\": \"dark\",\r\n  \"model\": \"opus\"\r\n}\r\n"
        );

        // A plain file stays plain.
        let path = temp("lf");
        std::fs::write(&path, "{\n  \"theme\": \"dark\"\n}\n").unwrap();
        let plan = preview(&path, add_model).unwrap();
        apply(&path, &plan.fingerprint, add_model, SystemTime::now()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\n  \"theme\": \"dark\",\n  \"model\": \"opus\"\n}\n"
        );
    }

    #[test]
    fn no_change_means_an_empty_diff() {
        let path = temp("noop");
        std::fs::write(&path, b"{}").unwrap();
        assert!(preview(&path, Value::clone).unwrap().is_noop());
    }
}
