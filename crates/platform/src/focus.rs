//! Read-only foreground context. An observation never authorizes typing into a window or pane.

pub(crate) mod kwin;
pub(crate) mod x11;

use vults_protocol::Terminal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub class: String,
    pub title: String,
    pub pid: Option<u32>,
    /// X11 window id or KWin's internal UUID; titles are not identities.
    pub identity: String,
}

impl Window {
    pub fn is_terminal(&self) -> bool {
        let class = self.class.to_ascii_lowercase();
        let name = class.strip_suffix(".desktop").unwrap_or(&class);
        let name = name.rsplit('.').next().unwrap_or(name);
        matches!(class.as_str(), "org.gnome.terminal" | "org.gnome.terminal-server")
            || matches!(
                name,
                "konsole"
                    | "kitty"
                    | "alacritty"
                    | "wezterm"
                    | "foot"
                    | "footclient"
                    | "ghostty"
                    | "gnome-terminal"
                    | "gnome-terminal-server"
                    | "ptyxis"
                    | "xterm"
                    | "uxterm"
            )
            || (matches!(name, "code" | "code-oss" | "code-insiders")
                && self.title.to_ascii_lowercase().contains("terminal"))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Method {
    X11,
    KWin,
}

fn method(session: &str, wayland: &str, display: &str, desktop: &str) -> Option<Method> {
    if session.eq_ignore_ascii_case("wayland") || !wayland.trim().is_empty() {
        // XWayland's active-window property can name an app that is no longer foreground.
        desktop
            .split(':')
            .any(|d| d.eq_ignore_ascii_case("kde"))
            .then_some(Method::KWin)
    } else {
        (!display.trim().is_empty()).then_some(Method::X11)
    }
}

/// Call off the UI thread immediately on key-down. Missing, changed-during-read or late
/// observations are unknown. KWin's asynchronous observation is context, not proof of key-down focus.
/// ponytail: one-shot reads can lag key-down; exact targeting needs compositor focus tracking.
pub fn read() -> Option<Window> {
    let env = |key| std::env::var(key).unwrap_or_default();
    match method(
        &env("XDG_SESSION_TYPE"),
        &env("WAYLAND_DISPLAY"),
        &env("DISPLAY"),
        &env("XDG_CURRENT_DESKTOP"),
    )? {
        Method::X11 => x11::read(),
        Method::KWin => kwin::read(),
    }
}

/// Only an unambiguous ancestry match is context for a flock session. Shared terminal servers,
/// tabs and multiplexers do not establish which pane has focus, even with just one known session.
pub fn session<'a, K>(window: &Window, sessions: &'a [(K, Terminal)]) -> Option<&'a K> {
    session_with(window.pid?, sessions, crate::jump::ancestors)
}

fn session_with<K>(
    pid: u32,
    sessions: &[(K, Terminal)],
    mut ancestors: impl FnMut(u32) -> Vec<u32>,
) -> Option<&K> {
    if pid <= 1 {
        return None;
    }
    let mut matches = sessions
        .iter()
        .filter(|(_, terminal)| terminal.pid.is_some_and(|agent| ancestors(agent).contains(&pid)));
    let (key, _) = matches.next()?;
    matches.next().is_none().then_some(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wayland_never_guesses_from_xwayland() {
        assert_eq!(method("wayland", "wayland-0", ":0", "GNOME"), None);
        assert_eq!(method("wayland", "", ":0", "sway"), None);
        assert_eq!(method("", "wayland-0", ":0", "KDE"), Some(Method::KWin));
        assert_eq!(method("x11", "", ":0", "KDE"), Some(Method::X11));
        assert_eq!(method("", "", "", ""), None);
    }
    #[test]
    fn terminal_context_is_conservative() {
        let mut w = Window {
            class: String::new(),
            title: String::new(),
            pid: None,
            identity: "x11:7".into(),
        };
        for name in [
            "org.kde.konsole",
            "kitty",
            "Alacritty",
            "org.wezfurlong.wezterm",
            "foot",
            "com.mitchellh.ghostty",
            "Gnome-terminal",
            "org.gnome.Terminal",
            "org.gnome.Ptyxis",
            "XTerm",
        ] {
            w.class = name.into();
            assert!(w.is_terminal(), "{name}");
        }
        w.class = "Code".into();
        assert!(!w.is_terminal());
        w.title = "Terminal - Visual Studio Code".into();
        assert!(w.is_terminal());
        w.class = "firefox".into();
        assert!(!w.is_terminal());
    }
    #[test]
    fn ancestry_stops_at_foreign_missing_cyclic_and_root_processes() {
        let parents = [
            (40, Some(30)),
            (30, Some(20)),
            (20, Some(1)),
            (60, Some(61)),
            (61, Some(60)),
            (70, None),
        ];
        let walk = |pid| {
            crate::jump::ancestors_with(pid, |p| {
                parents.iter().find(|(id, _)| *id == p).map(|(_, parent)| *parent)
            })
        };
        assert_eq!(walk(40), [40, 30, 20]);
        assert_eq!(walk(60), [60, 61]);
        assert_eq!(walk(70), [70]);
        assert!(walk(1).is_empty());
        assert!(walk(99).is_empty());
        let sessions = [
            (
                "one",
                Terminal {
                    pid: Some(40),
                    ..Default::default()
                },
            ),
            (
                "two",
                Terminal {
                    pid: Some(60),
                    ..Default::default()
                },
            ),
        ];
        assert_eq!(session_with(20, &sessions, walk), Some(&"one"));
        assert_eq!(session_with(30, &sessions, walk), Some(&"one"));
        assert_eq!(session_with(2, &sessions, walk), None);
        let shared = [
            ("one", sessions[0].1.clone()),
            (
                "two",
                Terminal {
                    pid: Some(30),
                    ..Default::default()
                },
            ),
        ];
        assert_eq!(session_with(20, &shared, walk), None);
    }
}
