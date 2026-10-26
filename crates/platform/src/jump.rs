//! Bringing an agent's terminal forward on Linux, in two steps:
//!
//! 1. the multiplexer it runs in (herdr, tmux, kitty, wezterm) switches to its pane, from the
//!    ids the hook passed along;
//! 2. the window owned by the nearest ancestor of the agent is activated, the best way the
//!    desktop offers ([`methods`]): a short KWin script on Plasma (Wayland lets no app raise
//!    another's window; the compositor can), EWMH on X11 and for XWayland windows.
//!
//! Every value goes to a command as an argument, never through a shell; a script only holds
//! numbers read from /proc and names we checked.

pub(crate) mod kwin;
#[cfg(test)]
mod wezterm_tests;
pub(crate) mod x11;

use std::collections::BTreeMap;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::MetadataExt;
use std::process::{Command, Stdio};

use vults_protocol::Terminal;

/// False when nothing worked: no multiplexer answered and no window was raised.
pub fn jump(t: &Terminal) -> bool {
    jump_if(t, &|| true)
}

pub fn jump_if(t: &Terminal, allowed: &dyn Fn() -> bool) -> bool {
    if !allowed() {
        return false;
    }
    let pane = multiplexer(&t.env, allowed);
    let window = allowed() && t.pid.is_some_and(|pid| raise_window(t, pid, allowed));
    pane || window
}

/// Read-only capability check; an available route still needs a native acknowledgement.
pub fn available(t: &Terminal) -> bool {
    !multiplexer_commands(&t.env).is_empty()
        || t.pid.is_some_and(|pid| {
            !ancestors(pid).is_empty()
                && !methods(
                    t.env.get("XDG_SESSION_TYPE").map_or("", String::as_str),
                    t.env.get("WAYLAND_DISPLAY").map_or("", String::as_str),
                    t.env.get("DISPLAY").map_or("", String::as_str),
                    t.env.get("XDG_CURRENT_DESKTOP").map_or("", String::as_str),
                )
                .is_empty()
        })
}

/// A way to activate another program's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    KWin,
    X11,
}

/// Best first, from the session's environment. On Plasma's Wayland session KWin sees every
/// window, native or XWayland; elsewhere EWMH is the standard way, with KWin as a second try on
/// Plasma's X11 session. On other Wayland desktops only XWayland windows can be reached, so
/// core's `raises` does not promise them. Keep in step with it.
fn methods(session_type: &str, wayland_display: &str, display: &str, desktop: &str) -> Vec<Method> {
    let wayland = session_type.eq_ignore_ascii_case("wayland") || !wayland_display.trim().is_empty();
    let x11 = !display.trim().is_empty();
    let kde = desktop.split(':').any(|d| d.eq_ignore_ascii_case("kde"));
    let mut out = Vec::new();
    if kde && wayland {
        out.push(Method::KWin);
    }
    if x11 {
        out.push(Method::X11);
    }
    if kde && !wayland {
        out.push(Method::KWin);
    }
    out
}

fn raise_window(t: &Terminal, pid: u32, allowed: &dyn Fn() -> bool) -> bool {
    let chain = ancestors(pid);
    if chain.is_empty() {
        return false;
    }
    let folder = t
        .cwd
        .as_deref()
        .and_then(|c| std::path::Path::new(c).file_name())
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let env = |k: &str| t.env.get(k).map_or("", String::as_str);
    methods(
        env("XDG_SESSION_TYPE"),
        env("WAYLAND_DISPLAY"),
        env("DISPLAY"),
        env("XDG_CURRENT_DESKTOP"),
    )
    .into_iter()
    .any(|method| {
        allowed()
            && match method {
                Method::KWin => kwin::activate(&chain, &folder, allowed),
                Method::X11 => x11::activate(&chain, &folder, allowed),
            }
    })
}

/// Among the windows of the session's ancestors (`pid` and title each), the one to raise: the
/// nearest ancestor that has any, and among its windows the one whose title names the session's
/// folder (a terminal or an editor with several windows), else its first.
fn choose_window<'a, W>(ancestors: &[u32], windows: &'a [(W, u32, String)], folder: &str) -> Option<&'a W> {
    let folder = folder.to_lowercase();
    ancestors.iter().find_map(|pid| {
        let mut own = windows.iter().filter(|(_, p, _)| p == pid).peekable();
        let first = &own.peek()?.0;
        let named = if folder.is_empty() {
            None
        } else {
            own.find(|(_, _, title)| title.to_lowercase().contains(&folder))
        };
        Some(named.map_or(first, |(w, _, _)| w))
    })
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

/// kitty's `KITTY_LISTEN_ON`, when it is a Unix socket of plain path characters (a path or an
/// abstract `@name`). `kitty @` run from outside kitty has no other way to reach it.
fn kitty_socket(v: Option<&String>) -> Option<&str> {
    let v = v?.as_str();
    let path = v.strip_prefix("unix:")?;
    let plain = |c: char| c.is_ascii_alphanumeric() || "/_.-@+:".contains(c);
    // 107 bytes: the room in a Unix socket address.
    (!path.is_empty() && path.len() <= 107 && path.chars().all(plain)).then_some(v)
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
    if let (Some(win), Some(to)) = (
        id(env.get("KITTY_WINDOW_ID")),
        kitty_socket(env.get("KITTY_LISTEN_ON")),
    ) {
        out.push(cmd(&[
            "kitty",
            "@",
            "--to",
            to,
            "focus-window",
            "--match",
            &format!("id:{win}"),
        ]));
    }
    if let Some(pane) = id(env.get("WEZTERM_PANE")).filter(|pane| pane.parse::<u64>().is_ok())
        && wezterm_socket(env).is_some()
    {
        out.push(cmd(&[
            "wezterm",
            "--skip-config",
            "cli",
            "--no-auto-start",
            "activate-pane",
            "--pane-id",
            pane,
        ]));
    }
    out
}

/// True when at least one multiplexer command succeeded.
fn multiplexer(env: &BTreeMap<String, String>, allowed: &dyn Fn() -> bool) -> bool {
    let mut any = false;
    for argv in multiplexer_commands(env) {
        if !allowed() {
            break;
        }
        let Some(mut command) = multiplexer_command(&argv, env) else {
            continue;
        };
        let ok = command_if(&mut command, allowed);
        any |= ok;
    }
    any
}

fn command_if(command: &mut Command, allowed: &dyn Fn() -> bool) -> bool {
    if !allowed() {
        return false;
    }
    let Ok(mut child) = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    loop {
        if !allowed() {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

fn wezterm_socket(env: &BTreeMap<String, String>) -> Option<&str> {
    let path = env.get("WEZTERM_UNIX_SOCKET")?.as_str();
    if !path.starts_with('/') || path.len() > 107 || path.chars().any(char::is_control) {
        return None;
    }
    let meta = std::fs::symlink_metadata(path).ok()?;
    (meta.file_type().is_socket() && meta.uid() == my_uid()?).then_some(path)
}

fn multiplexer_command(argv: &[String], env: &BTreeMap<String, String>) -> Option<Command> {
    let mut command = Command::new(argv.first()?);
    command.args(&argv[1..]);
    if argv[0] == "wezterm" {
        // A pane number belongs to one server; ambient/default discovery can select another.
        command.env("WEZTERM_UNIX_SOCKET", wezterm_socket(env)?);
        command.env_remove("WEZTERM_PANE");
    }
    Some(command)
}

/// Our own user id, read from /proc.
pub(crate) fn my_uid() -> Option<u32> {
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
}

/// `pid` and its parents, nearest first, read from /proc. The walk stops at a process of another
/// user (sshd, a display manager, sudo): no window of theirs is the session's.
pub(crate) fn ancestors(pid: u32) -> Vec<u32> {
    let Some(uid) = my_uid() else {
        return Vec::new();
    };
    ancestors_with(pid, |current| {
        let dir = format!("/proc/{current}");
        if std::fs::metadata(&dir).ok()?.uid() != uid {
            return None;
        }
        let parent = std::fs::read_to_string(format!("{dir}/stat"))
            .ok()
            .and_then(|stat| {
                // A command name may contain spaces or parentheses.
                stat.rsplit_once(')')?.1.split_whitespace().nth(1)?.parse().ok()
            });
        Some(parent)
    })
}

pub(crate) fn ancestors_with(pid: u32, mut parent: impl FnMut(u32) -> Option<Option<u32>>) -> Vec<u32> {
    let mut chain = Vec::new();
    let mut current = pid;
    while current > 1 && chain.len() < 32 && !chain.contains(&current) {
        let Some(next) = parent(current) else { break };
        chain.push(current);
        let Some(next) = next else { break };
        current = next;
    }
    chain
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
    fn kitty_is_reached_through_its_socket_or_not_at_all() {
        let cmds = multiplexer_commands(&env(&[
            ("KITTY_WINDOW_ID", "3"),
            ("KITTY_LISTEN_ON", "unix:/tmp/kitty-4242"),
        ]));
        assert_eq!(
            cmds,
            vec![vec![
                "kitty",
                "@",
                "--to",
                "unix:/tmp/kitty-4242",
                "focus-window",
                "--match",
                "id:3"
            ]]
        );
        let abstract_socket = env(&[("KITTY_WINDOW_ID", "3"), ("KITTY_LISTEN_ON", "unix:@mykitty")]);
        assert_eq!(multiplexer_commands(&abstract_socket)[0][3], "unix:@mykitty");
        assert!(multiplexer_commands(&env(&[("KITTY_WINDOW_ID", "3")])).is_empty());
        let long = format!("unix:/{}", "a".repeat(107));
        for bad in [
            "tcp:localhost:5000",
            "unix:/tmp/a b",
            "unix:$(rm)",
            "unix:",
            "/tmp/kitty",
            &long,
        ] {
            let e = env(&[("KITTY_WINDOW_ID", "3"), ("KITTY_LISTEN_ON", bad)]);
            assert!(multiplexer_commands(&e).is_empty(), "{bad}");
        }
    }

    #[test]
    fn odd_values_are_never_passed_on() {
        assert!(multiplexer_commands(&env(&[("HERDR_WORKSPACE_ID", "--help")])).is_empty());
        let kitty = env(&[
            ("KITTY_WINDOW_ID", "1; rm -rf /"),
            ("KITTY_LISTEN_ON", "unix:/tmp/k"),
        ]);
        assert!(multiplexer_commands(&kitty).is_empty());
        assert!(multiplexer_commands(&env(&[("WEZTERM_PANE", "")])).is_empty());
    }

    #[test]
    fn wezterm_navigation_pins_the_hook_socket_without_loading_config_or_starting_a_server() {
        use std::os::unix::{fs::symlink, net::UnixListener};
        let path = std::env::temp_dir().join(format!("vults-wezterm-jump-{}", std::process::id()));
        let listener = UnixListener::bind(&path).expect("owned temporary socket");
        let value = path.to_str().expect("socket path");
        let context = env(&[("WEZTERM_PANE", "17"), ("WEZTERM_UNIX_SOCKET", value)]);
        let commands = multiplexer_commands(&context);
        assert_eq!(
            commands,
            [vec![
                "wezterm",
                "--skip-config",
                "cli",
                "--no-auto-start",
                "activate-pane",
                "--pane-id",
                "17"
            ]]
        );
        let command = multiplexer_command(&commands[0], &context).expect("command");
        assert!(
            command
                .get_envs()
                .any(|(key, val)| key == "WEZTERM_UNIX_SOCKET" && val == Some(std::ffi::OsStr::new(value)))
        );
        assert!(
            command
                .get_envs()
                .any(|(key, val)| key == "WEZTERM_PANE" && val.is_none())
        );
        assert!(multiplexer_commands(&env(&[("WEZTERM_PANE", "17")])).is_empty());
        for pane in ["", "--help", "1;bad", "%17", "default:1"] {
            assert!(
                multiplexer_commands(&env(&[("WEZTERM_PANE", pane), ("WEZTERM_UNIX_SOCKET", value)]))
                    .is_empty()
            );
        }
        let link = path.with_extension("link");
        symlink(&path, &link).expect("test symlink");
        assert!(
            multiplexer_commands(&env(&[
                ("WEZTERM_PANE", "17"),
                ("WEZTERM_UNIX_SOCKET", link.to_str().expect("link"))
            ]))
            .is_empty()
        );
        std::fs::remove_file(link).expect("remove link");
        drop(listener);
        std::fs::remove_file(&path).expect("remove socket");
        assert!(
            multiplexer_command(&commands[0], &context).is_none(),
            "a disappeared socket must not start or discover a replacement"
        );
    }

    #[test]
    fn the_best_way_comes_first_for_each_desktop() {
        use Method::*;
        // Plasma on Wayland: KWin sees everything; XWayland as a second try.
        assert_eq!(methods("wayland", "wayland-0", ":0", "KDE"), [KWin, X11]);
        assert_eq!(methods("", "wayland-0", "", "KDE"), [KWin]);
        // Plasma on X11: EWMH, then KWin.
        assert_eq!(methods("x11", "", ":0", "KDE"), [X11, KWin]);
        // Any X11 session.
        assert_eq!(methods("x11", "", ":0", "XFCE"), [X11]);
        assert_eq!(methods("", "", ":0", "i3"), [X11]);
        // GNOME or wlroots on Wayland: only XWayland windows can be reached.
        assert_eq!(methods("wayland", "wayland-0", ":0", "ubuntu:GNOME"), [X11]);
        assert_eq!(methods("wayland", "wayland-0", "", "GNOME"), []);
        assert_eq!(methods("tty", "", "", ""), []);
    }

    #[test]
    fn the_nearest_ancestor_with_a_window_wins_then_the_folders_title() {
        let windows = [
            ("editor", 30, "notes - Code".to_string()),
            ("other", 20, "~/elsewhere".to_string()),
            ("mine", 20, "Claude: Vults-Jump".to_string()),
        ];
        // 10 (the agent) owns none; 20 (the terminal) owns two, one titled after the folder.
        assert_eq!(
            choose_window(&[10, 20, 30], &windows, "vults-jump"),
            Some(&"mine")
        );
        assert_eq!(choose_window(&[10, 20, 30], &windows, "nowhere"), Some(&"other"));
        assert_eq!(choose_window(&[10, 20, 30], &windows, ""), Some(&"other"));
        assert_eq!(choose_window(&[30], &windows, "vults-jump"), Some(&"editor"));
        assert_eq!(choose_window(&[99], &windows, ""), None);
    }

    #[test]
    fn ancestors_walk_up_to_our_last_process() {
        let chain = ancestors(std::process::id());
        assert_eq!(chain.first(), Some(&std::process::id()));
        assert!(!chain.contains(&1), "{chain:?}");
        assert!(ancestors(u32::MAX - 1).is_empty());
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[test]
    fn closed_admission_never_connects_and_a_cancelled_child_is_reaped() {
        let target = Terminal::default();
        assert!(!available(&target));
        assert!(!jump_if(&target, &|| false));
        assert!(!kwin::activate(&[std::process::id()], "", &|| false));
        assert!(!x11::activate(&[std::process::id()], "", &|| false));
        let checks = AtomicUsize::new(0);
        let started = std::time::Instant::now();
        assert!(!command_if(Command::new("/bin/sleep").arg("2"), &|| checks
            .fetch_add(1, Ordering::SeqCst)
            < 3));
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert!(checks.load(Ordering::SeqCst) >= 3);
        assert!(command_if(&mut Command::new("/bin/true"), &|| true));
        assert!(!command_if(&mut Command::new("/bin/false"), &|| true));
    }
}
