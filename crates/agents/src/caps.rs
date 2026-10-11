//! What each agent can do with Vults (`docs/dev/plan-zeca.md` W1): whether its permissions get a
//! card, where its questions go, what a diff shows, how it could be stopped, its own sandbox, and
//! whether Vults can start it. One table, read by the Nest's doctor, and later by the router (W8),
//! which only sends work an agent can take, and the policy (W2), which only answers where a card
//! does. Each row is what the code does today (the parsers beside this file) and the stop research
//! (road-to-0.2 C7); a capability nobody has tried is `No`, never a guess.

use serde::Serialize;

/// Where an agent's permission request goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Permissions {
    /// A card on the island answers it.
    Card,
    /// Its own terminal asks; the island only shows it.
    Terminal,
}

/// Where an agent's questions (choices it asks the user) go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Questions {
    Card,
    Terminal,
}

/// What the island shows of an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Diffs {
    /// The lines changed, with their line numbers.
    Numbered,
    /// The lines changed, rebuilt from the edit, without numbers.
    Lines,
    /// Nothing of the edit.
    None,
}

/// How a turn of the agent could be stopped without a signal (a signal ends the whole session).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stop {
    /// Our hook's reply before its next tool says `continue: false`: the turn ends there, the
    /// session stays. A turn that only writes text can't be stopped this way.
    BeforeNextTool,
    /// The app-server's `turn/interrupt`, as Esc in its terminal: only for a session Vults started.
    Interrupt,
    /// No way found.
    No,
}

/// The agent's own sandbox for the commands it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sandbox {
    /// Claude Code's sandbox (bubblewrap on Linux).
    Bubblewrap,
    /// Codex's own (Landlock and seccomp on Linux): writes only inside its folder.
    Codex,
    /// A container, with `--sandbox` (Docker or Podman).
    Container,
    No,
}

/// One agent's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Caps {
    /// The installer's id (`claude`, `codex`…).
    pub id: &'static str,
    pub name: &'static str,
    /// The command its CLI is run by.
    pub command: &'static str,
    pub permissions: Permissions,
    pub questions: Questions,
    pub diffs: Diffs,
    pub stop: Stop,
    pub sandbox: Sandbox,
    /// Vults can run it itself (the chat's engines today; tasks from W4).
    pub starts: bool,
}

/// Every agent Vults knows by name, in the order Settings lists them.
pub const ALL: [Caps; 6] = [
    Caps {
        id: "claude",
        name: "Claude Code",
        command: "claude",
        permissions: Permissions::Card,
        questions: Questions::Card,
        diffs: Diffs::Numbered,
        stop: Stop::BeforeNextTool,
        sandbox: Sandbox::Bubblewrap,
        starts: true,
    },
    Caps {
        id: "codex",
        name: "Codex",
        command: "codex",
        permissions: Permissions::Card,
        // Codex sends no question event: what it asks stays in its terminal.
        questions: Questions::Terminal,
        diffs: Diffs::Lines,
        stop: Stop::Interrupt,
        sandbox: Sandbox::Codex,
        starts: true,
    },
    Caps {
        id: "gemini",
        name: "Gemini CLI",
        command: "gemini",
        // Its hooks can't approve a tool: its terminal always asks.
        permissions: Permissions::Terminal,
        questions: Questions::Terminal,
        diffs: Diffs::Lines,
        // BeforeTool's `continue: false` (documented, untried here).
        stop: Stop::BeforeNextTool,
        sandbox: Sandbox::Container,
        starts: false,
    },
    Caps {
        id: "antigravity",
        name: "Antigravity",
        command: "agy",
        permissions: Permissions::Terminal,
        questions: Questions::Terminal,
        diffs: Diffs::None,
        // Only through a hook Vults does not register (C7).
        stop: Stop::No,
        sandbox: Sandbox::No,
        starts: false,
    },
    Caps {
        id: "opencode",
        name: "OpenCode",
        command: "opencode",
        permissions: Permissions::Card,
        // OpenCode 1's questions get a card; OpenCode 2's stay in its prompt.
        questions: Questions::Card,
        diffs: Diffs::None,
        stop: Stop::No,
        sandbox: Sandbox::No,
        starts: false,
    },
    Caps {
        id: "qwen",
        name: "Qwen Code",
        command: "qwen",
        permissions: Permissions::Card,
        // It asks as a permission it won't honor: the island shows it, its terminal answers.
        questions: Questions::Terminal,
        diffs: Diffs::Lines,
        stop: Stop::No,
        sandbox: Sandbox::Container,
        starts: false,
    },
];

/// An agent's row by the installer's id.
pub fn of(id: &str) -> Option<&'static Caps> {
    ALL.iter().find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_matches_what_the_parsers_and_the_drivers_do() {
        // One row per agent Settings installs, the ids the installer knows.
        let ids: Vec<&str> = ALL.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            ["claude", "codex", "gemini", "antigravity", "opencode", "qwen"]
        );
        // Only the engines the chat runs start from Vults today.
        let starts: Vec<&str> = ALL.iter().filter(|c| c.starts).map(|c| c.id).collect();
        assert_eq!(starts, ["claude", "codex"]);
        // Gemini CLI and Antigravity are the agents whose permissions get no card.
        let terminal: Vec<&str> = ALL
            .iter()
            .filter(|c| c.permissions == Permissions::Terminal)
            .map(|c| c.id)
            .collect();
        assert_eq!(terminal, ["gemini", "antigravity"]);
        // Only Claude Code's diffs carry line numbers (its structuredPatch).
        assert_eq!(of("claude").map(|c| c.diffs), Some(Diffs::Numbered));
        assert_eq!(of("nope"), None);
    }

    #[test]
    fn the_row_names_what_the_protocol_names() {
        // The protocol's agent names, but Antigravity's, which travels as `other`.
        for c in ALL.iter().filter(|c| c.id != "antigravity") {
            assert!(vults_protocol::AgentKind::parse(c.id).is_some(), "{}", c.id);
        }
    }
}
