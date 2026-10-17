//! Sessions Vults starts for a task (`docs/dev/plan-zeca.md` W4): on the user's click, a clone
//! of the task's repository of its own (`git clone --shared`: the repository's objects are read
//! through alternates, never written), on the task's branch, and Claude Code or Codex in it through
//! the driver (S8, S11), sandboxed (a card before each command), with the user's own settings, so
//! the session shows in the flock like any other and its permissions are island cards, answered by
//! a human, a rule or a policy, never here. The worker writes only its clone: nothing it does there
//! can touch the user's repository, its objects, refs, hooks or config. Vults brings the task's
//! branch back by fetching that one ref; its own git in a clone runs with no hooks and no fsmonitor,
//! and never runs a command (`status`) that could start a filter the clone's config names.
//!
//! Only these sessions can be stopped from Vults (D8): ask the turn to stop, wait, then end the
//! process. A watched session never is.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use vults_agents::driver::{
    Answer, Ask, BoxFuture, Event, Input, Interrupter, Role, Sandbox, Settings, Sink, Spec,
};
use vults_store::Store;
use vults_store::tasks::{Change, Task};

/// Told to every window when a run starts or ends (the Tasks tab draws it).
const CHANGED: &str = "tasks-changed";
/// How long a stopped turn has to end before its process is ended.
const STOP_WAIT: Duration = Duration::from_secs(10);

/// A run Vults holds: its generation (so a run that ended late never removes a newer one), what
/// stops its turn, the turn itself, and what it has cost so far (kept when the turn is ended).
struct Run {
    generation: u64,
    /// A review (W6): Stop ends it without touching the task's status.
    review: bool,
    /// Set by Stop: a run between turns (a handoff's pack, then its new session) starts no more.
    stopped: Arc<std::sync::atomic::AtomicBool>,
    interrupter: Option<Arc<dyn Interrupter>>,
    turn: Option<tauri::async_runtime::JoinHandle<()>>,
    cost: Arc<Mutex<Spent>>,
}

/// What a run has spent, until its task is charged. A session's running total (Claude Code's)
/// waits in `pending` for the turn's end, when the session's id is known for the ledger; a run
/// whose turn Stop had to end settles it when charged.
#[derive(Debug, Default)]
struct Spent {
    usd: f64,
    tokens: u64,
    pending: Option<crate::spend::Pending>,
    session: Option<String>,
}

/// The runs going on now, by task; a task is in it from the click on Start, before anything else.
fn runs() -> &'static Mutex<HashMap<i64, Run>> {
    static RUNS: std::sync::OnceLock<Mutex<HashMap<i64, Run>>> = std::sync::OnceLock::new();
    RUNS.get_or_init(Mutex::default)
}

/// Whether the task has a session Vults started, running (or starting) now.
pub fn running(id: i64) -> bool {
    runs().lock().is_ok_and(|r| r.contains_key(&id))
}

/// Holds the task's place in [`runs`] for one generation; a start that fails before its turn runs
/// gives it back.
fn reserve(id: i64) -> Result<u64, String> {
    reserve_as(id, false)
}

fn reserve_as(id: i64, review: bool) -> Result<u64, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let mut r = runs().lock().map_err(|e| e.to_string())?;
    if r.contains_key(&id) {
        return Err("This task is already running.".into());
    }
    let generation = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    r.insert(
        id,
        Run {
            generation,
            review,
            stopped: Arc::default(),
            interrupter: None,
            turn: None,
            cost: Arc::default(),
        },
    );
    Ok(generation)
}

/// Lets the task's place go, only if it is still this generation's.
fn release(id: i64, generation: u64) {
    if let Ok(mut r) = runs().lock()
        && r.get(&id).is_some_and(|run| run.generation == generation)
    {
        r.remove(&id);
    }
}

fn open() -> Result<Store, String> {
    Store::open(&crate::history::dir()).map_err(|e| e.to_string())
}

/// The windows the user acts in.
fn from_user(window: &tauri::Webview) -> Result<(), String> {
    if [crate::ISLAND, crate::NEST].contains(&window.label()) {
        Ok(())
    } else {
        Err("only the chat and the Nest can do this".into())
    }
}

/// The task's branch: its own, so two tasks never write the same tree.
fn branch(id: i64) -> String {
    format!("vults/task-{id}")
}

/// Where the task's clone lives: under Vults' data folder, named after its roost so the flock
/// reads it as that project's (`site-task-12`).
pub fn worktree_dir(task: &Task) -> PathBuf {
    let slug: String = task
        .roost_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(40)
        .collect();
    let slug = if slug.trim_matches('-').is_empty() {
        "task".to_string()
    } else {
        slug
    };
    crate::paths::data_dir()
        .join("worktrees")
        .join(format!("{slug}-task-{}", task.id))
}

/// Runs git in `repo`, no prompt, no pager, no network of its own: an error is git's own words.
fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    // A clone's config is the worker's to write: none of it may run anything here.
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "core.fsmonitor=false", "-c", "core.hooksPath=/dev/null"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("can't run git: {e}"))?;
    if out.status.success() {
        // Only the end: a status line's leading space is part of it.
        Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Where Vults keeps the commit a task's branch began from: beside its clone, outside anything the
/// worker may write.
fn base_file(dir: &Path) -> PathBuf {
    dir.with_extension("base")
}

/// The task's clone, made on first start on the task's branch (the repository's own, when it has
/// one; else from its current commit); reused when it is there.
fn workspace(task: &Task) -> Result<PathBuf, String> {
    let dir = worktree_dir(task);
    workspace_at(task, &dir)?;
    Ok(dir)
}

fn workspace_at(task: &Task, dir: &Path) -> Result<(), String> {
    let repo =
        crate::roosts::repo_of(Path::new(&task.folder)).ok_or("This task is not in a git repository.")?;
    let dir = dir.to_path_buf();
    if std::fs::symlink_metadata(dir.join(".git")).is_ok_and(|m| m.is_dir()) {
        return untouched(&dir, &repo.root);
    }
    if dir.exists() {
        // An earlier Vults' worktree, or something else: never reused, never written over.
        return Err("This task's folder is in the way: remove it first.".into());
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let name = branch(task.id);
    let existing = git(
        &repo.root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}^{{commit}}"),
        ],
    )
    .ok();
    let head = git(&repo.root, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let start = existing.clone().unwrap_or_else(|| head.clone());
    let (root, target) = (
        repo.root.to_string_lossy().into_owned(),
        dir.to_string_lossy().into_owned(),
    );
    git(
        &repo.root,
        &[
            "clone",
            "--quiet",
            "--shared",
            "--no-checkout",
            "--no-tags",
            &root,
            &target,
        ],
    )?;
    git(&dir, &["checkout", "--quiet", "-b", &name, &start])?;
    std::fs::copy(dir.join(".git/config"), config_file(&dir)).map_err(|e| e.to_string())?;
    let base = match existing {
        Some(b) => git(&repo.root, &["merge-base", &b, &head]).unwrap_or(head),
        None => head,
    };
    std::fs::write(base_file(&dir), base).map_err(|e| e.to_string())
}

/// Brings the task's branch back into the repository: that one ref, fetched from its clone. The
/// fetch runs nothing of the clone's (no hooks, no filters); the objects are checked as they come.
fn bring_back(task: &Task) -> Result<(), String> {
    bring_back_at(task, &worktree_dir(task))
}

fn bring_back_at(task: &Task, dir: &Path) -> Result<(), String> {
    let repo =
        crate::roosts::repo_of(Path::new(&task.folder)).ok_or("This task is not in a git repository.")?;
    if !dir.join(".git").exists() {
        return Ok(());
    }
    // The clone borrows only this repository's objects, and nothing in it may run: else its branch
    // is never fetched (alternates elsewhere would carry another repository's objects in).
    untouched(dir, &repo.root)?;
    let name = branch(task.id);
    git(
        &repo.root,
        &[
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-recurse-submodules",
            "--no-write-fetch-head",
            &dir.to_string_lossy(),
            &format!("+refs/heads/{name}:refs/heads/{name}"),
        ],
    )?;
    // A fetch can refuse a ref and still exit 0: what the clone offers must now be here.
    let offered = git(
        &repo.root,
        &["ls-remote", &dir.to_string_lossy(), &format!("refs/heads/{name}")],
    )?;
    let offered = offered.split_whitespace().next().unwrap_or_default().to_string();
    let here = git(
        &repo.root,
        &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{name}")],
    )
    .unwrap_or_default();
    if offered.is_empty() || offered != here {
        return Err("The task's branch didn't come back into the repository.".into());
    }
    Ok(())
}

/// The repository's objects folder, as a clone's alternates name it.
fn objects_of(repo: &Path) -> Result<PathBuf, String> {
    let common = git(repo, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
    Ok(PathBuf::from(common).join("objects"))
}

/// Whether a clone is as Vults left it where git would read code or other repositories from it:
/// a real `.git` folder, no `commondir` or `gitdir` redirect, alternates naming only the
/// repository's objects, no hook but git's samples, no attributes of its own, and the config Vults
/// kept a copy of. A clone that isn't is never reused, and its branch never fetched.
fn untouched(dir: &Path, repo: &Path) -> Result<(), String> {
    let changed =
        || Err("This task's folder was changed in a way Vults won't trust: remove its folder.".to_string());
    let dot = dir.join(".git");
    if !std::fs::symlink_metadata(&dot).is_ok_and(|m| m.is_dir()) {
        return changed();
    }
    if dot.join("commondir").exists() || dot.join("gitdir").exists() || dot.join("info/attributes").exists() {
        return changed();
    }
    let alternates = std::fs::read_to_string(dot.join("objects/info/alternates")).unwrap_or_default();
    let expected = objects_of(repo)?;
    let lines: Vec<&str> = alternates
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() != 1 || Path::new(lines[0]) != expected {
        return changed();
    }
    if let Ok(hooks) = std::fs::read_dir(dot.join("hooks"))
        && hooks
            .flatten()
            .any(|h| !h.file_name().to_string_lossy().ends_with(".sample"))
    {
        return changed();
    }
    let kept = std::fs::read(config_file(dir)).ok();
    if kept.is_none() || kept != std::fs::read(dot.join("config")).ok() {
        return changed();
    }
    Ok(())
}

/// Where Vults keeps a copy of a clone's config, beside it, outside anything the worker may write.
fn config_file(dir: &Path) -> PathBuf {
    dir.with_extension("config")
}

/// What the worker reads before anything: the task, where it works, and what it must not do.
fn brief(task: &Task) -> String {
    let mut text = format!(
        "You work on one task, alone, in a git worktree of {roost} on the branch {branch}. Stay in this folder.\n\nGoal: {goal}\n",
        roost = if task.roost_name.is_empty() {
            "the project"
        } else {
            &task.roost_name
        },
        branch = branch(task.id),
        goal = task.goal,
    );
    if !task.criteria.is_empty() {
        text.push_str("\nIt is done when:\n");
        for c in &task.criteria {
            text.push_str(&format!("- {}\n", c.text));
        }
    }
    text.push_str(
        "\nWhen the criteria are met, commit your work on this branch and say what you did and how each criterion is met. Never push, and never change another branch. If you are stuck, say what you need.",
    );
    text
}

/// Where a run's driver reports and asks. Hooks observe; the driver owns permission cards.
struct RunSink {
    task: i64,
    cost: Arc<Mutex<Spent>>,
    reply: Mutex<String>,
    app: AppHandle,
    /// The task's clone, the folder a card names.
    dir: String,
    /// The running driver's agent and session, for a card the app makes for it.
    engine: Mutex<Engine>,
    session: Mutex<Option<String>>,
    /// The run's generation: an app-made card's id is this run's alone.
    generation: u64,
    /// The roost its spend counts in (W9): its task's, or the folder's for a `/continue`.
    roost: String,
}

impl Sink for RunSink {
    fn event(&self, event: Event) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            match event {
                Event::Session(id) => {
                    if let Ok(mut session) = self.session.lock() {
                        *session = Some(id);
                    }
                }
                Event::Text(text) => {
                    if let Ok(mut r) = self.reply.lock() {
                        r.push_str(&text);
                    }
                }
                Event::Usage(u) => {
                    // Tokens read back from the cache are not new work: left out of the count.
                    let tokens = u.input.saturating_sub(u.cache_read) + u.output;
                    // The turn's own cost counts now. Only a turn whose cost the driver could not
                    // tell (a resumed session's first) is counted from the session's running total,
                    // at the turn's end; either way that total becomes the ledger's baseline.
                    let usd = u.cost_usd.unwrap_or(0.0);
                    let engine = self.engine.lock().map(|e| *e).unwrap_or(Engine::Claude);
                    let plan = crate::spend::plan(&self.app, engine.agent());
                    if let Ok(mut c) = self.cost.lock() {
                        c.tokens += tokens;
                        c.usd += usd;
                        if let Some(total) = u.session_cost_usd {
                            c.pending = Some(crate::spend::Pending {
                                total,
                                counted: u.cost_usd.is_some(),
                                agent: engine.agent(),
                                roost: self.roost.clone(),
                                plan,
                            });
                        }
                    }
                    let (app, task, roost) = (self.app.clone(), self.task, self.roost.clone());
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        crate::spend::run(&app, task, &roost, plan, usd, tokens);
                    })
                    .await;
                }
                // What it writes after a tool starts on a line of its own: a heading there is still
                // a heading (a pack's).
                Event::ToolStart { .. } => {
                    if let Ok(mut r) = self.reply.lock()
                        && !r.is_empty()
                        && !r.ends_with('\n')
                    {
                        r.push('\n');
                    }
                }
                Event::TurnEnd(reason) => {
                    tracing::info!(task = self.task, ?reason, "task run's turn ended");
                }
                _ => {}
            }
        })
    }

    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        Box::pin(async move {
            let engine = self.engine.lock().map(|e| *e).unwrap_or(Engine::Claude);
            // The agents ask on their own channel: the app makes the card
            // a hook would, so the island, rules, policies and the phone answer it as ever.
            let session = self
                .session
                .lock()
                .ok()
                .and_then(|s| s.clone())
                .unwrap_or_else(|| format!("vults-task-{}", self.task));
            // A call the card couldn't name whole (no command, edits whose files Codex never said)
            // is a no, never a card with less on it than what it allows.
            let card = if engine == Engine::Claude {
                claude_card(self.task, self.generation, &ask, &session, &self.dir)
            } else if engine == Engine::Codex {
                codex_card(self.task, self.generation, &ask, &session, &self.dir)
            } else {
                acp_card(engine, self.task, self.generation, &ask, &session, &self.dir)
            };
            let Some(event) = card else {
                tracing::info!(task = self.task, tool = %ask.tool, "a call that can't be named whole: no");
                return Answer::Deny;
            };
            match crate::runtime::ask_human(&self.app, event).await {
                vults_ipc::Local::Decide(vults_protocol::Decision::Allow) => Answer::Allow,
                _ => Answer::Deny,
            }
        })
    }
}

/// Claude's control channel carries the original tool input, just like its permission hook.
fn claude_card(
    task: i64,
    generation: u64,
    ask: &Ask,
    session: &str,
    dir: &str,
) -> Option<vults_protocol::Event> {
    if ask.tool.is_empty() || !ask.input.is_object() || ask.tool == vults_protocol::QUESTION_TOOL {
        return None;
    }
    Some(vults_protocol::Event {
        v: vults_protocol::VERSION,
        id: format!("task-{task}-{generation}-{}", ask.id),
        agent: vults_protocol::AgentKind::Claude,
        agent_name: None,
        event: "PermissionRequest".into(),
        wants_reply: true,
        zeca: false,
        terminal: vults_protocol::Terminal::default(),
        payload: serde_json::json!({
            "session_id": session, "cwd": dir, "tool_name": ask.tool, "tool_input": ask.input,
        }),
    })
}

/// The hook event a Codex `PermissionRequest` would send for `ask`, in the session and folder of
/// the task's run: a command whole (its own folder when Codex names one), an edit as the patch of
/// its changes, so the card shows its files and line counts as a hook's would. None when the call
/// can't be named whole.
fn codex_card(
    task: i64,
    generation: u64,
    ask: &Ask,
    session: &str,
    dir: &str,
) -> Option<vults_protocol::Event> {
    let (tool, input) = match ask.tool.as_str() {
        "Bash" => {
            let command = ask
                .full
                .clone()
                .or_else(|| ask.input["command"].as_str().map(str::to_owned))?;
            (!command.trim().is_empty()).then_some(())?;
            ("Bash", serde_json::json!({ "command": command }))
        }
        "Edit" => (
            "apply_patch",
            serde_json::json!({ "command": patch(&ask.input["changes"])? }),
        ),
        _ => return None,
    };
    let cwd = ask.input["cwd"]
        .as_str()
        .filter(|c| c.starts_with('/'))
        .unwrap_or(dir);
    Some(vults_protocol::Event {
        v: vults_protocol::VERSION,
        id: format!("task-{task}-{generation}-{}", ask.id),
        agent: vults_protocol::AgentKind::Codex,
        agent_name: None,
        event: "PermissionRequest".into(),
        wants_reply: true,
        zeca: false,
        terminal: vults_protocol::Terminal::default(),
        payload: serde_json::json!({
            "hook_event_name": "PermissionRequest",
            "session_id": session,
            "cwd": cwd,
            "tool_name": tool,
            "tool_input": input,
        }),
    })
}

/// The hook event an ACP agent's permission would be if its hook could wait for an answer, in the
/// task's clone: a command whole, one file's edit with its change, an address. None when it can't
/// be named whole, or when it reaches out of the clone (a file elsewhere or in its `.git`, a
/// command run elsewhere): the worker writes its clone only, and a no needs no card.
fn acp_card(
    engine: Engine,
    task: i64,
    generation: u64,
    ask: &Ask,
    session: &str,
    dir: &str,
) -> Option<vults_protocol::Event> {
    let root = Path::new(dir);
    let (tool, input) = match ask.tool.as_str() {
        "Bash" => {
            let command = ask
                .full
                .clone()
                .or_else(|| ask.input["command"].as_str().map(str::to_owned))?;
            (!command.trim().is_empty()).then_some(())?;
            if let Some(cwd) = ask.input["cwd"].as_str()
                && !vults_chat::acp::inside(root, cwd)
                && Path::new(cwd) != root
            {
                return None;
            }
            ("Bash".to_string(), serde_json::json!({ "command": command }))
        }
        "Edit" => {
            if ask.input["delete"] == true {
                return None;
            }
            let paths: Vec<&str> = ask.input["paths"]
                .as_array()?
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect();
            let [path] = paths.as_slice() else {
                return None;
            };
            if !vults_chat::acp::inside(root, path) {
                return None;
            }
            let diff = ask.input["diffs"]
                .as_array()?
                .iter()
                .find(|d| d["path"].as_str() == Some(path))?;
            let (old, new) = (diff["oldText"].as_str()?, diff["newText"].as_str()?);
            if old.is_empty() {
                (
                    "Write".to_string(),
                    serde_json::json!({ "file_path": path, "content": new }),
                )
            } else {
                (
                    "Edit".to_string(),
                    serde_json::json!({ "file_path": path, "old_string": old, "new_string": new }),
                )
            }
        }
        "WebFetch" => (
            "WebFetch".to_string(),
            serde_json::json!({ "url": ask.input["url"].as_str()? }),
        ),
        other => (other.to_string(), ask.input["input"].clone()),
    };
    Some(vults_protocol::Event {
        v: vults_protocol::VERSION,
        id: format!("task-{task}-{generation}-{}", ask.id),
        agent: engine.agent(),
        agent_name: engine.agent_name(),
        event: "PermissionRequest".into(),
        wants_reply: true,
        zeca: false,
        terminal: vults_protocol::Terminal::default(),
        payload: serde_json::json!({
            "hook_event_name": "PermissionRequest",
            "session_id": session,
            "cwd": dir,
            "tool_name": tool,
            "tool_input": input,
        }),
    })
}

/// End the task's turn and withdraw unanswered cards. Task hooks do not remove the session
/// when their short-lived CLI exits; its finished bird remains until the normal expiry.
async fn withdraw(app: &AppHandle, sink: &RunSink, failure: Option<&str>) {
    let engine = sink.engine.lock().map(|e| *e).unwrap_or(Engine::Claude);
    if let Some(session) = sink.session.lock().ok().and_then(|s| s.clone()) {
        crate::runtime::run_ended(app, ended(engine, &session, failure)).await;
    }
}

/// A failed driver must not overwrite a failed bird with a successful Stop.
fn ended(engine: Engine, session: &str, failure: Option<&str>) -> vults_core::AgentUpdate {
    vults_core::AgentUpdate {
        session: vults_core::SessionKey {
            agent: engine.agent(),
            session_id: session.into(),
        },
        cwd: None,
        terminal: vults_protocol::Terminal::default(),
        agent_id: None,
        received: None,
        event: match failure {
            Some(error) => vults_core::AgentEvent::StopFailed {
                error: Some(error.into()),
            },
            None => vults_core::AgentEvent::Stopped { message: None },
        },
    }
}

fn outcome_reply(reply: String, failure: Option<&str>) -> String {
    match failure {
        Some(error) if reply.trim().is_empty() => error.into(),
        Some(error) => format!("{reply}\n\n{error}"),
        None => reply,
    }
}

/// Codex's file changes (`[{path, kind, diff}]`) as the `apply_patch` text its hook would carry;
/// none without a file, for a move (it writes where the card would not name), or for a diff line
/// that reads as a patch header (it would add files the change doesn't touch).
fn patch(changes: &serde_json::Value) -> Option<String> {
    let changes = changes.as_array().filter(|c| !c.is_empty())?;
    let mut out = String::from("*** Begin Patch\n");
    for c in changes {
        let path = c["path"]
            .as_str()
            .filter(|p| !p.is_empty() && !p.contains('\n'))?;
        if c["kind"]["move_path"].as_str().is_some_and(|m| !m.is_empty()) {
            return None;
        }
        let kind = c["kind"]
            .as_str()
            .or_else(|| c["kind"]["type"].as_str())
            .unwrap_or("update");
        let head = match kind {
            "add" => "Add",
            "delete" => "Delete",
            _ => "Update",
        };
        out.push_str(&format!("*** {head} File: {path}\n"));
        for line in c["diff"].as_str().unwrap_or_default().lines() {
            if line.starts_with("*** ") {
                return None;
            }
            // An added file's diff is its content: each line is one added.
            if head == "Add" && !line.starts_with('+') {
                out.push('+');
            }
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("*** End Patch\n");
    Some(out)
}

/// Adds what a run cost to its task, and takes the cost out of the run so it is counted once.
fn charge(id: i64, cost: &Mutex<Spent>) {
    // A turn Stop had to end never settled its total.
    settle(None, cost);
    let spent = cost
        .lock()
        .map(|mut c| std::mem::take(&mut *c))
        .unwrap_or_default();
    let tokens = i64::try_from(spent.tokens).unwrap_or(i64::MAX);
    if (spent.usd > 0.0 || tokens > 0)
        && id != CONTINUING
        && let Err(e) = open().and_then(|mut s| s.task_cost(id, spent.usd, tokens).map_err(|e| e.to_string()))
    {
        tracing::warn!(task = id, error = %e, "a run's cost not kept");
    }
}

/// Which agent a run starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Claude,
    Codex,
    /// The agents that speak ACP (W10), by their key in `vults_chat::acp::agents()`.
    Opencode,
    Qwen,
    Copilot,
    Kimi,
}

impl Engine {
    const ALL: [Engine; 6] = [
        Self::Claude,
        Self::Codex,
        Self::Opencode,
        Self::Qwen,
        Self::Copilot,
        Self::Kimi,
    ];

    /// A driver for it in `spec`, going on with `session`.
    fn driver(
        self,
        spec: Spec,
        session: Option<String>,
    ) -> Option<Box<dyn vults_agents::driver::AgentDriver>> {
        match self {
            Self::Claude => vults_chat::drivers::resumed(vults_chat::Provider::Claude, spec, session),
            Self::Codex => vults_chat::drivers::resumed(vults_chat::Provider::Codex, spec, session),
            acp => vults_chat::drivers::acp(acp.owner(), spec, session),
        }
    }

    /// The owner a task gets, and the engine of a task's session by its owner.
    fn owner(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Opencode => "opencode",
            Self::Qwen => "qwen",
            Self::Copilot => "copilot",
            Self::Kimi => "kimi",
        }
    }

    /// Its kind on the wire, and its name when that is `other` (the registry's).
    fn wire(self) -> (vults_protocol::AgentKind, Option<&'static str>) {
        vults_agents::registry::of(self.owner())
            .map(|e| e.agent())
            .unwrap_or((vults_protocol::AgentKind::Other, None))
    }

    fn agent(self) -> vults_protocol::AgentKind {
        self.wire().0
    }

    /// Its name on a card when its kind is `other` (the island's bird for a tool it doesn't know).
    fn agent_name(self) -> Option<String> {
        self.wire().1.map(str::to_owned)
    }

    fn of(owner: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|e| e.owner() == owner)
            .unwrap_or(Self::Claude)
    }
}

/// A task's stored session: the agent it belongs to, then its id (`codex:<thread>`); one without
/// its agent is Claude Code's, as the first runs kept it.
fn stored(session: &str) -> (Engine, &str) {
    match session.split_once(':') {
        Some((owner, id)) if Engine::ALL.iter().any(|e| e.owner() == owner) => (Engine::of(owner), id),
        _ => (Engine::Claude, session),
    }
}

/// The agent a stored session belongs to (`claude`, `codex`, `qwen`…): what **Go on** starts.
pub fn session_owner(session: &str) -> &'static str {
    stored(session).0.owner()
}

/// What a run's Claude Code runs with, on `brief`: the user's own settings (our hooks run, so the
/// flock shows it and its cards are the island's; never the project's, a clone's path Claude Code
/// never trusted), its sandbox, a worker's role, Zeca's MCP.
fn spec(id: i64, dir: PathBuf, brief: String, engine: Engine) -> Spec {
    Spec {
        // Codex keeps a `.git` read-only even in its folder: its clone's own may be written (Claude
        // Code allows it by itself). Neither reaches the user's repository.
        add_dirs: if engine == Engine::Codex {
            vec![dir.join(".git")]
        } else {
            Vec::new()
        },
        settings: Settings::User,
        sandbox: Sandbox::Native,
        role: Role::Worker,
        system_append: Some(brief),
        mcp_config: crate::mcp::config_file(),
        env: vec![
            ("VULTS_TASK".to_string(), id.to_string()),
            (vults_protocol::RUN_ENV.to_string(), "1".to_string()),
        ],
        ..Spec::in_dir(dir)
    }
}

/// Hands the run's interrupter to Stop, while the run is still this generation's.
fn holds(id: i64, generation: u64, interrupter: Arc<dyn Interrupter>) {
    if let Ok(mut r) = runs().lock()
        && let Some(run) = r.get_mut(&id).filter(|run| run.generation == generation)
    {
        run.interrupter = Some(interrupter);
    }
}

/// One turn of a driver on `text`, into `sink`; its session id after.
async fn turn(
    id: i64,
    generation: u64,
    driver: Box<dyn vults_agents::driver::AgentDriver>,
    text: &str,
    sink: &RunSink,
    stopped: &std::sync::atomic::AtomicBool,
) -> (Result<(), String>, Option<String>) {
    let input = Input {
        text: text.into(),
        ..Input::default()
    };
    turn_with(id, generation, driver, &input, sink, stopped).await
}

/// One turn of a driver on `input` (a review's carries its instructions), into `sink`; its session
/// id after.
async fn turn_with(
    id: i64,
    generation: u64,
    mut driver: Box<dyn vults_agents::driver::AgentDriver>,
    input: &Input,
    sink: &RunSink,
    stopped: &std::sync::atomic::AtomicBool,
) -> (Result<(), String>, Option<String>) {
    holds(id, generation, driver.interrupter());
    // A Stop that came before Stop could reach this driver: it never starts.
    if stopped.load(std::sync::atomic::Ordering::SeqCst) {
        return (Ok(()), None);
    }
    let result = match driver.start().await {
        Ok(()) => {
            if let Ok(mut s) = sink.session.lock() {
                *s = driver.session().map(str::to_owned);
            }
            if let Ok(mut c) = sink.cost.lock() {
                c.session = driver.session().map(str::to_owned);
            }
            let sent = driver.send(input, sink).await;
            // A new session's id came during the turn.
            if let Ok(mut c) = sink.cost.lock() {
                c.session = driver.session().map(str::to_owned);
            }
            sent
        }
        Err(e) => Err(e),
    };
    ledger(sink).await;
    (result, driver.session().map(str::to_owned))
}

/// A session's running total, after its turn: its baseline in the ledger, and the turn's cost when
/// the driver could not tell it (counted from what the total grew by since the ledger last saw
/// the session).
async fn ledger(sink: &RunSink) {
    let (cost, app) = (sink.cost.clone(), sink.app.clone());
    let _ = tauri::async_runtime::spawn_blocking(move || settle(Some(&app), &cost)).await;
}

/// Settles a run's waiting running total. Blocking.
fn settle(app: Option<&AppHandle>, cost: &Mutex<Spent>) {
    let Ok(mut c) = cost.lock() else {
        return;
    };
    if let (Some(p), Some(session)) = (c.pending.take(), c.session.clone()) {
        c.usd += crate::spend::settle(app, &session, &p);
    }
}

/// Refused once today's budget is spent (W9): no new session starts from here.
async fn within_budget(app: &AppHandle) -> Result<(), String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || crate::spend::refuse(&app))
        .await
        .map_err(|e| e.to_string())?
}

/// The files the task's branch changed since it began, read in the repository after its branch came
/// back (never by running git's status in the clone, whose config could start a filter).
fn changed_files(task: &Task) -> Vec<String> {
    changed_files_at(task, &worktree_dir(task))
}

fn changed_files_at(task: &Task, dir: &Path) -> Vec<String> {
    let Some(repo) = crate::roosts::repo_of(Path::new(&task.folder)) else {
        return Vec::new();
    };
    let Some(base) = std::fs::read_to_string(base_file(dir))
        .ok()
        .map(|b| b.trim().to_string())
    else {
        return Vec::new();
    };
    let name = format!("refs/heads/{}", branch(task.id));
    let mut files: Vec<String> = git(&repo.root, &["diff", "--name-only", "--no-renames", &base, &name])
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .filter(|l| !l.is_empty())
        .collect();
    files.truncate(50);
    files
}

/// Starts the task's run: Claude Code or Codex in its clone. `fresh` hands off first (W5): the session it
/// had writes a context pack, Vults adds what it knows, and a new session starts from that pack
/// alone, as a compaction of all it did.
async fn launch(app: AppHandle, id: i64, fresh: bool, engine: Engine) -> Result<(), String> {
    within_budget(&app).await?;
    let generation = reserve(id)?;
    let ready = tauri::async_runtime::spawn_blocking(move || {
        let mut store = open()?;
        let task = store
            .task(id)
            .map_err(|e| e.to_string())?
            .ok_or("No such task.")?;
        if matches!(task.status.as_str(), "done" | "dropped") {
            return Err("This task is finished.".to_string());
        }
        if fresh
            && !task
                .session
                .as_deref()
                .is_some_and(|s| crate::chat::valid_session(stored(s).1))
        {
            return Err("This task has no session to hand off.".to_string());
        }
        let dir = workspace(&task)?;
        let now = crate::traces::now_ms();
        store
            .task_change(id, &Change::Status("doing".into()), now)
            .map_err(|e| e.to_string())?;
        Ok::<_, String>((task, dir))
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    let (task, dir) = match ready {
        Ok(ready) => ready,
        Err(e) => {
            release(id, generation);
            return Err(e);
        }
    };
    let (cost, stopped) = runs()
        .lock()
        .ok()
        .and_then(|r| r.get(&id).map(|run| (run.cost.clone(), run.stopped.clone())))
        .unwrap_or_default();
    let done_app = app.clone();
    let running = tauri::async_runtime::spawn(async move {
        // A session belongs to the agent that owned the task when it ran.
        // A session belongs to the agent it ran on, whoever the task's owner says now.
        let (before, kept) = match task.session.as_deref().map(stored) {
            Some((engine, id)) => (engine, Some(id.to_string())),
            None => (Engine::of(&task.owner), None),
        };
        let sink = RunSink {
            task: id,
            cost,
            reply: Mutex::new(String::new()),
            app: done_app.clone(),
            dir: dir.to_string_lossy().into_owned(),
            engine: Mutex::new(before),
            session: Mutex::new(None),
            generation,
            roost: task.roost.clone(),
        };
        let session = kept.filter(|s| crate::chat::valid_session(s));
        let mut pack: Option<String> = task.pack.clone();
        let mut handoff_failed = false;
        let (resume, text) = if fresh {
            // The session it had writes the pack, in the template; whatever it writes, Vults' own
            // facts (the goal, the ticks, the files on the branch) win.
            let old = Engine::driver(before, spec(id, dir.clone(), brief(&task), before), session);
            let (ok, written) = match old {
                Some(d) => {
                    let (r, _) = turn(id, generation, d, vults_zeca::pack::ASK, &sink, &stopped).await;
                    let written = std::mem::take(&mut *sink.reply.lock().unwrap_or_else(|e| e.into_inner()));
                    (r.is_ok(), written)
                }
                None => (false, String::new()),
            };
            let parsed = vults_zeca::pack::Pack::parse(&written);
            // No pack, a failed or stopped turn: the task keeps its session, nothing is handed off.
            if !ok || parsed.is_empty() || stopped.load(std::sync::atomic::Ordering::SeqCst) {
                handoff_failed = !stopped.load(std::sync::atomic::Ordering::SeqCst);
                (None, "")
            } else {
                let criteria: Vec<(String, bool)> =
                    task.criteria.iter().map(|c| (c.text.clone(), c.done)).collect();
                let files = {
                    let t = task.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        // What it committed comes back first: the files are read in the repository.
                        if let Err(e) = bring_back(&t) {
                            tracing::warn!(task = t.id, error = %e, "a task's branch didn't come back");
                        }
                        changed_files(&t)
                    })
                    .await
                    .unwrap_or_default()
                };
                let packed = parsed.with_facts(&task.goal, &criteria, &files).render();
                let kept = packed.clone();
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    open()?
                        .task_handoff(id, &kept, crate::traces::now_ms())
                        .map_err(|e| e.to_string())
                })
                .await;
                pack = Some(packed);
                (None, "Start on the task from the context pack.")
            }
        } else if session.is_some() && before == engine {
            (session, "Go on with the task.")
        } else {
            (None, "Start on the task.")
        };
        let mut text_brief = brief(&task);
        if resume.is_none()
            && let Some(p) = &pack
        {
            text_brief.push_str("\n\nWhat the session before you left, as a context pack (the goal and criteria above win where they differ):\n\n");
            text_brief.push_str(p);
        }
        let result = if handoff_failed {
            (
                Err("The session wrote no handoff notes: it keeps its session.".to_string()),
                None,
            )
        } else if stopped.load(std::sync::atomic::Ordering::SeqCst) {
            // Stopped during the handoff: no new session starts, and the old one stays.
            (Ok(()), None)
        } else {
            if let Ok(mut e) = sink.engine.lock() {
                *e = engine;
            }
            // The task is this agent's from the moment its session starts, not before: a handoff
            // that wrote no pack leaves it with the agent whose session it keeps.
            let owner = engine.owner();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                open()?
                    .task_change(id, &Change::Owner(owner.into()), crate::traces::now_ms())
                    .map_err(|e| e.to_string())
            })
            .await;
            match Engine::driver(engine, spec(id, dir, text_brief, engine), resume) {
                Some(d) => turn(id, generation, d, text, &sink, &stopped).await,
                None => (Err("That agent can't run here.".to_string()), None),
            }
        };
        let (result, session) = result;
        if let Err(e) = &result {
            tracing::warn!(task = id, error = %e, "task run failed");
        }
        // A run that ends with a card still waiting (stopped, or its turn gone): the app says the
        // session stopped, as its hook would, and the core withdraws the card.
        withdraw(&done_app, &sink, result.as_ref().err().map(String::as_str)).await;
        let reply = outcome_reply(
            sink.reply.lock().map(|r| r.clone()).unwrap_or_default(),
            result.as_ref().err().map(String::as_str),
        );
        let cost = sink.cost.clone();
        let failed = result.is_err();
        let back = task.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            charge(id, &cost);
            // Whatever it committed is in the repository now, on the task's branch.
            if let Err(e) = bring_back(&back) {
                tracing::warn!(task = id, error = %e, "a task's branch didn't come back");
            }
            let mut store = open()?;
            let now = crate::traces::now_ms();
            store
                .task_run_ended(
                    id,
                    &reply,
                    session.map(|s| format!("{}:{s}", engine.owner())).as_deref(),
                    now,
                )
                .map_err(|e| e.to_string())?;
            if failed {
                store
                    .task_change(id, &Change::Status("blocked".into()), now)
                    .map_err(|e| e.to_string())?;
            }
            Ok::<_, String>(())
        })
        .await;
        release(id, generation);
        let _ = done_app.emit(CHANGED, ());
    });
    if let Ok(mut r) = runs().lock()
        && let Some(run) = r.get_mut(&id).filter(|run| run.generation == generation)
    {
        run.turn = Some(running);
    }
    let _ = app.emit(CHANGED, ());
    Ok(())
}

/// Starts a task on `engine`, for the router (W8): as **Start** does.
pub(crate) async fn start_task(app: AppHandle, id: i64, engine: Engine) -> Result<(), String> {
    launch(app, id, false, engine).await
}

/// **Start**: the task's clone, and the chosen agent in it on its brief (going on with its last
/// session, when it had one). The task becomes Claude Code's and *Doing* before the turn runs.
#[tauri::command]
pub async fn task_start(
    app: AppHandle,
    window: tauri::Webview,
    id: i64,
    engine: Engine,
) -> Result<(), String> {
    from_user(&window)?;
    launch(app, id, false, engine).await
}

/// An agent that speaks ACP, installed here: what **Start with** offers besides Claude Code and
/// Codex (W10).
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct OtherAgent {
    pub name: &'static str,
    pub title: &'static str,
}

#[tauri::command]
pub async fn task_agents() -> Vec<OtherAgent> {
    tauri::async_runtime::spawn_blocking(|| {
        vults_chat::acp::agents()
            .iter()
            .filter(|a| a.installed())
            .map(|a| OtherAgent {
                name: a.name,
                title: a.title,
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// **Start fresh**: a handoff to a new session through a context pack (W5).
#[tauri::command]
pub async fn task_start_fresh(
    app: AppHandle,
    window: tauri::Webview,
    id: i64,
    engine: Engine,
) -> Result<(), String> {
    from_user(&window)?;
    launch(app, id, true, engine).await
}

/// The branch a review compares with, in the task's clone: the commit the task's branch began from.
fn base_branch(id: i64) -> String {
    format!("vults/base-{id}")
}

/// **Review** (W6): the other agent reviews the task's branch (Codex with its own reviewer,
/// `review/start`; Claude Code on the same words), up to [`vults_zeca::review::ROUNDS`] rounds; a
/// review that asks for changes sends its findings to the task's session, which fixes and commits,
/// and the next round reviews again. A pass ends it; no verdict, or the last round still asking,
/// leaves it to the user. Only a task whose session did the work; a human still marks it done.
async fn review(app: AppHandle, id: i64) -> Result<(), String> {
    use vults_zeca::review::{self, Verdict};
    within_budget(&app).await?;
    let generation = reserve_as(id, true)?;
    let ready = tauri::async_runtime::spawn_blocking(move || {
        let store = open()?;
        let task = store
            .task(id)
            .map_err(|e| e.to_string())?
            .ok_or("No such task.")?;
        let (worker, session) = match task.session.as_deref().map(stored) {
            Some((engine, s)) if crate::chat::valid_session(s) => (engine, s.to_string()),
            _ => return Err("This task has no work to review yet.".to_string()),
        };
        if matches!(task.status.as_str(), "done" | "dropped") {
            return Err("This task is finished.".to_string());
        }
        let dir = workspace(&task)?;
        let base = std::fs::read_to_string(base_file(&dir))
            .map_err(|_| "This task has no work to review yet.".to_string())?;
        // In the clone Vults just checked: a branch at the base, for the reviewer's diff.
        git(&dir, &["branch", "--force", &base_branch(id), base.trim()])?;
        Ok::<_, String>((task, dir, worker, session))
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    let (task, dir, worker, mut session) = match ready {
        Ok(r) => r,
        Err(e) => {
            release(id, generation);
            return Err(e);
        }
    };
    // Codex's work goes to Claude Code; anyone else's (Claude Code's, an ACP agent's) to Codex's
    // own reviewer, as the Tasks tab says.
    let reviewer = if worker == Engine::Codex {
        Engine::Claude
    } else {
        Engine::Codex
    };
    let (cost, stopped) = runs()
        .lock()
        .ok()
        .and_then(|r| r.get(&id).map(|run| (run.cost.clone(), run.stopped.clone())))
        .unwrap_or_default();
    let done_app = app.clone();
    let running = tauri::async_runtime::spawn(async move {
        let sink = RunSink {
            task: id,
            cost,
            reply: Mutex::new(String::new()),
            app: done_app.clone(),
            dir: dir.to_string_lossy().into_owned(),
            engine: Mutex::new(reviewer),
            session: Mutex::new(None),
            generation,
            roost: task.roost.clone(),
        };
        let criteria: Vec<String> = task.criteria.iter().map(|c| c.text.clone()).collect();
        let ask = review::ask(&task.goal, &criteria, &base_branch(id));
        let mut worker_said = String::new();
        let mut failed = None;
        for round in 1..=review::ROUNDS {
            if stopped.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            // Each round starts a reviewer: none once today's budget is spent (the first was
            // checked on the click).
            if round > 1 && within_budget(&done_app).await.is_err() {
                tracing::info!(task = id, round, "review ended: today's budget is spent");
                break;
            }
            // The reviewer: a fresh session that wrote none of it.
            if let Ok(mut e) = sink.engine.lock() {
                *e = reviewer;
            }
            let role =
                "You review a task's work in this folder. Change no file and commit nothing.".to_string();
            // It writes nothing: Codex's thread is read-only (its review mode may run commands
            // unasked), Claude Code's every write a card; and no git folder is opened to it.
            let mut look = spec(id, dir.clone(), role, reviewer);
            look.sandbox = Sandbox::ReadOnly;
            look.add_dirs.clear();
            let Some(driver) = Engine::driver(reviewer, look, None) else {
                // The work is not at fault: nothing blocks the task.
                tracing::warn!(task = id, "the reviewer can't run here");
                break;
            };
            let tip = {
                let d = dir.clone();
                tauri::async_runtime::spawn_blocking(move || git(&d, &["rev-parse", "HEAD"]).ok())
                    .await
                    .ok()
                    .flatten()
            };
            let input = Input {
                text: ask.clone(),
                review: (reviewer == Engine::Codex).then(|| ask.clone()),
                ..Input::default()
            };
            let (result, _) = turn_with(id, generation, driver, &input, &sink, &stopped).await;
            withdraw(&done_app, &sink, result.as_ref().err().map(String::as_str)).await;
            let said = std::mem::take(&mut *sink.reply.lock().unwrap_or_else(|e| e.into_inner()));
            // Stopped: the round is no review, and the last real one stays on the task.
            if stopped.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            if let Err(e) = result {
                tracing::warn!(task = id, error = %e, "the review didn't run");
                break;
            }
            let moved = {
                let d = dir.clone();
                tauri::async_runtime::spawn_blocking(move || git(&d, &["rev-parse", "HEAD"]).ok())
                    .await
                    .ok()
                    .flatten()
            } != tip;
            let verdict = if moved {
                // A reviewer that changed the branch reviewed its own work: a human reads it.
                None
            } else if reviewer == Engine::Codex {
                review::codex_verdict(&said)
            } else {
                review::verdict(&said)
            };
            let name = match verdict {
                Some(Verdict::Pass) => "pass",
                Some(Verdict::Changes) => "changes",
                None => "none",
            };
            let (kept, r) = (said.clone(), round as i64);
            let _ = tauri::async_runtime::spawn_blocking(move || {
                open()?
                    .task_reviewed(id, &kept, name, r, crate::traces::now_ms())
                    .map_err(|e| e.to_string())
            })
            .await;
            let _ = done_app.emit(CHANGED, ());
            if verdict != Some(Verdict::Changes)
                || round == review::ROUNDS
                || stopped.load(std::sync::atomic::Ordering::SeqCst)
            {
                break;
            }
            if within_budget(&done_app).await.is_err() {
                tracing::info!(task = id, round, "no fix round: today's budget is spent");
                break;
            }
            // The worker fixes what the review found, in its own session.
            if let Ok(mut e) = sink.engine.lock() {
                *e = worker;
            }
            let Some(driver) = Engine::driver(
                worker,
                spec(id, dir.clone(), brief(&task), worker),
                Some(session.clone()),
            ) else {
                failed = Some("That agent can't run here.".to_string());
                break;
            };
            let input = Input {
                text: review::fix(&review::findings(&said), round),
                ..Input::default()
            };
            let (result, next) = turn_with(id, generation, driver, &input, &sink, &stopped).await;
            withdraw(&done_app, &sink, result.as_ref().err().map(String::as_str)).await;
            worker_said = std::mem::take(&mut *sink.reply.lock().unwrap_or_else(|e| e.into_inner()));
            if let Some(next) = next {
                session = next;
            }
            if let Err(e) = result {
                failed = Some(e);
                break;
            }
            let back = task.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || bring_back(&back)).await;
        }
        if let Some(e) = &failed {
            tracing::warn!(task = id, error = %e, "the fix after a review failed");
        }
        let cost = sink.cost.clone();
        let back = task.clone();
        let stored_session = format!("{}:{session}", worker.owner());
        let worker_said = outcome_reply(worker_said, failed.as_deref());
        let _ = tauri::async_runtime::spawn_blocking(move || {
            charge(id, &cost);
            if let Err(e) = bring_back(&back) {
                tracing::warn!(task = id, error = %e, "a task's branch didn't come back");
            }
            let mut store = open()?;
            let now = crate::traces::now_ms();
            store
                .task_run_ended(id, &worker_said, Some(&stored_session), now)
                .map_err(|e| e.to_string())?;
            if failed.is_some() {
                store
                    .task_change(id, &Change::Status("blocked".into()), now)
                    .map_err(|e| e.to_string())?;
            }
            Ok::<_, String>(())
        })
        .await;
        release(id, generation);
        let _ = done_app.emit(CHANGED, ());
    });
    if let Ok(mut r) = runs().lock()
        && let Some(run) = r.get_mut(&id).filter(|run| run.generation == generation)
    {
        run.turn = Some(running);
    }
    let _ = app.emit(CHANGED, ());
    Ok(())
}

#[tauri::command]
pub async fn task_review(app: AppHandle, window: tauri::Webview, id: i64) -> Result<(), String> {
    from_user(&window)?;
    review(app, id).await
}

/// The slot in [`runs`] a continuation holds: no task has id 0, and one at a time (W7).
pub const CONTINUING: i64 = 0;

/// What `/continue` says back: an English sentence the chat translates, and its detail.
pub type Said = (&'static str, Option<String>);

/// Whether a process is alive, by its id (Linux: its `/proc` entry). Elsewhere it can't be told.
fn alive(pid: u32) -> Option<bool> {
    if cfg!(target_os = "linux") {
        Some(Path::new(&format!("/proc/{pid}")).exists())
    } else {
        None
    }
}

/// `/continue <words>` (W7): the session in `cwd` that stopped last, at most ten minutes ago, takes
/// `words` as its next turn, resumed here (its own settings, sandboxed, its cards the island's, in
/// the flock as itself). Never a session whose process still runs: its terminal answers it. One at
/// a time; `/continue stop` ends it.
pub async fn continue_session(app: AppHandle, cwd: String, words: String) -> Said {
    let Some(c) = crate::runtime::continuable(&app, cwd.clone())
        .await
        .into_iter()
        .next()
    else {
        return ("No session in this folder stopped in the last ten minutes.", None);
    };
    // Its process must be gone, ended or not: Claude Code ends a session on `/clear` and goes on
    // in the same process. Only an ended session with no process id known may go without it.
    match (c.pid.map(alive), c.ended) {
        (Some(Some(false)), _) | (None, true) => {}
        (Some(Some(true)), _) => return ("That session is still open in its terminal: tell it there.", None),
        _ => {
            return (
                "Vults can't tell whether that session is still open: tell it in its terminal.",
                None,
            );
        }
    }
    // A task's session (a clone of its own) goes on from its task, with its brief and its cost.
    if Path::new(&c.cwd).starts_with(crate::paths::data_dir().join("worktrees")) {
        return ("That session can't be continued from here.", None);
    }
    let engine = match c.key.agent {
        vults_protocol::AgentKind::Codex => Engine::Codex,
        _ => Engine::Claude,
    };
    if !crate::chat::valid_session(&c.key.session_id) {
        return ("That session can't be continued from here.", None);
    }
    if within_budget(&app).await.is_err() {
        return (
            "Today's budget is spent: Vults starts no new session until tomorrow.",
            None,
        );
    }
    let Ok(generation) = reserve_as(CONTINUING, true) else {
        return (
            "Another session is being continued: wait for it, or /continue stop.",
            None,
        );
    };
    let dir = PathBuf::from(&c.cwd);
    let cwd = c.cwd.clone();
    let settings = {
        let (d, state) = (
            dir.clone(),
            vults_agents::claude_state_file(&crate::paths::agent_dirs()),
        );
        tauri::async_runtime::spawn_blocking(move || crate::chat::cli_settings(true, &d, &state))
            .await
            .unwrap_or(Settings::User)
    };
    // As the user ran it, with its own settings and sandboxed; nothing of Zeca's.
    let spec = Spec {
        settings,
        sandbox: Sandbox::Native,
        role: Role::Worker,
        env: vec![
            ("VULTS_CONTINUE".into(), "1".into()),
            (vults_protocol::RUN_ENV.into(), "1".into()),
        ],
        ..Spec::in_dir(dir.clone())
    };
    let Some(driver) = Engine::driver(engine, spec, Some(c.key.session_id.clone())) else {
        release(CONTINUING, generation);
        return ("That session can't be continued from here.", None);
    };
    let (cost, stopped) = runs()
        .lock()
        .ok()
        .and_then(|r| {
            r.get(&CONTINUING)
                .map(|run| (run.cost.clone(), run.stopped.clone()))
        })
        .unwrap_or_default();
    let done_app = app.clone();
    let roost = {
        let dir = dir.clone();
        tauri::async_runtime::spawn_blocking(move || crate::roosts::repo_of(&dir).map(|r| r.key()))
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
    };
    let project = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    // A resume that fails at once (a transcript it can't find) is said in the chat, not only logged.
    let (first, failed_fast) = tokio::sync::oneshot::channel::<String>();
    let running = tauri::async_runtime::spawn(async move {
        let sink = RunSink {
            task: CONTINUING,
            cost,
            reply: Mutex::new(String::new()),
            app: done_app.clone(),
            dir: cwd,
            engine: Mutex::new(engine),
            session: Mutex::new(None),
            generation,
            roost,
        };
        let (result, _) = turn(CONTINUING, generation, driver, &words, &sink, &stopped).await;
        withdraw(&done_app, &sink, result.as_ref().err().map(String::as_str)).await;
        if let Err(e) = result {
            tracing::warn!(error = %e, "a continued session's turn failed");
            let _ = first.send(e);
        }
        release(CONTINUING, generation);
    });
    if let Ok(mut r) = runs().lock()
        && let Some(run) = r.get_mut(&CONTINUING).filter(|run| run.generation == generation)
    {
        run.turn = Some(running);
    }
    if let Ok(Ok(e)) = tokio::time::timeout(Duration::from_secs(4), failed_fast).await {
        return ("That session couldn't be resumed:", Some(e));
    }
    ("Continuing this session; it shows in the flock:", Some(project))
}

/// `/continue stop`: ends the continuation going on; false when there is none.
pub async fn stop_continuation(app: AppHandle) -> Said {
    if !running(CONTINUING) {
        return ("No session is being continued.", None);
    }
    match stop(app, CONTINUING).await {
        Ok(()) => ("Stopped.", None),
        Err(_) => ("It is still starting: try again in a moment.", None),
    }
}

/// **Stop**: asks the run's turn to stop, waits for it, and ends its process when it doesn't. Only
/// a run Vults started; the task goes back to *To do*.
#[tauri::command]
pub async fn task_stop(app: AppHandle, window: tauri::Webview, id: i64) -> Result<(), String> {
    from_user(&window)?;
    stop(app, id).await
}

async fn stop(app: AppHandle, id: i64) -> Result<(), String> {
    let (generation, interrupter, turn, cost, was_review) = {
        let mut r = runs().lock().map_err(|e| e.to_string())?;
        let run = r.get_mut(&id).ok_or("This task isn't running.")?;
        let Some(turn) = run.turn.take() else {
            return Err("This task is still starting.".into());
        };
        run.stopped.store(true, std::sync::atomic::Ordering::SeqCst);
        (
            run.generation,
            run.interrupter.clone(),
            turn,
            run.cost.clone(),
            run.review,
        )
    };
    if let Some(i) = interrupter {
        let _ = i.interrupt().await;
    }
    let mut turn = turn;
    // A turn that won't end in time is dropped, which drops the driver and kills its process.
    if tokio::time::timeout(STOP_WAIT, &mut turn).await.is_err() {
        turn.abort();
        // Its own bookkeeping never ran: what it cost still counts, and its place goes.
        let _ = tauri::async_runtime::spawn_blocking(move || charge(id, &cost)).await;
        release(id, generation);
    }
    // Back to *To do*, unless a new run started meanwhile; a stopped review leaves the status be.
    if !running(id) && !was_review {
        tauri::async_runtime::spawn_blocking(move || {
            open()?
                .task_change(id, &Change::Status("open".into()), crate::traces::now_ms())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
    }
    let _ = app.emit(CHANGED, ());
    Ok(())
}

/// **Remove its folder**: the task's clone goes, once its branch came back into the repository (the
/// branch and its commits stay there). Uncommitted work in the folder goes with it.
#[tauri::command]
pub async fn task_remove_worktree(app: AppHandle, window: tauri::Webview, id: i64) -> Result<(), String> {
    from_user(&window)?;
    if running(id) {
        return Err("Stop the task first.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let task = open()?
            .task(id)
            .map_err(|e| e.to_string())?
            .ok_or("No such task.")?;
        let dir = worktree_dir(&task);
        if !dir.exists() {
            return Ok::<(), String>(());
        }
        // Its branch comes back first; what was never committed goes with the folder (the Nest
        // asks twice). An earlier Vults' worktree is unregistered from its repository too.
        let legacy = std::fs::symlink_metadata(dir.join(".git")).is_ok_and(|m| m.is_file());
        // A clone Vults trusts brings its branch back first; one it doesn't is removed with
        // nothing taken from it.
        let trusted =
            crate::roosts::repo_of(Path::new(&task.folder)).is_some_and(|r| untouched(&dir, &r.root).is_ok());
        if !legacy && trusted {
            bring_back(&task)?;
        }
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(base_file(&dir));
        let _ = std::fs::remove_file(config_file(&dir));
        // An earlier Vults' worktree is unregistered from its repository; nothing else is pruned.
        if legacy && let Some(repo) = crate::roosts::repo_of(Path::new(&task.folder)) {
            let _ = git(&repo.root, &["worktree", "prune"]);
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit(CHANGED, ());
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn failed_owned_runs_keep_their_cause_and_release_all_waiting_cards() {
        use vults_core::{AgentEvent, Effect, Input as CoreInput, RequestId, State, Status};
        let now = std::time::Instant::now();
        for engine in Engine::ALL {
            let update = ended(engine, "session", Some("The CLI failed."));
            let key = update.session.clone();
            let mut state = State::default();
            for (request, agent) in [("main", None), ("child", Some("subagent"))] {
                let mut pending = update.clone();
                pending.agent_id = agent.map(str::to_owned);
                pending.event = AgentEvent::PermissionRequested {
                    request: RequestId(request.into()),
                    tool: "Bash".into(),
                    target: "Bash · cargo test".into(),
                    ask: Default::default(),
                };
                vults_core::reduce(&mut state, CoreInput::Agent(pending), now);
            }
            let effects = vults_core::reduce(&mut state, CoreInput::RunEnded(update), now);
            assert!(state.pending.is_empty(), "{engine:?}");
            assert_eq!(state.sessions[&key].status, Status::Failed);
            assert_eq!(state.sessions[&key].note.as_deref(), Some("The CLI failed."));
            assert_eq!(
                effects
                    .iter()
                    .filter(|e| matches!(e, Effect::ReleasePermission(_)))
                    .count(),
                2
            );
            assert!(!effects.iter().any(|e| matches!(
                e,
                Effect::RespondPermission { .. } | Effect::AnswerQuestion { .. }
            )));
        }
        assert_eq!(
            outcome_reply("Partial work".into(), Some("The CLI failed.")),
            "Partial work\n\nThe CLI failed."
        );
        assert_eq!(
            outcome_reply("".into(), Some("The CLI failed.")),
            "The CLI failed."
        );
        assert_eq!(outcome_reply("Done".into(), None), "Done");
    }

    fn task(id: i64, roost: &str) -> Task {
        Task {
            id,
            created_ms: 0,
            updated_ms: 0,
            goal: "Fix the login test".into(),
            criteria: vec![vults_store::tasks::Criterion {
                text: "passes 20 runs".into(),
                done: false,
            }],
            roost: String::new(),
            roost_name: roost.into(),
            folder: String::new(),
            owner: "you".into(),
            status: "open".into(),
            cost_usd: None,
            issue: None,
            reply: None,
            session: None,
            pack: None,
            review: None,
            review_verdict: None,
            review_round: None,
            route: None,
            tokens: None,
        }
    }

    // Unix paths: on Windows they are not absolute.
    #[cfg(unix)]
    #[test]
    fn an_acp_agents_card_names_the_call_and_stays_in_the_clone() {
        let dir = "/data/worktrees/site-task-3";
        let ask = |tool: &str, input: serde_json::Value| Ask {
            id: "call_a".into(),
            tool: tool.into(),
            target: String::new(),
            full: None,
            input,
        };
        let card = |a: &Ask| acp_card(Engine::Opencode, 3, 7, a, "ses_1", dir);
        let e = card(&ask(
            "Bash",
            serde_json::json!({ "command": "cargo test", "cwd": dir }),
        ))
        .unwrap();
        assert_eq!(
            (e.agent, e.id.as_str()),
            (vults_protocol::AgentKind::OpenCode, "task-3-7-call_a")
        );
        assert!(e.wants_reply);
        assert_eq!(e.payload["tool_input"]["command"], "cargo test");
        // A command run elsewhere, or a file out of the clone or in its .git: no card, a no.
        assert!(
            card(&ask(
                "Bash",
                serde_json::json!({ "command": "ls", "cwd": "/home/me" })
            ))
            .is_none()
        );
        let edit = |path: &str, old: &str| {
            ask(
                "Edit",
                serde_json::json!({ "paths": [path], "diffs": [{ "path": path, "oldText": old, "newText": "hi" }], "delete": false }),
            )
        };
        let new_file = card(&edit("/data/worktrees/site-task-3/a.txt", "")).unwrap();
        assert_eq!(new_file.payload["tool_name"], "Write");
        let changed = card(&edit("/data/worktrees/site-task-3/a.txt", "ho")).unwrap();
        assert_eq!(changed.payload["tool_input"]["old_string"], "ho");
        assert!(card(&edit("/home/me/site/a.txt", "")).is_none());
        assert!(card(&edit("/data/worktrees/site-task-3/.git/config", "x")).is_none());
        // Two files at once, or a delete, can't be one card.
        let two = ask(
            "Edit",
            serde_json::json!({ "paths": ["/data/worktrees/site-task-3/a", "/data/worktrees/site-task-3/b"], "diffs": [], "delete": false }),
        );
        assert!(card(&two).is_none());
        let gone = ask(
            "Edit",
            serde_json::json!({ "paths": ["/data/worktrees/site-task-3/a"], "diffs": [], "delete": true }),
        );
        assert!(card(&gone).is_none());
        // Copilot CLI's card goes by its own kind, as its hooks' do.
        let e = acp_card(
            Engine::Copilot,
            3,
            7,
            &ask("Bash", serde_json::json!({ "command": "ls" })),
            "s",
            dir,
        )
        .unwrap();
        assert_eq!(
            (e.agent, e.agent_name.as_deref()),
            (vults_protocol::AgentKind::Copilot, None)
        );
        // Kimi Code's goes as another tool's, by its name, as its hooks' do.
        let e = acp_card(
            Engine::Kimi,
            3,
            7,
            &ask("Bash", serde_json::json!({ "command": "ls" })),
            "s",
            dir,
        )
        .unwrap();
        assert_eq!(
            (e.agent, e.agent_name.as_deref()),
            (vults_protocol::AgentKind::Other, Some("kimi"))
        );
        assert_eq!(
            ended(Engine::Codex, "s", None).event,
            vults_core::AgentEvent::Stopped { message: None }
        );
    }

    #[test]
    fn the_engines_are_the_registrys_runnable_agents() {
        use vults_agents::registry::{ALL, Run};
        let engines: Vec<&str> = Engine::ALL.iter().map(|e| e.owner()).collect();
        let runnable: Vec<&str> = ALL.iter().filter(|e| e.run != Run::No).map(|e| e.id).collect();
        let (mut a, mut b) = (engines.clone(), runnable);
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        // A native engine is one of the chat's own drivers; every other one speaks ACP.
        for e in Engine::ALL {
            let native = vults_agents::registry::of(e.owner()).is_some_and(|r| r.run == Run::Native);
            assert_eq!(
                native,
                matches!(e, Engine::Claude | Engine::Codex),
                "{}",
                e.owner()
            );
        }
    }

    #[test]
    fn a_stored_session_knows_every_agent() {
        assert_eq!(stored("qwen:abc"), (Engine::Qwen, "abc"));
        assert_eq!(stored("opencode:ses_1"), (Engine::Opencode, "ses_1"));
        assert_eq!(Engine::of("copilot"), Engine::Copilot);
        assert_eq!(stored("kimi:session_1"), (Engine::Kimi, "session_1"));
        assert_eq!(Engine::of("you"), Engine::Claude);
        // Go on starts the agent the session ran on.
        assert_eq!(session_owner("qwen:abc"), "qwen");
        // A Gemini CLI session, from before its support ended, is no session to go on: the task
        // starts afresh on Claude Code.
        assert_eq!(stored("gemini:abc"), (Engine::Claude, "gemini:abc"));
        assert!(!crate::chat::valid_session(stored("gemini:abc").1));
        assert_eq!(session_owner("abc-123"), "claude");
    }

    #[test]
    fn each_task_has_its_own_branch_and_worktree_named_after_its_roost() {
        assert_eq!(branch(12), "vults/task-12");
        let a = worktree_dir(&task(12, "site"));
        let b = worktree_dir(&task(13, "site"));
        assert_ne!(a, b, "two tasks never share a tree");
        assert!(a.ends_with("worktrees/site-task-12"));
        assert!(
            worktree_dir(&task(4, "../../etc")).ends_with("worktrees/------etc-task-4"),
            "no way out of the folder"
        );
        assert!(worktree_dir(&task(5, "")).ends_with("worktrees/task-task-5"));
    }

    #[test]
    fn a_task_runs_once_and_a_late_end_never_removes_a_newer_run() {
        let id = 9_000_001;
        let first = reserve(id).unwrap();
        assert!(reserve(id).is_err(), "a second Start while one runs is refused");
        assert!(running(id));
        release(id, first);
        assert!(!running(id));
        let second = reserve(id).unwrap();
        release(id, first);
        assert!(running(id), "the first run, ending late, leaves the second alone");
        release(id, second);
        assert!(!running(id));
    }

    #[test]
    fn the_brief_holds_the_goal_the_criteria_and_the_limits() {
        let b = brief(&task(12, "site"));
        assert!(b.contains("Goal: Fix the login test"));
        assert!(b.contains("- passes 20 runs"));
        assert!(b.contains("vults/task-12"));
        assert!(b.contains("Never push"));
    }

    /// A repository with one commit, under a temporary folder, and the task in it.
    #[cfg(unix)]
    fn repo(name: &str, id: i64) -> (PathBuf, Task) {
        let root = std::env::temp_dir().join(format!("vults-runs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("site");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]).unwrap();
        std::fs::write(repo.join("a.txt"), "a").unwrap();
        git(&repo, &["add", "."]).unwrap();
        git(
            &repo,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "first",
            ],
        )
        .unwrap();
        let mut t = task(id, "site");
        t.folder = repo.to_string_lossy().into_owned();
        (root, t)
    }

    #[cfg(unix)]
    #[test]
    fn two_tasks_get_two_clones_and_only_their_branch_comes_back() {
        let (root, a) = repo("clones", 1);
        let mut b = a.clone();
        b.id = 2;
        let repo_dir = PathBuf::from(&a.folder);
        let (da, db) = (root.join("site-task-1"), root.join("site-task-2"));
        workspace_at(&a, &da).unwrap();
        workspace_at(&b, &db).unwrap();
        assert_eq!(git(&da, &["branch", "--show-current"]).unwrap(), "vults/task-1");
        assert_eq!(git(&db, &["branch", "--show-current"]).unwrap(), "vults/task-2");
        // The clone reads the repository's objects, never holds the only copy of them.
        assert!(da.join(".git/objects/info/alternates").exists(), "a shared clone");
        let commit = [
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "work",
        ];
        std::fs::write(da.join("new.rs"), "fn main() {}").unwrap();
        git(&da, &["add", "."]).unwrap();
        git(&da, &commit).unwrap();
        // Whatever else the clone holds (its main moved, another branch) stays there.
        git(&da, &["branch", "-f", "main", "HEAD"]).unwrap();
        git(&da, &["branch", "elsewhere"]).unwrap();
        bring_back_at(&a, &da).unwrap();
        let repo_main = git(&repo_dir, &["rev-parse", "main"]).unwrap();
        assert_eq!(
            git(&repo_dir, &["rev-parse", "vults/task-1"]).unwrap(),
            git(&da, &["rev-parse", "HEAD"]).unwrap()
        );
        assert_ne!(
            repo_main,
            git(&da, &["rev-parse", "main"]).unwrap(),
            "the repository's main never moves"
        );
        assert!(
            git(
                &repo_dir,
                &["rev-parse", "--verify", "--quiet", "refs/heads/elsewhere"]
            )
            .is_err()
        );
        assert_eq!(changed_files_at(&a, &da), ["new.rs"]);
        // A folder in the way is never reused or written over.
        let c = Task { id: 3, ..a.clone() };
        let dc = root.join("site-task-3");
        std::fs::create_dir_all(&dc).unwrap();
        assert!(workspace_at(&c, &dc).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn nothing_in_a_clone_runs_when_vults_reads_it() {
        let (root, a) = repo("hostile", 4);
        let da = root.join("site-task-4");
        workspace_at(&a, &da).unwrap();
        // The worker writes its clone's config and hooks to run a program.
        let mark = root.join("ran");
        let script = root.join("evil.sh");
        std::fs::write(&script, format!("#!/bin/sh\ntouch {}\n", mark.display())).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let hooks = da.join(".git/hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        for hook in ["post-checkout", "pre-push", "reference-transaction", "post-merge"] {
            std::fs::copy(&script, hooks.join(hook)).unwrap();
        }
        git(&da, &["config", "core.fsmonitor", &script.to_string_lossy()]).unwrap();
        git(
            &da,
            &["config", "uploadpack.packObjectsHook", &script.to_string_lossy()],
        )
        .unwrap();
        // Vults won't fetch from it at all; and reading it runs nothing it names.
        assert!(
            bring_back_at(&a, &da).is_err(),
            "a clone with hooks and code in its config is never fetched"
        );
        let _ = changed_files_at(&a, &da);
        assert!(!mark.exists(), "Vults' own git ran something the clone named");
        let _ = std::fs::remove_dir_all(&root);
    }

    fn ask(tool: &str, target: &str, full: Option<&str>, input: serde_json::Value) -> Ask {
        Ask {
            id: "7".into(),
            tool: tool.into(),
            target: target.into(),
            full: full.map(str::to_owned),
            input,
        }
    }

    #[test]
    fn a_codex_approval_becomes_the_card_its_hook_would_send() {
        let cmd = ask(
            "Bash",
            "Bash · cargo test",
            None,
            serde_json::json!({ "command": "cargo test" }),
        );
        let event = codex_card(12, 3, &cmd, "thread-1", "/w/site-task-12").unwrap();
        assert_eq!(
            event.id, "task-12-3-7",
            "a run's own id: a dead card of an earlier run never collides"
        );
        let update = vults_agents::parse(&event).unwrap();
        assert_eq!(update.session.session_id, "thread-1");
        assert_eq!(update.cwd.as_deref(), Some("/w/site-task-12"));
        match update.event {
            vults_core::AgentEvent::PermissionRequested { tool, target, .. } => {
                assert_eq!((tool.as_str(), target.as_str()), ("Bash", "Bash · cargo test"));
            }
            other => panic!("{other:?}"),
        }
        assert!(
            event.wants_reply && !event.zeca,
            "a card a human answers, never the chat's"
        );
        // A long command is bound whole; a folder Codex names is the card's.
        let long = ask(
            "Bash",
            "Bash · cargo test && rm…",
            Some("cargo test && rm -rf build"),
            serde_json::json!({ "command": "cargo test && rm -rf build", "cwd": "/elsewhere" }),
        );
        let e = codex_card(12, 3, &long, "t", "/w").unwrap();
        assert_eq!(e.payload["tool_input"]["command"], "cargo test && rm -rf build");
        assert_eq!(e.payload["cwd"], "/elsewhere");
        // An edit is the patch of its changes: its files and line counts, as a hook's card.
        let edit = ask(
            "Edit",
            "Edit · /w/a.rs",
            None,
            serde_json::json!({ "changes": [{ "path": "/w/a.rs", "kind": "update", "diff": "@@ -1 +1,2 @@\n-old\n+new\n+more\n" }] }),
        );
        let update = vults_agents::parse(&codex_card(12, 3, &edit, "t", "/w").unwrap()).unwrap();
        match update.event {
            vults_core::AgentEvent::PermissionRequested { target, ask, .. } => {
                assert_eq!(target, "Edit · /w/a.rs");
                assert_eq!((ask.added, ask.removed), (2, 1));
            }
            other => panic!("{other:?}"),
        }
        // What can't be named whole is a no, never a card: an edit with no files, an empty command.
        assert!(
            codex_card(
                12,
                3,
                &ask("Edit", "Edit · add notes", None, serde_json::Value::Null),
                "t",
                "/w"
            )
            .is_none()
        );
        assert!(
            codex_card(
                12,
                3,
                &ask("Bash", "Bash", None, serde_json::json!({ "command": " " })),
                "t",
                "/w"
            )
            .is_none()
        );
        assert!(
            codex_card(
                12,
                3,
                &ask("Other", "x", None, serde_json::Value::Null),
                "t",
                "/w"
            )
            .is_none()
        );
    }

    #[test]
    fn a_stored_session_knows_its_agent() {
        assert_eq!(stored("codex:th-1"), (Engine::Codex, "th-1"));
        assert_eq!(stored("claude:abc"), (Engine::Claude, "abc"));
        assert_eq!(
            stored("abc-123"),
            (Engine::Claude, "abc-123"),
            "the first runs kept Claude Code's bare"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_tampered_clone_is_never_reused_or_fetched() {
        let (root, a) = repo("tampered", 5);
        let da = root.join("site-task-5");
        workspace_at(&a, &da).unwrap();
        bring_back_at(&a, &da).unwrap();
        workspace_at(&a, &da).unwrap();
        let config = da.join(".git/config");
        let original = std::fs::read(&config).unwrap();
        let alternates = da.join(".git/objects/info/alternates");
        let saved = std::fs::read(&alternates).unwrap();
        let cases: [(&str, &dyn Fn()); 4] = [
            ("config", &|| {
                git(&da, &["config", "core.fsmonitor", "/bin/true"]).unwrap();
            }),
            ("hook", &|| {
                std::fs::write(da.join(".git/hooks/post-checkout"), "x").unwrap()
            }),
            ("alternates", &|| {
                std::fs::write(&alternates, "/elsewhere/.git/objects\n").unwrap()
            }),
            ("commondir", &|| {
                std::fs::write(da.join(".git/commondir"), "../site/.git").unwrap()
            }),
        ];
        for (what, change) in cases {
            change();
            assert!(workspace_at(&a, &da).is_err(), "{what}: reused");
            assert!(bring_back_at(&a, &da).is_err(), "{what}: fetched");
            std::fs::write(&config, &original).unwrap();
            let _ = std::fs::remove_file(da.join(".git/hooks/post-checkout"));
            let _ = std::fs::remove_file(da.join(".git/commondir"));
            std::fs::write(&alternates, &saved).unwrap();
        }
        workspace_at(&a, &da).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_edits_patch_names_only_what_it_changes() {
        let p = patch(&serde_json::json!([{ "path": "/w/new.md", "kind": "add", "diff": "one\ntwo\n" }]))
            .unwrap();
        assert!(
            p.contains("*** Add File: /w/new.md\n+one\n+two\n"),
            "an added file's lines are added lines"
        );
        let moved = serde_json::json!([{ "path": "/w/a.rs", "kind": { "type": "update", "move_path": "/home/u/.bashrc" }, "diff": "" }]);
        assert!(patch(&moved).is_none(), "a move is no card");
        let injected =
            serde_json::json!([{ "path": "/w/a", "kind": "add", "diff": "*** Update File: /x\n" }]);
        assert!(patch(&injected).is_none(), "an injected header is no card");
        assert!(patch(&serde_json::json!([])).is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_live_process_is_told_from_a_gone_one() {
        // This test's own process: alive, so a session it ran would be refused.
        assert_eq!(alive(std::process::id()), Some(true));
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        assert_eq!(alive(pid), Some(false), "gone once it exited and was reaped");
    }
    #[test]
    fn claudes_control_channel_makes_one_card_on_its_actual_session() {
        let command = ask("Bash", "Bash", None, serde_json::json!({"command": "cargo test"}));
        let event = claude_card(12, 3, &command, "claude-session", "/w/task-12").unwrap();
        let update = vults_agents::parse(&event).unwrap();
        assert_eq!(event.id, "task-12-3-7");
        assert_eq!(update.session.session_id, "claude-session");
        assert_eq!(update.cwd.as_deref(), Some("/w/task-12"));
        assert!(
            matches!(update.event, vults_core::AgentEvent::PermissionRequested { target, .. } if target == "Bash · cargo test")
        );
        let look = spec(12, PathBuf::from("/w/task-12"), "work".into(), Engine::Claude);
        assert!(look.env.contains(&(vults_protocol::RUN_ENV.into(), "1".into())));
        assert!(claude_card(12, 3, &ask("Bash", "", None, serde_json::Value::Null), "s", "/w").is_none());
        // AskUserQuestion keeps its answer-bearing PreToolUse hook, never an Allow/Deny card.
        assert!(
            claude_card(
                12,
                3,
                &ask(vults_protocol::QUESTION_TOOL, "", None, serde_json::json!({})),
                "s",
                "/w"
            )
            .is_none()
        );
    }
}
