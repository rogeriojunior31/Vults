//! Chat from the island through the CLIs the user already logged into (`claude`, `codex`): their
//! subscription, their credentials, which we never read. The conversation continues with the
//! CLI's own resume id. Without a CLI, the user may give an Anthropic API key instead (kept in the
//! OS keyring); that chat only talks, it has no tools.
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
with line breaks, no markdown."
    };
}

mod api;
mod claude;
mod codex;
mod codex_server;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

/// A turn thinking this long (not counting time spent waiting for the user) is stuck.
const TURN_TIMEOUT: Duration = Duration::from_secs(600);
/// A permission nobody answers is a no.
const DECISION_TIMEOUT: Duration = Duration::from_secs(600);

const PERSONA: &str = concat!(
    persona!(),
    " Just use your tools: the app shows the user every command and edit and asks them to approve \
it, so never ask for permission in your reply. If they say no, accept it and suggest another way."
);

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Codex,
    /// Claude through the Messages API with the user's key.
    #[serde(rename = "api")]
    ClaudeApi,
}

#[derive(Debug, Clone, Default)]
pub struct Turn {
    pub text: String,
    /// Files dropped on the island, already copied into our inbox.
    pub files: Vec<PathBuf>,
}

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
    },
    Done,
    Error {
        message: String,
    },
}

/// Gets the user's answer to a permission. Only a human's click may resolve it with `true`.
pub trait Approver: Send + Sync {
    fn wait(&self, id: &str) -> oneshot::Receiver<bool>;
}

/// Asks, and waits for the answer; silence or a dropped answer is a no.
async fn ask(
    approver: &dyn Approver,
    out: &mpsc::Sender<Delta>,
    id: String,
    tool: String,
    target: String,
) -> bool {
    let answer = approver.wait(&id);
    let _ = out.send(Delta::Permission { id, tool, target }).await;
    matches!(tokio::time::timeout(DECISION_TIMEOUT, answer).await, Ok(Ok(true)))
}

#[derive(Debug)]
pub struct Chat {
    provider: Provider,
    work_dir: PathBuf,
    /// The CLI's id for this conversation, after the first turn.
    session: Option<String>,
    /// Codex: the long-lived app-server, when this Codex has one.
    server: Option<codex_server::AppServer>,
    /// API: the conversation so far, replayed on every turn.
    history: Vec<serde_json::Value>,
}

impl Chat {
    pub fn new(provider: Provider, work_dir: PathBuf) -> Self {
        Self {
            provider,
            work_dir,
            session: None,
            server: None,
            history: Vec::new(),
        }
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn folder(&self) -> &Path {
        &self.work_dir
    }

    /// The folder can only change before the first turn: a conversation lives in one place.
    pub fn set_folder(&mut self, dir: PathBuf) -> bool {
        if self.session.is_some() || !self.history.is_empty() || !dir.is_dir() {
            return false;
        }
        self.work_dir = dir;
        true
    }

    /// Starts a new conversation (the next turn has no memory of this one).
    pub fn reset(&mut self) {
        self.session = None;
        self.server = None;
        self.history.clear();
    }

    /// Runs one turn, streaming deltas to `out`. Ends with `Done` or `Error`.
    pub async fn send(&mut self, turn: Turn, out: mpsc::Sender<Delta>, approver: Arc<dyn Approver>) {
        let result = tokio::time::timeout(TURN_TIMEOUT, self.run(&turn, &out, approver.as_ref())).await;
        let last = match result {
            Ok(Ok(())) => Delta::Done,
            Ok(Err(message)) => Delta::Error { message },
            Err(_) => Delta::Error {
                message: format!("{} took too long to answer.", self.name()),
            },
        };
        let _ = out.send(last).await;
    }

    fn name(&self) -> &'static str {
        match self.provider {
            Provider::Claude => "Claude",
            Provider::Codex => "Codex",
            Provider::ClaudeApi => "Claude",
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
            Provider::Claude => {
                let session =
                    claude::turn(&self.work_dir, self.session.as_deref(), turn, out, approver).await?;
                self.session = Some(session);
                Ok(())
            }
            Provider::Codex => match self.run_app_server(turn, out, approver).await {
                Some(result) => result,
                // Without an app-server nothing can ask the user, so this fallback stays read-only.
                None => codex::turn(&self.work_dir, &mut self.session, turn, out).await,
            },
            Provider::ClaudeApi => {
                let key = api_key().await?;
                api::turn(&key, &mut self.history, turn, out).await
            }
        }
    }

    /// Codex through its app-server. `None` when the app-server can't be used here (older Codex,
    /// failed start): the caller falls back to one `codex exec` per turn.
    async fn run_app_server(
        &mut self,
        turn: &Turn,
        out: &mpsc::Sender<Delta>,
        approver: &dyn Approver,
    ) -> Option<Result<(), String>> {
        if self.server.is_none() {
            match codex_server::AppServer::start(&self.work_dir, self.session.as_deref()).await {
                Ok(server) => {
                    self.session = Some(server.thread.clone());
                    self.server = Some(server);
                }
                Err(_) => return None,
            }
        }
        let server = self.server.as_mut()?;
        let result = server.turn(turn, out, approver).await;
        if result.is_err() {
            // A dead or confused server is restarted next turn and resumes the same thread.
            self.server = None;
        }
        Some(result.map_err(|e| format!("Codex: {e}")))
    }
}

/// Read from the keyring for each turn, so removing the key in Settings takes effect at once.
async fn api_key() -> Result<String, String> {
    use vultures_ai_secrets::{Secret, get};
    match tokio::task::spawn_blocking(|| get(Secret::AnthropicApiKey)).await {
        Ok(Ok(Some(key))) => Ok(key),
        Ok(Ok(None)) => Err("Add an Anthropic API key in Settings to use this chat.".into()),
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
        p.push_str(PERSONA);
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
        chat.session = Some("s".into());
        assert!(!chat.set_folder(PathBuf::from("/")));
        assert_eq!(chat.folder(), tmp);
    }

    struct Never;
    impl Approver for Never {
        fn wait(&self, _: &str) -> oneshot::Receiver<bool> {
            oneshot::channel().1
        }
    }

    #[tokio::test]
    async fn a_dropped_answer_is_a_no() {
        let (tx, mut rx) = mpsc::channel(4);
        assert!(!ask(&Never, &tx, "1".into(), "Bash".into(), "Bash · rm".into()).await);
        assert_eq!(
            rx.recv().await,
            Some(Delta::Permission {
                id: "1".into(),
                tool: "Bash".into(),
                target: "Bash · rm".into()
            })
        );
    }
}
