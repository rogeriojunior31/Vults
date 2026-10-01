//! Chat from the island through the CLIs the user already logged into (`claude`, `codex`): their
//! subscription, their credentials, which we never read. One process per turn; the conversation
//! continues with the CLI's own resume id.
//!
//! Every turn is read-only (the model may read a dropped file, never change anything), runs in
//! an empty folder of ours so no project's instructions leak in, and ignores the user's hooks
//! and settings, so a chat never shows up on the island as an agent session.

mod claude;
mod codex;
mod codex_server;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

/// A turn thinking this long is stuck.
const TURN_TIMEOUT: Duration = Duration::from_secs(240);

const PERSONA: &str = "You are Zeca, a friendly vulture who lives at the top of the user's screen and \
answers in a small chat bubble. Answer in the user's language. Be concise unless the task needs detail. \
Plain text with line breaks, no markdown. You may read files the user points to; never modify anything.";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Codex,
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
    Done,
    Error {
        message: String,
    },
}

/// One line of a CLI's JSON stream, already understood.
#[derive(Debug, PartialEq)]
enum Line {
    Session(String),
    Text(String),
    Failed(String),
    Other,
}

#[derive(Debug)]
pub struct Chat {
    provider: Provider,
    work_dir: PathBuf,
    /// The CLI's id for this conversation, after the first turn.
    session: Option<String>,
    /// Codex: the long-lived app-server, when this Codex has one.
    server: Option<codex_server::AppServer>,
}

impl Chat {
    pub fn new(provider: Provider, work_dir: PathBuf) -> Self {
        Self {
            provider,
            work_dir,
            session: None,
            server: None,
        }
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// Starts a new conversation (the next turn has no memory of this one).
    pub fn reset(&mut self) {
        self.session = None;
        self.server = None;
    }

    /// Runs one turn, streaming deltas to `out`. Ends with `Done` or `Error`.
    pub async fn send(&mut self, turn: Turn, out: mpsc::Sender<Delta>) {
        let result = tokio::time::timeout(TURN_TIMEOUT, self.run(&turn, &out)).await;
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
        }
    }

    async fn run(&mut self, turn: &Turn, out: &mpsc::Sender<Delta>) -> Result<(), String> {
        std::fs::create_dir_all(&self.work_dir).map_err(|e| format!("Can't prepare the chat folder: {e}"))?;
        if self.provider == Provider::Codex
            && let Some(result) = self.run_app_server(turn, out).await
        {
            return result;
        }
        let mut cmd = match self.provider {
            Provider::Claude => claude::command(turn, self.session.as_deref()),
            Provider::Codex => codex::command(turn, self.session.as_deref()),
        };
        cmd.current_dir(&self.work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => format!("The {} CLI isn't installed.", self.name()),
            _ => format!("Can't start {}: {e}", self.name()),
        })?;

        let stdout = child.stdout.take().ok_or("no output")?;
        let mut lines = BufReader::new(stdout).lines();
        let mut said_anything = false;
        let mut failure = None;
        while let Ok(Some(raw)) = lines.next_line().await {
            let line = match self.provider {
                Provider::Claude => claude::parse(&raw),
                Provider::Codex => codex::parse(&raw),
            };
            match line {
                Line::Session(id) => self.session = Some(id),
                Line::Text(text) => {
                    said_anything = true;
                    let _ = out.send(Delta::Text { text }).await;
                }
                Line::Failed(message) => failure = Some(message),
                Line::Other => {}
            }
        }
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if let Some(message) = failure {
            return Err(format!("{}: {message}", self.name()));
        }
        if said_anything {
            return Ok(());
        }
        let mut detail = String::new();
        if let Some(err) = child.stderr.take() {
            let mut err = BufReader::new(err).lines();
            while let Ok(Some(l)) = err.next_line().await {
                if !l.trim().is_empty() && !l.contains("Reading additional input") {
                    detail = l;
                }
            }
        }
        if !status.success() && detail.to_lowercase().contains("login") {
            return Err(format!(
                "{} isn't logged in. Log in from a terminal first.",
                self.name()
            ));
        }
        Err(if detail.is_empty() {
            format!("{} returned no answer.", self.name())
        } else {
            detail
        })
    }
}

impl Chat {
    /// Codex through its app-server, streaming. `None` when the app-server can't be used here
    /// (older Codex, failed start): the caller falls back to one `codex exec` per turn.
    async fn run_app_server(&mut self, turn: &Turn, out: &mpsc::Sender<Delta>) -> Option<Result<(), String>> {
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
        let result = server.turn(turn, out).await;
        if result.is_err() {
            // A dead or confused server is restarted next turn and resumes the same thread.
            self.server = None;
        }
        Some(result.map_err(|e| format!("Codex: {e}")))
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

/// The first turn carries the persona and the attachments; later ones just the message.
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

/// Directories the model may read: those of the dropped files.
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
}
