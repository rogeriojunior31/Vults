//! Where the app keeps its own files: `~/.local/share/vults/` (Linux),
//! `%LOCALAPPDATA%\Vults\` (Windows).

use std::path::PathBuf;

pub fn home() -> PathBuf {
    std::env::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// A path to show: under $HOME it starts with `~`.
pub fn shown(path: &std::path::Path) -> String {
    tilde(path, &home())
}

fn tilde(path: &std::path::Path, home: &std::path::Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if home.is_absolute() && home != std::path::Path::new("/") => {
            std::path::Path::new("~").join(rest).display().to_string()
        }
        _ => path.display().to_string(),
    }
}

pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home);
        return base.join(vults_brand::NAME);
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local").join("share"))
        .join(vults_brand::SLUG)
}

/// The hook relay the agents' configs point at: a stable path, whatever rebuilds the app.
pub fn hook_exe() -> PathBuf {
    data_dir().join("bin").join(vults_brand::HOOK_EXE)
}

/// An empty folder for chat turns, so no project's instructions or settings leak in.
pub fn chat_dir() -> PathBuf {
    data_dir().join("chat")
}

/// Copies of the files dropped on the island.
pub fn inbox_dir() -> PathBuf {
    data_dir().join("inbox")
}

/// Moves the folders an install from before the rename left (`~/.config/vultures-ai`, …) to the
/// new names, once: a folder already there under the new name wins and the old one stays.
/// Runs first, before the log or the settings open their files.
pub fn move_legacy() {
    use vults_brand::{LEGACY_NAME, LEGACY_SLUG, NAME, SLUG};
    let var = |name: &str, fallback: PathBuf| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or(fallback)
    };
    let moved_data = if cfg!(windows) {
        let local = var("LOCALAPPDATA", home());
        move_dir(&var("APPDATA", home()), LEGACY_NAME, NAME);
        move_dir(&local, LEGACY_NAME, NAME)
    } else {
        let h = home();
        move_dir(&var("XDG_CONFIG_HOME", h.join(".config")), LEGACY_SLUG, SLUG);
        move_dir(&var("XDG_STATE_HOME", h.join(".local/state")), LEGACY_SLUG, SLUG);
        move_dir(&var("XDG_CACHE_HOME", h.join(".cache")), LEGACY_SLUG, SLUG);
        move_dir(&var("XDG_DATA_HOME", h.join(".local/share")), LEGACY_SLUG, SLUG)
    };
    // The agents' configs still run the old hook path until the user updates them from
    // Settings: leave a link there to the new hook, so their hooks keep reaching the app.
    #[cfg(unix)]
    if moved_data {
        let new = hook_exe();
        let old = new.parent().and_then(|bin| bin.parent()?.parent()).map(|base| {
            base.join(LEGACY_SLUG)
                .join("bin")
                .join(vults_brand::LEGACY_HOOK_BIN)
        });
        if let Some(old) = old
            && old.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok())
        {
            let _ = std::os::unix::fs::symlink(&new, &old);
        }
    }
    #[cfg(not(unix))]
    let _ = moved_data;
}

/// `base/old` renamed to `base/new` when only the old one is there.
fn move_dir(base: &std::path::Path, old: &str, new: &str) -> bool {
    let (from, to) = (base.join(old), base.join(new));
    from.is_dir() && !to.exists() && std::fs::rename(&from, &to).is_ok()
}

// Unix paths: on Windows "/home/z" is not absolute and the separator is "\\".
#[cfg(all(test, unix))]
mod tests {
    use super::{move_dir, tilde};
    use std::path::Path;

    #[test]
    fn home_is_shortened_and_elsewhere_is_kept() {
        let home = Path::new("/home/z");
        assert_eq!(
            tilde(Path::new("/home/z/.config/x/settings.json"), home),
            "~/.config/x/settings.json"
        );
        assert_eq!(tilde(Path::new("/home/zz/x"), home), "/home/zz/x");
        assert_eq!(tilde(Path::new("/srv/data/x"), home), "/srv/data/x");
        assert_eq!(tilde(Path::new("/srv/x"), Path::new("/")), "/srv/x");
        // No home found falls back to "."; a relative path is no home.
        assert_eq!(tilde(Path::new("./.config/x"), Path::new(".")), "./.config/x");
    }

    #[test]
    fn a_legacy_folder_moves_once_and_never_over_a_new_one() {
        let base = std::env::temp_dir().join(format!("vults-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("old")).unwrap();
        std::fs::write(base.join("old/settings.json"), "{}").unwrap();
        assert!(move_dir(&base, "old", "new"));
        assert!(base.join("new/settings.json").exists() && !base.join("old").exists());
        assert!(!move_dir(&base, "old", "new"), "nothing left to move");
        std::fs::create_dir_all(base.join("old")).unwrap();
        assert!(!move_dir(&base, "old", "new"), "the new folder wins");
        assert!(base.join("old").exists());
        std::fs::remove_dir_all(&base).unwrap();
    }
}
