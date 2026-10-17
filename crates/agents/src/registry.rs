//! Every agent Vults knows, in one table (`docs/dev/plan-every-agent.md`, F1): its id (the
//! installer's id, a CLI engine, an ACP key), its name, how it travels on the wire, how Settings
//! sets it up, how Vults runs it, and what it can do. The installer, the doctor, the chat's ACP
//! engines and the UI (`ui/src/agents.gen.ts`, generated from here) all read
//! it, so a new agent is a row and its parser. Two places stay apart on purpose: the hook's reply
//! table (`vults-hook` is std and serde_json only) and the protocol's kinds (a wire change).

use vults_protocol::AgentKind;

use crate::caps::{Caps, Diffs, Permissions, Questions, Sandbox, Stop};
use crate::{Agent, PluginAgent};

/// How an agent's events name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    /// Its own kind: its *Always* rules and policies are keyed by it.
    Kind(AgentKind),
    /// `other`, with its id as `agent_name`: watched, never answered by a hook.
    Named,
}

/// What Settings → Agents does with it.
#[derive(Clone, Copy)]
pub enum Setup {
    /// Entries in its config, written by the installer.
    Hooks(&'static dyn Agent),
    /// A whole file of ours that it loads by itself.
    Plugin(&'static PluginAgent),
    /// Tables in its TOML config, written by the installer.
    Toml(&'static crate::TomlAgent),
    /// No longer supported: the installer only takes out what an earlier Vults wrote.
    RemoveOnly(&'static dyn Agent),
    /// Nothing: Vults only runs it.
    None,
}

impl std::fmt::Debug for Setup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Setup::Hooks(_) => "Hooks",
            Setup::Plugin(_) => "Plugin",
            Setup::Toml(_) => "Toml",
            Setup::RemoveOnly(_) => "RemoveOnly",
            Setup::None => "None",
        })
    }
}

/// How Vults starts its CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    /// Its own driver in `vults-chat` (Claude Code's stream-json, Codex's app-server).
    Native,
    /// The Agent Client Protocol, started this way.
    Acp(Acp),
    No,
}

/// An ACP agent's command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acp {
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub env: &'static [(&'static str, &'static str)],
    /// The session mode it is put in once its session starts (`session/set_mode`), when its own
    /// config could start it in one that asks nothing.
    pub mode: Option<&'static str>,
}

/// One agent.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub id: &'static str,
    pub name: &'static str,
    /// The command its CLI is run by.
    pub command: &'static str,
    pub wire: Wire,
    pub setup: Setup,
    pub run: Run,
    /// What it can do with Vults; none for an agent Settings does not list.
    pub caps: Option<Caps>,
}

/// OpenCode runs every tool unasked by default. Its inline config (`OPENCODE_CONFIG_CONTENT`)
/// outranks the user's and the project's, for this process alone: each write, command and fetch
/// asks, in its own agent (never one the project names as the default) and its subagents. Checked
/// with OpenCode 1.18.35.
const OPENCODE_ASK: &str = r#"{"default_agent":"build","permission":{"edit":"ask","bash":"ask","webfetch":"ask","external_directory":"ask","doom_loop":"ask"},"agent":{"build":{"permission":{"edit":"ask","bash":"ask","webfetch":"ask","external_directory":"ask","doom_loop":"ask"}},"plan":{"permission":{"edit":"ask","bash":"ask","webfetch":"ask","external_directory":"ask","doom_loop":"ask"}},"general":{"permission":{"edit":"ask","bash":"ask","webfetch":"ask","external_directory":"ask","doom_loop":"ask"}},"explore":{"permission":{"edit":"ask","bash":"ask","webfetch":"ask","external_directory":"ask","doom_loop":"ask"}}}}"#;

/// Every agent, in the order Settings lists them.
pub static ALL: [Entry; 10] = [
    Entry {
        id: "claude",
        name: "Claude Code",
        command: "claude",
        wire: Wire::Kind(AgentKind::Claude),
        setup: Setup::Hooks(&crate::Claude),
        run: Run::Native,
        caps: Some(Caps {
            permissions: Permissions::Card,
            questions: Questions::Card,
            diffs: Diffs::Numbered,
            stop: Stop::Interrupt,
            sandbox: Sandbox::Bubblewrap,
            starts: true,
        }),
    },
    Entry {
        id: "codex",
        name: "Codex",
        command: "codex",
        wire: Wire::Kind(AgentKind::Codex),
        setup: Setup::Hooks(&crate::Codex),
        run: Run::Native,
        caps: Some(Caps {
            permissions: Permissions::Card,
            // Codex sends no question event: what it asks stays in its terminal.
            questions: Questions::Terminal,
            diffs: Diffs::Lines,
            stop: Stop::Interrupt,
            sandbox: Sandbox::Codex,
            starts: true,
        }),
    },
    Entry {
        id: crate::antigravity::NAME,
        name: "Antigravity",
        command: "agy",
        wire: Wire::Named,
        setup: Setup::Hooks(&crate::Antigravity),
        run: Run::No,
        caps: Some(Caps {
            permissions: Permissions::Terminal,
            questions: Questions::Terminal,
            diffs: Diffs::Lines,
            stop: Stop::No,
            sandbox: Sandbox::No,
            starts: false,
        }),
    },
    Entry {
        id: crate::opencode::NAME,
        name: "OpenCode",
        command: "opencode",
        wire: Wire::Kind(AgentKind::OpenCode),
        setup: Setup::Plugin(&crate::OPENCODE),
        run: Run::Acp(Acp {
            program: "opencode",
            args: &["acp"],
            env: &[("OPENCODE_CONFIG_CONTENT", OPENCODE_ASK)],
            mode: None,
        }),
        caps: Some(Caps {
            permissions: Permissions::Card,
            // OpenCode 1's questions get a card; OpenCode 2's stay in its prompt.
            questions: Questions::Card,
            // Its edits, rebuilt from their old and new text by the plugin.
            diffs: Diffs::Lines,
            stop: Stop::No,
            sandbox: Sandbox::No,
            starts: false,
        }),
    },
    Entry {
        id: "qwen",
        name: "Qwen Code",
        command: "qwen",
        wire: Wire::Kind(AgentKind::Qwen),
        setup: Setup::Hooks(&crate::Qwen),
        // Its prompts, whatever the user's settings say (a flag outranks them): never yolo or
        // auto-edit in a session Vults runs.
        run: Run::Acp(Acp {
            program: "qwen",
            args: &["--acp", "--approval-mode", "default"],
            env: &[],
            mode: None,
        }),
        caps: Some(Caps {
            permissions: Permissions::Card,
            // It asks as a permission it won't honor: the island shows it, its terminal answers.
            questions: Questions::Terminal,
            diffs: Diffs::Lines,
            stop: Stop::No,
            sandbox: Sandbox::Container,
            starts: false,
        }),
    },
    Entry {
        id: crate::copilot::NAME,
        name: "Copilot CLI",
        command: "copilot",
        wire: Wire::Kind(AgentKind::Copilot),
        setup: Setup::Hooks(&crate::Copilot),
        run: Run::Acp(Acp {
            program: "copilot",
            args: &["--acp", "--stdio"],
            env: &[],
            mode: None,
        }),
        caps: Some(Caps {
            permissions: Permissions::Card,
            // It has no question tool.
            questions: Questions::Terminal,
            diffs: Diffs::Lines,
            stop: Stop::No,
            sandbox: Sandbox::No,
            starts: false,
        }),
    },
    Entry {
        id: crate::cursor::NAME,
        name: "Cursor",
        command: "cursor-agent",
        wire: Wire::Named,
        setup: Setup::Hooks(&crate::Cursor),
        run: Run::No,
        caps: Some(Caps {
            // Its hooks that could answer read silence as a decision: it asks in its own prompt.
            permissions: Permissions::Terminal,
            questions: Questions::Terminal,
            diffs: Diffs::Lines,
            stop: Stop::No,
            sandbox: Sandbox::No,
            starts: false,
        }),
    },
    Entry {
        id: crate::droid::NAME,
        name: "Factory Droid",
        command: "droid",
        wire: Wire::Named,
        setup: Setup::Hooks(&crate::Droid),
        run: Run::No,
        caps: Some(Caps {
            // Its PreToolUse runs before every tool, asked or not: it asks in its own prompt.
            permissions: Permissions::Terminal,
            questions: Questions::Terminal,
            diffs: Diffs::Lines,
            stop: Stop::No,
            sandbox: Sandbox::No,
            starts: false,
        }),
    },
    Entry {
        id: crate::kimi::NAME,
        name: "Kimi Code",
        command: "kimi",
        wire: Wire::Named,
        setup: Setup::Toml(&crate::KIMI),
        // A session over ACP says "default" while the user's `default_permission_mode` runs
        // every tool unasked: its mode is set, so each write and command asks (checked with Kimi
        // Code 2.1.1).
        run: Run::Acp(Acp {
            program: "kimi",
            args: &["acp"],
            env: &[],
            mode: Some("default"),
        }),
        caps: Some(Caps {
            // Its PermissionRequest hook is told, never asked: it asks in its own prompt.
            permissions: Permissions::Terminal,
            questions: Questions::Terminal,
            diffs: Diffs::Lines,
            stop: Stop::No,
            sandbox: Sandbox::No,
            starts: false,
        }),
    },
    Entry {
        id: crate::gemini::NAME,
        name: "Gemini CLI",
        command: "gemini",
        wire: Wire::Named,
        setup: Setup::RemoveOnly(&crate::Gemini),
        run: Run::No,
        caps: None,
    },
];

/// The agent with this id.
pub fn of(id: &str) -> Option<&'static Entry> {
    ALL.iter().find(|e| e.id == id)
}

/// The agents Settings → Agents lists with what they can do: the doctor's rows.
pub fn listed() -> impl Iterator<Item = (&'static Entry, Caps)> {
    ALL.iter().filter_map(|e| e.caps.map(|c| (e, c)))
}

impl Entry {
    /// What a session of it carries as `agent`, and as `agent_name` when that is `other`.
    pub fn agent(&self) -> (AgentKind, Option<&'static str>) {
        match self.wire {
            Wire::Kind(kind) => (kind, None),
            Wire::Named => (AgentKind::Other, Some(self.id)),
        }
    }

    /// Whether it is still supported and may be offered as a CLI engine.
    pub fn current(&self) -> bool {
        !matches!(self.setup, Setup::RemoveOnly(_))
    }
}

/// The TypeScript twin of the table, committed as `ui/src/agents.gen.ts`.
pub fn typescript() -> String {
    let mut out = String::from(
        "// Generated by `VULTS_REGEN=1 cargo test -p vults-agents agents_ts`. Do not edit.\n\
         import type { AgentKind } from \"./view.gen\";\n\n\
         /** What Settings → Agents does with it: hooks or a plugin to install, hooks left to remove, or nothing. */\n\
         export type Setup = \"hooks\" | \"plugin\" | \"remove-only\" | \"none\";\n\
         /** How Vults starts it: its own driver, the Agent Client Protocol, or not at all. */\n\
         export type Run = \"native\" | \"acp\" | \"no\";\n\n\
         export interface AgentEntry {\n  id: string;\n  name: string;\n  kind: AgentKind;\n  setup: Setup;\n  run: Run;\n}\n\n\
         export const AGENTS: readonly AgentEntry[] = [\n",
    );
    for e in &ALL {
        let setup = match e.setup {
            Setup::Hooks(_) | Setup::Toml(_) => "hooks",
            Setup::Plugin(_) => "plugin",
            Setup::RemoveOnly(_) => "remove-only",
            Setup::None => "none",
        };
        let run = match e.run {
            Run::Native => "native",
            Run::Acp(_) => "acp",
            Run::No => "no",
        };
        out += &format!(
            "  {{ id: {:?}, name: {:?}, kind: {:?}, setup: {setup:?}, run: {run:?} }},\n",
            e.id,
            e.name,
            e.agent().0.name()
        );
    }
    let union = |keep: &dyn Fn(&Entry) -> bool| {
        ALL.iter()
            .filter(|e| keep(e))
            .map(|e| format!("{:?}", e.id))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    out += "];\n\n/** An agent the installer knows, by the name its hooks run with. */\n";
    out += &format!(
        "export type InstallAgent = {};\n",
        union(&|e| !matches!(e.setup, Setup::None))
    );
    out += "/** An agent with a supported CLI execution mode. */\n";
    out += &format!("export type Engine = {};\n", union(&|e| e.run != Run::No));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/agents.gen.ts");

    #[test]
    fn agents_ts_is_up_to_date() {
        let fresh = typescript();
        if std::env::var_os("VULTS_REGEN").is_some() {
            std::fs::write(PATH, &fresh).expect("write agents.gen.ts");
        }
        let committed = std::fs::read_to_string(PATH).unwrap_or_default();
        assert!(
            committed == fresh,
            "ui/src/agents.gen.ts is stale: run `VULTS_REGEN=1 cargo test -p vults-agents agents_ts`"
        );
    }

    #[test]
    fn the_table_keeps_its_promises() {
        // The table's own claims, pinned so a change to one is a change on purpose: the parsers and
        // the chat's drivers sit in other crates, which this one can't see.
        let ids: Vec<&str> = ALL.iter().map(|e| e.id).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "ids are unique");
        // Settings lists the agents it installs, with what each can do.
        let shown: Vec<&str> = listed().map(|(e, _)| e.id).collect();
        assert_eq!(
            shown,
            [
                "claude",
                "codex",
                "antigravity",
                "opencode",
                "qwen",
                "copilot",
                "cursor",
                "droid",
                "kimi"
            ]
        );
        assert!(
            ALL.iter().all(|e| e.caps.is_some()
                == matches!(e.setup, Setup::Hooks(_) | Setup::Plugin(_) | Setup::Toml(_)))
        );
        // Only the engines the chat runs start from Vults today.
        let starts: Vec<&str> = listed().filter(|(_, c)| c.starts).map(|(e, _)| e.id).collect();
        assert_eq!(starts, ["claude", "codex"]);
        // Only a turn Vults started can be stopped, so only an agent it starts.
        assert!(listed().all(|(_, c)| c.starts || c.stop == Stop::No));
        // Antigravity, Cursor, Droid and Kimi are the agents whose permissions get no card.
        let terminal: Vec<&str> = listed()
            .filter(|(_, c)| c.permissions == Permissions::Terminal)
            .map(|(e, _)| e.id)
            .collect();
        assert_eq!(terminal, ["antigravity", "cursor", "droid", "kimi"]);
        // Kimi Code's session is put in its asking mode: its config's own may ask nothing.
        assert!(matches!(
            of("kimi").map(|e| e.run),
            Some(Run::Acp(Acp {
                mode: Some("default"),
                ..
            }))
        ));
        // Only Claude Code's diffs carry line numbers (its structuredPatch).
        assert_eq!(
            of("claude").and_then(|e| e.caps).map(|c| c.diffs),
            Some(Diffs::Numbered)
        );
        assert!(of("nope").is_none());
    }

    #[test]
    fn each_agent_travels_by_its_own_name() {
        for e in &ALL {
            match e.wire {
                // Its kind is the protocol's, under the same name.
                Wire::Kind(kind) => assert_eq!(AgentKind::parse(e.id), Some(kind), "{}", e.id),
                // A named one goes by a name the protocol lets a tool use, but Gemini CLI's,
                // kept taken so its old hooks send nothing.
                Wire::Named => assert_eq!(vults_protocol::valid_agent_name(e.id), e.current(), "{}", e.id),
            }
            // The installer's agent is the one whose kind it is.
            if let Setup::Hooks(a) | Setup::RemoveOnly(a) = e.setup {
                assert_eq!(a.kind(), e.agent().0, "{}", e.id);
            }
        }
    }
}
