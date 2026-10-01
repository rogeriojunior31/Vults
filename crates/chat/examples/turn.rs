//! Real turns against a CLI, to check streaming, resume and permissions end to end.
//! Usage: cargo run -p vultures-ai-chat --example turn -- claude|codex|api [allow|deny] [folder]
use std::sync::Arc;

use tokio::sync::oneshot;
use vultures_ai_chat::{Approver, Chat, Delta, Provider, Turn};

/// Answers every permission the same way, as a stand-in for the island's card.
struct Fixed(bool);

impl Approver for Fixed {
    fn wait(&self, id: &str) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        println!(
            "\n  [asked {id}: answering {}]",
            if self.0 { "allow" } else { "deny" }
        );
        let _ = tx.send(self.0);
        rx
    }
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let provider = match args.next().as_deref() {
        Some("codex") => Provider::Codex,
        Some("api") => Provider::ClaudeApi,
        _ => Provider::Claude,
    };
    let allow = args.next().as_deref() == Some("allow");
    let dir = args
        .next()
        .map(Into::into)
        .unwrap_or_else(|| std::env::temp_dir().join("vultures-ai-chat-example"));
    std::fs::create_dir_all(&dir).ok();
    let mut chat = Chat::new(provider, dir);
    let approver: Arc<dyn Approver> = Arc::new(Fixed(allow));
    for text in [
        "Remember the word 'jaboticaba'. Reply only: ok", // check-english:allow
        "Create a file named zeca.txt containing the word you remembered, then say done.",
    ] {
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let turn = Turn {
            text: text.into(),
            files: vec![],
        };
        let task = async { chat.send(turn, tx, approver.clone()).await };
        let print = async {
            while let Some(d) = rx.recv().await {
                match d {
                    Delta::Text { text } => print!("[{text}]"),
                    other => println!(" -> {other:?}"),
                }
            }
        };
        tokio::join!(task, print);
    }
}
