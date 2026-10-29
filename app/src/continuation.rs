//! A single watched session resumed by the user's `/continue` (ADR 0031, W7).
//! Only this owned driver can be interrupted; permissions still go through the core ledger.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::AppHandle;
use vults_agents::driver::{
    Answer, Ask, BoxFuture, Event, Input, Interrupter, Role, Sandbox, Settings, Sink, Spec,
};
use vults_protocol::AgentKind;

const STOP_WAIT: Duration = Duration::from_secs(10);
pub type Said = (&'static str, Option<String>);

struct Run {
    generation: u64,
    stopped: Arc<std::sync::atomic::AtomicBool>,
    interrupter: Option<Arc<dyn Interrupter>>,
    turn: Option<tauri::async_runtime::JoinHandle<()>>,
    cost: Arc<Mutex<Spent>>,
    session: Option<(AgentKind, Arc<Mutex<Option<String>>>)>,
}

#[derive(Default)]
struct Spent {
    pending: Option<crate::spend::Pending>,
    session: Option<String>,
}

fn current() -> &'static Mutex<Option<Run>> {
    static RUN: std::sync::OnceLock<Mutex<Option<Run>>> = std::sync::OnceLock::new();
    RUN.get_or_init(Mutex::default)
}

fn reserve() -> Result<u64, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let mut slot = current().lock().map_err(|e| e.to_string())?;
    if slot.is_some() {
        return Err("A continuation is already running.".into());
    }
    let generation = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    *slot = Some(Run {
        generation,
        stopped: Arc::default(),
        interrupter: None,
        turn: None,
        cost: Arc::default(),
        session: None,
    });
    Ok(generation)
}

fn release(generation: u64) {
    if let Ok(mut slot) = current().lock()
        && slot.as_ref().is_some_and(|run| run.generation == generation)
    {
        *slot = None;
    }
}

struct RunSink {
    cost: Arc<Mutex<Spent>>,
    app: AppHandle,
    dir: String,
    agent: AgentKind,
    session: Arc<Mutex<Option<String>>>,
    generation: u64,
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
                Event::Usage(u) => {
                    let tokens = u.input.saturating_sub(u.cache_read) + u.output;
                    let usd = u.cost_usd.unwrap_or(0.0);
                    let plan = crate::spend::plan(&self.app, self.agent);
                    if let Ok(mut c) = self.cost.lock()
                        && let Some(total) = u.session_cost_usd
                    {
                        c.pending = Some(crate::spend::Pending {
                            total,
                            counted: u.cost_usd.is_some(),
                            agent: self.agent,
                            roost: self.roost.clone(),
                            plan,
                        });
                    }
                    let (app, roost) = (self.app.clone(), self.roost.clone());
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        crate::spend::continuation(&app, &roost, plan, usd, tokens);
                    })
                    .await;
                }
                _ => {}
            }
        })
    }

    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        Box::pin(async move {
            let session = self
                .session
                .lock()
                .ok()
                .and_then(|s| s.clone())
                .unwrap_or_else(|| format!("vults-continue-{}", self.generation));
            let card = match self.agent {
                AgentKind::Claude => claude_card(self.generation, &ask, &session, &self.dir),
                AgentKind::Codex => codex_card(self.generation, &ask, &session, &self.dir),
                _ => None,
            };
            let Some(event) = card else {
                return Answer::Deny;
            };
            match crate::runtime::ask_human(&self.app, event).await {
                vults_ipc::Local::Decide(vults_protocol::Decision::Allow) => Answer::Allow,
                _ => Answer::Deny,
            }
        })
    }
}
fn claude_card(generation: u64, ask: &Ask, session: &str, dir: &str) -> Option<vults_protocol::Event> {
    if ask.tool.is_empty() || !ask.input.is_object() || ask.tool == vults_protocol::QUESTION_TOOL {
        return None;
    }
    Some(vults_protocol::Event {
        v: vults_protocol::VERSION,
        id: format!("continue-{generation}-{}", ask.id),
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

fn codex_card(generation: u64, ask: &Ask, session: &str, dir: &str) -> Option<vults_protocol::Event> {
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
        id: format!("continue-{generation}-{}", ask.id),
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

async fn withdraw(app: &AppHandle, agent: AgentKind, session: &Mutex<Option<String>>, failure: Option<&str>) {
    if let Some(session) = session.lock().ok().and_then(|s| s.clone()) {
        crate::runtime::run_ended(app, ended(agent, &session, failure)).await;
    }
}

fn ended(agent: AgentKind, session: &str, failure: Option<&str>) -> vults_core::AgentUpdate {
    vults_core::AgentUpdate {
        session: vults_core::SessionKey {
            agent,
            session_id: session.into(),
        },
        cwd: None,
        terminal: vults_protocol::Terminal::default(),
        agent_id: None,
        call_id: None,
        received: None,
        event: match failure {
            Some(error) => vults_core::AgentEvent::StopFailed {
                error: Some(error.into()),
            },
            None => vults_core::AgentEvent::Stopped { message: None },
        },
    }
}

fn alive(pid: u32) -> Option<bool> {
    if cfg!(target_os = "linux") {
        Some(Path::new(&format!("/proc/{pid}")).exists())
    } else {
        None
    }
}

async fn within_budget(app: &AppHandle) -> Result<(), String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || crate::spend::refuse(&app))
        .await
        .map_err(|e| e.to_string())?
}
async fn turn(
    generation: u64,
    mut driver: Box<dyn vults_agents::driver::AgentDriver>,
    text: &str,
    sink: &RunSink,
    stopped: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    if let Ok(mut slot) = current().lock()
        && let Some(run) = slot.as_mut().filter(|run| run.generation == generation)
    {
        run.interrupter = Some(driver.interrupter());
    }
    // A Stop that arrived before the driver was ready still prevents its first turn.
    if stopped.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }
    let result = match driver.start().await {
        Ok(()) => {
            if let Ok(mut s) = sink.session.lock() {
                *s = driver.session().map(str::to_owned);
            }
            if let Ok(mut c) = sink.cost.lock() {
                c.session = driver.session().map(str::to_owned);
            }
            let sent = driver
                .send(
                    &Input {
                        text: text.into(),
                        ..Input::default()
                    },
                    sink,
                )
                .await;
            if let Ok(mut c) = sink.cost.lock() {
                c.session = driver.session().map(str::to_owned);
            }
            sent
        }
        Err(e) => Err(e),
    };
    let (cost, app) = (sink.cost.clone(), sink.app.clone());
    let _ = tauri::async_runtime::spawn_blocking(move || settle(Some(&app), &cost)).await;
    result
}

fn settle(app: Option<&AppHandle>, cost: &Mutex<Spent>) {
    if let Ok(mut c) = cost.lock()
        && let (Some(p), Some(session)) = (c.pending.take(), c.session.clone())
    {
        crate::spend::settle(app, &session, &p);
    }
}
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
    // Never resume a legacy isolated clone as an ordinary watched session.
    if Path::new(&c.cwd).starts_with(crate::paths::data_dir().join("worktrees")) {
        return ("That session can't be continued from here.", None);
    }
    let engine = match c.key.agent {
        vults_protocol::AgentKind::Codex => vults_chat::Provider::Codex,
        vults_protocol::AgentKind::Claude => vults_chat::Provider::Claude,
        _ => return ("That session can't be continued from here.", None),
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
    let Ok(generation) = reserve() else {
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
    let Some(driver) = vults_chat::drivers::resumed(engine, spec, Some(c.key.session_id.clone())) else {
        release(generation);
        return ("That session can't be continued from here.", None);
    };
    let (cost, stopped) = current()
        .lock()
        .ok()
        .and_then(|r| r.as_ref().map(|run| (run.cost.clone(), run.stopped.clone())))
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
    let session = Arc::new(Mutex::new(Some(c.key.session_id.clone())));
    let sink = RunSink {
        cost,
        app: done_app.clone(),
        dir: cwd,
        agent: c.key.agent,
        session: session.clone(),
        generation,
        roost,
    };
    if let Ok(mut slot) = current().lock()
        && let Some(run) = slot.as_mut().filter(|run| run.generation == generation)
    {
        run.session = Some((c.key.agent, session));
    }
    let running = tauri::async_runtime::spawn(async move {
        let result = turn(generation, driver, &words, &sink, &stopped).await;
        withdraw(
            &done_app,
            sink.agent,
            &sink.session,
            result.as_ref().err().map(String::as_str),
        )
        .await;
        if let Err(e) = result {
            tracing::warn!(error = %e, "a continued session's turn failed");
            let _ = first.send(e);
        }
        release(generation);
    });
    if let Ok(mut r) = current().lock()
        && let Some(run) = r.as_mut().filter(|run| run.generation == generation)
    {
        run.turn = Some(running);
    }
    if let Ok(Ok(e)) = tokio::time::timeout(Duration::from_secs(4), failed_fast).await {
        return ("That session couldn't be resumed:", Some(e));
    }
    ("Continuing this session; it shows in the flock:", Some(project))
}
pub async fn stop_continuation(app: AppHandle) -> Said {
    let (generation, interrupter, turn, cost, session) = {
        let Ok(mut slot) = current().lock() else {
            return ("It is still starting: try again in a moment.", None);
        };
        let Some(run) = slot.as_mut() else {
            return ("No session is being continued.", None);
        };
        let Some(turn) = run.turn.take() else {
            return ("It is still starting: try again in a moment.", None);
        };
        run.stopped.store(true, std::sync::atomic::Ordering::SeqCst);
        (
            run.generation,
            run.interrupter.clone(),
            turn,
            run.cost.clone(),
            run.session.clone(),
        )
    };
    if let Some(i) = interrupter {
        let _ = i.interrupt().await;
    }
    let mut turn = turn;
    // Dropping this owned driver ends its process; watched processes never enter this slot.
    if tokio::time::timeout(STOP_WAIT, &mut turn).await.is_err() {
        turn.abort();
        let _ = turn.await;
        if let Some((agent, session)) = session {
            withdraw(&app, agent, &session, None).await;
        }
        let _ = tauri::async_runtime::spawn_blocking(move || settle(Some(&app), &cost)).await;
        release(generation);
    }
    ("Stopped.", None)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
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
        let event = codex_card(3, &cmd, "thread-1", "/w/site").unwrap();
        assert_eq!(
            event.id, "continue-3-7",
            "a run's own id: a dead card of an earlier run never collides"
        );
        let update = vults_agents::parse(&event).unwrap();
        assert_eq!(update.session.session_id, "thread-1");
        assert_eq!(update.cwd.as_deref(), Some("/w/site"));
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
        let e = codex_card(3, &long, "t", "/w").unwrap();
        assert_eq!(e.payload["tool_input"]["command"], "cargo test && rm -rf build");
        assert_eq!(e.payload["cwd"], "/elsewhere");
        // An edit is the patch of its changes: its files and line counts, as a hook's card.
        let edit = ask(
            "Edit",
            "Edit · /w/a.rs",
            None,
            serde_json::json!({ "changes": [{ "path": "/w/a.rs", "kind": "update", "diff": "@@ -1 +1,2 @@\n-old\n+new\n+more\n" }] }),
        );
        let update = vults_agents::parse(&codex_card(3, &edit, "t", "/w").unwrap()).unwrap();
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
                3,
                &ask("Edit", "Edit · add notes", None, serde_json::Value::Null),
                "t",
                "/w"
            )
            .is_none()
        );
        assert!(
            codex_card(
                3,
                &ask("Bash", "Bash", None, serde_json::json!({ "command": " " })),
                "t",
                "/w"
            )
            .is_none()
        );
        assert!(codex_card(3, &ask("Other", "x", None, serde_json::Value::Null), "t", "/w").is_none());
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
        let event = claude_card(3, &command, "claude-session", "/w/site").unwrap();
        let update = vults_agents::parse(&event).unwrap();
        assert_eq!(event.id, "continue-3-7");
        assert_eq!(update.session.session_id, "claude-session");
        assert_eq!(update.cwd.as_deref(), Some("/w/site"));
        assert!(
            matches!(update.event, vults_core::AgentEvent::PermissionRequested { target, .. } if target == "Bash · cargo test")
        );
        assert!(claude_card(3, &ask("Bash", "", None, serde_json::Value::Null), "s", "/w").is_none());
        // AskUserQuestion keeps its answer-bearing PreToolUse hook, never an Allow/Deny card.
        assert!(
            claude_card(
                3,
                &ask(vults_protocol::QUESTION_TOOL, "", None, serde_json::json!({})),
                "s",
                "/w"
            )
            .is_none()
        );
    }
    #[test]
    fn failed_owned_runs_keep_their_cause_and_release_all_waiting_cards() {
        use vults_core::{AgentEvent, Effect, Input as CoreInput, RequestId, State, Status};
        let now = std::time::Instant::now();
        for agent in [AgentKind::Claude, AgentKind::Codex] {
            let update = ended(agent, "session", Some("The CLI failed."));
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
            assert!(state.pending.is_empty(), "{agent:?}");
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
    }
    #[test]
    fn only_one_continuation_and_a_late_end_cannot_release_its_successor() {
        let a = reserve().unwrap();
        assert!(reserve().is_err());
        release(a);
        let b = reserve().unwrap();
        release(a);
        assert_eq!(current().lock().unwrap().as_ref().unwrap().generation, b);
        release(b);
        assert!(current().lock().unwrap().is_none());
    }
}
