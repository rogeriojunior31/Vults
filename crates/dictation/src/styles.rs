//! Key-down style resolution from application identity, never window titles or executable text.
use crate::cleanup::{Language, Style};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Preferences {
    pub default: Style,
    pub write_in: Option<Language>,
    pub sessions_english: bool,
    pub apps: BTreeMap<String, Style>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            default: Style::Natural,
            write_in: None,
            sessions_english: false,
            apps: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub style: Style,
    pub write_in: Option<Language>,
}

/// Only exact, known desktop aliases are joined; an arbitrary suffix must not impersonate an app.
pub fn app_id(class: &str) -> Option<String> {
    let class = class.trim();
    if class.is_empty() || class.len() > 1024 || class.chars().any(char::is_control) {
        return None;
    }
    let lower = class.to_lowercase();
    let id = lower.strip_suffix(".desktop").unwrap_or(&lower);
    Some(
        match id {
            "org.kde.konsole" => "konsole",
            "org.gnome.terminal" | "org.gnome.terminal-server" | "gnome-terminal-server" => "gnome-terminal",
            "net.kovidgoyal.kitty" => "kitty",
            "org.wezfurlong.wezterm" => "wezterm",
            "com.mitchellh.ghostty" => "ghostty",
            "org.gnome.ptyxis" => "ptyxis",
            "com.visualstudio.code" => "code",
            "com.vscodium.codium" => "codium",
            "org.kde.kate" => "kate",
            "org.kde.kwrite" => "kwrite",
            "org.gnome.texteditor" | "gnome-text-editor" => "gnome-text-editor",
            "org.gnome.gedit" => "gedit",
            "md.obsidian.obsidian" => "obsidian",
            "org.mozilla.thunderbird" => "thunderbird",
            "org.gnome.evolution" => "evolution",
            "org.kde.kmail2" | "kmail2" => "kmail",
            "com.slack.slack" => "slack",
            "com.discordapp.discord" => "discord",
            "org.telegram.desktop" | "telegramdesktop" => "telegram",
            "org.signal.signal" => "signal",
            "im.riot.riot" | "im.riot.riot-web" | "element-desktop" => "element",
            "org.mozilla.firefox" => "firefox",
            _ => id,
        }
        .into(),
    )
}

fn builtin(app: &str, terminal: bool) -> Option<Style> {
    if terminal
        || matches!(
            app,
            "konsole"
                | "kitty"
                | "alacritty"
                | "wezterm"
                | "foot"
                | "footclient"
                | "ghostty"
                | "gnome-terminal"
                | "ptyxis"
                | "xterm"
                | "uxterm"
                | "code"
                | "code-oss"
                | "code-insiders"
                | "codium"
                | "cursor"
                | "kate"
                | "kwrite"
                | "gnome-text-editor"
                | "gedit"
                | "mousepad"
                | "leafpad"
                | "obsidian"
                | "sublime_text"
                | "zed"
        )
    {
        Some(Style::Verbatim)
    } else if matches!(app, "thunderbird" | "evolution" | "kmail" | "geary") {
        Some(Style::Polished)
    } else if matches!(app, "slack" | "discord" | "telegram" | "signal" | "element") {
        Some(Style::Natural)
    } else {
        None
    }
}

pub fn resolve(
    enabled: bool,
    class: Option<&str>,
    terminal: bool,
    session: bool,
    prefs: &Preferences,
) -> Resolved {
    let local = Resolved {
        style: Style::Verbatim,
        write_in: None,
    };
    if !enabled {
        return local;
    }
    let Some(app) = class.and_then(app_id) else {
        return local;
    };
    let style = if session {
        Style::Prompt
    } else {
        prefs
            .apps
            .get(&app)
            .copied()
            .or_else(|| builtin(&app, terminal))
            .unwrap_or(prefs.default)
    };
    let write_in = if style == Style::Verbatim {
        None
    } else if session && prefs.sessions_english {
        Some(Language::English)
    } else {
        prefs.write_in
    };
    Resolved { style, write_in }
}
