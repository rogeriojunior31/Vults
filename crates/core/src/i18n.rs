//! User-facing text. Whole sentences per language, never assembled from fragments, so a
//! translation can reorder words freely. pt-BR comes after 1.0.

use serde::{Deserialize, Serialize};

use crate::Activity;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
}

/// `Editing main.rs`, or just `Editing`.
pub fn step(lang: Lang, activity: Activity, tool: &str, detail: Option<&str>) -> String {
    let Lang::En = lang;
    let verb = match activity {
        Activity::Read => "Reading",
        Activity::Search => "Searching",
        Activity::Edit => "Editing",
        Activity::Run if detail.is_some_and(is_test_run) => "Testing",
        Activity::Run => "Running",
        Activity::Web if tool.starts_with("mcp__") => "Calling",
        Activity::Web => "Browsing",
        Activity::Plan => "Planning",
        Activity::Subagent => "Delegating",
        Activity::Think => "Thinking",
        Activity::Work => tool,
    };
    match detail {
        Some(d) => format!("{verb} {d}"),
        None => verb.to_string(),
    }
}

/// A command that runs a test suite: `cargo test -p core`, `npm test`, `pytest -k slow`.
fn is_test_run(command: &str) -> bool {
    const RUNNERS: &[&str] = &[
        "cargo test",
        "cargo nextest",
        "npm test",
        "npm run test",
        "pnpm test",
        "yarn test",
        "bun test",
        "npx vitest",
        "npx jest",
        "npx playwright test",
        "vitest",
        "jest",
        "pytest",
        "python -m pytest",
        "go test",
        "mvn test",
        "gradle test",
        "./gradlew test",
        "make test",
        "ctest",
        "rspec",
        "phpunit",
    ];
    let command = command.trim_start();
    RUNNERS.iter().any(|r| {
        command
            .strip_prefix(r)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', ':', '…']))
    })
}

/// A step one of the user's Always rules allowed, without a card: `Testing cargo test · always
/// allowed`.
pub fn ruled_step(lang: Lang, step: &str) -> String {
    let Lang::En = lang;
    format!("{step} · always allowed")
}

/// Who a notification is about when nothing names it.
pub fn some_agent(lang: Lang) -> &'static str {
    let Lang::En = lang;
    "An agent"
}

/// A desktop notification's title: `vultures-ai needs you`.
pub fn notice_title(lang: Lang, kind: crate::notify::Kind, who: &str) -> String {
    use crate::notify::Kind;
    let Lang::En = lang;
    match kind {
        Kind::NeedsYou => format!("{who} needs you"),
        Kind::Finished => format!("{who} finished"),
        Kind::Failed => format!("{who} stopped on an error"),
        Kind::Silent => format!("{who} has gone quiet"),
    }
}

/// "While you were away: 2 finished, 1 failed, 1 waits for you for 12 min." The counts that are
/// zero are left out; the sentence is made here, whole, per language.
pub fn digest(lang: Lang, d: &crate::away::Digest) -> String {
    let Lang::En = lang;
    let mut parts = Vec::new();
    if d.finished > 0 {
        parts.push(format!("{} finished", d.finished));
    }
    if d.failed > 0 {
        parts.push(format!("{} failed", d.failed));
    }
    if d.waiting > 0 {
        let mins = (d.waited.as_secs() / 60).max(1);
        let verb = if d.waiting == 1 { "waits" } else { "wait" };
        parts.push(format!("{} {verb} for you for {mins} min", d.waiting));
    }
    format!("While you were away: {}.", parts.join(", "))
}

/// A quiet bird's notification: it only informs (`crate::silence`).
pub fn silent_body(lang: Lang) -> &'static str {
    let Lang::En = lang;
    "No news for 15 minutes while it works. It may be waiting on something in its terminal."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_run_and_an_mcp_call_get_their_own_verbs() {
        let run = |cmd| step(Lang::En, Activity::Run, "Bash", Some(cmd));
        assert_eq!(run("cargo test -p core"), "Testing cargo test -p core");
        assert_eq!(run("npm test"), "Testing npm test");
        assert_eq!(run("pytest…"), "Testing pytest…");
        assert_eq!(run("cargo build"), "Running cargo build");
        assert_eq!(run("cargo testify"), "Running cargo testify");
        assert_eq!(step(Lang::En, Activity::Run, "Bash", None), "Running");
        assert_eq!(
            step(
                Lang::En,
                Activity::Web,
                "mcp__github__list_prs",
                Some("github · list_prs")
            ),
            "Calling github · list_prs"
        );
        assert_eq!(
            step(Lang::En, Activity::Web, "WebFetch", Some("a.dev")),
            "Browsing a.dev"
        );
    }
}
