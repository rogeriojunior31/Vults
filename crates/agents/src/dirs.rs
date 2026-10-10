//! Where each agent keeps its files: the folder its own variable names, when set, else its folder
//! under the home. Each variable is read the way its agent reads it.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The variables that move an agent's folder. Claude Code, Codex and Qwen Code take the folder
/// itself (`~/.claude`, `~/.codex`, `~/.qwen`); Gemini CLI puts its `.gemini` inside the one named.
pub const DIR_VARS: &[&str] = &["CLAUDE_CONFIG_DIR", "CODEX_HOME", "GEMINI_CLI_HOME", "QWEN_HOME"];

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

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(set: &[(&str, &str)]) -> Dirs {
        let set: Vec<(String, String)> = set.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Dirs::new(Path::new("/home/me"), move |name| {
            set.iter().find(|(k, _)| k == name).map(|(_, v)| v.into())
        })
    }

    #[test]
    fn a_folder_variable_replaces_the_usual_folder() {
        let d = dirs(&[
            ("CODEX_HOME", "/work/codex"),
            ("CLAUDE_CONFIG_DIR", "~/claude-work"),
        ]);
        assert_eq!(codex_home(&d), Path::new("/work/codex"));
        assert_eq!(
            d.agent("CLAUDE_CONFIG_DIR", ".claude"),
            Path::new("/home/me/claude-work")
        );
        assert_eq!(d.agent("QWEN_HOME", ".qwen"), Path::new("/home/me/.qwen"));
        assert_eq!(
            codex_home(&Dirs::home_only(Path::new("/home/me"))),
            Path::new("/home/me/.codex")
        );
    }

    /// Relative to the folder an agent was started in, a value names no one folder.
    #[test]
    fn a_relative_or_empty_value_is_ignored() {
        let d = dirs(&[("CODEX_HOME", "codex"), ("QWEN_HOME", "")]);
        assert_eq!(codex_home(&d), Path::new("/home/me/.codex"));
        assert_eq!(d.agent("QWEN_HOME", ".qwen"), Path::new("/home/me/.qwen"));
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
