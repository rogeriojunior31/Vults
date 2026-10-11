//! Real turns against a CLI, to check streaming, resume and permissions end to end.
//! Usage: cargo run -p vults-chat --example turn -- claude|codex|api [allow|deny|stop] [folder]
//! `stop` presses Stop once the first turn starts talking, then checks the next turn resumes.
//! `VULTS_CHAT_SETTINGS=user|project` runs it in native mode (the user's settings, and the project's);
//! `VULTS_CHAT_MODEL` and `VULTS_CHAT_EFFORT` pick the model and effort.
use std::sync::Arc;

use tokio::sync::oneshot;
use vults_chat::{Approver, Chat, Delta, Provider, Turn};

/// Answers every permission the same way, as a stand-in for the island's card.
struct Fixed(bool);

impl Approver for Fixed {
    fn wait(&self, id: &str, _tool: &str, _target: &str) -> oneshot::Receiver<vults_chat::Answer> {
        let (tx, rx) = oneshot::channel();
        println!(
            "\n  [asked {id}: answering {}]",
            if self.0 { "allow" } else { "deny" }
        );
        let _ = tx.send(if self.0 {
            vults_chat::Answer::Allow
        } else {
            vults_chat::Answer::Deny
        });
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
    let mode = args.next();
    let allow = mode.as_deref() == Some("allow");
    let stop_first = mode.as_deref() == Some("stop");
    let dir = args
        .next()
        .map(Into::into)
        .unwrap_or_else(|| std::env::temp_dir().join("vults-chat-example"));
    std::fs::create_dir_all(&dir).ok();
    let mut chat = Chat::new(provider, dir);
    match std::env::var("VULTS_CHAT_SETTINGS").as_deref() {
        Ok("user") => chat.set_settings(vults_chat::Settings::User),
        Ok("project") => chat.set_settings(vults_chat::Settings::Project),
        _ => true,
    };
    chat.set_pick(
        std::env::var("VULTS_CHAT_MODEL").ok(),
        std::env::var("VULTS_CHAT_EFFORT")
            .ok()
            .as_deref()
            .and_then(vults_chat::Effort::parse),
    );
    let approver: Arc<dyn Approver> = Arc::new(Fixed(allow));
    let texts: [&str; 2] = if stop_first {
        [
            "Remember the word 'jaboticaba'. Then write a 40 stanza poem about vultures, no tools.", // check-english:allow
            "Reply only with the word you remembered.",
        ]
    } else {
        [
            "Remember the word 'jaboticaba'. Reply only: ok", // check-english:allow
            "Create a file named zeca.txt containing the word you remembered, then say done.",
        ]
    };
    for (i, text) in texts.into_iter().enumerate() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let turn = Turn {
            text: text.into(),
            files: vec![],
        };
        // Pressed at the first words of the first turn in `stop` mode; else held until the end.
        let (press, stop) = tokio::sync::oneshot::channel();
        let mut press = (stop_first && i == 0).then_some(press);
        let task = async { chat.send(turn, tx, approver.clone(), stop).await };
        let print = async {
            while let Some(d) = rx.recv().await {
                match d {
                    Delta::Text { text } => {
                        print!("[{text}]");
                        if let Some(p) = press.take() {
                            println!("\n  [pressing Stop]");
                            let _ = p.send(());
                        }
                    }
                    other => println!(" -> {other:?}"),
                }
            }
        };
        tokio::join!(task, print);
    }
}
