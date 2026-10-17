//! What each agent can do with Vults (`docs/dev/plan-zeca.md` W1): whether its permissions get a
//! card, where its questions go, what a diff shows, how it could be stopped, its own sandbox, and
//! whether Vults can start it. Each agent's row is in [`crate::registry`], read by the Nest's
//! doctor, the router (W8), which only sends work an agent can take, and the policy (W2), which
//! only answers where a card does. Each row is what the code does today (the parsers beside this
//! file) and the stop research (plan-zeca.md C7); a capability nobody has tried is `No`, never a
//! guess.

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

/// Whether Vults can stop a turn of the agent and keep its session (a signal ends the whole
/// session). A session Vults only watches can't be stopped from here yet: a hook reply with
/// `continue: false` is documented by Claude Code (C7) but neither built nor tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stop {
    /// At once, as Esc in its terminal, but only a turn Vults started: Claude Code's control
    /// `interrupt`, the Codex app-server's `turn/interrupt` (not its `codex exec` fallback).
    Interrupt,
    No,
}

/// The agent's own sandbox for the commands it runs. None was tried with Vults; Claude Code's and
/// the containers are off until the user turns them on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sandbox {
    /// Claude Code's sandbox (bubblewrap on Linux).
    Bubblewrap,
    /// Codex's sandbox mode (bubblewrap on Linux): `workspace-write` writes only in its folder,
    /// the temp folders and the roots it is given; `danger-full-access` has none.
    Codex,
    /// A container, with `--sandbox` (Docker or Podman).
    Container,
    No,
}

/// What one agent can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Caps {
    pub permissions: Permissions,
    pub questions: Questions,
    pub diffs: Diffs,
    pub stop: Stop,
    pub sandbox: Sandbox,
    /// Vults can run it itself (Chat and continued sessions).
    pub starts: bool,
}
