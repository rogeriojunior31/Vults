//! The ACP driver against the real agents on this computer (their own accounts and models):
//! `VULTS_ACP_LIVE=antigravity cargo test -p vults-chat --test acp_live -- --ignored --nocapture`
//! (or `opencode`). Each writes one file in a folder of its own and runs one command, asking
//! before both.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::Mutex;

use vults_agents::driver::{Answer, Ask, BoxFuture, EndReason, Event, Input, Sink, Spec};
use vults_chat::drivers;

#[derive(Default)]
struct Record {
    events: Mutex<Vec<Event>>,
    asks: Mutex<Vec<Ask>>,
}

impl Sink for Record {
    fn event(&self, event: Event) -> BoxFuture<'_, ()> {
        self.events.lock().unwrap().push(event);
        Box::pin(async {})
    }

    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        self.asks.lock().unwrap().push(ask);
        Box::pin(async { Answer::Allow })
    }
}

#[tokio::test]
#[ignore = "runs a real ACP agent"]
async fn a_real_agent_writes_a_file_after_asking() {
    let agent = std::env::var("VULTS_ACP_LIVE").unwrap_or_else(|_| "antigravity".into());
    let work = std::env::temp_dir().join(format!("vults-acp-live-{agent}"));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let mut spec = Spec::in_dir(work.clone());
    // A model of the agent's own, as the chat's picker gives it: VULTS_ACP_MODEL.
    spec.model = std::env::var("VULTS_ACP_MODEL").ok();
    let mut driver = drivers::acp(&agent, spec, None).unwrap();
    let sink = Record::default();
    let input = Input {
        text: "Create a file named hello.txt containing the word hi, then run the shell command `type hello.txt` (or `cat hello.txt`). Then say done.".into(),
        ..Input::default()
    };
    let said = driver.send(&input, &sink).await;
    let asks = sink.asks.lock().unwrap().clone();
    let events = sink.events.lock().unwrap().clone();
    eprintln!("said: {said:?}\nsession: {:?}\nasks: {asks:#?}", driver.session());
    for e in &events {
        let line = format!("{e:?}");
        eprintln!("event: {}", line.chars().take(300).collect::<String>());
    }
    assert_eq!(said, Ok(()));
    assert_eq!(
        std::fs::read_to_string(work.join("hello.txt"))
            .unwrap_or_default()
            .trim(),
        "hi",
        "the file was written"
    );
    eprintln!("asked {} time(s); the rest ran by its own settings", asks.len());
    assert_eq!(events.last(), Some(&Event::TurnEnd(EndReason::Done)));
    let _: PathBuf = work.clone();
    let _ = std::fs::remove_dir_all(&work);
}

#[tokio::test]
#[ignore = "runs a real ACP agent"]
async fn a_real_agent_lists_its_models() {
    let agent = std::env::var("VULTS_ACP_LIVE").unwrap_or_else(|_| "antigravity".into());
    let dir = std::env::temp_dir().join(format!("vults-acp-models-{agent}"));
    std::fs::create_dir_all(&dir).unwrap();
    let models = vults_chat::acp_models(&agent, &dir).await;
    eprintln!("{models:#?}");
    assert!(models.is_ok_and(|m| !m.is_empty()));
}
