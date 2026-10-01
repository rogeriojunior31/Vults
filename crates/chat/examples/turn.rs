//! Two real turns against a CLI, to check streaming and resume end to end.
//! Usage: cargo run -p vultures-ai-chat --example turn -- claude|codex
use vultures_ai_chat::{Chat, Delta, Provider, Turn};

#[tokio::main]
async fn main() {
    let provider = match std::env::args().nth(1).as_deref() {
        Some("codex") => Provider::Codex,
        _ => Provider::Claude,
    };
    let dir = std::env::temp_dir().join("vultures-ai-chat-example");
    let mut chat = Chat::new(provider, dir);
    for text in [
        "Remember the word 'jaboticaba'. Reply only: ok",
        "Which word did I ask you to remember? One word.",
    ] {
        // check-english:allow
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let turn = Turn {
            text: text.into(),
            files: vec![],
        };
        let task = async { chat.send(turn, tx).await };
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
