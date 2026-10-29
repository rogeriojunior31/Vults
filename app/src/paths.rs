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

/// The user's config folder, where agents such as OpenCode keep theirs: `$XDG_CONFIG_HOME` when
/// it is absolute (the spec ignores a relative one), else `~/.config`.
pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".config"))
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

/// The shared XDG data folder used by local agent CLIs, independent of Vults' own folder.
pub fn agent_data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home().join(".local").join("share"))
}

/// The hook relay the agents' configs point at: a stable path, whatever rebuilds the app.
pub fn hook_exe() -> PathBuf {
    data_dir().join("bin").join(vults_brand::HOOK_EXE)
}

/// An empty folder for chat turns, so no project's instructions or settings leak in.
pub fn chat_dir() -> PathBuf {
    data_dir().join("chat")
}

/// Zeca's memory, a git repository of Markdown (plan-zeca M1).
pub fn memory_dir() -> PathBuf {
    data_dir().join("memory")
}

/// Copies of the files dropped on the island.
pub fn inbox_dir() -> PathBuf {
    data_dir().join("inbox")
}

/// Each agent's folder variables (`CODEX_HOME`…) as the user's agents see them: this process's,
/// or, for one it lacks (started from the desktop, it never read the shell's startup files), the
/// login shell's. The shell is asked once, off the main thread at start ([`warm_agent_dirs`]).
pub fn agent_dirs() -> vults_agents::Dirs {
    let shell = login_shell_vars();
    vults_agents::Dirs::new(&home(), |name| {
        std::env::var_os(name)
            .filter(|v| !v.is_empty())
            .or_else(|| shell.get(name).cloned())
    })
}

/// Asks the login shell now, so Settings never waits on it, and hands what it sets to the agents
/// the app starts itself ([`vults_agents::set_shell_env`]).
pub fn warm_agent_dirs() {
    vults_agents::set_shell_env(|| only_in_shell(login_shell_vars(), |name| std::env::var_os(name)));
    std::thread::spawn(login_shell_vars);
}

/// The login shell's folder variables that this process lacks (unset or empty): a child the app
/// starts already inherits the others.
fn only_in_shell(
    shell: &std::collections::HashMap<String, std::ffi::OsString>,
    process: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Vec<(String, std::ffi::OsString)> {
    let mut vars: Vec<_> = shell
        .iter()
        .filter(|(name, value)| !value.is_empty() && process(name).is_none_or(|v| v.is_empty()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    vars.sort();
    vars
}

fn login_shell_vars() -> &'static std::collections::HashMap<String, std::ffi::OsString> {
    static VARS: std::sync::OnceLock<std::collections::HashMap<String, std::ffi::OsString>> =
        std::sync::OnceLock::new();
    VARS.get_or_init(|| {
        #[cfg(unix)]
        if let Some(shell) = std::env::var_os("SHELL") {
            return login_shell::read(std::path::Path::new(&shell), vults_agents::DIR_VARS);
        }
        Default::default()
    })
}

/// The user's login shell, interactive (exports often live in `.zshrc` or `.bashrc`), printing its
/// environment after a marker: what its startup files print before it is skipped. Only the names
/// asked for are kept. A shell that fails, prints no marker or takes more than two seconds gives
/// nothing, and the agents' usual folders stand.
#[cfg(unix)]
mod login_shell {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::io::Read;
    use std::os::unix::ffi::OsStringExt;
    use std::path::Path;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    const MARKER: &[u8] = b"\n__vults_env__\n";
    const LIMIT: Duration = Duration::from_secs(2);

    pub fn read(shell: &Path, names: &[&str]) -> HashMap<String, OsString> {
        // printf and env are the same in sh, bash, zsh and fish; a shell that differs gives no marker.
        let script = "printf '\\n__vults_env__\\n'; env -0";
        let child = Command::new(shell)
            .args(["-l", "-i", "-c", script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = child else {
            return HashMap::new();
        };
        let Some(mut out) = child.stdout.take() else {
            return HashMap::new();
        };
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = out.read_to_end(&mut bytes);
            bytes
        });
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if start.elapsed() < LIMIT => std::thread::sleep(Duration::from_millis(20)),
                _ => {
                    // Its own children may keep stdout open: the reader is left, never joined.
                    let _ = child.kill();
                    let _ = child.wait();
                    tracing::info!(shell = %shell.display(), "the login shell took too long; agents keep their usual folders");
                    return HashMap::new();
                }
            }
        }
        reader
            .join()
            .map(|bytes| parse(&bytes, names))
            .unwrap_or_default()
    }

    /// `env -0`'s `NAME=value` records after the last marker, only `names`.
    pub fn parse(bytes: &[u8], names: &[&str]) -> HashMap<String, OsString> {
        let Some(at) = bytes.windows(MARKER.len()).rposition(|w| w == MARKER) else {
            return HashMap::new();
        };
        bytes[at + MARKER.len()..]
            .split(|&b| b == 0)
            .filter_map(|record| {
                let eq = record.iter().position(|&b| b == b'=')?;
                let name = std::str::from_utf8(&record[..eq]).ok()?;
                let value = &record[eq + 1..];
                (names.contains(&name) && !value.is_empty())
                    .then(|| (name.to_string(), OsString::from_vec(value.to_vec())))
            })
            .collect()
    }
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
    use super::{login_shell, move_dir, tilde};
    use std::path::Path;

    /// What the startup files print comes before the marker; a value may hold `=` and newlines.
    #[test]
    fn only_what_the_process_lacks_is_handed_on() {
        let shell = std::collections::HashMap::from([
            ("CODEX_HOME".to_string(), "/w/codex".into()),
            ("CLAUDE_CONFIG_DIR".to_string(), "/w/claude".into()),
            ("QWEN_HOME".to_string(), "/w/qwen".into()),
            ("GEMINI_CLI_HOME".to_string(), "".into()),
        ]);
        let process = |name: &str| match name {
            "CLAUDE_CONFIG_DIR" => Some("/mine".into()),
            "QWEN_HOME" => Some("".into()),
            _ => None,
        };
        assert_eq!(
            super::only_in_shell(&shell, process),
            [
                ("CODEX_HOME".to_string(), "/w/codex".into()),
                ("QWEN_HOME".to_string(), "/w/qwen".into()),
            ]
        );
    }

    #[test]
    fn the_login_shells_variables_are_read_after_the_marker() {
        let out = b"Welcome to fish\nCODEX_HOME=/not/this\n\n__vults_env__\nPATH=/bin\0CODEX_HOME=/w/codex\0QWEN_HOME=\0CLAUDE_CONFIG_DIR=/w/a=b\nc\0";
        let vars = login_shell::parse(out, vults_agents::DIR_VARS);
        assert_eq!(
            vars.get("CODEX_HOME").map(|v| v.as_os_str()),
            Some("/w/codex".as_ref())
        );
        assert_eq!(
            vars.get("CLAUDE_CONFIG_DIR").map(|v| v.as_os_str()),
            Some("/w/a=b\nc".as_ref())
        );
        assert!(!vars.contains_key("PATH"), "only the agents' variables");
        assert!(!vars.contains_key("QWEN_HOME"), "empty is unset");
        assert!(login_shell::parse(b"CODEX_HOME=/w\0", vults_agents::DIR_VARS).is_empty());
    }

    /// A real shell, its startup files and all; one that hangs is given up on.
    #[test]
    fn a_login_shell_is_asked_and_a_hung_one_given_up() {
        let dir = std::env::temp_dir().join(format!("vults-login-shell-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let shell = dir.join("sh");
        std::fs::write(
            &shell,
            "#!/bin/sh\necho noise from a startup file\nexport CODEX_HOME=/w/codex\nexec /bin/sh -c \"$4\"\n",
        )
        .unwrap();
        let hung = dir.join("hung");
        std::fs::write(&hung, "#!/bin/sh\nexec sleep 30\n").unwrap();
        for f in [&shell, &hung] {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(f, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let vars = login_shell::read(&shell, vults_agents::DIR_VARS);
        assert_eq!(
            vars.get("CODEX_HOME").map(|v| v.as_os_str()),
            Some("/w/codex".as_ref())
        );
        let start = std::time::Instant::now();
        assert!(login_shell::read(&hung, vults_agents::DIR_VARS).is_empty());
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        let _ = std::fs::remove_dir_all(&dir);
    }

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
