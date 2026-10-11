//! The review loop's words (`docs/dev/plan-zeca.md` W6): what the reviewer is asked (the tests, the
//! diff against the branch's base, each criterion), the verdict it must end with, and what the
//! worker hears when a review sends the task back. The loop itself, up to [`ROUNDS`], is the app's.

/// Most review rounds a task gets before a human decides.
pub const ROUNDS: usize = 3;

/// Longest findings passed back to the worker: their start, where a reviewer puts what matters.
pub const FINDINGS_MAX: usize = 6000;

/// What a review answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every criterion met, the tests pass, nothing to fix.
    Pass,
    /// Something to fix: the findings say what.
    Changes,
}

/// What the reviewer is asked: the task, its criteria, and the base its branch is compared with.
pub fn ask(goal: &str, criteria: &[String], base: &str) -> String {
    let mut text = format!(
        "Review the work on this branch against `{base}` (`git diff {base}...HEAD`), as a careful reviewer who wrote none of it. The task was:\n\nGoal: {goal}\n"
    );
    if !criteria.is_empty() {
        text.push_str("\nIt is done when:\n");
        for c in criteria {
            text.push_str(&format!("- {c}\n"));
        }
    }
    text.push_str(
        "\nRun the project's tests. Read the whole diff. Check each criterion, and look for bugs the tests miss: wrong logic, edge cases, broken behavior elsewhere. Change no file and commit nothing.\n\nList each problem with its file and line and why it is wrong, most serious first. End with exactly one line, the last of your answer: `VERDICT: pass` when every criterion is met and nothing needs fixing, or `VERDICT: changes` when anything does.",
    );
    text
}

/// The verdict a review ended with: its last `VERDICT:` line, else a Codex finding's priority tag
/// (changes). None without either (the review is then a human's to read, never a pass).
pub fn verdict(text: &str) -> Option<Verdict> {
    let said = text.lines().rev().find_map(|l| {
        let l = l.trim().trim_matches(['`', '*', '_']).trim();
        let rest = l
            .get(..8)
            .filter(|p| p.eq_ignore_ascii_case("verdict:"))
            .map(|_| l[8..].trim())?;
        let word: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_ascii_lowercase();
        match word.as_str() {
            "pass" => Some(Verdict::Pass),
            "changes" => Some(Verdict::Changes),
            _ => None,
        }
    });
    // Codex's own reviewer writes no verdict line: its findings carry a priority (`- [P2] …`), and
    // any finding sends the work back.
    said.or_else(|| text.lines().any(finding).then_some(Verdict::Changes))
}

/// A Codex review finding: a list line opening with its priority, `[P0]` to `[P3]`.
fn finding(line: &str) -> bool {
    let l = line.trim_start().trim_start_matches(['-', '*']).trim_start();
    l.len() >= 4
        && l.as_bytes()[0] == b'['
        && l.as_bytes()[1] == b'P'
        && l.as_bytes()[2].is_ascii_digit()
        && l.as_bytes()[3] == b']'
}

/// What a review found, without its verdict line, cut at [`FINDINGS_MAX`].
pub fn findings(text: &str) -> String {
    let kept: Vec<&str> = text
        .lines()
        .filter(|l| {
            !l.trim()
                .trim_matches(['`', '*', '_'])
                .to_ascii_lowercase()
                .starts_with("verdict:")
        })
        .collect();
    let text = kept.join("\n");
    let text = text.trim();
    match text.char_indices().nth(FINDINGS_MAX) {
        Some((i, _)) => format!("{}…", &text[..i]),
        None => text.to_string(),
    }
}

/// What the worker hears when a review sends the task back.
pub fn fix(findings: &str, round: usize) -> String {
    format!(
        "A review of your work (round {round} of {ROUNDS}) found what follows. Fix each point that is right, say why for any you think is wrong, run the tests, commit on this branch, and say what you changed.\n\n{findings}"
    )
}

/// The review's prompts, for the evals' snapshots.
pub fn prompts() -> Vec<(&'static str, String)> {
    vec![
        (
            "review-ask",
            ask(
                "Fix the flaky login test",
                &["passes 20 runs in a row".into(), "no sleep added".into()],
                "vults/base-12",
            ),
        ),
        ("review-fix", fix("- auth.spec.ts:12: still waits on a timer", 1)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_verdict_line_decides() {
        assert_eq!(verdict("ok\nVERDICT: pass"), Some(Verdict::Pass));
        assert_eq!(verdict("- bug\n\n**VERDICT: changes**\n"), Some(Verdict::Changes));
        assert_eq!(verdict("`verdict: Changes.`"), Some(Verdict::Changes));
        assert_eq!(
            verdict("VERDICT: pass\nlater: VERDICT: changes"),
            Some(Verdict::Pass),
            "only a line that starts with it"
        );
        assert_eq!(
            verdict("VERDICT: changes\n...\nVERDICT: pass"),
            Some(Verdict::Pass),
            "the last one"
        );
        assert_eq!(verdict("looks fine to me"), None, "no verdict is never a pass");
        assert_eq!(verdict("VERDICT: maybe"), None);
        // Codex's reviewer: no verdict line, findings tagged by priority.
        assert_eq!(
            verdict("Review comment:\n\n- [P2] Return an empty list when n is zero — lists.py:6"),
            Some(Verdict::Changes)
        );
        assert_eq!(
            verdict("The diff looks correct and the tests pass."),
            None,
            "no finding and no verdict: a human reads it"
        );
        assert_eq!(
            verdict("- [P1] bug\nVERDICT: pass"),
            Some(Verdict::Pass),
            "an explicit verdict wins"
        );
    }

    #[test]
    fn findings_leave_the_verdict_out_and_stay_short() {
        assert_eq!(
            findings("- a.rs:3 off by one\n\nVERDICT: changes\n"),
            "- a.rs:3 off by one"
        );
        assert!(findings(&"x".repeat(FINDINGS_MAX + 9)).chars().count() <= FINDINGS_MAX + 1);
    }

    #[test]
    fn the_ask_names_the_base_and_every_criterion() {
        let a = ask("Ship", &["tests pass".into()], "vults/base-3");
        assert!(
            a.contains("git diff vults/base-3...HEAD")
                && a.contains("- tests pass")
                && a.contains("VERDICT: changes")
        );
    }
}
