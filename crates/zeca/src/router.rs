//! The router (`docs/dev/plan-zeca.md` W8): which agent a delegated task goes to, and why. A
//! roost's own rule (or Claude Code), unless that agent's plan is above [`QUOTA`] and the other's
//! is known to be under it; after [`LEARNED`] finished tasks of a kind in a roost, the agent that
//! finished more of them there replaces the rule for that kind. The quota still holds then: an
//! agent near its limit isn't sent work.

/// Above this share of its plan, an agent's work goes to the other.
pub const QUOTA: u8 = 85;
/// Finished tasks of a kind in a roost before its scores replace the roost's rule for that kind.
pub const LEARNED: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    Claude,
    Codex,
}

impl Agent {
    pub fn other(self) -> Self {
        match self {
            Self::Claude => Self::Codex,
            Self::Codex => Self::Claude,
        }
    }
}

/// What a task is about, from its goal's words: scores are kept per kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Fix,
    Tests,
    Docs,
    Feature,
}

/// A task's kind from its goal: its first matching word, else a feature.
pub fn kind(goal: &str) -> Kind {
    let words: Vec<String> = goal
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect();
    // Whole words: `Dockerfile` is no doc, `fixtures` no fix.
    let has = |list: &[&str]| words.iter().any(|w| list.contains(&w.as_str()));
    if has(&[
        "fix",
        "fixes",
        "fixed",
        "bug",
        "bugs",
        "crash",
        "crashes",
        "broken",
        "flaky",
        "regression",
    ]) {
        Kind::Fix
    } else if has(&["test", "tests", "testing", "coverage", "spec", "specs"]) {
        Kind::Tests
    } else if has(&[
        "doc",
        "docs",
        "documentation",
        "readme",
        "translate",
        "translation",
        "changelog",
    ]) {
        Kind::Docs
    } else {
        Kind::Feature
    }
}

/// A finished task, as the router learns from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Past {
    pub agent: Agent,
    pub roost: String,
    pub kind: Kind,
    /// Done (rather than dropped or left blocked).
    pub done: bool,
}

/// Why the router chose as it did: what the task's card says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The roost's own rule.
    Rule,
    /// No rule: Claude Code.
    Default,
    /// The agent the rule or score picked is above the quota; the other isn't.
    Quota { busy: Agent, used: u8 },
    /// The roost has learned: this agent finished `done` of `of` tasks of this kind there.
    Score { done: usize, of: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub agent: Agent,
    pub why: Why,
}

/// Which agent a task of `kind` in `roost` goes to. `quota(agent)` is its plan's highest window
/// in use, when known.
pub fn choose(
    roost: &str,
    kind: Kind,
    rule: Option<Agent>,
    quota: impl Fn(Agent) -> Option<u8>,
    past: &[Past],
) -> Choice {
    // Only tasks of this kind here: a score needs evidence of its own.
    let here: Vec<&Past> = past
        .iter()
        .filter(|p| p.roost == roost && p.kind == kind)
        .collect();
    let learned = (here.len() >= LEARNED).then(|| {
        // Laplace's rule of succession: a few tasks never decide on their own.
        let score = |a: Agent| {
            let mine: Vec<&&Past> = here.iter().filter(|p| p.agent == a).collect();
            let done = mine.iter().filter(|p| p.done).count();
            (done, mine.len(), (done as f64 + 1.0) / (mine.len() as f64 + 2.0))
        };
        let (claude, codex) = (score(Agent::Claude), score(Agent::Codex));
        let best = if (claude.2 - codex.2).abs() < f64::EPSILON {
            None
        } else if claude.2 > codex.2 {
            Some((Agent::Claude, claude))
        } else {
            Some((Agent::Codex, codex))
        };
        // The winner must have done some: an agent never tried never wins on its prior alone.
        best.filter(|(_, (_, of, _))| *of > 0)
    });
    let first = match learned.flatten() {
        Some((agent, (done, of, _))) => Choice {
            agent,
            why: Why::Score { done, of },
        },
        None => match rule {
            Some(agent) => Choice {
                agent,
                why: Why::Rule,
            },
            None => Choice {
                agent: Agent::Claude,
                why: Why::Default,
            },
        },
    };
    // The other agent's usage must be known to be under the quota: unknown is often an agent not
    // installed or not logged in, which would only fail the task.
    match quota(first.agent) {
        Some(used) if used > QUOTA && quota(first.agent.other()).is_some_and(|o| o <= QUOTA) => Choice {
            agent: first.agent.other(),
            why: Why::Quota {
                busy: first.agent,
                used,
            },
        },
        _ => first,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn past(agent: Agent, kind: Kind, done: bool) -> Past {
        Past {
            agent,
            roost: "github:me/site".into(),
            kind,
            done,
        }
    }

    #[test]
    fn a_goal_has_a_kind_by_its_whole_words() {
        assert_eq!(kind("Fix the flaky login test"), Kind::Fix);
        assert_eq!(kind("Add tests for the parser"), Kind::Tests);
        assert_eq!(kind("Translate the policies page"), Kind::Docs);
        assert_eq!(kind("Add dark mode"), Kind::Feature);
        for feature in [
            "Add a Dockerfile",
            "Add fixtures loader",
            "Better error messages",
            "A special offer",
            "Add testimonials",
        ] {
            assert_eq!(kind(feature), Kind::Feature, "{feature}");
        }
    }

    #[test]
    fn a_rule_or_claude_code_unless_its_plan_is_nearly_spent() {
        let none = |_| None;
        assert_eq!(
            choose("r", Kind::Fix, None, none, &[]),
            Choice {
                agent: Agent::Claude,
                why: Why::Default
            }
        );
        assert_eq!(
            choose("r", Kind::Fix, Some(Agent::Codex), none, &[]),
            Choice {
                agent: Agent::Codex,
                why: Why::Rule
            }
        );
        let busy = |a| Some(if a == Agent::Claude { 91 } else { 20 });
        assert_eq!(
            choose("r", Kind::Fix, None, busy, &[]),
            Choice {
                agent: Agent::Codex,
                why: Why::Quota {
                    busy: Agent::Claude,
                    used: 91
                }
            }
        );
        // The other's usage unknown (not installed, logged out): the choice stands.
        let unknown = |a| (a == Agent::Claude).then_some(91);
        assert_eq!(choose("r", Kind::Fix, None, unknown, &[]).why, Why::Default);
        // Both near their limit: the first choice stands.
        assert_eq!(
            choose("r", Kind::Fix, None, |_| Some(95), &[]).agent,
            Agent::Claude
        );
        assert_eq!(
            choose("r", Kind::Fix, None, |_| Some(85), &[]).why,
            Why::Default,
            "85 is not above 85"
        );
    }

    #[test]
    fn after_ten_tasks_of_a_kind_the_roost_learns_who_finishes_them() {
        let mut p: Vec<Past> = (0..6).map(|_| past(Agent::Codex, Kind::Fix, true)).collect();
        p.extend((0..3).map(|_| past(Agent::Claude, Kind::Fix, false)));
        let r = "github:me/site";
        assert_eq!(
            choose(r, Kind::Fix, None, |_| None, &p).why,
            Why::Default,
            "nine fixes: not learned yet"
        );
        p.push(past(Agent::Claude, Kind::Fix, true));
        assert_eq!(
            choose(r, Kind::Fix, Some(Agent::Claude), |_| None, &p),
            Choice {
                agent: Agent::Codex,
                why: Why::Score { done: 6, of: 6 }
            }
        );
        // Another kind has learned nothing from fixes: one task of it never overrides the rule.
        p.push(past(Agent::Codex, Kind::Feature, false));
        p.push(past(Agent::Claude, Kind::Docs, true));
        assert_eq!(
            choose(r, Kind::Feature, Some(Agent::Codex), |_| None, &p).why,
            Why::Rule
        );
        assert_eq!(
            choose(r, Kind::Docs, Some(Agent::Codex), |_| None, &p).why,
            Why::Rule
        );
        // An agent never tried never wins on its prior: ten failed Codex fixes elsewhere-kind.
        let failed: Vec<Past> = (0..10).map(|_| past(Agent::Codex, Kind::Tests, false)).collect();
        assert_eq!(
            choose(r, Kind::Tests, Some(Agent::Codex), |_| None, &failed).why,
            Why::Rule
        );
        // Another roost has learned nothing.
        assert_eq!(
            choose("github:me/other", Kind::Fix, None, |_| None, &p).why,
            Why::Default
        );
        // The quota still holds after learning.
        let busy = |a| Some(if a == Agent::Codex { 99 } else { 10 });
        assert_eq!(choose(r, Kind::Fix, None, busy, &p).agent, Agent::Claude);
    }
}
