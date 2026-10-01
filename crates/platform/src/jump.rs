//! Bringing an agent's terminal forward on Linux, in two steps:
//!
//! 1. the multiplexer it runs in (herdr, tmux, kitty, wezterm) switches to its pane, from the
//!    ids the hook passed along;
//! 2. on KDE, a short KWin script activates the terminal window: the first window whose process
//!    is an ancestor of the agent. Wayland lets no app raise another's window; the compositor can.
//!
//! Every value goes to a command as an argument, never through a shell.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use vultures_ai_protocol::Terminal;

pub fn jump(t: &Terminal) {
    multiplexer(&t.env);
    if t.env
        .get("XDG_CURRENT_DESKTOP")
        .is_some_and(|d| d.contains("KDE"))
        && let Some(pid) = t.pid
    {
        let _ = kwin_activate(&ancestors(pid));
    }
}

/// An id we pass on as an argument: no option-looking or odd values.
fn id(v: Option<&String>) -> Option<&str> {
    let v = v?.as_str();
    let ok = !v.is_empty()
        && !v.starts_with('-')
        && v.len() < 128
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || ":_-%@.".contains(c));
    ok.then_some(v)
}

/// What to run, in order, to focus the agent's pane. Pure, for the tests.
fn multiplexer_commands(env: &BTreeMap<String, String>) -> Vec<Vec<String>> {
    let cmd = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let mut out = Vec::new();
    if let Some(ws) = id(env.get("HERDR_WORKSPACE_ID")) {
        out.push(cmd(&["herdr", "workspace", "focus", ws]));
        if let Some(tab) = id(env.get("HERDR_TAB_ID")) {
            out.push(cmd(&["herdr", "tab", "focus", tab]));
        }
    }
    if let Some(pane) = id(env.get("TMUX_PANE")) {
        // $TMUX is "<socket>,<pid>,<session>": talk to that server, not the default one.
        let socket = env
            .get("TMUX")
            .and_then(|t| t.split(',').next())
            .filter(|s| s.starts_with('/'));
        let base: Vec<&str> = match socket {
            Some(s) => vec!["tmux", "-S", s],
            None => vec!["tmux"],
        };
        out.push(cmd(&[&base[..], &["select-window", "-t", pane]].concat()));
        out.push(cmd(&[&base[..], &["select-pane", "-t", pane]].concat()));
    }
    if let Some(win) = id(env.get("KITTY_WINDOW_ID")) {
        out.push(cmd(&[
            "kitty",
            "@",
            "focus-window",
            "--match",
            &format!("id:{win}"),
        ]));
    }
    if let Some(pane) = id(env.get("WEZTERM_PANE")) {
        out.push(cmd(&["wezterm", "cli", "activate-pane", "--pane-id", pane]));
    }
    out
}

fn multiplexer(env: &BTreeMap<String, String>) {
    for argv in multiplexer_commands(env) {
        let _ = Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// `pid` and its parents, nearest first, read from /proc.
fn ancestors(pid: u32) -> Vec<u32> {
    let mut chain = Vec::new();
    let mut current = pid;
    while current > 1 && chain.len() < 32 {
        chain.push(current);
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{current}/stat")) else {
            break;
        };
        // The command name is in parentheses and may contain spaces: read after the last ')'.
        let Some(rest) = stat.rsplit_once(')').map(|(_, r)| r) else {
            break;
        };
        let Some(ppid) = rest.split_whitespace().nth(1).and_then(|p| p.parse().ok()) else {
            break;
        };
        current = ppid;
    }
    chain
}

/// The KWin script: activate the first normal window owned by one of `pids` (nearest first).
fn kwin_script(pids: &[u32]) -> String {
    let list = pids.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    format!(
        "const pids = [{list}];\n\
         const wins = workspace.windowList ? workspace.windowList() : workspace.clientList();\n\
         for (const pid of pids) {{\n\
           const w = wins.find((w) => w.pid === pid && w.normalWindow);\n\
           if (w) {{ if (\"activeWindow\" in workspace) workspace.activeWindow = w; else workspace.activeClient = w; break; }}\n\
         }}\n"
    )
}

fn kwin_activate(pids: &[u32]) -> std::io::Result<()> {
    let name = format!("{}-jump-{}", vultures_ai_brand::SLUG, std::process::id());
    let path = std::env::temp_dir().join(format!("{name}.js"));
    std::fs::write(&path, kwin_script(pids))?;
    let call = |method: &str, args: &[&str]| {
        Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.kde.KWin",
                "--object-path",
                "/Scripting",
                "--method",
            ])
            .arg(format!("org.kde.kwin.Scripting.{method}"))
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    };
    let path_str = path.to_string_lossy().into_owned();
    call("loadScript", &[&path_str, &name])?;
    call("start", &[])?;
    // Give KWin a moment to run it, then leave nothing behind.
    std::thread::sleep(std::time::Duration::from_millis(300));
    let _ = call("unloadScript", &[&name]);
    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn herdr_focuses_workspace_then_tab() {
        let cmds = multiplexer_commands(&env(&[("HERDR_WORKSPACE_ID", "w2"), ("HERDR_TAB_ID", "w2:t5")]));
        assert_eq!(
            cmds,
            vec![
                vec!["herdr", "workspace", "focus", "w2"],
                vec!["herdr", "tab", "focus", "w2:t5"],
            ]
        );
    }

    #[test]
    fn tmux_uses_the_sessions_own_socket() {
        let cmds = multiplexer_commands(&env(&[
            ("TMUX", "/tmp/tmux-1000/work,123,0"),
            ("TMUX_PANE", "%3"),
        ]));
        assert_eq!(
            cmds[0],
            vec!["tmux", "-S", "/tmp/tmux-1000/work", "select-window", "-t", "%3"]
        );
        assert_eq!(
            cmds[1],
            vec!["tmux", "-S", "/tmp/tmux-1000/work", "select-pane", "-t", "%3"]
        );
    }

    #[test]
    fn odd_values_are_never_passed_on() {
        assert!(multiplexer_commands(&env(&[("HERDR_WORKSPACE_ID", "--help")])).is_empty());
        assert!(multiplexer_commands(&env(&[("KITTY_WINDOW_ID", "1; rm -rf /")])).is_empty());
        assert!(multiplexer_commands(&env(&[("WEZTERM_PANE", "")])).is_empty());
    }

    #[test]
    fn ancestors_walk_up_to_init() {
        let chain = ancestors(std::process::id());
        assert_eq!(chain.first(), Some(&std::process::id()));
        assert!(chain.len() >= 2, "{chain:?}");
    }

    #[test]
    fn the_script_lists_pids_nearest_first() {
        let s = kwin_script(&[300, 20]);
        assert!(s.starts_with("const pids = [300,20];"));
        assert!(s.contains("workspace.activeWindow = w"));
    }
}
