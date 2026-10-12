//! The Agent Client Protocol (`docs/dev/plan-zeca.md` W10): one long-lived process per agent,
//! JSON-RPC 2.0 over stdio, one object per line (`initialize`, `session/new` or `session/load`,
//! `session/prompt`, `session/cancel`), for Gemini CLI, OpenCode, Qwen Code, Copilot CLI and the
//! rest of the registry. Vults offers no file system and no terminal of its own: the agent uses its
//! own tools, and asks (`session/request_permission`) before each one that writes or runs. Each ask
//! is a card; an answer is only ever "this once", never the agent's "always", which would skip
//! every later card.

use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use vults_agents::driver::{
    AgentDriver, Ask, BoxFuture, EndReason, Event, Input, Interrupter, Sink, Spec, Usage,
};

use crate::drivers::{Release, Stop};
use crate::{full, prompt, target};

/// An agent that speaks ACP, and how it is started.
#[derive(Debug)]
pub struct Agent {
    /// Its key (a task's owner): `gemini`, `opencode`…
    pub name: &'static str,
    /// What the Nest calls it.
    pub title: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    /// Added to its process alone, never written to its config.
    env: &'static [(&'static str, &'static str)],
}

/// OpenCode runs every tool unasked by default. Its inline config (`OPENCODE_CONFIG_CONTENT`)
/// outranks the user's and the project's, for this process alone: each write, command and fetch
/// asks, for the default agent too. Checked with OpenCode 1.18.35.
const OPENCODE_ASK: &str = r#"{"permission":{"edit":"ask","bash":"ask","webfetch":"ask","external_directory":"ask","doom_loop":"ask"},"agent":{"build":{"permission":{"edit":"ask","bash":"ask","webfetch":"ask","external_directory":"ask","doom_loop":"ask"}}}}"#;

pub const AGENTS: &[Agent] = &[
    Agent {
        name: "gemini",
        title: "Gemini CLI",
        program: "gemini",
        args: &["--acp"],
        env: &[],
    },
    Agent {
        name: "opencode",
        title: "OpenCode",
        program: "opencode",
        args: &["acp"],
        env: &[("OPENCODE_CONFIG_CONTENT", OPENCODE_ASK)],
    },
    Agent {
        name: "qwen",
        title: "Qwen Code",
        program: "qwen",
        args: &["--acp"],
        env: &[],
    },
    Agent {
        name: "copilot",
        title: "Copilot CLI",
        program: "copilot",
        args: &["--acp", "--stdio"],
        env: &[],
    },
];

/// The agent with this key.
pub fn agent(name: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|a| a.name == name)
}

impl Agent {
    /// On the PATH the agent's terminal would have.
    pub fn installed(&self) -> bool {
        let path = vults_agents::shell_env()
            .into_iter()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v)
            .or_else(|| std::env::var_os("PATH"));
        path.is_some_and(|path| {
            std::env::split_paths(&path).any(|dir| {
                let file = dir.join(self.program);
                file.is_file() || (cfg!(windows) && dir.join(format!("{}.cmd", self.program)).is_file())
            })
        })
    }
}

/// How long the agent has to start, and to answer a call outside a turn.
const CALL_WAIT: Duration = Duration::from_secs(60);
/// How long a cancelled turn has to end before its process is dropped.
const CANCEL_GRACE: Duration = Duration::from_secs(10);

/// What a message from the agent means.
#[derive(Debug, PartialEq)]
enum Msg {
    Response {
        id: u64,
        result: Value,
    },
    Failure {
        id: u64,
        message: String,
    },
    /// The agent asks to run a tool. `ask` is none when the call can't be named whole: a no.
    Permission {
        id: Value,
        ask: Option<Ask>,
        /// `(optionId, kind)`, as the agent offers them.
        options: Vec<(String, String)>,
    },
    /// Any other request of us (a file system or terminal we did not offer): refused.
    Request {
        id: Value,
    },
    Text(String),
    /// A tool call began or moved on: what it acts on (its command, its file) once the agent says,
    /// and its end (`Some(ok)`).
    Tool {
        id: String,
        tool: String,
        detail: Option<String>,
        end: Option<bool>,
    },
    /// The session's cost so far, in US dollars (`usage_update`).
    Cost(f64),
    Other,
}

fn classify(raw: &str) -> Msg {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return Msg::Other;
    };
    let method = v.get("method").and_then(Value::as_str);
    match (v.get("id").filter(|id| !id.is_null()), method) {
        (Some(id), Some("session/request_permission")) => {
            let options = v["params"]["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|o| {
                    Some((
                        o["optionId"].as_str()?.to_string(),
                        o["kind"].as_str()?.to_string(),
                    ))
                })
                .collect();
            Msg::Permission {
                id: id.clone(),
                ask: ask_of(&v["params"]["toolCall"]),
                options,
            }
        }
        (Some(id), Some(_)) => Msg::Request { id: id.clone() },
        (Some(id), None) => {
            let id = id.as_u64().unwrap_or(0);
            match v.get("error") {
                Some(e) => Msg::Failure {
                    id,
                    message: e["message"].as_str().unwrap_or("error").to_string(),
                },
                None => Msg::Response {
                    id,
                    result: v.get("result").cloned().unwrap_or(Value::Null),
                },
            }
        }
        (None, Some("session/update")) => update(&v["params"]["update"]),
        _ => Msg::Other,
    }
}

fn update(u: &Value) -> Msg {
    let text = |k: &str| u[k].as_str().unwrap_or_default().to_string();
    match u["sessionUpdate"].as_str() {
        Some("agent_message_chunk") => match u["content"]["text"].as_str() {
            Some(t) if !t.is_empty() => Msg::Text(t.to_string()),
            _ => Msg::Other,
        },
        Some(kind @ ("tool_call" | "tool_call_update")) => {
            let raw = &u["rawInput"];
            // What it acts on: the command or the file, which come with the call or a moment after
            // (OpenCode's first word is the tool's own name). A command's is its command only: the
            // folder it names first is not what it runs.
            let fields: &[&str] = if u["kind"] == "execute" {
                &["command", "cmd"]
            } else {
                &[
                    "filePath",
                    "filepath",
                    "file_path",
                    "path",
                    "url",
                    "query",
                    "pattern",
                ]
            };
            let located = u["kind"] != "execute";
            let detail = fields
                .iter()
                .find_map(|k| raw[*k].as_str())
                .or_else(|| located.then(|| u["locations"][0]["path"].as_str()).flatten())
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(str::to_string);
            let end = match u["status"].as_str() {
                Some("completed") => Some(true),
                Some("failed") => Some(false),
                _ => None,
            };
            if kind == "tool_call_update" && detail.is_none() && end.is_none() {
                return Msg::Other;
            }
            Msg::Tool {
                id: text("toolCallId"),
                tool: tool_name(u["kind"].as_str().unwrap_or_default(), &text("title")),
                detail,
                end,
            }
        }
        Some("usage_update") if u["cost"]["currency"] == "USD" => u["cost"]["amount"]
            .as_f64()
            .filter(|a| a.is_finite() && *a >= 0.0)
            .map_or(Msg::Other, Msg::Cost),
        _ => Msg::Other,
    }
}

/// A tool's kind in the island's words (Claude Code's tool names, which cards and rules know).
fn tool_name(kind: &str, title: &str) -> String {
    match kind {
        "execute" => "Bash".into(),
        "edit" | "delete" | "move" => "Edit".into(),
        "read" => "Read".into(),
        "search" => "Grep".into(),
        "fetch" => "WebFetch".into(),
        _ if !title.trim().is_empty() => title.trim().to_string(),
        _ => "Tool".into(),
    }
}

/// The card for a permission, named whole: the command, or the files and their changes, or the
/// address. None when it can't be (no command, no file, a move that writes where no card would
/// say): the answer is then no, never a card with less on it than what it allows.
fn ask_of(call: &Value) -> Option<Ask> {
    let id = call["toolCallId"].as_str()?.to_string();
    let raw = &call["rawInput"];
    let field = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| raw[*k].as_str().map(str::trim).filter(|s| !s.is_empty()))
            .map(str::to_string)
    };
    match call["kind"].as_str().unwrap_or_default() {
        "execute" => {
            let command = field(&["command", "cmd"]).or_else(|| {
                raw["command"].as_array().map(|parts| {
                    parts
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
            })?;
            (!command.trim().is_empty()).then_some(())?;
            Some(Ask {
                id,
                tool: "Bash".into(),
                target: target("Bash", &command),
                full: full(&command),
                input: json!({ "command": command, "cwd": field(&["cwd", "workdir"]) }),
            })
        }
        "move" => None,
        kind @ ("edit" | "delete") => {
            let diffs: Vec<Value> = call["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| c["type"] == "diff")
                .filter_map(|c| {
                    Some(json!({
                        "path": c["path"].as_str()?,
                        "oldText": c["oldText"].as_str().unwrap_or_default(),
                        "newText": c["newText"].as_str().unwrap_or_default(),
                    }))
                })
                .collect();
            let mut paths: Vec<String> = diffs
                .iter()
                .filter_map(|d| d["path"].as_str().map(str::to_string))
                .collect();
            if paths.is_empty() {
                paths = call["locations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|l| l["path"].as_str().map(str::to_string))
                    .collect();
            }
            if paths.is_empty() {
                paths.extend(field(&["filePath", "filepath", "file_path", "path"]));
            }
            if paths.is_empty() || paths.iter().any(|p| p.contains('\n')) {
                return None;
            }
            let detail = paths.join(", ");
            Some(Ask {
                id,
                tool: "Edit".into(),
                target: target("Edit", &detail),
                full: full(&detail),
                input: json!({ "paths": paths, "diffs": diffs, "delete": kind == "delete" }),
            })
        }
        "fetch" => {
            let url = field(&["url", "uri"])?;
            Some(Ask {
                id,
                tool: "WebFetch".into(),
                target: target("WebFetch", &url),
                full: full(&url),
                input: json!({ "url": url }),
            })
        }
        kind => {
            let title = call["title"].as_str().map(str::trim).filter(|t| !t.is_empty())?;
            let tool = tool_name(kind, title);
            Some(Ask {
                id,
                target: target(&tool, title),
                full: full(title),
                input: json!({ "title": title, "input": raw }),
                tool,
            })
        }
    }
}

/// The agent's option for a yes or a no: only ever its "once" ones. A yes it offers only as
/// "always" is not given (cancelled), nor is a no it has no option for.
fn outcome(options: &[(String, String)], allow: bool) -> Value {
    let want = if allow { "allow_once" } else { "reject_once" };
    match options.iter().find(|(_, kind)| kind == want) {
        Some((id, _)) => json!({ "outcome": { "outcome": "selected", "optionId": id } }),
        None => json!({ "outcome": { "outcome": "cancelled" } }),
    }
}

/// A prompt's `usage` (`inputTokens` leaves the cached ones out: added back, as every other engine
/// counts them).
fn usage_of(result: &Value) -> Option<Usage> {
    let u = &result["usage"];
    let n = |k: &str| u[k].as_u64().unwrap_or(0);
    let (read, write) = (n("cachedReadTokens"), n("cachedWriteTokens"));
    let usage = Usage {
        input: n("inputTokens") + read + write,
        output: n("outputTokens") + n("thoughtTokens"),
        cache_read: read,
        cache_write: write,
        ..Usage::default()
    };
    (!usage.is_empty()).then_some(usage)
}

/// The turn's own share of the session's running cost, as `claude::share` reckons it: nothing
/// known before it on a session this driver resumed; a total below the last is a new count.
fn cost_share(total: f64, seen: Option<f64>, fresh: bool) -> Option<f64> {
    match seen {
        Some(s) if total < s => Some(total),
        Some(s) => Some(total - s),
        None if fresh => Some(total),
        None => None,
    }
}

#[derive(Debug)]
struct Conn {
    _child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    next_id: u64,
    /// The agent can resume a session (`loadSession`).
    load: bool,
}

impl Conn {
    async fn spawn(agent: &Agent, spec: &Spec) -> Result<Self, String> {
        let mut cmd = Command::new(agent.program);
        // The folders and PATH the user's terminal gives it, when the app was started without.
        cmd.envs(vults_agents::shell_env());
        cmd.args(agent.args);
        cmd.envs(agent.env.iter().copied());
        cmd.envs(spec.env.iter().map(|(k, v)| (k, v)));
        // It asks on this channel: its own hooks ask nothing (two cards for one call).
        cmd.env(vults_protocol::RUN_ENV, "1");
        let mut child = crate::dies_with_app(&mut cmd)
            .current_dir(&spec.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Never read: a full pipe would stall the agent.
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => format!("{} isn't installed.", agent.title),
                _ => format!("Can't start {}: {e}", agent.title),
            })?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let mut conn = Self {
            _child: child,
            stdin,
            lines: BufReader::new(stdout).lines(),
            next_id: 1,
            load: false,
        };
        let init = conn
            .call(
                "initialize",
                json!({
                    "protocolVersion": 1,
                    // No file system and no terminal of ours: the agent's own tools, each asked.
                    "clientCapabilities": { "fs": { "readTextFile": false, "writeTextFile": false }, "terminal": false },
                    "clientInfo": { "name": "vults", "title": "Vults", "version": env!("CARGO_PKG_VERSION") },
                }),
            )
            .await?;
        conn.load = init["agentCapabilities"]["loadSession"] == true;
        Ok(conn)
    }

    async fn write(&mut self, msg: &Value) -> Result<(), String> {
        let mut line = msg.to_string();
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        self.stdin.flush().await.map_err(|e| e.to_string())
    }

    async fn send(&mut self, method: &str, params: Value) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await?;
        Ok(id)
    }

    async fn refuse(&mut self, id: &Value) -> Result<(), String> {
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "not offered by Vults" } }))
            .await
    }

    async fn answer(&mut self, id: &Value, outcome: Value) -> Result<(), String> {
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "result": outcome }))
            .await
    }

    async fn next(&mut self) -> Result<String, String> {
        self.lines
            .next_line()
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "the agent stopped".to_string())
    }

    /// A call outside a turn: what streams meanwhile (a resumed session's history) is not news,
    /// and nothing is asked then.
    async fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.send(method, params).await?;
        let reply = async {
            loop {
                let line = self.next().await?;
                match classify(&line) {
                    Msg::Response { id: got, result } if got == id => return Ok(result),
                    Msg::Failure { id: got, message } if got == id => return Err(message),
                    Msg::Permission { id: req, .. } => {
                        self.answer(&req, json!({ "outcome": { "outcome": "cancelled" } }))
                            .await?;
                    }
                    Msg::Request { id: req } => self.refuse(&req).await?,
                    _ => {}
                }
            }
        };
        tokio::time::timeout(CALL_WAIT, reply)
            .await
            .map_err(|_| format!("{method}: no answer"))?
    }
}

/// A driver for an ACP agent: one process for the conversation, its session resumed by a new
/// process (`session/load`) when the old one is gone.
#[derive(Debug)]
pub struct AcpDriver {
    agent: &'static Agent,
    spec: Spec,
    session: Option<String>,
    conn: Option<Conn>,
    stop: Arc<Stop>,
    in_turn: bool,
    /// The brief went in this session's first prompt (ACP has no system prompt).
    primed: bool,
    /// The session began in this driver: its first cost is all its own.
    fresh: bool,
    /// The session's running cost after the last turn.
    seen_cost: Option<f64>,
}

impl AcpDriver {
    pub fn new(agent: &'static Agent, spec: Spec, session: Option<String>) -> Self {
        Self {
            agent,
            spec,
            primed: session.is_some(),
            session,
            conn: None,
            stop: Arc::default(),
            in_turn: false,
            fresh: false,
            seen_cost: None,
        }
    }

    async fn connect(&mut self) -> Result<(), String> {
        if self.conn.is_some() {
            return Ok(());
        }
        let mut conn = Conn::spawn(self.agent, &self.spec).await?;
        let cwd = self.spec.dir.to_string_lossy().into_owned();
        let resumed = match self.session.clone() {
            Some(id) if conn.load => conn
                .call(
                    "session/load",
                    json!({ "sessionId": id, "cwd": cwd, "mcpServers": [] }),
                )
                .await
                .map_err(|e| format!("{} can't resume its session: {e}", self.agent.title))
                .map(|_| ()),
            Some(_) => Err(format!("{} can't resume a session.", self.agent.title)),
            None => {
                let made = conn
                    .call("session/new", json!({ "cwd": cwd, "mcpServers": [] }))
                    .await
                    .map_err(|e| format!("{}: {e}", self.agent.title))?;
                let id = made["sessionId"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or("no session id")?;
                self.session = Some(id.to_string());
                self.primed = false;
                self.fresh = true;
                Ok(())
            }
        };
        resumed?;
        if let (Some(model), Some(id)) = (self.spec.model.clone(), self.session.clone()) {
            // Not every agent takes it (an unstable method): its own default then.
            let _ = conn
                .call("session/set_model", json!({ "sessionId": id, "modelId": model }))
                .await;
        }
        self.conn = Some(conn);
        Ok(())
    }

    async fn turn(&mut self, input: &Input, sink: &dyn Sink) -> Result<EndReason, String> {
        self.connect().await?;
        let session = self.session.clone().ok_or("no session")?;
        let instructions = crate::instructions(&self.spec);
        let system = (!self.primed && !instructions.trim().is_empty()).then_some(instructions);
        let text = prompt(input, system.as_deref());
        // Held by the turn; put back when it ends well. A turn that fails drops it.
        let mut conn = self.conn.take().ok_or("no agent")?;
        let id = conn
            .send(
                "session/prompt",
                json!({ "sessionId": session, "prompt": [{ "type": "text", "text": text }] }),
            )
            .await?;
        self.primed = true;
        let stop = self.stop.clone();
        let mut stopping = false;
        let mut cost: Option<f64> = None;
        let mut deadline: Option<tokio::time::Instant> = None;
        // Tool calls seen, by id: their tool, and whether their step was told.
        let mut tools: std::collections::HashMap<String, (String, bool)> = std::collections::HashMap::new();
        loop {
            let line = tokio::select! {
                biased;
                () = stop.wait(), if !stopping => {
                    stopping = true;
                    conn.write(&json!({ "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": session } }))
                        .await?;
                    deadline = Some(tokio::time::Instant::now() + CANCEL_GRACE);
                    continue;
                }
                () = async {
                    match deadline {
                        Some(d) => tokio::time::sleep_until(d).await,
                        None => std::future::pending().await,
                    }
                } => {
                    // A cancel it never answered: its process goes; the next turn resumes it.
                    return Ok(EndReason::Interrupted);
                }
                line = conn.next() => line?,
            };
            match classify(&line) {
                Msg::Response { id: got, result } if got == id => {
                    let mut usage = usage_of(&result);
                    if let Some(total) = cost {
                        let u = usage.get_or_insert_with(Usage::default);
                        u.cost_usd = cost_share(total, self.seen_cost, self.fresh);
                        u.session_cost_usd = Some(total);
                        self.seen_cost = Some(total);
                        self.fresh = false;
                    }
                    if let Some(u) = usage {
                        sink.event(Event::Usage(u)).await;
                    }
                    self.conn = Some(conn);
                    return Ok(match result["stopReason"].as_str() {
                        Some("cancelled") => EndReason::Interrupted,
                        Some("refusal") => EndReason::Failed(format!("{} refused.", self.agent.title)),
                        _ if stopping => EndReason::Interrupted,
                        _ => EndReason::Done,
                    });
                }
                Msg::Failure { id: got, message } if got == id => return Err(message),
                Msg::Permission {
                    id: req,
                    ask,
                    options,
                } => {
                    let allow = match ask {
                        Some(ask) if !stopping => tokio::select! {
                            biased;
                            () = stop.wait() => false,
                            a = sink.ask(ask) => a.allows(),
                        },
                        _ => false,
                    };
                    let answer = if stopping {
                        json!({ "outcome": { "outcome": "cancelled" } })
                    } else {
                        outcome(&options, allow)
                    };
                    conn.answer(&req, answer).await?;
                }
                Msg::Request { id: req } => conn.refuse(&req).await?,
                Msg::Text(t) => sink.event(Event::Text(t)).await,
                Msg::Tool {
                    id,
                    tool,
                    detail,
                    end,
                } => {
                    let (tool, told) = tools.entry(id.clone()).or_insert((tool, false)).clone();
                    // Its step is told once it says what it acts on, or when it ends without.
                    if !told && (detail.is_some() || end.is_some()) {
                        let detail = detail.unwrap_or_default();
                        sink.event(Event::ToolStart {
                            id: id.clone(),
                            target: target(&tool, &detail),
                            tool: tool.clone(),
                            detail,
                        })
                        .await;
                        tools.insert(id.clone(), (tool, true));
                    }
                    if let Some(ok) = end {
                        sink.event(Event::ToolEnd { id: id.clone(), ok }).await;
                        tools.remove(&id);
                    }
                }
                Msg::Cost(total) => cost = Some(total),
                _ => {}
            }
        }
    }
}

impl AgentDriver for AcpDriver {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn interrupter(&self) -> Arc<dyn Interrupter> {
        self.stop.clone()
    }

    fn start(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(self.connect())
    }

    fn send<'a>(&'a mut self, input: &'a Input, sink: &'a dyn Sink) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let _release = Release(self.stop.clone());
            // A turn dropped midway leaves its process maybe still busy with it: not reused.
            if std::mem::replace(&mut self.in_turn, true) {
                self.conn = None;
            }
            let result = self.turn(input, sink).await;
            self.in_turn = false;
            if result.is_err() {
                // A dead or confused process is started again next turn, resuming the session.
                self.conn = None;
            }
            let end = match &result {
                Ok(end) => end.clone(),
                Err(e) => EndReason::Failed(e.clone()),
            };
            sink.event(Event::TurnEnd(end)).await;
            result.map(|_| ())
        })
    }

    fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }
}

/// Whether `path` is the folder's own: inside it, never in its `.git`. A task's worker edits
/// nothing else without a card a human would have to read twice.
pub fn inside(dir: &Path, path: &str) -> bool {
    let path = Path::new(path);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        dir.join(path)
    };
    let mut clean = std::path::PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                if !clean.pop() {
                    return false;
                }
            }
            std::path::Component::CurDir => {}
            other => clean.push(other),
        }
    }
    clean.starts_with(dir) && !clean.starts_with(dir.join(".git"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    // Recorded from OpenCode 1.18.35 over ACP, with its permissions set to ask (trimmed).
    const OPENCODE_EDIT: &str = r#"{"jsonrpc":"2.0","id":0,"method":"session/request_permission","params":{"sessionId":"ses_1","toolCall":{"toolCallId":"call_a","title":"/w/hello.txt","kind":"edit","status":"pending","locations":[{"path":"/w/hello.txt"}],"rawInput":{"filepath":"/w/hello.txt","diff":"@@ -0,0 +1 @@\n+hi"},"content":[{"type":"diff","path":"/w/hello.txt","oldText":"","newText":"hi"}]},"options":[{"optionId":"once","kind":"allow_once","name":"Allow once"},{"optionId":"always","kind":"allow_always","name":"Always allow"},{"optionId":"reject","kind":"reject_once","name":"Reject"}]}}"#;
    const OPENCODE_BASH: &str = r#"{"jsonrpc":"2.0","id":2,"method":"session/request_permission","params":{"sessionId":"ses_1","toolCall":{"toolCallId":"call_b","title":"cat hello.txt","kind":"execute","status":"pending","locations":[],"rawInput":{"command":"cat hello.txt"}},"options":[{"optionId":"once","kind":"allow_once","name":"Allow once"},{"optionId":"always","kind":"allow_always","name":"Always allow"},{"optionId":"reject","kind":"reject_once","name":"Reject"}]}}"#;

    #[test]
    fn a_permission_is_named_whole() {
        let Msg::Permission {
            ask: Some(ask),
            options,
            ..
        } = classify(OPENCODE_EDIT)
        else {
            panic!("not a permission");
        };
        assert_eq!(
            (ask.tool.as_str(), ask.target.as_str()),
            ("Edit", "Edit · /w/hello.txt")
        );
        assert_eq!(ask.input["diffs"][0]["newText"], "hi");
        assert_eq!(options.len(), 3);
        let Msg::Permission { ask: Some(ask), .. } = classify(OPENCODE_BASH) else {
            panic!("not a permission");
        };
        assert_eq!(
            (ask.tool.as_str(), ask.input["command"].as_str()),
            ("Bash", Some("cat hello.txt"))
        );
        // A command it doesn't say, or a move, is a no: no card.
        let blind = r#"{"jsonrpc":"2.0","id":3,"method":"session/request_permission","params":{"toolCall":{"toolCallId":"c","kind":"execute","rawInput":{}},"options":[]}}"#;
        assert!(matches!(classify(blind), Msg::Permission { ask: None, .. }));
        let moving = r#"{"jsonrpc":"2.0","id":4,"method":"session/request_permission","params":{"toolCall":{"toolCallId":"c","kind":"move","locations":[{"path":"/w/a"}]},"options":[]}}"#;
        assert!(matches!(classify(moving), Msg::Permission { ask: None, .. }));
    }

    #[test]
    fn an_answer_is_only_ever_for_this_once() {
        let options = vec![
            ("once".to_string(), "allow_once".to_string()),
            ("always".to_string(), "allow_always".to_string()),
            ("reject".to_string(), "reject_once".to_string()),
        ];
        assert_eq!(outcome(&options, true)["outcome"]["optionId"], "once");
        assert_eq!(outcome(&options, false)["outcome"]["optionId"], "reject");
        // Offered only as "always": not given.
        let always = vec![("always".to_string(), "allow_always".to_string())];
        assert_eq!(outcome(&always, true)["outcome"]["outcome"], "cancelled");
        assert_eq!(outcome(&always, false)["outcome"]["outcome"], "cancelled");
    }

    #[test]
    fn updates_become_the_turns_events() {
        let chunk = r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"done"}}}}"#;
        assert_eq!(classify(chunk), Msg::Text("done".into()));
        // OpenCode names the call first, and what it runs a moment later.
        let call = r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"tool_call","toolCallId":"t1","title":"bash","kind":"execute","status":"pending","rawInput":{}}}}"#;
        assert!(
            matches!(classify(call), Msg::Tool { ref tool, detail: None, end: None, .. } if tool == "Bash")
        );
        let going = r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"tool_call_update","toolCallId":"t1","status":"in_progress","kind":"execute","title":"cat hello.txt","rawInput":{"command":"cat hello.txt"}}}}"#;
        assert!(matches!(classify(going), Msg::Tool { detail: Some(ref d), .. } if d == "cat hello.txt"));
        let done = r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"tool_call_update","toolCallId":"t1","status":"failed"}}}"#;
        assert!(matches!(classify(done), Msg::Tool { end: Some(false), .. }));
        let cost = r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"usage_update","used":11766,"size":200000,"cost":{"amount":0.25,"currency":"USD"}}}}"#;
        assert_eq!(classify(cost), Msg::Cost(0.25));
        // Anything else asked of us (we offered no file system) is refused.
        let fs = r#"{"jsonrpc":"2.0","id":1,"method":"fs/write_text_file","params":{"path":"/w/a","content":"x"}}"#;
        assert!(matches!(classify(fs), Msg::Request { .. }));
        assert_eq!(classify("not json"), Msg::Other);
    }

    #[test]
    fn a_prompts_usage_counts_the_cache_in() {
        let result = json!({ "stopReason": "end_turn", "usage": { "inputTokens": 118, "outputTokens": 2, "totalTokens": 11768, "cachedReadTokens": 11648 } });
        let u = usage_of(&result).unwrap();
        assert_eq!((u.input, u.output, u.cache_read), (11_766, 2, 11_648));
        assert_eq!(cost_share(0.5, None, true), Some(0.5));
        assert_eq!(cost_share(0.8, Some(0.5), false), Some(0.8 - 0.5));
        assert_eq!(cost_share(0.1, Some(0.5), false), Some(0.1));
        assert_eq!(cost_share(0.8, None, false), None);
    }

    #[test]
    fn only_the_folders_own_files_are_inside() {
        let dir = Path::new("/data/worktrees/site-task-3");
        assert!(inside(dir, "/data/worktrees/site-task-3/src/a.rs"));
        assert!(inside(dir, "src/a.rs"));
        assert!(!inside(dir, "/data/worktrees/site-task-3/.git/hooks/pre-commit"));
        assert!(!inside(dir, "/data/worktrees/site-task-3/../site/a.rs"));
        assert!(!inside(dir, "/home/me/site/a.rs"));
        assert!(!inside(dir, "../../etc/passwd"));
    }

    #[test]
    fn every_agent_has_a_known_key() {
        for a in AGENTS {
            assert_eq!(agent(a.name).map(|x| x.title), Some(a.title));
        }
        assert!(agent("claude").is_none());
    }
}
