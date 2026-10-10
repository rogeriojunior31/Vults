//! Chat from the island through the CLIs the user already logged into (`claude`, `codex`): their
//! subscription, their credentials, which we never read. The conversation continues with the
//! CLI's own resume id. Without a CLI, the user may give an API key instead (Anthropic, OpenAI,
//! OpenRouter… kept in the OS keyring) or use a model running locally; that chat only talks, it
//! has no tools.
//!
//! The chat works in a folder (the focused session's project, or an empty folder of ours). It may
//! read freely; every command and every edit stops the turn and asks the user through
//! [`Approver`], and nothing runs without a yes. It ignores the user's hooks, settings and MCP
//! servers, so a chat never shows up on the island as an agent session.

/// Who Zeca is, for every provider; each one adds what it can do.
macro_rules! persona {
    () => {
        "You are Zeca, a friendly vulture who lives at the top of the user's screen and answers in a \
small chat bubble. Answer in the user's language. Be concise unless the task needs detail. Plain text \
with line breaks, no markdown. The app turns what the user says into text for you: never run a \
program to speak aloud or to record audio yourself."
    };
}

mod api;
mod claude;
mod codex;
mod codex_server;
pub mod drivers;
mod openai;
pub mod providers;
pub mod user;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use vults_agents::driver::{AgentDriver, Ask as AgentAsk, BoxFuture, Event, Sink};

/// Server-sent events out of a byte stream. A chunk can end in the middle of a character (an
/// accent, an emoji): bytes wait here until their event is whole, and only then become text.
#[derive(Debug, Default)]
pub(crate) struct SseEvents(Vec<u8>);

impl SseEvents {
    /// The events this chunk completes, `\r` dropped.
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.0.extend(chunk.iter().filter(|&&b| b != b'\r'));
        let mut events = Vec::new();
        while let Some(end) = self.0.windows(2).position(|w| w == b"\n\n") {
            let event: Vec<u8> = self.0.drain(..end + 2).collect();
            events.push(String::from_utf8_lossy(&event).into_owned());
        }
        events
    }
}

/// The CLI dies with the app. `kill_on_drop` alone only runs when a turn is dropped, never when
/// the app is killed or quits through `exit`: a turn waiting on a permission then ran on forever.
pub(crate) fn dies_with_app(cmd: &mut tokio::process::Command) -> &mut tokio::process::Command {
    #[cfg(target_os = "linux")]
    // SAFETY: the hook only calls prctl, which is async-signal-safe and touches no parent memory.
    unsafe {
        cmd.pre_exec(|| {
            // Sent when the spawning thread exits: turns run on the runtime's workers, which live
            // as long as the app (never spawn a CLI from a blocking-pool thread).
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    cmd.kill_on_drop(true)
}

/// A turn thinking this long (not counting time spent waiting for the user) is stuck.
const TURN_TIMEOUT: Duration = Duration::from_secs(600);
/// A permission nobody answers is a no.
const DECISION_TIMEOUT: Duration = Duration::from_secs(600);
/// A usage read that takes longer is an app-server in trouble; the next poll tries again.
const USAGE_TIMEOUT: Duration = Duration::from_secs(20);

/// The Codex subscription's usage windows, from the `codex` the user logged into. `dir` is only
/// where the server starts: the read touches no project.
pub async fn codex_usage(dir: &Path) -> Result<Vec<vults_agents::usage::Window>, String> {
    let read = tokio::time::timeout(USAGE_TIMEOUT, codex_server::AppServer::rate_limits(dir))
        .await
        .map_err(|_| "Codex did not answer".to_string())??;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    Ok(vults_agents::usage::codex(&read, now))
}

const PERSONA: &str = concat!(
    persona!(),
    " Just use your tools: the app shows the user every command and edit and asks them to approve \
it, so never ask for permission in your reply. If they say no, accept it and suggest another way."
);

/// Every text the chat sends a model, exactly, for the eval snapshots (`crates/evals`): the
/// personas, the name added to them, the prompt around a turn, the files' wrappers and the answer
/// to a denied tool. A change to any of them shows up there and has to be accepted on purpose.
pub fn prompts() -> Vec<(&'static str, String)> {
    let dropped = Turn {
        text: String::new(),
        files: vec![PathBuf::from("/home/me/notes/plan.md")],
    };
    let asked = Turn {
        text: "What changed in this file?".into(),
        files: vec![PathBuf::from("/home/me/notes/plan.md")],
    };
    let denied = claude::decision("r", false, &serde_json::json!({}));
    vec![
        ("cli-persona", PERSONA.to_string()),
        ("api-persona", api::PERSONA.to_string()),
        ("with-name", with_name("<persona>", Some("Ana"))),
        (
            "first-turn-with-persona",
            prompt(&asked, true, true).replace(&personal(PERSONA), PERSONA),
        ),
        ("files-only", prompt(&dropped, false, false)),
        (
            "denied",
            denied["response"]["response"]["message"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        ),
        ("file-text", api::file_text("plan.md", "Step one.")),
        ("file-unreadable", api::file_unreadable("plan.md")),
        ("file-foreign", api::file_foreign("photo.heic")),
    ]
}

/// A persona, with the user's first name when their account has one.
pub(crate) fn personal(persona: &str) -> String {
    with_name(persona, user::first_name().as_deref())
}

fn with_name(persona: &str, name: Option<&str>) -> String {
    match name {
        Some(name) => format!("{persona} The user's first name is {name}."),
        None => persona.to_string(),
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Codex,
    /// A model through a provider's API (see [`providers`]), with the user's key.
    #[serde(rename = "api")]
    ClaudeApi,
}

/// One chat turn: what the user typed and the files they dropped (already in our inbox).
pub type Turn = vults_agents::driver::Input;
pub use vults_agents::driver::Usage;

/// What the UI receives while a turn runs.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Delta {
    /// More reply text, to append.
    Text {
        text: String,
    },
    /// The model wants to run or change something: the UI asks, then answers via the approver.
    Permission {
        id: String,
        tool: String,
        target: String,
        #[serde(flatten)]
        detail: Detail,
    },
    Done,
    /// What the turn cost, as its engine reported it; sent before `Done`, for the app's traces
    /// (never shown in the chat: it can't even be serialized for the view by mistake).
    #[serde(skip_serializing)]
    Usage(Usage),
    /// The user stopped the turn.
    Stopped,
    Error {
        message: String,
    },
}

/// What a permission card shows besides the target, as on the island's own cards: the model's
/// words for the action, the whole command when the target cut it, the lines an edit changes.
#[derive(Serialize, Debug, Clone, PartialEq, Default)]
pub struct Detail {
    pub description: Option<String>,
    pub full: Option<String>,
    pub added: u32,
    pub removed: u32,
}

impl Detail {
    /// From a tool call's input, the way the island reads an agent's.
    pub fn of(tool: &str, input: &serde_json::Value) -> Self {
        let a = vults_agents::ask(tool, input);
        Self {
            description: a.description,
            full: a.full,
            added: a.added,
            removed: a.removed,
        }
    }
}

/// Gets the user's answer to a permission. Only a human's click may resolve it with `true`.
pub trait Approver: Send + Sync {
    /// `tool` and `target` are what the card shows, for the app's audit log.
    fn wait(&self, id: &str, tool: &str, target: &str) -> oneshot::Receiver<bool>;
}

/// Asks, and waits for the answer; silence or a dropped answer is a no.
/// The chat's side of a driver's [`Sink`]: what the agent says goes to the chat, and its tool
/// requests become cards the approver answers.
struct TurnSink<'a> {
    out: &'a mpsc::Sender<Delta>,
    approver: &'a dyn Approver,
}

impl Sink for TurnSink<'_> {
    fn event(&self, event: Event) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            let delta = match event {
                Event::Text(text) => Delta::Text { text },
                Event::Usage(usage) => Delta::Usage(usage),
            };
            let _ = self.out.send(delta).await;
        })
    }

    fn ask(&self, a: AgentAsk) -> BoxFuture<'_, bool> {
        Box::pin(async move {
            let detail = if a.input.is_null() {
                Detail::default()
            } else {
                Detail::of(&a.tool, &a.input)
            };
            ask(self.approver, self.out, a.id, a.tool, a.target, detail).await
        })
    }
}

async fn ask(
    approver: &dyn Approver,
    out: &mpsc::Sender<Delta>,
    id: String,
    tool: String,
    target: String,
    detail: Detail,
) -> bool {
    let answer = approver.wait(&id, &tool, &target);
    let _ = out
        .send(Delta::Permission {
            id,
            tool,
            target,
            detail,
        })
        .await;
    matches!(tokio::time::timeout(DECISION_TIMEOUT, answer).await, Ok(Ok(true)))
}

#[derive(Debug)]
pub struct Chat {
    provider: Provider,
    work_dir: PathBuf,
    /// Claude Code or Codex, made on the first turn; it keeps the conversation going.
    driver: Option<Box<dyn AgentDriver>>,
    /// API: the conversation so far, replayed on every turn.
    history: Vec<serde_json::Value>,
    /// API: where the conversation goes, and with which model.
    api: (&'static providers::Provider, String),
}

impl Chat {
    pub fn new(provider: Provider, work_dir: PathBuf) -> Self {
        Self {
            provider,
            work_dir,
            driver: None,
            history: Vec::new(),
            api: (
                &providers::PROVIDERS[0],
                providers::PROVIDERS[0].default_model.to_string(),
            ),
        }
    }

    /// The API provider and model for the next turns. Another provider starts a new
    /// conversation: a history is written in its provider's wire format.
    pub fn set_api(&mut self, provider: &'static providers::Provider, model: String) {
        if provider.id != self.api.0.id {
            self.history.clear();
        }
        self.api = (provider, model);
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// Who runs the turns, for traces (`gen_ai.provider.name`), and the model when this side
    /// chooses it (the CLIs pick their own and report it in their usage).
    pub fn engine(&self) -> (&'static str, Option<String>) {
        match self.provider {
            Provider::Claude => ("claude-code", None),
            Provider::Codex => ("codex", None),
            Provider::ClaudeApi => (self.api.0.id, Some(self.api.1.clone())),
        }
    }

    pub fn folder(&self) -> &Path {
        &self.work_dir
    }

    /// The folder can only change before the first turn: a conversation lives in one place.
    pub fn set_folder(&mut self, dir: PathBuf) -> bool {
        let started = self.driver.as_ref().is_some_and(|d| d.session().is_some());
        if started || !self.history.is_empty() || !dir.is_dir() {
            return false;
        }
        self.work_dir = dir;
        true
    }

    /// Starts a new conversation (the next turn has no memory of this one).
    pub fn reset(&mut self) {
        self.driver = None;
        self.history.clear();
    }

    /// Runs one turn, streaming deltas to `out`. Ends with `Done`, `Error`, or `Stopped` when
    /// `stop` fires first: the turn is dropped, and with it the CLI it was talking to.
    pub async fn send(
        &mut self,
        turn: Turn,
        out: mpsc::Sender<Delta>,
        approver: Arc<dyn Approver>,
        stop: oneshot::Receiver<()>,
    ) {
        let name = self.name();
        let run = tokio::time::timeout(TURN_TIMEOUT, self.run(&turn, &out, approver.as_ref()));
        let last = tokio::select! {
            // A press on Stop wins over whatever the turn was about to say.
            biased;
            Ok(()) = stop => {
                if let Some(d) = self.driver.as_mut() {
                    let _ = d.interrupt().await;
                }
                Delta::Stopped
            }
            result = run => match result {
                Ok(Ok(())) => Delta::Done,
                Ok(Err(message)) => Delta::Error { message },
                Err(_) => Delta::Error {
                    message: format!("{name} took too long to answer."),
                },
            },
        };
        let _ = out.send(last).await;
    }

    fn name(&self) -> &'static str {
        match self.provider {
            Provider::Claude => "Claude",
            Provider::Codex => "Codex",
            Provider::ClaudeApi => self.api.0.label,
        }
    }

    async fn run(
        &mut self,
        turn: &Turn,
        out: &mpsc::Sender<Delta>,
        approver: &dyn Approver,
    ) -> Result<(), String> {
        std::fs::create_dir_all(&self.work_dir).map_err(|e| format!("Can't prepare the chat folder: {e}"))?;
        match self.provider {
            Provider::Claude | Provider::Codex => {
                if self.driver.is_none() {
                    self.driver = drivers::for_cli(self.provider, self.work_dir.clone());
                }
                let Some(driver) = self.driver.as_mut() else {
                    return Err("This chat has no agent to run.".into());
                };
                let sink = TurnSink { out, approver };
                driver.send(turn, &sink).await
            }
            Provider::ClaudeApi => {
                let (provider, model) = (self.api.0, self.api.1.clone());
                if model.is_empty() {
                    return Err(format!("Choose a {} model in Settings → Chat.", provider.label));
                }
                let key = if provider.local {
                    None
                } else {
                    Some(api_key(provider).await?)
                };
                match provider.wire {
                    providers::Wire::Anthropic => {
                        api::turn(
                            provider,
                            key.as_deref().unwrap_or_default(),
                            &model,
                            &mut self.history,
                            turn,
                            out,
                        )
                        .await
                    }
                    providers::Wire::OpenAi => {
                        openai::turn(provider, key.as_deref(), &model, &mut self.history, turn, out).await
                    }
                }
            }
        }
    }
}

/// Read from the keyring for each turn, so removing the key in Settings takes effect at once.
async fn api_key(provider: &'static providers::Provider) -> Result<String, String> {
    match tokio::task::spawn_blocking(|| vults_secrets::get(provider.secret())).await {
        Ok(Ok(Some(key))) => Ok(key),
        Ok(Ok(None)) => Err(format!(
            "Add a {} API key in Settings → Chat to use this chat.",
            provider.label
        )),
        _ => Err("Can't read the API key from the system keyring.".into()),
    }
}

fn is_image(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp")
}

/// The first turn may carry the persona; every turn names the attachments.
fn prompt(turn: &Turn, first: bool, inline_persona: bool) -> String {
    let mut p = String::new();
    if first && inline_persona {
        p.push_str(&personal(PERSONA));
        p.push_str("\n\n");
    }
    for f in &turn.files {
        p.push_str(&format!(
            "The user dropped the file \"{}\". Read it to answer.\n",
            f.display()
        ));
    }
    if !turn.files.is_empty() {
        p.push('\n');
    }
    // Only files dropped, nothing typed: the question is what they are.
    if turn.text.trim().is_empty() && !turn.files.is_empty() {
        p.push_str(api::WHAT_IS_IN_THIS);
    }
    p.push_str(&turn.text);
    p
}

/// Directories the model may read besides its folder: those of the dropped files.
fn file_dirs(turn: &Turn) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = turn
        .files
        .iter()
        .filter_map(|f| f.parent().map(Path::to_path_buf))
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

/// Short text for an approval card: `Bash · cargo test`.
fn target(tool: &str, detail: &str) -> String {
    let detail = detail.trim();
    if detail.is_empty() {
        return tool.to_string();
    }
    let mut cut: String = detail.chars().take(300).collect();
    if cut.len() < detail.len() {
        cut.push('…');
    }
    format!("{tool} · {cut}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_events_wait_for_a_whole_character() {
        let wire = "event: x\r\ndata: {\"text\":\"a\u{e7}\u{e3}o 🦅\"}\r\n\r\ndata: end\n\n".as_bytes();
        let mut events = SseEvents::default();
        // Fed one byte at a time: every cut there is, through the c-cedilla and the emoji too.
        let got: Vec<String> = wire.iter().flat_map(|b| events.push(&[*b])).collect();
        assert_eq!(
            got,
            [
                "event: x\ndata: {\"text\":\"a\u{e7}\u{e3}o 🦅\"}\n\n",
                "data: end\n\n"
            ]
        );
    }

    #[test]
    fn prompts() {
        let turn = Turn {
            text: "what is this?".into(),
            files: vec![PathBuf::from("/inbox/a.rs")],
        };
        let first = prompt(&turn, true, true);
        assert!(first.starts_with(PERSONA));
        assert!(first.contains("\"/inbox/a.rs\""));
        assert!(first.ends_with("what is this?"));
        assert_eq!(
            prompt(
                &Turn {
                    text: "hi".into(),
                    files: vec![]
                },
                false,
                true
            ),
            "hi"
        );
        assert_eq!(file_dirs(&turn), vec![PathBuf::from("/inbox")]);
        assert!(is_image(Path::new("x.PNG")));
    }

    #[test]
    fn the_folder_is_fixed_once_the_conversation_starts() {
        let tmp = std::env::temp_dir();
        let mut chat = Chat::new(Provider::Claude, PathBuf::from("/nonexistent"));
        assert!(!chat.set_folder(PathBuf::from("/nonexistent/either")));
        assert!(chat.set_folder(tmp.clone()));
        chat.driver = Some(Box::new(drivers::ClaudeDriver::resumed(tmp.clone(), "s")));
        assert!(!chat.set_folder(PathBuf::from("/")));
        assert_eq!(chat.folder(), tmp);
    }

    struct Never;
    impl Approver for Never {
        fn wait(&self, _: &str, _: &str, _: &str) -> oneshot::Receiver<bool> {
            oneshot::channel().1
        }
    }

    #[tokio::test]
    async fn stop_ends_the_turn() {
        let (tx, mut rx) = mpsc::channel(8);
        let (stop_tx, stop) = oneshot::channel();
        stop_tx.send(()).unwrap();
        let mut chat = Chat::new(Provider::Codex, std::env::temp_dir());
        chat.send(Turn::default(), tx, Arc::new(Never), stop).await;
        assert_eq!(rx.recv().await, Some(Delta::Stopped));
    }

    #[tokio::test]
    async fn a_dropped_answer_is_a_no() {
        let (tx, mut rx) = mpsc::channel(4);
        assert!(
            !ask(
                &Never,
                &tx,
                "1".into(),
                "Bash".into(),
                "Bash · rm".into(),
                Detail::default()
            )
            .await
        );
        assert_eq!(
            rx.recv().await,
            Some(Delta::Permission {
                id: "1".into(),
                tool: "Bash".into(),
                target: "Bash · rm".into(),
                detail: Detail::default(),
            })
        );
    }
}
