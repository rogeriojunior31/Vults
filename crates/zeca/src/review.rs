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
        "\nRun the project's tests. Read the whole diff. Check each criterion, and look for bugs the tests miss: wrong logic, inputs the criteria cover that it gets wrong, broken behavior elsewhere. Change no file and commit nothing.\n\nList each problem with its file and line and why it is wrong, most serious first. Ask for changes only for a criterion not met, a real bug, or a failing test; an optional improvement, a style point or an input the task never asked about may be mentioned but asks for nothing. End with exactly one line, the last of your answer: `VERDICT: pass` when every criterion is met and nothing is wrong, or `VERDICT: changes` when something is.",
    );
    text
}

/// The verdict a review ended with: its last non-empty line, which must say exactly `VERDICT: pass`
/// or `VERDICT: changes` (as the line itself, or its last words). Anything else is no verdict: a
/// human reads it, never a pass (a verdict quoted earlier, or `pass once that is fixed`, is none).
pub fn verdict(text: &str) -> Option<Verdict> {
    let last = text.lines().rev().map(str::trim).find(|l| !l.is_empty())?;
    said(last)
}

/// The verdict of a review by Codex's own reviewer, which tags findings with a priority (`[P1]`)
/// and seldom writes a verdict line: a finding of priority 0 to 2 sends the work back whatever the
/// rest says; else its last verdict line, or (a finished review with nothing to fix) a pass. Nits
/// (`[P3]`) alone never loop the task.
pub fn codex_verdict(text: &str) -> Option<Verdict> {
    if text.trim().is_empty() {
        return None;
    }
    if text.lines().filter_map(priority).any(|p| p <= 2) {
        return Some(Verdict::Changes);
    }
    Some(
        text.lines()
            .rev()
            .find_map(|l| said(l.trim()))
            .unwrap_or(Verdict::Pass),
    )
}

/// The verdict a line says: `VERDICT: pass` or `VERDICT: changes`, the whole line or its end, with
/// nothing after the word but punctuation.
fn said(line: &str) -> Option<Verdict> {
    let l = line.trim_matches(['`', '*', '_', ' ']);
    let at = l.to_ascii_lowercase().rfind("verdict:")?;
    let word = l[at + 8..]
        .trim()
        .trim_end_matches(['.', '!', '`', '*', '_', ' ']);
    match word.to_ascii_lowercase().as_str() {
        "pass" => Some(Verdict::Pass),
        "changes" => Some(Verdict::Changes),
        _ => None,
    }
}

/// A Codex finding's priority: a list line (`-`, `*`, `1.`) opening with `[P0]` to `[P3]`, bold or
/// not, in either case.
fn priority(line: &str) -> Option<u8> {
    let mut l = line.trim_start();
    l = l.trim_start_matches(['-', '*', '+']).trim_start();
    let digits = l.len() - l.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits > 0 && (l[digits..].starts_with(". ") || l[digits..].starts_with(") ")) {
        l = l[digits + 1..].trim_start();
    }
    let l = l.trim_start_matches(['*', '_']);
    let b = l.as_bytes();
    (b.len() >= 4 && b[0] == b'[' && (b[1] == b'P' || b[1] == b'p') && b[2].is_ascii_digit() && b[3] == b']')
        .then(|| b[2] - b'0')
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
    fn only_a_last_line_that_says_exactly_the_verdict_counts() {
        assert_eq!(verdict("ok\nVERDICT: pass"), Some(Verdict::Pass));
        assert_eq!(verdict("- bug\n\n**VERDICT: changes**\n"), Some(Verdict::Changes));
        assert_eq!(verdict("`verdict: Changes.`"), Some(Verdict::Changes));
        assert_eq!(
            verdict("All criteria hold. VERDICT: pass"),
            Some(Verdict::Pass),
            "at the last line's end"
        );
        // Never a pass it didn't end with, or one with conditions.
        for text in [
            "- a.py:3 wrong\n```\nVERDICT: pass\n```\nI would write the above if fixed.",
            "VERDICT: pass\n\nWait, a.py:3 is wrong; needs changes.",
            "VERDICT: pass once the off-by-one above is fixed",
            "VERDICT: pass|changes",
            "looks fine to me",
            "VERDICT: maybe",
        ] {
            assert_eq!(verdict(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_codex_finding_that_matters_sends_it_back_whatever_else_it_says() {
        let p1 = "The code looks fine.\nVERDICT: pass\n\nReview comment:\n\n- [P1] Off by one — a.py:3-3";
        assert_eq!(codex_verdict(p1), Some(Verdict::Changes));
        for tagged in ["1. [P1] bug", "- **[P1] bug**", "- [p2] bug", "* [P0] bug"] {
            assert_eq!(codex_verdict(tagged), Some(Verdict::Changes), "{tagged}");
        }
        assert_eq!(
            codex_verdict("Fine overall.\n- [P3] a nit"),
            Some(Verdict::Pass),
            "nits alone never loop the task"
        );
        assert_eq!(
            codex_verdict("The diff meets both criteria."),
            Some(Verdict::Pass),
            "a finished review that found nothing"
        );
        assert_eq!(codex_verdict("Summary. VERDICT: changes"), Some(Verdict::Changes));
        assert_eq!(codex_verdict("   "), None, "no review at all is no verdict");
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
