//! Where each agent keeps its files: the folder its own variable names, when set, else its folder
//! under the home. Each variable is read the way its agent reads it.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The variables that move an agent's folder. Claude Code, Codex and Qwen Code take the folder
/// itself (`~/.claude`, `~/.codex`, `~/.qwen`); `GEMINI_CLI_HOME` is still read to find the hooks
/// Gemini CLI kept, which only the installer removes now.
pub const DIR_VARS: &[&str] = &[
    "CLAUDE_CONFIG_DIR",
    "CODEX_HOME",
    "GEMINI_CLI_HOME",
    "QWEN_HOME",
    "COPILOT_HOME",
    // Factory Droid's: a home of its own, its `.factory` inside.
    "FACTORY_HOME_OVERRIDE",
    // Kimi Code's folder itself (`~/.kimi-code`).
    "KIMI_CODE_HOME",
];

#[derive(Debug, Clone)]
pub struct Dirs {
    home: PathBuf,
    vars: BTreeMap<&'static str, PathBuf>,
}

impl Dirs {
    /// `var` reads one variable; only [`DIR_VARS`] are asked for. `~/` is the home. A value that
    /// is still relative names no one folder (it depends on where the agent started): ignored.
    pub fn new(home: &Path, var: impl Fn(&str) -> Option<OsString>) -> Dirs {
        let vars = DIR_VARS
            .iter()
            .filter_map(|&name| {
                let value = PathBuf::from(var(name)?);
                let value = match value.strip_prefix("~") {
                    Ok(rest) => home.join(rest),
                    Err(_) => value,
                };
                value.is_absolute().then_some((name, value))
            })
            .collect();
        Dirs {
            home: home.to_path_buf(),
            vars,
        }
    }

    /// No variable set: every agent in its usual folder.
    pub fn home_only(home: &Path) -> Dirs {
        Dirs::new(home, |_| None)
    }

    /// `$var` when set, else `<home>/<default>`.
    pub fn agent(&self, var: &str, default: &str) -> PathBuf {
        self.vars
            .get(var)
            .cloned()
            .unwrap_or_else(|| self.home.join(default))
    }

    /// The variable's folder, if set.
    pub fn var(&self, var: &str) -> Option<&Path> {
        self.vars.get(var).map(PathBuf::as_path)
    }

    pub fn home(&self) -> &Path {
        &self.home
    }
}

/// Codex's folder: its config.toml and hooks.json.
pub fn codex_home(dirs: &Dirs) -> PathBuf {
    dirs.agent("CODEX_HOME", ".codex")
}

type ShellEnv = Box<dyn Fn() -> Vec<(String, OsString)> + Send + Sync>;
static SHELL_ENV: std::sync::OnceLock<ShellEnv> = std::sync::OnceLock::new();

/// Where an agent the app starts itself (the chat's `claude` and `codex`, the usage read) gets
/// the [`DIR_VARS`] the user's login shell sets and this process lacks: started from the desktop,
/// the app never saw them, and that agent would use other folders than the one in the terminal.
/// Set once, by the app; a second call is ignored.
pub fn set_shell_env(env: impl Fn() -> Vec<(String, OsString)> + Send + Sync + 'static) {
    let _ = SHELL_ENV.set(Box::new(env));
}

/// The variables to give an agent the app starts (see [`set_shell_env`]); none when unset.
pub fn shell_env() -> Vec<(String, OsString)> {
    SHELL_ENV.get().map(|env| env()).unwrap_or_default()
}

/// `path` made absolute on this OS: unchanged on Unix, under `C:` on Windows, where a path with
/// no drive is relative.
#[cfg(test)]
pub(crate) fn abs(path: &str) -> PathBuf {
    PathBuf::from(if cfg!(windows) {
        format!("C:{path}")
    } else {
        path.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(set: &[(&str, &str)]) -> Dirs {
        let set: Vec<(String, PathBuf)> = set
            .iter()
            .map(|(k, v)| (k.to_string(), if v.starts_with('/') { abs(v) } else { v.into() }))
            .collect();
        Dirs::new(&abs("/home/me"), move |name| {
            set.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone().into())
        })
    }

    #[test]
    fn a_folder_variable_replaces_the_usual_folder() {
        let d = dirs(&[
            ("CODEX_HOME", "/work/codex"),
            ("CLAUDE_CONFIG_DIR", "~/claude-work"),
        ]);
        assert_eq!(codex_home(&d), abs("/work/codex"));
        assert_eq!(
            d.agent("CLAUDE_CONFIG_DIR", ".claude"),
            abs("/home/me/claude-work")
        );
        assert_eq!(d.agent("QWEN_HOME", ".qwen"), abs("/home/me/.qwen"));
        assert_eq!(
            codex_home(&Dirs::home_only(&abs("/home/me"))),
            abs("/home/me/.codex")
        );
    }

    /// Relative to the folder an agent was started in, a value names no one folder.
    #[test]
    fn a_relative_or_empty_value_is_ignored() {
        let d = dirs(&[("CODEX_HOME", "codex"), ("QWEN_HOME", "")]);
        assert_eq!(codex_home(&d), abs("/home/me/.codex"));
        assert_eq!(d.agent("QWEN_HOME", ".qwen"), abs("/home/me/.qwen"));
    }

    #[test]
    fn only_the_agents_variables_are_read() {
        let asked = std::cell::RefCell::new(Vec::new());
        Dirs::new(Path::new("/h"), |name| {
            asked.borrow_mut().push(name.to_string());
            None
        });
        assert_eq!(*asked.borrow(), DIR_VARS);
    }
}
