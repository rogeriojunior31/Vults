//! Zeca's flock tools for the API chat (`docs/dev/plan-zeca.md` Z9): what the island shows, read
//! through his own snapshot (`vults_zeca::FlockReader`), so "what is Codex doing?" is answered
//! without pasting anything. They only read: none of them can reach an intent, let alone a card.

use std::sync::Arc;

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};
use vults_agents::driver::BoxFuture;
use vults_chat::tools::{Tool, Tools};
use vults_zeca::flock::{Flock, FlockReader, Session};

/// Where the tools read the flock from: the app's last view, or a test's.
pub trait Source: Send + Sync + 'static {
    /// The island's view now, as every window got it (JSON).
    fn view(&self) -> Option<Value>;
    /// The plans' usage windows (JSON).
    fn usage(&self) -> Value;
}

impl Source for AppHandle {
    fn view(&self) -> Option<Value> {
        let view = self.state::<crate::runtime::LastView>().get()?;
        serde_json::to_value(view).ok()
    }

    fn usage(&self) -> Value {
        serde_json::to_value(crate::usage::usage(self.clone())).unwrap_or_default()
    }
}

/// Every flock tool, reading from `source`.
pub fn all(source: Arc<dyn Source>) -> Tools {
    vec![
        Arc::new(FlockList(source.clone())),
        Arc::new(SessionDetail(source.clone())),
        Arc::new(PendingAsks(source.clone())),
        Arc::new(Usage(source.clone())),
        Arc::new(GithubSnapshot(source)),
    ]
}

/// Zeca's snapshot of the flock from the view. Every agent-supplied text (a card's target, what a
/// session is doing, its steps) is redacted here: the core's view keeps them as the agent sent
/// them, and these go to the provider. They stay local agents' words, not outside content (Z8): an
/// agent's own fetches are its session's business, and the chat's cards are not the agents'.
pub fn flock(view: &Value) -> Flock {
    let text = |v: &Value| v.as_str().map(str::to_string);
    let approval = &view["approval"];
    let sessions = view["sessions"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|s| {
            let agent = text(&s["agent_name"])
                .or_else(|| text(&s["agent"]))
                .unwrap_or_default();
            // `waiting`: a card waits on it, in front or queued (`card` is only the one in front).
            let waiting = (s["waiting"] == true || s["card"] == true).then(|| {
                // Ids are unique per agent: the front card is the one of this agent's session.
                if approval["session"] == s["id"] && approval["agent"] == s["agent"] {
                    (
                        text(&approval["tool"]).unwrap_or_default(),
                        vults_core::redact::secrets(approval["target"].as_str().unwrap_or_default()),
                    )
                } else {
                    // Another card than the one in front: the island does not show its target.
                    (String::new(), String::new())
                }
            });
            Session {
                agent,
                project: text(&s["project"]).unwrap_or_default(),
                status: text(&s["status"]).unwrap_or_default(),
                doing: s["step"].as_str().map(vults_core::redact::secrets),
                steps: s["steps"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|v| v.as_str().map(vults_core::redact::secrets))
                    .collect(),
                waiting,
            }
        })
        .collect();
    Flock { sessions }
}

fn reader(source: &dyn Source) -> FlockReader {
    FlockReader::new(source.view().map(|v| flock(&v)).unwrap_or_default())
}

/// One session as a line of the tools' answers.
fn brief(s: &Session) -> Value {
    json!({
        "agent": s.agent,
        "project": s.project,
        "status": s.status,
        "doing": s.doing,
        "waiting_on": s.waiting.as_ref().map(|(tool, target)| {
            if tool.is_empty() { "a card".to_string() } else { format!("{tool} · {target}") }
        }),
    })
}

fn detail(s: &Session) -> Value {
    let mut value = brief(s);
    value["steps"] = json!(s.steps);
    value
}

/// Voice commands already resolved an exact identity; do not guess again from its display label.
pub fn session_detail_exact(view: &Value, key: &vults_core::SessionKey) -> Option<Value> {
    let session = view["sessions"]
        .as_array()?
        .iter()
        .find(|s| s["id"].as_str() == Some(&key.session_id) && s["agent"] == json!(key.agent))?;
    let mut selected = view.clone();
    selected["sessions"] = json!([session]);
    let flock = flock(&selected);
    flock.sessions.first().map(detail)
}

const NO_INPUT: fn() -> Value = || json!({ "type": "object", "properties": {} });

#[derive(Debug)]
struct FlockList(Arc<dyn Source>);
#[derive(Debug)]
struct SessionDetail(Arc<dyn Source>);
#[derive(Debug)]
struct PendingAsks(Arc<dyn Source>);
#[derive(Debug)]
struct Usage(Arc<dyn Source>);
#[derive(Debug)]
struct GithubSnapshot(Arc<dyn Source>);

impl std::fmt::Debug for dyn Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Source")
    }
}

fn answer(v: Value) -> BoxFuture<'static, Result<String, String>> {
    Box::pin(async move { Ok(v.to_string()) })
}

/// An error the model reads as one (`is_error`), with what it can do instead.
fn failed(v: Value) -> BoxFuture<'static, Result<String, String>> {
    Box::pin(async move { Err(v.to_string()) })
}

/// Whether `asked` names this session: its project, or its agent by any of the names people use
/// (`claude code` for `claude`, `qwen code` for `qwen`).
fn named(s: &Session, asked: &str) -> bool {
    let asked = asked.trim().to_lowercase();
    let agent = s.agent.to_lowercase();
    s.project.to_lowercase() == asked
        || agent == asked
        || asked
            .split_whitespace()
            .next()
            .is_some_and(|first| first == agent)
}

impl Tool for FlockList {
    fn name(&self) -> &str {
        "flock_list"
    }
    fn description(&self) -> &str {
        "The agent sessions on the user's island now (Claude Code, Codex, Antigravity and others): \
         each one's agent, project, status, what it is doing and the card it waits on, if any."
    }
    fn schema(&self) -> Value {
        NO_INPUT()
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        let r = reader(self.0.as_ref());
        answer(json!({ "sessions": r.sessions().iter().map(brief).collect::<Vec<_>>() }))
    }
}

impl Tool for SessionDetail {
    fn name(&self) -> &str {
        "session_detail"
    }
    fn description(&self) -> &str {
        "One session's latest steps, by its project name (or its agent's name when only one runs)."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "session": { "type": "string", "description": "The project, or the agent" } },
            "required": ["session"],
        })
    }
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        let r = reader(self.0.as_ref());
        let wanted = input["session"].as_str().unwrap_or_default();
        let found: Vec<&Session> = r.sessions().iter().filter(|s| named(s, wanted)).collect();
        match found.as_slice() {
            [s] => answer(detail(s)),
            [] => failed(json!({
                "error": format!("No session named {wanted}."),
                "sessions": r.sessions().iter().map(brief).collect::<Vec<_>>(),
            })),
            many => failed(json!({
                "error": "More than one session goes by that name: name the project.",
                "sessions": many.iter().map(|s| brief(s)).collect::<Vec<_>>(),
            })),
        }
    }
}

impl Tool for PendingAsks {
    fn name(&self) -> &str {
        "pending_asks"
    }
    fn description(&self) -> &str {
        "The sessions waiting on the user's answer to a card. Only the user answers them, on the island."
    }
    fn schema(&self) -> Value {
        NO_INPUT()
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        answer(
            cached_read(&self.0.view().unwrap_or_default(), Value::Null, "pending_asks")
                .expect("Known local read"),
        )
    }
}

impl Tool for Usage {
    fn name(&self) -> &str {
        "usage"
    }
    fn description(&self) -> &str {
        "How much of each plan's rate-limit windows is used (Claude Code, Codex), and when they reset."
    }
    fn schema(&self) -> Value {
        NO_INPUT()
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        answer(json!({ "windows": self.0.usage() }))
    }
}

/// What is open, for the model: releases left out, and of each row's checks only those not
/// passed, so twenty pull requests of thirty checks each stay a short answer.
fn open_rows(rows: &Value) -> Value {
    let rows = rows
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["group"] != "releases");
    Value::Array(
        rows.map(|r| {
            let mut r = r.clone();
            if let Some(runs) = r["facts"]["runs"].as_array_mut() {
                runs.retain(|c| c["state"] != "passed" && c["state"] != "skipped");
            }
            r
        })
        .collect(),
    )
}

/// Synchronous local reads let command admission and cancellation share one short lock.
/// No request is made: both values are already cached by their existing owners.
pub fn cached_read(view: &Value, usage: Value, name: &str) -> Option<Value> {
    Some(match name {
        "pending_asks" => {
            json!({"waiting":FlockReader::new(flock(view)).waiting().map(brief).collect::<Vec<_>>()})
        }
        "usage" => json!({"windows":usage}),
        "github_snapshot" => {
            let board = view["boards"]
                .as_array()?
                .iter()
                .find(|board| board["connector"] == "github")?;
            json!({"rows":open_rows(&board["rows"])})
        }
        _ => return None,
    })
}

impl Tool for GithubSnapshot {
    fn name(&self) -> &str {
        "github_snapshot"
    }
    fn description(&self) -> &str {
        "The user's GitHub card as the island shows it: their pull requests and reviews, with checks."
    }
    fn schema(&self) -> Value {
        NO_INPUT()
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        match cached_read(&self.0.view().unwrap_or_default(), Value::Null, "github_snapshot") {
            Some(value) => answer(value),
            None => failed(json!({"error":"The GitHub connector is off, or has nothing yet."})),
        }
    }

    /// Titles and names come from GitHub: outside content (Z8).
    fn reads_outside(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Value);

    impl Source for Fake {
        fn view(&self) -> Option<Value> {
            Some(self.0.clone())
        }
        fn usage(&self) -> Value {
            json!([{ "agent": "claude", "minutes": 300, "used_percent": 42, "resets_at": null }])
        }
    }

    fn view() -> Value {
        json!({
            "sessions": [
                { "id": "a", "agent": "claude", "project": "vults", "status": "approval", "card": true,
                  "waiting": true, "step": null, "steps": ["Read lib.rs"] },
                { "id": "b", "agent": "codex", "project": "site", "status": "working", "card": false,
                  "waiting": false, "step": "Running cargo test",
                  "steps": ["Edit main.rs", "Running export GITHUB_TOKEN=ghp_secret", "Running cargo test"] },
                // A card queued behind the one in front: `waiting`, not `card`.
                { "id": "c", "agent": "claude", "project": "docs", "status": "approval", "card": false,
                  "waiting": true, "step": null, "steps": [] },
                { "id": "d", "agent": "other", "agent_name": "antigravity", "project": "notes", "status": "idle",
                  "card": false, "waiting": false, "step": null, "steps": [] },
            ],
            "approval": { "session": "a", "agent": "claude", "tool": "Bash", "target": "Bash · TOKEN=abc123 ./deploy.sh" },
            "boards": [{ "connector": "github", "rows": [{ "name": "vults#12", "title": "Nest" }] }],
        })
    }

    fn tool(tools: &Tools, name: &str) -> Arc<dyn Tool> {
        tools.iter().find(|t| t.name() == name).unwrap().clone()
    }

    async fn call(tools: &Tools, name: &str, input: Value) -> Value {
        serde_json::from_str(&tool(tools, name).call(&input).await.unwrap()).unwrap()
    }

    /// A call the model reads as an error.
    async fn error(tools: &Tools, name: &str, input: Value) -> Value {
        serde_json::from_str(&tool(tools, name).call(&input).await.unwrap_err()).unwrap()
    }

    #[tokio::test]
    async fn what_is_codex_doing_is_answered_from_the_island() {
        let tools = all(Arc::new(Fake(view())));
        let list = call(&tools, "flock_list", json!({})).await;
        let codex = &list["sessions"][1];
        assert_eq!(
            (codex["agent"].as_str(), codex["doing"].as_str()),
            (Some("codex"), Some("Running cargo test"))
        );
        let detail = call(&tools, "session_detail", json!({ "session": "Codex" })).await;
        let steps = detail["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 3);
        // A secret in a step never leaves.
        assert!(!detail.to_string().contains("ghp_secret"), "{detail}");
        // Two sessions go by "claude": the project is asked for.
        let two = error(&tools, "session_detail", json!({ "session": "claude code" })).await;
        assert!(two["error"].as_str().unwrap().contains("More than one"));
        // Another tool goes by its own name, said the way people say it.
        assert_eq!(
            call(&tools, "session_detail", json!({ "session": "Antigravity IDE" })).await["project"],
            "notes"
        );
        let none = error(&tools, "session_detail", json!({ "session": "aider" })).await;
        assert!(none["error"].is_string());
    }

    #[tokio::test]
    async fn the_cards_waiting_name_what_they_ask_with_secrets_redacted() {
        let tools = all(Arc::new(Fake(view())));
        let asks = call(&tools, "pending_asks", json!({})).await;
        let waiting = asks["waiting"].as_array().unwrap();
        assert_eq!(waiting.len(), 2);
        let front = waiting[0]["waiting_on"].as_str().unwrap();
        assert!(
            front.starts_with("Bash · ") && !front.contains("abc123"),
            "{front}"
        );
        // A card not in front: the island doesn't show its target, nor does the tool.
        assert_eq!(waiting[1]["waiting_on"], "a card");
    }

    #[test]
    fn the_model_gets_what_is_open_and_only_the_checks_that_did_not_pass() {
        let rows = json!([
            { "name": "app#1", "group": "yours", "facts": { "runs": [
                { "name": "test", "state": "passed" }, { "name": "visual", "state": "failed" },
                { "name": "docs", "state": "skipped" } ] } },
            { "name": "app v1", "group": "releases", "facts": { "runs": [] } },
        ]);
        let open = open_rows(&rows);
        assert_eq!(open.as_array().unwrap().len(), 1);
        assert_eq!(
            open[0]["facts"]["runs"],
            json!([{ "name": "visual", "state": "failed" }])
        );
    }

    #[tokio::test]
    async fn usage_and_github_come_through_and_github_is_outside_content() {
        let tools = all(Arc::new(Fake(view())));
        assert_eq!(
            call(&tools, "usage", json!({})).await["windows"][0]["used_percent"],
            42
        );
        assert_eq!(
            call(&tools, "github_snapshot", json!({})).await["rows"][0]["name"],
            "vults#12"
        );
        assert!(tool(&tools, "github_snapshot").reads_outside());
        assert!(!tool(&tools, "flock_list").reads_outside());
        let off = all(Arc::new(Fake(json!({ "sessions": [] }))));
        assert!(error(&off, "github_snapshot", json!({})).await["error"].is_string());
        assert_eq!(call(&off, "flock_list", json!({})).await["sessions"], json!([]));
        // No view yet (the app just started): an empty flock, not a failure.
        struct Nothing;
        impl Source for Nothing {
            fn view(&self) -> Option<Value> {
                None
            }
            fn usage(&self) -> Value {
                json!([])
            }
        }
        let none = all(Arc::new(Nothing));
        assert_eq!(call(&none, "pending_asks", json!({})).await["waiting"], json!([]));
    }

    #[test]
    fn the_snapshot_reads_the_cores_own_view() {
        use vults_core::{AgentEvent, AgentUpdate, Ask, Input, RequestId, SessionKey, State, Step, Terminal};
        let mut state = State::default();
        let now = std::time::Instant::now();
        let update = |session: &str, agent, event| {
            Input::Agent(AgentUpdate {
                session: SessionKey {
                    agent,
                    session_id: session.into(),
                },
                cwd: Some(format!("/home/me/{session}")),
                terminal: Terminal::default(),
                agent_id: None,
                event,
                received: None,
            })
        };
        let codex = vults_core::AgentKind::Codex;
        let claude = vults_core::AgentKind::Claude;
        vults_core::reduce(
            &mut state,
            update(
                "site",
                codex,
                AgentEvent::ToolStarted(Step {
                    activity: vults_core::Activity::Run,
                    tool: "Bash".into(),
                    detail: Some("cargo test".into()),
                }),
            ),
            now,
        );
        vults_core::reduce(
            &mut state,
            update(
                "vults",
                claude,
                AgentEvent::PermissionRequested {
                    request: RequestId("r".into()),
                    tool: "Bash".into(),
                    target: "Bash · TOKEN=abc123 ./deploy.sh".into(),
                    ask: Ask::default(),
                },
            ),
            now,
        );
        // A second card, queued behind the first.
        vults_core::reduce(
            &mut state,
            update(
                "docs",
                claude,
                AgentEvent::PermissionRequested {
                    request: RequestId("r2".into()),
                    tool: "Edit".into(),
                    target: "Edit · README.md".into(),
                    ask: Ask::default(),
                },
            ),
            now,
        );
        let flock = flock(&serde_json::to_value(state.view()).unwrap());
        let queued = flock.sessions.iter().find(|s| s.project == "docs").unwrap();
        assert!(queued.waiting.is_some(), "a queued card waits too: {queued:?}");
        let codex = flock.sessions.iter().find(|s| s.agent == "codex").unwrap();
        assert_eq!(codex.project, "site");
        assert!(
            codex.doing.as_deref().is_some_and(|d| d.contains("cargo test")),
            "{codex:?}"
        );
        let claude = flock.sessions.iter().find(|s| s.project == "vults").unwrap();
        let (tool, target) = claude.waiting.clone().unwrap();
        assert_eq!(tool, "Bash");
        assert!(
            target.contains("deploy.sh") && !target.contains("abc123"),
            "{target}"
        );
    }
}
