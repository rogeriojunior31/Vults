//! The context pack (`docs/dev/plan-zeca.md` W5): one template for whatever carries work from one
//! context to the next: a handoff to a fresh session or another agent, a compaction, a summary, a
//! delegation. Eight parts, always in this order, so the next reader finds each in the same place:
//! what it is for, what binds it, what is done, what is half done, what was decided, which files,
//! what failed, and what comes next.
//!
//! A pack is text: an agent writes it in the template, and the app reads it back leniently (any
//! order, a missing part empty, an unknown heading kept with the part before it), then adds what it
//! knows for certain (the goal, the criteria's ticks, the files on the branch), which beats what the
//! agent said.

use std::fmt::Write as _;

/// The parts, in the template's order, by their headings.
pub const PARTS: [&str; 8] = [
    "Objective",
    "Constraints",
    "Done",
    "In progress",
    "Decisions",
    "Files",
    "Failures",
    "Next",
];

/// Longest an item may be, and how many items a part keeps: a whole pack must fit a system prompt
/// with room to spare (8 parts of 30 items of 500 characters at most, far more than any real one).
pub const ITEM_MAX: usize = 500;
pub const PART_ITEMS: usize = 30;

/// A pack, part by part, each as its lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pack {
    pub objective: String,
    pub constraints: Vec<String>,
    pub done: Vec<String>,
    pub in_progress: Vec<String>,
    pub decisions: Vec<String>,
    pub files: Vec<String>,
    pub failures: Vec<String>,
    pub next: Vec<String>,
}

/// What an agent is asked to write: the template, empty, and what each part holds.
pub const ASK: &str = "Write a context pack of this task for whoever takes it over, who will know nothing else. Use exactly these headings, in this order, each part short, as bullet points (none for a part with nothing):\n\n## Objective\n## Constraints\n## Done\n## In progress\n## Decisions\n## Files\n## Failures\n## Next\n\nObjective: the goal in one or two sentences. Constraints: what must hold or must not be done. Done and In progress: what is finished and what is half done, by the criteria. Decisions: what was chosen and why. Files: the files that matter, with a few words each. Failures: what was tried and did not work, so it is not tried again. Next: the next concrete steps. Write nothing before the first heading or after the last part. Don't run any tools: write from what you know.";

fn cut(text: &str) -> String {
    // One line: a pack's item never spans two (a newline would read back as another item).
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match text.char_indices().nth(ITEM_MAX) {
        Some((i, _)) => format!("{}…", &text[..i]),
        None => text,
    }
}

/// A line without its list marker: `- `, `* `, `• `, `1. `, `1) `.
fn unlisted(line: &str) -> &str {
    let l = line.trim();
    if let Some(rest) = ["- ", "* ", "• "].iter().find_map(|m| l.strip_prefix(m)) {
        return rest.trim();
    }
    let digits = l.len() - l.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    match l[digits..]
        .strip_prefix(". ")
        .or_else(|| l[digits..].strip_prefix(") "))
    {
        Some(rest) if digits > 0 => rest.trim(),
        _ => l,
    }
}

/// A part's lines: the text under a heading, one item per line, list markers taken off; at most
/// [`PART_ITEMS`].
fn items(text: &str) -> Vec<String> {
    text.lines()
        .map(unlisted)
        .filter(|l| {
            !l.is_empty() && !matches!(l.to_ascii_lowercase().as_str(), "none" | "none." | "n/a" | "-")
        })
        .map(cut)
        .take(PART_ITEMS)
        .collect()
}

/// The part a heading line names: `## Files`, `### Next steps`, `**Objective**`, `Done:` in bold;
/// a name followed by more words still counts (`Files changed`), never one that only starts the
/// same (`Doneness`).
fn heading(line: &str) -> Option<usize> {
    let l = line.trim();
    let text = match l.strip_prefix('#') {
        Some(h) => h.trim_start_matches('#').trim(),
        None => l.strip_prefix("**")?.strip_suffix("**")?.trim(),
    };
    let text = text.trim_end_matches(':').trim().to_ascii_lowercase();
    PARTS.iter().position(|p| {
        let p = p.to_ascii_lowercase();
        text.strip_prefix(&p)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(|c: char| !c.is_ascii_alphanumeric()))
    })
}

/// A done or in-progress line without the tick an agent may put first (`[x]`, `✅`…).
fn unticked(line: &str) -> &str {
    let mut l = line.trim();
    for m in ["[x]", "[X]", "[ ]", "✅", "✔️", "✔", "☑️", "☑", "✓", "❌", "⏳"] {
        if let Some(rest) = l.strip_prefix(m) {
            l = rest.trim_start();
        }
    }
    l
}

/// Whether `line` is about criterion `c`: it starts with its words, and what follows (if anything)
/// starts a remark (`:`, `-`, `—`, `(`, `,`), so `a` never claims `a different thing`.
fn about(line: &str, c: &str) -> bool {
    let line = unticked(line).to_lowercase();
    let c = c.trim().to_lowercase();
    line.strip_prefix(&c).is_some_and(|rest| {
        let rest = rest.trim_start();
        rest.is_empty() || rest.starts_with([':', '-', '—', '(', ','])
    })
}

/// The path a Files line starts with: `` `src/a.rs`: words ``, `./src/a.rs - words`.
fn path_of(line: &str) -> &str {
    let first = line.split_whitespace().next().unwrap_or_default();
    let first = first
        .trim_end_matches([':', ','])
        .trim_matches('`')
        .trim_end_matches(':');
    first.strip_prefix("./").unwrap_or(first)
}

impl Pack {
    /// Reads a pack an agent wrote. Headings are matched without case or `#` count; text before the
    /// first heading is dropped (an agent's preamble), and a heading we don't know stays in the part
    /// before it, so nothing it wrote under a known one is lost.
    pub fn parse(text: &str) -> Self {
        let mut parts: [String; 8] = Default::default();
        let mut at: Option<usize> = None;
        let mut fenced = false;
        for line in text.lines() {
            // A fenced block's lines are text, never headings; its fences are no items.
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
                continue;
            }
            if !fenced && let Some(i) = heading(line) {
                at = Some(i);
                continue;
            }
            if let Some(i) = at {
                parts[i].push_str(line);
                parts[i].push('\n');
            }
        }
        let [
            objective,
            constraints,
            done,
            in_progress,
            decisions,
            files,
            failures,
            next,
        ] = parts;
        Self {
            objective: cut(&items(&objective).join(" ")),
            constraints: items(&constraints),
            done: items(&done),
            in_progress: items(&in_progress),
            decisions: items(&decisions),
            files: items(&files),
            failures: items(&failures),
            next: items(&next),
        }
    }

    /// The pack in the template: every heading, in order, `- None` for an empty part, so the reader
    /// knows it was looked at.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let lists: [&[String]; 7] = [
            &self.constraints,
            &self.done,
            &self.in_progress,
            &self.decisions,
            &self.files,
            &self.failures,
            &self.next,
        ];
        let _ = writeln!(
            out,
            "## Objective\n\n{}\n",
            if self.objective.is_empty() {
                "None"
            } else {
                &self.objective
            }
        );
        for (heading, list) in PARTS[1..].iter().zip(lists) {
            let _ = writeln!(out, "## {heading}\n");
            if list.is_empty() {
                out.push_str("- None\n");
            }
            for item in list {
                let _ = writeln!(out, "- {item}");
            }
            out.push('\n');
        }
        out.trim_end().to_string() + "\n"
    }

    /// Whether the agent wrote anything at all.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// What the app knows for certain, over what the agent said: the goal is the objective; each
    /// criterion is done or in progress by its tick (the agent's other lines in those parts stay
    /// after them); the files changed on the branch lead the files, each once.
    pub fn with_facts(mut self, goal: &str, criteria: &[(String, bool)], changed: &[String]) -> Self {
        if !goal.trim().is_empty() {
            self.objective = cut(goal);
        }
        // Each criterion goes where its tick says; the agent's line about it (one that starts with
        // its words) keeps its detail in place of the bare criterion, and is used once.
        let self_done = self.done.clone();
        let mut said: Vec<String> = self.done.into_iter().chain(self.in_progress).collect();
        let (mut done, mut open) = (Vec::new(), Vec::new());
        for (c, ticked) in criteria {
            let line = match said.iter().position(|l| about(l, c)) {
                Some(i) => cut(unticked(&said.remove(i))),
                None => cut(c),
            };
            if *ticked { done.push(line) } else { open.push(line) }
        }
        // What the agent said about anything else stays in the part it wrote it in.
        let was_done = |l: &String| self_done.contains(l);
        done.extend(said.iter().filter(|l| was_done(l)).cloned());
        open.extend(said.into_iter().filter(|l| !was_done(l)));
        self.done = done;
        self.in_progress = open;
        let mut files: Vec<String> = changed.iter().map(|f| cut(f)).collect();
        for f in self.files {
            // The agent's line for a changed file keeps its words in place of the bare path.
            let path = path_of(&f);
            match files.iter().position(|c| c == path) {
                Some(i) => files[i] = f,
                None => files.push(f),
            }
        }
        self.files = files;
        self
    }
}

/// The pack's prompts, for the evals' snapshots: what an agent is asked to write.
pub fn prompts() -> Vec<(&'static str, String)> {
    vec![("pack-ask", ASK.to_string())]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRITTEN: &str = "Sure, here it is.\n\n## Objective\nFix the flaky login test.\n\n## constraints:\n- no sleep in tests\n- keep the public API\n\n### Done\n- Found the race in auth.spec.ts\n\n## In progress\n- passes 20 runs in a row\n\n## Decisions\n- Wait on the redirect, not a timer: it is what the user sees\n## Notes\n- an unknown heading stays with the part before it\n\n## Files\n- `auth.spec.ts` the test\n\n## Failures\nNone\n\n## Next\n- run it 20 times\n";

    #[test]
    fn it_reads_what_an_agent_writes_leniently() {
        let p = Pack::parse(WRITTEN);
        assert_eq!(p.objective, "Fix the flaky login test.");
        assert_eq!(p.constraints, ["no sleep in tests", "keep the public API"]);
        assert_eq!(p.done, ["Found the race in auth.spec.ts"]);
        assert_eq!(
            p.decisions.len(),
            3,
            "the unknown heading and its line stay in Decisions"
        );
        assert!(p.failures.is_empty(), "None is no item");
        assert_eq!(p.next, ["run it 20 times"]);
        assert!(Pack::parse("no headings at all").is_empty());
    }

    #[test]
    fn it_writes_every_part_in_order_and_reads_back_the_same() {
        let p = Pack::parse(WRITTEN);
        let text = p.render();
        let at: Vec<usize> = PARTS
            .iter()
            .map(|h| text.find(&format!("## {h}\n")).unwrap_or(usize::MAX))
            .collect();
        assert!(at.windows(2).all(|w| w[0] < w[1]), "every heading, in order");
        assert!(text.contains("## Failures\n\n- None"));
        assert_eq!(Pack::parse(&text), p, "a rendered pack reads back as itself");
    }

    #[test]
    fn what_the_app_knows_beats_what_the_agent_said() {
        let p = Pack::parse(WRITTEN).with_facts(
            "Fix the flaky login test, for good",
            &[
                ("passes 20 runs in a row".into(), true),
                ("no sleep added".into(), false),
            ],
            &["auth.spec.ts".into(), "src/auth.ts".into()],
        );
        assert_eq!(p.objective, "Fix the flaky login test, for good");
        assert_eq!(
            p.done[0], "passes 20 runs in a row",
            "ticked: done, whatever the agent said"
        );
        assert!(!p.in_progress.contains(&"passes 20 runs in a row".to_string()));
        assert_eq!(p.in_progress, ["no sleep added"]);
        let detailed = Pack::parse("## In progress\n- passes 20 runs in a row: 14 of 20\n- also refactoring")
            .with_facts("g", &[("passes 20 runs in a row".into(), false)], &[]);
        assert_eq!(
            detailed.in_progress,
            ["passes 20 runs in a row: 14 of 20", "also refactoring"],
            "the agent's detail, once"
        );
        assert_eq!(
            p.files,
            ["`auth.spec.ts` the test", "src/auth.ts"],
            "a changed file keeps the agent's words, each once"
        );
        let long = "x".repeat(ITEM_MAX + 50);
        assert!(
            Pack::default()
                .with_facts(&long, &[], &[])
                .objective
                .chars()
                .count()
                <= ITEM_MAX + 1
        );
    }

    #[test]
    fn it_reads_the_ways_agents_actually_write() {
        let p = Pack::parse(
            "**Objective**\nShip it.\n\n## Files changed\n- `a.rs`: the parser\n\n### Next steps:\n1. one\n2) two\n\n```\n## Done\nnot a heading\n```\n## Doneness\n- stays in Next\n",
        );
        assert_eq!(p.objective, "Ship it.");
        assert_eq!(
            p.files,
            ["`a.rs`: the parser"],
            "a heading with more words names its part"
        );
        assert_eq!(
            p.next,
            [
                "one",
                "two",
                "## Done",
                "not a heading",
                "## Doneness",
                "stays in Next"
            ],
            "fences hide headings; Doneness is no Done"
        );
        assert!(p.done.is_empty());
    }

    #[test]
    fn a_criterion_claims_only_lines_about_it() {
        let p =
            Pack::parse("## Done\n- [x] tests pass\n- ✅ lint clean (CI)\n- a different thing entirely\n")
                .with_facts(
                    "g",
                    &[
                        ("tests pass".into(), true),
                        ("lint clean".into(), true),
                        ("a".into(), false),
                    ],
                    &[],
                );
        assert_eq!(
            p.done,
            ["tests pass", "lint clean (CI)", "a different thing entirely"],
            "ticks taken off, each once"
        );
        assert_eq!(
            p.in_progress,
            ["a"],
            "a short criterion never claims a longer line"
        );
    }

    #[test]
    fn a_changed_file_is_listed_once_however_the_agent_wrote_it() {
        let p = Pack::parse("## Files\n- auth.rs: the login\n- ./src/x.rs changed\n- `docs/a.md`: notes\n")
            .with_facts(
                "g",
                &[],
                &[
                    "auth.rs".into(),
                    "src/x.rs".into(),
                    "docs/a.md".into(),
                    "new.rs".into(),
                ],
            );
        assert_eq!(
            p.files,
            [
                "auth.rs: the login",
                "./src/x.rs changed",
                "`docs/a.md`: notes",
                "new.rs"
            ]
        );
    }

    #[test]
    fn a_pack_stays_small_and_one_line_per_item() {
        let many: String = (0..100).map(|i| format!("- item {i}\n")).collect();
        let p = Pack::parse(&format!("## Decisions\n{many}"));
        assert_eq!(p.decisions.len(), PART_ITEMS);
        let p = Pack::default().with_facts("a goal\nover two lines", &[], &[]);
        assert_eq!(p.objective, "a goal over two lines");
        assert_eq!(Pack::parse(&p.render()), p);
    }
}
