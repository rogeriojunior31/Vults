//! What the audit log keeps (ADR 0014, `docs/dev/plan-zeca.md` S2): who answered a card, how, and
//! on what. `reduce` says it with [`crate::Effect::Audit`] beside every answer; the app writes it to
//! the local database, where rows are never changed and leave only after 90 days.

use serde::Serialize;

/// Longer targets are cut: a whole heredoc is not needed to know what was allowed.
pub const MAX_TARGET: usize = 4096;

/// Who gave the answer.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Actor {
    /// A click (or a key) by the user.
    Human,
    /// One of the user's *Always* rules.
    Rule,
    /// A policy the user wrote and accepted (ADR 0014); none exists yet.
    Policy,
    /// The app itself: the card's time ran out.
    System,
}

/// What the answer did.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Act {
    Allow,
    Deny,
    /// Allowed, and saved as a rule.
    AlwaysAllow,
    /// A question card answered (the answers themselves are not kept).
    Answer,
    /// Sent to the terminal from the card.
    Release,
    /// Nobody answered in time: the terminal asks.
    Expire,
    /// An agent's config or plugin written by the installer.
    ConfigInstall,
    ConfigRemove,
}

impl Actor {
    pub fn name(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Rule => "rule",
            Self::Policy => "policy",
            Self::System => "system",
        }
    }
}

impl Act {
    pub fn name(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::AlwaysAllow => "always-allow",
            Self::Answer => "answer",
            Self::Release => "release",
            Self::Expire => "expire",
            Self::ConfigInstall => "config-install",
            Self::ConfigRemove => "config-remove",
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Audit {
    pub actor: Actor,
    pub act: Act,
    /// The agent's name (`claude`, `codex`…).
    pub agent: String,
    /// The project's folder name; empty when the session has none.
    pub project: String,
    pub tool: String,
    /// The exact command or path, secrets redacted, cut at [`MAX_TARGET`].
    pub target: String,
}

impl Audit {
    pub fn new(actor: Actor, act: Act, agent: &str, project: &str, tool: &str, target: &str) -> Self {
        let mut target = crate::redact::secrets(target);
        if target.len() > MAX_TARGET {
            let mut cut = MAX_TARGET;
            while !target.is_char_boundary(cut) {
                cut -= 1;
            }
            target.truncate(cut);
            target.push('…');
        }
        Self {
            actor,
            act,
            agent: agent.to_owned(),
            project: project.to_owned(),
            tool: tool.to_owned(),
            target,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_is_redacted_and_cut() {
        let a = Audit::new(Actor::Human, Act::Allow, "claude", "site", "Bash", "TOKEN=abc ls");
        assert_eq!(a.target, "TOKEN=[redacted] ls");
        let long = "x".repeat(MAX_TARGET + 10);
        let a = Audit::new(Actor::Human, Act::Allow, "claude", "site", "Bash", &long);
        assert_eq!(a.target.chars().count(), MAX_TARGET + 1);
    }
}
