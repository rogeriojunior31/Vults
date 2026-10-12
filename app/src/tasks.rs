//! Tasks (`docs/dev/plan-zeca.md` W3): `/task` in the chat and the Nest's Tasks tab. A task is the
//! user's: its goal, the criteria that say it is done, its roost (the repository of the folder it
//! was made in), who works on it, where it stands and what it cost. With the GitHub connector on, a
//! task in a GitHub repository links to an issue, or opens one on the user's click.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use vults_store::Store;
use vults_store::tasks::{Change, New, Task};

/// Told to every window when the tasks change.
const CHANGED: &str = "tasks-changed";
/// Who may own a task: the user, Zeca, or an agent.
const OWNERS: &[&str] = &[
    "you",
    "zeca",
    "claude",
    "codex",
    "gemini",
    "opencode",
    "qwen",
    "antigravity",
    "copilot",
];

/// A task, for the Nest.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: i64,
    pub goal: String,
    pub criteria: Vec<Criterion>,
    /// The roost's name; empty when it was made outside a repository.
    pub roost: String,
    /// Its key, which tells apart repositories that share a name (two local `site`s).
    pub roost_key: String,
    /// The folder it was made in, shown with `~`.
    pub folder: String,
    pub owner: String,
    pub status: String,
    pub cost_usd: Option<f64>,
    /// The tokens its runs used (W9).
    pub tokens: Option<i64>,
    pub issue: Option<String>,
    /// Whether **Open an issue** can work: a GitHub repository and the connector on.
    pub can_open_issue: bool,
    /// A session Vults started for it is running (W4).
    pub running: bool,
    /// Whether **Start** can work: a repository to make its worktree in, and not finished.
    pub can_start: bool,
    /// Its worktree is on disk.
    pub has_worktree: bool,
    /// The end of what its last session said.
    pub reply: Option<String>,
    /// It has a session to go on with, or to hand off (W5).
    pub has_session: bool,
    /// The agent that session ran on (`claude`, `codex`).
    pub session_engine: Option<&'static str>,
    /// The context pack its last handoff left.
    pub pack: Option<String>,
    /// Why the router sent it to its agent (W8): `rule:codex`, `quota:codex:claude:91`…
    pub route: Option<String>,
    /// Its last review (W6): what the reviewer said, its verdict (`pass`, `changes`, `none`) and
    /// the round.
    pub review: Option<String>,
    pub review_verdict: Option<String>,
    pub review_round: Option<i64>,
    pub updated_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Criterion {
    pub text: String,
    pub done: bool,
}

/// What a `/task` says back, as `/remember` does: an English sentence the window translates, and
/// the task's own words.
#[derive(Serialize, Debug, PartialEq)]
pub struct Said {
    pub said: &'static str,
    pub detail: Option<String>,
}

fn open() -> Result<Store, String> {
    Store::open(&crate::history::dir()).map_err(|e| e.to_string())
}

fn now_ms() -> i64 {
    crate::traces::now_ms()
}

/// The chat's and the Nest's windows only: the user acts there.
fn from_user(window: &tauri::Webview) -> Result<(), String> {
    if [crate::ISLAND, crate::NEST].contains(&window.label()) {
        Ok(())
    } else {
        Err("only the chat and the Nest can do this".into())
    }
}

fn changed(app: &AppHandle) {
    let _ = app.emit(CHANGED, ());
}

/// `owner/name` of a GitHub roost key.
fn github_repo(roost: &str) -> Option<&str> {
    roost
        .strip_prefix("github:")
        .filter(|r| r.split('/').count() == 2)
}

fn github_on(app: &AppHandle) -> bool {
    let state = app.state::<crate::settings::SettingsState>();
    state
        .0
        .lock()
        .is_ok_and(|s| s.connectors.get("github").copied().unwrap_or(false))
}

fn row(t: Task, github: bool) -> Row {
    let running = crate::runs::running(t.id);
    Row {
        can_open_issue: github && t.issue.is_none() && github_repo(&t.roost).is_some(),
        running,
        can_start: !running
            && !t.roost.is_empty()
            && !t.folder.is_empty()
            && !matches!(t.status.as_str(), "done" | "dropped"),
        has_worktree: crate::runs::worktree_dir(&t).join(".git").exists(),
        reply: t.reply.clone(),
        has_session: t.session.is_some(),
        session_engine: t.session.as_deref().map(crate::runs::session_owner),
        pack: t.pack.clone(),
        route: t.route.clone(),
        review: t.review.clone(),
        review_verdict: t.review_verdict.clone(),
        review_round: t.review_round,
        id: t.id,
        goal: t.goal,
        criteria: t
            .criteria
            .into_iter()
            .map(|c| Criterion {
                text: c.text,
                done: c.done,
            })
            .collect(),
        roost_key: t.roost,
        roost: t.roost_name,
        folder: if t.folder.is_empty() {
            String::new()
        } else {
            crate::paths::shown(Path::new(&t.folder))
        },
        owner: t.owner,
        status: t.status,
        cost_usd: t.cost_usd,
        tokens: t.tokens,
        issue: t.issue,
        updated_ms: t.updated_ms,
    }
}

/// A goal and its criteria from what the user typed: each line that starts with `-`, `*`, `•` or
/// `[ ]` is a criterion, the other lines are the goal (a `done when:` line alone is skipped). Within
/// the goal, `done when:` starts more criteria, split by `;`, so one line holds a whole task.
pub fn parse(text: &str) -> (String, Vec<String>) {
    let mut goal = Vec::new();
    let mut criteria = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        match criterion(line) {
            Some(c) if !c.is_empty() => criteria.push(c.to_string()),
            Some(_) => {}
            None if line.eq_ignore_ascii_case("done when:") => {}
            None => goal.push(line),
        }
    }
    let goal = goal.join(" ");
    // ASCII lowering keeps every byte where it was, so the offset holds in the original.
    let goal = match goal.to_ascii_lowercase().find("done when:") {
        Some(i) => {
            let (head, rest) = goal.split_at(i);
            let inline = rest[10..]
                .split(';')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_owned);
            criteria.splice(0..0, inline);
            head.trim()
                .trim_end_matches(['.', ',', '-', '—'])
                .trim()
                .to_string()
        }
        None => goal,
    };
    (goal, criteria)
}

/// A line's criterion, without its one marker; none for a line that has no marker.
fn criterion(line: &str) -> Option<&str> {
    let rest = ["- ", "* ", "• ", "-\t", "[ ]", "[x]", "[X]"]
        .iter()
        .find_map(|m| line.strip_prefix(m))
        .or_else(|| (line == "-" || line == "*").then_some(""))?;
    let rest = rest.trim_start();
    Some(
        ["[ ]", "[x]", "[X]"]
            .iter()
            .find_map(|m| rest.strip_prefix(m))
            .unwrap_or(rest)
            .trim(),
    )
}

/// A new task from `folder` (the chat's, or the Nest's choice): its roost is that folder's
/// repository.
fn new_task(goal: String, criteria: Vec<String>, folder: Option<&Path>) -> New {
    let repo = folder.and_then(crate::roosts::repo_of);
    New {
        goal,
        criteria,
        roost: repo.as_ref().map(|r| r.key()).unwrap_or_default(),
        roost_name: repo.as_ref().map(|r| r.name()).unwrap_or_default(),
        folder: folder
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default(),
        owner: "you".into(),
    }
}

/// A `/task` typed in the chat; none for any other text.
#[tauri::command]
pub async fn task_command(
    app: AppHandle,
    window: tauri::Webview,
    text: String,
    folder: Option<String>,
) -> Result<Option<Said>, String> {
    from_user(&window)?;
    let trimmed = text.trim();
    let word = |w: &str| {
        trimmed
            .strip_prefix(w)
            .filter(|r| r.is_empty() || r.starts_with(char::is_whitespace))
    };
    if let Some(rest) = word("/delegate") {
        return delegate(app, rest, folder).await.map(Some);
    }
    let Some(rest) = word("/task") else {
        return Ok(None);
    };
    let (goal, criteria) = parse(rest);
    if goal.is_empty() {
        return Ok(Some(Said {
            said: "Say what the task is: /task, its goal, then a line for each criterion starting with -.",
            detail: None,
        }));
    }
    let folder = folder.map(PathBuf::from).filter(|d| d.is_absolute());
    let task = new_task(goal, criteria, folder.as_deref());
    let count = task.criteria.len();
    let detail = match &task.roost_name {
        name if name.is_empty() => task.goal.clone(),
        name => format!("{} ({name})", task.goal),
    };
    tauri::async_runtime::spawn_blocking(move || {
        open()?.task_add(&task, now_ms()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    changed(&app);
    Ok(Some(Said {
        said: match count {
            0 => "Task added, with no criteria yet:",
            1 => "Task added, with 1 criterion:",
            _ => "Task added, with its criteria:",
        },
        detail: Some(detail),
    }))
}

/// An agent's name as the user reads it.
fn agent_name(agent: vults_zeca::router::Agent) -> &'static str {
    match agent {
        vults_zeca::router::Agent::Claude => "Claude Code",
        vults_zeca::router::Agent::Codex => "Codex",
    }
}

fn agent_word(agent: vults_zeca::router::Agent) -> &'static str {
    match agent {
        vults_zeca::router::Agent::Claude => "claude",
        vults_zeca::router::Agent::Codex => "codex",
    }
}

/// `/delegate <goal>` (W8): a task, as `/task` makes it, started at once on the agent the router
/// chooses, and the card says why. `/delegate default claude|codex` sets the roost's starting rule.
async fn delegate(app: AppHandle, rest: &str, folder: Option<String>) -> Result<Said, String> {
    use vults_zeca::router::{self, Agent, Why};
    let folder = folder.map(PathBuf::from).filter(|d| d.is_absolute());
    let repo = folder.as_deref().and_then(crate::roosts::repo_of);
    let Some(repo) = repo else {
        return Ok(Said {
            said: "Delegate from a conversation in a repository: a task works in one.",
            detail: None,
        });
    };
    let rest = rest.trim();
    // `default` then an agent (or `none`), in any case: anything else after it is a goal.
    let mut words = rest.split_whitespace();
    if words.next().is_some_and(|w| w.eq_ignore_ascii_case("default")) {
        let pick = words.next().map(str::to_ascii_lowercase);
        let agent = match (pick.as_deref(), words.next()) {
            (Some("claude"), None) => Some(Some(Agent::Claude)),
            (Some("codex"), None) => Some(Some(Agent::Codex)),
            (Some("none"), None) => Some(None),
            (None, _) => {
                return Ok(Said {
                    said: "Say which agent: /delegate default claude, or /delegate default codex.",
                    detail: None,
                });
            }
            _ => None,
        };
        if let Some(agent) = agent {
            let key = repo.key();
            crate::settings::edit(&app, |s| match agent {
                Some(a) => {
                    s.delegate.insert(key, agent_word(a).to_string());
                }
                None => {
                    s.delegate.remove(&key);
                }
            })?;
            return Ok(match agent {
                Some(a) => Said {
                    said: "From now on, this project's delegated tasks start on:",
                    detail: Some(agent_name(a).to_string()),
                },
                None => Said {
                    said: "This project has no rule now: its delegated tasks start on Claude Code.",
                    detail: None,
                },
            });
        }
    }
    let (goal, criteria) = parse(rest);
    if goal.is_empty() {
        return Ok(Said {
            said: "Say what to delegate: /delegate, its goal, then a line for each criterion starting with -.",
            detail: None,
        });
    }
    let task = new_task(goal, criteria, folder.as_deref());
    let (roost, kind) = (task.roost.clone(), router::kind(&task.goal));
    let rule = {
        let state = app.state::<crate::settings::SettingsState>();
        state.0.lock().ok().and_then(|s| s.delegate.get(&roost).cloned())
    }
    .and_then(|a| match a.as_str() {
        "claude" => Some(Agent::Claude),
        "codex" => Some(Agent::Codex),
        _ => None,
    });
    let quota = |a: Agent| {
        crate::usage::quota(
            &app,
            match a {
                Agent::Claude => vults_protocol::AgentKind::Claude,
                Agent::Codex => vults_protocol::AgentKind::Codex,
            },
        )
    };
    // Spent: no task is made that could not start (W9).
    let a = app.clone();
    let spent = tauri::async_runtime::spawn_blocking(move || crate::spend::refuse(&a))
        .await
        .map_err(|e| e.to_string())?;
    if spent.is_err() {
        return Ok(Said {
            said: "Today's budget is spent: Vults starts no new session until tomorrow.",
            detail: None,
        });
    }
    let (id, past) = tauri::async_runtime::spawn_blocking(move || {
        let mut store = open()?;
        // What the roosts learned from: their finished tasks, by who owned them.
        let past: Vec<router::Past> = store
            .tasks(true, 500)
            .map_err(|e| e.to_string())?
            .into_iter()
            // Done or dropped: a blocked task is still open, and often blocked by its setting, not
            // by its agent.
            .filter(|t| matches!(t.status.as_str(), "done" | "dropped"))
            .filter_map(|t| {
                let agent = match t.owner.as_str() {
                    "claude" => Agent::Claude,
                    "codex" => Agent::Codex,
                    _ => return None,
                };
                Some(router::Past {
                    agent,
                    kind: router::kind(&t.goal),
                    roost: t.roost,
                    done: t.status == "done",
                })
            })
            .collect();
        let id = store.task_add(&task, now_ms()).map_err(|e| e.to_string())?;
        Ok::<_, String>((id, past))
    })
    .await
    .map_err(|e| e.to_string())??;
    let choice = router::choose(&roost, kind, rule, quota, &past);
    let (agent, why) = (choice.agent, choice.why);
    let route = match why {
        Why::Rule => format!("rule:{}", agent_word(agent)),
        Why::Default => format!("default:{}", agent_word(agent)),
        Why::Quota { busy, used } => format!("quota:{}:{}:{used}", agent_word(agent), agent_word(busy)),
        Why::Score { done, of } => format!("score:{}:{done}:{of}", agent_word(agent)),
    };
    let engine = match agent {
        Agent::Claude => crate::runs::Engine::Claude,
        Agent::Codex => crate::runs::Engine::Codex,
    };
    if let Err(e) = crate::runs::start_task(app.clone(), id, engine).await {
        changed(&app);
        return Ok(Said {
            said: "The task is in the Nest, but it didn't start:",
            detail: Some(e),
        });
    }
    // Kept once it started: a card never says a task was sent where it never went.
    let _ = tauri::async_runtime::spawn_blocking(move || {
        open()?.task_routed(id, &route).map_err(|e| e.to_string())
    })
    .await;
    changed(&app);
    let name = agent_name(agent);
    Ok(match why {
        Why::Rule => Said {
            said: "Delegated by this project's rule to:",
            detail: Some(name.into()),
        },
        Why::Default => Said {
            said: "Delegated to the default agent:",
            detail: Some(name.into()),
        },
        Why::Quota { busy, used } => Said {
            said: "Delegated away from a plan above 85% to:",
            detail: Some(format!("{name} ({} {used}%)", agent_name(busy))),
        },
        Why::Score { done, of } => Said {
            said: "Delegated to the agent that finished most tasks like it here:",
            detail: Some(format!("{name} ({done}/{of})")),
        },
    })
}

/// The tasks, those still to do first; `closed` adds the done and dropped ones.
#[tauri::command]
pub async fn tasks_list(app: AppHandle, closed: bool) -> Result<Vec<Row>, String> {
    let github = github_on(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let list = open()?.tasks(closed, 200).map_err(|e| e.to_string())?;
        Ok(list.into_iter().map(|t| row(t, github)).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The Nest's **New task**: a goal and its criteria, in a folder (or none).
#[tauri::command]
pub async fn task_create(
    app: AppHandle,
    window: tauri::Webview,
    goal: String,
    criteria: Vec<String>,
    folder: Option<String>,
) -> Result<i64, String> {
    from_user(&window)?;
    let folder = folder.map(PathBuf::from).filter(|d| d.is_absolute());
    let task = new_task(goal, criteria, folder.as_deref());
    let id = tauri::async_runtime::spawn_blocking(move || {
        open()?.task_add(&task, now_ms()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    changed(&app);
    Ok(id)
}

/// One change from the Nest.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Edit {
    Status {
        status: String,
    },
    Owner {
        owner: String,
    },
    Criterion {
        index: usize,
        done: bool,
    },
    /// An issue linked by its address, or `#12` in the task's GitHub repository; empty unlinks.
    Issue {
        issue: String,
    },
}

/// An issue's address from what the user typed: `#12` in the task's repository, or a GitHub
/// issue's address; none for anything else.
fn issue_url(typed: &str, roost: &str) -> Option<String> {
    let typed = typed.trim();
    if let Some(n) = typed.strip_prefix('#') {
        let repo = github_repo(roost)?;
        return (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
            .then(|| format!("https://github.com/{repo}/issues/{n}"));
    }
    let rest = typed.strip_prefix("https://github.com/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    let plain = |p: &str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    match parts.as_slice() {
        [owner, name, "issues", n]
            if plain(owner) && plain(name) && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) =>
        {
            Some(typed.to_owned())
        }
        _ => None,
    }
}

#[tauri::command]
pub async fn task_edit(app: AppHandle, window: tauri::Webview, id: i64, edit: Edit) -> Result<(), String> {
    from_user(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = open()?;
        let change = match edit {
            Edit::Status { status } => Change::Status(status),
            Edit::Owner { owner } => {
                if !OWNERS.contains(&owner.as_str()) {
                    return Err("No such owner.".to_string());
                }
                Change::Owner(owner)
            }
            Edit::Criterion { index, done } => Change::Criterion(index, done),
            Edit::Issue { issue } if issue.trim().is_empty() => Change::Issue(None),
            Edit::Issue { issue } => {
                let task = store
                    .task(id)
                    .map_err(|e| e.to_string())?
                    .ok_or("No such task.")?;
                Change::Issue(Some(
                    issue_url(&issue, &task.roost).ok_or("That is not a GitHub issue.")?,
                ))
            }
        };
        store
            .task_change(id, &change, now_ms())
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn task_delete(app: AppHandle, window: tauri::Webview, id: i64) -> Result<(), String> {
    from_user(&window)?;
    // A running task's row is where its Stop is: it goes only once stopped.
    if crate::runs::running(id) {
        return Err("Stop the task first.".into());
    }
    tauri::async_runtime::spawn_blocking(move || open()?.task_delete(id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??;
    changed(&app);
    Ok(())
}

/// The longest issue title GitHub keeps.
const TITLE_MAX: usize = 256;

/// The tasks whose issue `gh` is opening now.
static OPENING: std::sync::Mutex<Vec<i64>> = std::sync::Mutex::new(Vec::new());

/// Holds a task in [`OPENING`] while its issue is opened, and lets it go however that ends.
struct Opening(i64);

impl Opening {
    fn start(id: i64) -> Option<Self> {
        let mut list = OPENING.lock().ok()?;
        if list.contains(&id) {
            return None;
        }
        list.push(id);
        Some(Self(id))
    }
}

impl Drop for Opening {
    fn drop(&mut self) {
        if let Ok(mut list) = OPENING.lock() {
            list.retain(|i| *i != self.0);
        }
    }
}

/// **Open an issue**: the task becomes an issue in its GitHub repository, through the user's `gh`,
/// on their click and only with the GitHub connector on; the task links to it.
#[tauri::command]
pub async fn task_open_issue(app: AppHandle, window: tauri::Webview, id: i64) -> Result<String, String> {
    from_user(&window)?;
    if !github_on(&app) {
        return Err("Turn on the GitHub connector in Settings first.".into());
    }
    let task = tauri::async_runtime::spawn_blocking(move || open()?.task(id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??
        .ok_or("No such task.")?;
    if task.issue.is_some() {
        return Err("This task already has an issue.".into());
    }
    let repo = github_repo(&task.roost).ok_or("This task is not in a GitHub repository.")?;
    // One issue per task: a second click while `gh` runs is refused, not a second issue.
    let _opening = Opening::start(id).ok_or("This task's issue is being opened.")?;
    // GitHub keeps 256 characters of a title: a longer goal goes whole into the body.
    let title: String = task.goal.chars().take(TITLE_MAX).collect();
    let mut body = task
        .criteria
        .iter()
        .map(|c| format!("- [{}] {}", if c.done { "x" } else { " " }, c.text))
        .collect::<Vec<_>>()
        .join("\n");
    if title != task.goal {
        body = format!("{}\n\n{body}", task.goal);
    }
    let url = vults_connectors::github::create_issue(repo, &title, &body)
        .await
        .map_err(|e| e.to_string())?;
    let link = url.clone();
    tauri::async_runtime::spawn_blocking(move || {
        open()?
            .task_change(id, &Change::Issue(Some(link)), now_ms())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    changed(&app);
    Ok(url)
}

/// Opens the task's linked issue in the browser: the address kept with the task, never one the
/// window sends.
#[tauri::command]
pub async fn task_visit_issue(app: AppHandle, window: tauri::Webview, id: i64) -> Result<(), String> {
    from_user(&window)?;
    let task = tauri::async_runtime::spawn_blocking(move || open()?.task(id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??
        .ok_or("No such task.")?;
    let url = task
        .issue
        .as_deref()
        .and_then(vults_core::SafeUrl::parse)
        .ok_or("This task's issue can't be opened.")?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_message_makes_a_task_with_its_criteria() {
        let (goal, criteria) =
            parse(" Fix the flaky login test\n- passes 20 runs in a row\n* no sleep added\n[ ] CI green\n");
        assert_eq!(goal, "Fix the flaky login test");
        assert_eq!(
            criteria,
            ["passes 20 runs in a row", "no sleep added", "CI green"]
        );
        let (goal, criteria) =
            parse("Fix X\non two lines\nDone when:\n- a\n- --no-verify unused\n[X] shipped");
        assert_eq!(goal, "Fix X on two lines");
        assert_eq!(criteria, ["a", "--no-verify unused", "shipped"]);
        let (goal, criteria) = parse("Ship the docs. Done when: both languages; links checked");
        assert_eq!(goal, "Ship the docs");
        assert_eq!(criteria, ["both languages", "links checked"]);
        assert_eq!(parse("   "), (String::new(), Vec::new()));
        let (goal, criteria) = parse("写文档 — done when: ok");
        assert_eq!(
            (goal.as_str(), criteria.as_slice()),
            ("写文档", ["ok".to_string()].as_slice())
        ); // check-english:allow
    }

    #[test]
    fn an_issue_is_a_github_issue_of_the_task_or_typed_whole() {
        let roost = "github:me/site";
        assert_eq!(
            issue_url("#12", roost).as_deref(),
            Some("https://github.com/me/site/issues/12")
        );
        assert_eq!(
            issue_url("#12", "/home/me/site"),
            None,
            "no repository to number it in"
        );
        assert_eq!(issue_url("#x", roost), None);
        assert_eq!(
            issue_url("https://github.com/other/app/issues/3", roost).as_deref(),
            Some("https://github.com/other/app/issues/3")
        );
        for bad in [
            "https://github.com/me/site/pull/3",
            "https://evil.example/me/site/issues/3",
            "javascript:alert(1)",
            "https://github.com/me/site/issues/3?x",
        ] {
            assert_eq!(issue_url(bad, roost), None, "{bad}");
        }
    }
}
