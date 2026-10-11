//! Sessions Vults starts for a task (`docs/dev/plan-zeca.md` W4): on the user's click, a git
//! worktree of the task's repository on its own branch, and Claude Code in it through the driver
//! (S8, S11), sandboxed (bubblewrap, a card before each command), with the user's own settings, so
//! our hooks run and the session shows in the flock like any other: its permissions are island
//! cards, answered by a human, a rule or a policy, never here. Only these sessions can be stopped
//! from Vults (D8): ask the turn to stop, wait, then end the process. A watched session never is.
//!
//! Codex waits: its app-server asks for approvals on its own channel, which no card shows yet.

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
    /// Set by Stop: a run between turns (a handoff's pack, then its new session) starts no more.
    stopped: Arc<std::sync::atomic::AtomicBool>,
    interrupter: Option<Arc<dyn Interrupter>>,
    turn: Option<tauri::async_runtime::JoinHandle<()>>,
    cost: Arc<Mutex<f64>>,
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
fn from_user(window: &tauri::WebviewWindow) -> Result<(), String> {
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

/// Where the task's worktree lives: under Vults' data folder, named after its roost so the flock
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
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
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

/// The task's worktree, made on first start from the repository's current commit, on the task's
/// branch; reused when it is already there.
fn worktree(task: &Task) -> Result<PathBuf, String> {
    let folder = Path::new(&task.folder);
    let repo = crate::roosts::repo_of(folder).ok_or("This task is not in a git repository.")?;
    let dir = worktree_dir(task);
    if dir.join(".git").exists() {
        return Ok(dir);
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let name = branch(task.id);
    let exists = git(
        &repo.root,
        &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{name}")],
    )
    .is_ok();
    let dir_arg = dir.to_string_lossy().into_owned();
    if exists {
        git(&repo.root, &["worktree", "add", &dir_arg, &name])?;
    } else {
        git(&repo.root, &["worktree", "add", "-b", &name, &dir_arg, "HEAD"])?;
    }
    Ok(dir)
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

/// Where a run's driver reports. Cards come through the hooks; the driver's own channel asks only
/// when no card could (the app paused, a card gone), and then the answer is no.
struct RunSink {
    task: i64,
    cost: Arc<Mutex<f64>>,
    reply: Mutex<String>,
    app: AppHandle,
    /// The worktree, the folder a card names.
    dir: String,
    /// The running driver's agent and session, for a card the app makes for it.
    engine: Mutex<Engine>,
    session: Mutex<Option<String>>,
}

impl Sink for RunSink {
    fn event(&self, event: Event) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            match event {
                Event::Text(text) => {
                    if let Ok(mut r) = self.reply.lock() {
                        r.push_str(&text);
                    }
                }
                Event::Usage(u) => {
                    if let (Some(c), Ok(mut cost)) = (u.cost_usd, self.cost.lock()) {
                        *cost += c;
                    }
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
            if engine == Engine::Claude {
                // Claude Code's cards come through its hooks; its own channel asks only when no
                // card could, and then the answer is no.
                tracing::info!(task = self.task, tool = %ask.tool, "a task run asked outside a card: no");
                return Answer::Deny;
            }
            // Codex's app-server asks on its own channel: the app makes the card a hook would, so
            // the island, rules, policies and the phone answer it as ever.
            let session = self
                .session
                .lock()
                .ok()
                .and_then(|s| s.clone())
                .unwrap_or_else(|| format!("vults-task-{}", self.task));
            let event = codex_card(self.task, &ask, &session, &self.dir);
            match crate::runtime::ask_human(&self.app, event).await {
                vults_ipc::Local::Decide(vults_protocol::Decision::Allow) => Answer::Allow,
                _ => Answer::Deny,
            }
        })
    }
}

/// The hook event a Codex `PermissionRequest` would send for `ask`: its tool and the command or
/// files, in the session and folder of the task's run.
fn codex_card(task: i64, ask: &Ask, session: &str, dir: &str) -> vults_protocol::Event {
    let detail = ask.full.clone().unwrap_or_else(|| {
        ask.target
            .split_once(" · ")
            .map_or(ask.target.clone(), |(_, v)| v.to_string())
    });
    let input = match ask.tool.as_str() {
        "Bash" => serde_json::json!({ "command": detail }),
        _ => serde_json::json!({ "file_path": detail }),
    };
    vults_protocol::Event {
        v: vults_protocol::VERSION,
        id: format!("task-{task}-{}", ask.id),
        agent: vults_protocol::AgentKind::Codex,
        agent_name: None,
        event: "PermissionRequest".into(),
        wants_reply: true,
        zeca: false,
        terminal: vults_protocol::Terminal::default(),
        payload: serde_json::json!({
            "hook_event_name": "PermissionRequest",
            "session_id": session,
            "cwd": dir,
            "tool_name": ask.tool,
            "tool_input": input,
        }),
    }
}

/// Adds what a run cost to its task, and takes the cost out of the run so it is counted once.
fn charge(id: i64, cost: &Mutex<f64>) {
    let usd = cost.lock().map(|mut c| std::mem::take(&mut *c)).unwrap_or(0.0);
    if usd > 0.0
        && let Err(e) = open().and_then(|mut s| s.task_cost(id, usd).map_err(|e| e.to_string()))
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
}

impl Engine {
    fn provider(self) -> vults_chat::Provider {
        match self {
            Self::Claude => vults_chat::Provider::Claude,
            Self::Codex => vults_chat::Provider::Codex,
        }
    }

    /// The owner a task gets, and the engine of a task's session by its owner.
    fn owner(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    fn of(owner: &str) -> Self {
        if owner == "codex" {
            Self::Codex
        } else {
            Self::Claude
        }
    }
}

/// The git folders a commit on the task's branch writes, outside its worktree: the worktree's own
/// admin folder, the objects, and the task's branch refs and their logs. Never the repository's
/// hooks or config. Codex's sandbox needs them named (Claude Code's allows a worktree's git dir by
/// itself, checked live).
fn git_writes(dir: &Path) -> Vec<PathBuf> {
    let (Ok(admin), Ok(common)) = (
        git(dir, &["rev-parse", "--absolute-git-dir"]),
        git(dir, &["rev-parse", "--path-format=absolute", "--git-common-dir"]),
    ) else {
        return Vec::new();
    };
    let common = PathBuf::from(common);
    [
        PathBuf::from(admin),
        common.join("objects"),
        common.join("refs/heads/vults"),
        common.join("logs/refs/heads/vults"),
    ]
    .into_iter()
    .filter(|p| p.is_dir())
    .collect()
}

/// What a run's Claude Code runs with, on `brief`: the user's own settings (our hooks run, so the
/// flock shows it and its cards are the island's; never the project's, a worktree path Claude Code
/// never trusted), its sandbox, a worker's role, Zeca's MCP.
fn spec(id: i64, dir: PathBuf, brief: String, engine: Engine) -> Spec {
    Spec {
        add_dirs: if engine == Engine::Codex {
            git_writes(&dir)
        } else {
            Vec::new()
        },
        settings: Settings::User,
        sandbox: Sandbox::Native,
        role: Role::Worker,
        system_append: Some(brief),
        mcp_config: crate::mcp::config_file(),
        env: vec![("VULTS_TASK".into(), id.to_string())],
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
    mut driver: Box<dyn vults_agents::driver::AgentDriver>,
    text: &str,
    sink: &RunSink,
    stopped: &std::sync::atomic::AtomicBool,
) -> (Result<(), String>, Option<String>) {
    holds(id, generation, driver.interrupter());
    // A Stop that came before Stop could reach this driver: it never starts.
    if stopped.load(std::sync::atomic::Ordering::SeqCst) {
        return (Ok(()), None);
    }
    let input = Input {
        text: text.into(),
        ..Input::default()
    };
    let result = match driver.start().await {
        Ok(()) => {
            if let Ok(mut s) = sink.session.lock() {
                *s = driver.session().map(str::to_owned);
            }
            driver.send(&input, sink).await
        }
        Err(e) => Err(e),
    };
    (result, driver.session().map(str::to_owned))
}

/// The files changed on the task's branch since it left the repository's current commit, and
/// those changed and not committed yet: what a pack's Files must hold.
fn changed_files(task: &Task, dir: &Path) -> Vec<String> {
    let Some(repo) = crate::roosts::repo_of(Path::new(&task.folder)) else {
        return Vec::new();
    };
    let mut files: Vec<String> = Vec::new();
    let mut add = |out: Result<String, String>, porcelain: bool| {
        for line in out.unwrap_or_default().lines() {
            let path = if porcelain {
                line.get(3..).unwrap_or_default()
            } else {
                line
            };
            let path = path.rsplit(" -> ").next().unwrap_or(path).trim().to_string();
            if !path.is_empty() && !files.contains(&path) {
                files.push(path);
            }
        }
    };
    // Where the branch began: its reflog's first entry; else where it meets the repository's
    // current commit.
    let base = git(
        dir,
        &[
            "log",
            "-g",
            "--format=%H",
            &format!("refs/heads/{}", branch(task.id)),
        ],
    )
    .ok()
    .and_then(|log| log.lines().last().map(str::to_owned))
    .filter(|b| !b.is_empty())
    .and_then(|b| git(dir, &["rev-parse", &format!("{b}^{{commit}}")]).ok())
    .or_else(|| {
        let head = git(&repo.root, &["rev-parse", "HEAD"]).ok()?;
        git(dir, &["merge-base", "HEAD", &head]).ok()
    });
    if let Some(base) = base {
        add(git(dir, &["diff", "--name-only", &base, "HEAD"]), false);
    }
    // NUL-separated, every untracked file named: no quoting, no folder standing for its files.
    if let Ok(out) = git(dir, &["status", "--porcelain", "-z", "--untracked-files=all"]) {
        let mut entries = out.split('\0').filter(|e| !e.is_empty());
        while let Some(e) = entries.next() {
            if let Some(path) = e.get(3..).filter(|p| !p.is_empty()) {
                if !files.iter().any(|f| f == path) {
                    files.push(path.to_string());
                }
                // A rename's entry is followed by its old path, which is no change of its own.
                if e.starts_with('R') || e.starts_with('C') {
                    entries.next();
                }
            }
        }
    }
    files.truncate(50);
    files
}

/// Starts the task's run: Claude Code in its worktree. `fresh` hands off first (W5): the session it
/// had writes a context pack, Vults adds what it knows, and a new session starts from that pack
/// alone, as a compaction of all it did.
async fn launch(app: AppHandle, id: i64, fresh: bool, engine: Engine) -> Result<(), String> {
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
        if fresh && !task.session.as_deref().is_some_and(crate::chat::valid_session) {
            return Err("This task has no session to hand off.".to_string());
        }
        let dir = worktree(&task)?;
        let now = crate::traces::now_ms();
        store
            .task_change(id, &Change::Owner(engine.owner().into()), now)
            .map_err(|e| e.to_string())?;
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
        let before = Engine::of(&task.owner);
        let sink = RunSink {
            task: id,
            cost,
            reply: Mutex::new(String::new()),
            app: done_app.clone(),
            dir: dir.to_string_lossy().into_owned(),
            engine: Mutex::new(before),
            session: Mutex::new(None),
        };
        let session = task.session.clone().filter(|s| crate::chat::valid_session(s));
        let mut pack: Option<String> = task.pack.clone();
        let mut handoff_failed = false;
        let (resume, text) = if fresh {
            // The session it had writes the pack, in the template; whatever it writes, Vults' own
            // facts (the goal, the ticks, the files on the branch) win.
            let old = vults_chat::drivers::resumed(
                before.provider(),
                spec(id, dir.clone(), brief(&task), before),
                session,
            );
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
                    let (t, d) = (task.clone(), dir.clone());
                    tauri::async_runtime::spawn_blocking(move || changed_files(&t, &d))
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
            match vults_chat::drivers::resumed(engine.provider(), spec(id, dir, text_brief, engine), resume) {
                Some(d) => turn(id, generation, d, text, &sink, &stopped).await,
                None => (Err("That agent can't run here.".to_string()), None),
            }
        };
        let (result, session) = result;
        if let Err(e) = &result {
            tracing::warn!(task = id, error = %e, "task run failed");
        }
        let reply = sink.reply.lock().map(|r| r.clone()).unwrap_or_default();
        let cost = sink.cost.clone();
        let failed = result.is_err();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            charge(id, &cost);
            let mut store = open()?;
            let now = crate::traces::now_ms();
            store
                .task_run_ended(id, &reply, session.as_deref(), now)
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

/// **Start**: the task's worktree, and Claude Code in it on its brief (going on with its last
/// session, when it had one). The task becomes Claude Code's and *Doing* before the turn runs.
#[tauri::command]
pub async fn task_start(
    app: AppHandle,
    window: tauri::WebviewWindow,
    id: i64,
    engine: Engine,
) -> Result<(), String> {
    from_user(&window)?;
    launch(app, id, false, engine).await
}

/// **Start fresh**: a handoff to a new session through a context pack (W5).
#[tauri::command]
pub async fn task_start_fresh(
    app: AppHandle,
    window: tauri::WebviewWindow,
    id: i64,
    engine: Engine,
) -> Result<(), String> {
    from_user(&window)?;
    launch(app, id, true, engine).await
}

/// **Stop**: asks the run's turn to stop, waits for it, and ends its process when it doesn't. Only
/// a run Vults started; the task goes back to *To do*.
#[tauri::command]
pub async fn task_stop(app: AppHandle, window: tauri::WebviewWindow, id: i64) -> Result<(), String> {
    from_user(&window)?;
    let (generation, interrupter, turn, cost) = {
        let mut r = runs().lock().map_err(|e| e.to_string())?;
        let run = r.get_mut(&id).ok_or("This task isn't running.")?;
        let Some(turn) = run.turn.take() else {
            return Err("This task is still starting.".into());
        };
        run.stopped.store(true, std::sync::atomic::Ordering::SeqCst);
        (run.generation, run.interrupter.clone(), turn, run.cost.clone())
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
    // Back to *To do*, unless a new run started meanwhile.
    if !running(id) {
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

/// **Remove worktree**: the task's worktree goes (its branch, and the commits on it, stay). Git
/// refuses while it holds uncommitted changes; nothing is forced.
#[tauri::command]
pub async fn task_remove_worktree(
    app: AppHandle,
    window: tauri::WebviewWindow,
    id: i64,
) -> Result<(), String> {
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
            return Ok(());
        }
        let repo =
            crate::roosts::repo_of(Path::new(&task.folder)).ok_or("This task is not in a git repository.")?;
        git(&repo.root, &["worktree", "remove", &dir.to_string_lossy()]).map(|_| ())
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
        }
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

    #[cfg(unix)]
    #[test]
    fn two_tasks_get_two_worktrees_of_one_repository() {
        let root = std::env::temp_dir().join(format!("vults-runs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("site");
        std::fs::create_dir_all(&repo).unwrap();
        let g = |args: &[&str]| git(&repo, args).unwrap();
        g(&["init", "-q", "-b", "main"]);
        g(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "first",
        ]);
        let add = |id: i64| {
            let dir = root.join(format!("site-task-{id}"));
            git(
                &repo,
                &[
                    "worktree",
                    "add",
                    "-b",
                    &branch(id),
                    &dir.to_string_lossy(),
                    "HEAD",
                ],
            )
            .unwrap();
            dir
        };
        let (a, b) = (add(1), add(2));
        assert!(a.join(".git").exists() && b.join(".git").exists());
        assert_eq!(git(&a, &["branch", "--show-current"]).unwrap(), "vults/task-1");
        assert_eq!(git(&b, &["branch", "--show-current"]).unwrap(), "vults/task-2");
        // Removing one leaves its branch, and never forces a tree with changes away.
        std::fs::write(b.join("wip.txt"), "x").unwrap();
        assert!(
            git(&repo, &["worktree", "remove", &b.to_string_lossy()]).is_err(),
            "uncommitted work stays"
        );
        git(&repo, &["worktree", "remove", &a.to_string_lossy()]).unwrap();
        assert!(
            git(
                &repo,
                &["rev-parse", "--verify", "--quiet", "refs/heads/vults/task-1"]
            )
            .is_ok()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn a_packs_files_are_the_branchs_changes_and_the_uncommitted_ones() {
        let root = std::env::temp_dir().join(format!("vults-pack-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("site");
        std::fs::create_dir_all(&repo).unwrap();
        let g = |dir: &Path, args: &[&str]| git(dir, args).unwrap();
        let commit = [
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "x",
        ];
        g(&repo, &["init", "-q", "-b", "main"]);
        std::fs::write(repo.join("old.txt"), "a").unwrap();
        g(&repo, &["add", "."]);
        g(&repo, &commit);
        let dir = root.join("site-task-7");
        g(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "vults/task-7",
                &dir.to_string_lossy(),
                "HEAD",
            ],
        );
        std::fs::write(dir.join("new.rs"), "fn main() {}").unwrap();
        g(&dir, &["add", "."]);
        g(&dir, &commit);
        std::fs::write(dir.join("old.txt"), "b").unwrap();
        std::fs::create_dir_all(dir.join("docs/new dir")).unwrap();
        std::fs::write(dir.join("docs/new dir/a b.md"), "x").unwrap();
        // The repository moved on meanwhile: the branch's own start is still the base.
        std::fs::write(repo.join("other.txt"), "c").unwrap();
        g(&repo, &["add", "."]);
        g(&repo, &commit);
        let mut t = task(7, "site");
        t.folder = repo.to_string_lossy().into_owned();
        assert_eq!(
            changed_files(&t, &dir),
            ["new.rs", "old.txt", "docs/new dir/a b.md"],
            "each file, unquoted, nothing of others'"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_codex_approval_becomes_the_card_its_hook_would_send() {
        let ask = Ask {
            id: "7".into(),
            tool: "Bash".into(),
            target: "Bash · cargo test".into(),
            full: None,
            input: serde_json::json!({ "command": "cargo test" }),
        };
        let event = codex_card(12, &ask, "thread-1", "/w/site-task-12");
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
        // A long command is bound whole, never by its cut start.
        let long = Ask {
            full: Some("cargo test && rm -rf build".into()),
            target: "Bash · cargo test && rm…".into(),
            ..ask
        };
        assert_eq!(
            codex_card(12, &long, "t", "/w").payload["tool_input"]["command"],
            "cargo test && rm -rf build"
        );
    }

    #[cfg(unix)]
    #[test]
    fn codex_may_write_only_what_a_commit_on_its_branch_needs() {
        let root = std::env::temp_dir().join(format!("vults-git-writes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("site");
        std::fs::create_dir_all(&repo).unwrap();
        let g = |dir: &Path, args: &[&str]| git(dir, args).unwrap();
        g(&repo, &["init", "-q", "-b", "main"]);
        g(
            &repo,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "x",
            ],
        );
        let dir = root.join("site-task-3");
        g(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "vults/task-3",
                &dir.to_string_lossy(),
                "HEAD",
            ],
        );
        let writes = git_writes(&dir);
        let common = repo.join(".git").canonicalize().unwrap();
        assert!(writes.iter().any(|p| p.ends_with("worktrees/site-task-3")));
        assert!(writes.iter().any(|p| p.ends_with("objects")));
        assert!(writes.iter().any(|p| p.ends_with("refs/heads/vults")));
        for p in &writes {
            let p = p.canonicalize().unwrap();
            assert!(
                p != common && !p.ends_with("hooks") && !p.ends_with("config"),
                "{p:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
