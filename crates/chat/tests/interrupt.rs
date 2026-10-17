//! Stop in the middle of a turn (`docs/dev/plan-zeca.md` S11), against stand-in CLIs on the PATH
//! that speak as Claude Code 2.1.295 and codex-cli 0.160.1 do. Each scenario has its own folder,
//! whose `mode` file picks the stand-in's behavior. One test: it changes PATH. The stand-ins are
//! bash scripts, which Windows can't run as `claude` or `codex`: Unix only.
#![cfg(unix)]
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vults_agents::driver::{AgentDriver, Answer, Ask, BoxFuture, EndReason, Event, Input, Sink, Spec};
use vults_chat::{Provider, drivers};

const CLAUDE: &str = r#"#!/usr/bin/env bash
# talk: talks until interrupted. card: asks for a tool, records what comes back. deaf: ignores
# the interrupt. A resumed turn answers at once.
IFS= read -r -t 10 _message
echo "$*" >>"$PWD/args"
echo '{"type":"system","subtype":"init","session_id":"sess-1"}'
case " $* " in
  *" --resume sess-1 "*)
    echo '{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"again"}}}'
    echo '{"type":"result","subtype":"success","is_error":false,"result":"again"}'
    exit 0 ;;
esac
case $(cat "$PWD/mode") in
  card)
    echo '{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"rm -rf build"}}}'
    while IFS= read -r -t 20 line; do
      printf '%s\n' "$line" >>"$PWD/got"
      case $line in *'"subtype":"interrupt"'*)
        echo '{"type":"result","subtype":"error_during_execution","is_error":true}'; exit 1 ;;
      esac
    done ;;
  deaf)
    echo '{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"la la"}}}'
    sleep 30 ;;
  *)
    echo '{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"a long "}}}'
    echo '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"sleep 60"}}]}}'
    while IFS= read -r -t 20 line; do
      case $line in *'"subtype":"interrupt"'*)
        echo '{"type":"control_response","response":{"subtype":"success","request_id":"vults-interrupt","response":{}}}'
        echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","is_error":true}]}}'
        echo '{"type":"result","subtype":"error_during_execution","is_error":true,"usage":{"input_tokens":3,"output_tokens":2}}'
        exit 1 ;;
      esac
    done ;;
esac
"#;

/// The app-server's side, by request id: the chat sends initialize (1), thread/start (2),
/// turn/start (3); the first turn's answer comes a moment after its first words, so Stop lands
/// before the turn has an id.
const CODEX: &str = r#"#!/usr/bin/env bash
while IFS= read -r -t 20 line; do
  printf '%s\n' "$line" >>"$PWD/got"
  case $line in
    *'"method":"initialize"'*) echo '{"id":1,"result":{}}' ;;
    *'"method":"thread/start"'*) echo '{"id":2,"result":{"thread":{"id":"th1"}}}' ;;
    *'"id":3,"method":"turn/start"'*)
      echo '{"method":"item/agentMessage/delta","params":{"delta":"a long "}}'
      sleep 0.5
      echo '{"id":3,"result":{"turn":{"id":"u1"}}}' ;;
    *'"method":"turn/interrupt"'*)
      echo '{"id":4,"result":{}}'
      echo '{"method":"turn/completed","params":{"threadId":"th1","turn":{"id":"u1","status":"interrupted","error":null}}}' ;;
    *'"method":"turn/start"'*)
      echo '{"id":5,"result":{"turn":{"id":"u2"}}}'
      echo '{"method":"item/agentMessage/delta","params":{"delta":"again"}}'
      echo '{"method":"turn/completed","params":{"threadId":"th1","turn":{"id":"u2","status":"completed","error":null}}}' ;;
  esac
done
"#;

#[derive(Default)]
struct Record {
    events: Mutex<Vec<Event>>,
    asked: AtomicBool,
}

impl Record {
    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }
}

impl Sink for Record {
    fn event(&self, event: Event) -> BoxFuture<'_, ()> {
        self.events.lock().unwrap().push(event);
        Box::pin(async {})
    }

    /// Nobody ever answers: only Stop can end the wait.
    fn ask(&self, _: Ask) -> BoxFuture<'_, Answer> {
        self.asked.store(true, Ordering::SeqCst);
        Box::pin(std::future::pending())
    }
}

fn input(text: &str) -> Input {
    Input {
        text: text.into(),
        ..Input::default()
    }
}

fn folder(root: &Path, name: &str, mode: &str) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("mode"), mode).unwrap();
    dir
}

async fn until(what: &str, check: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

/// Starts a turn on its own task, presses Stop once `ready` holds, and gives the driver back.
async fn stopped_turn(
    mut driver: Box<dyn AgentDriver>,
    sink: Arc<Record>,
    ready: impl Fn(&Record) -> bool,
) -> (Box<dyn AgentDriver>, Result<(), String>) {
    let stop = driver.interrupter();
    let turn = {
        let sink = sink.clone();
        tokio::spawn(async move {
            let result = driver.send(&input("write a long poem"), &*sink).await;
            (driver, result)
        })
    };
    until("the turn to get going", || ready(&sink)).await;
    stop.interrupt().await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), turn)
        .await
        .expect("the turn ended soon after the interrupt")
        .unwrap()
}

async fn resumes(driver: &mut Box<dyn AgentDriver>) {
    let again = Record::default();
    assert_eq!(driver.send(&input("go on"), &again).await, Ok(()));
    let events = again.events();
    assert_eq!(
        events.last(),
        Some(&Event::TurnEnd(EndReason::Done)),
        "{events:?}"
    );
    assert!(events.contains(&Event::Text("again".into())));
}

#[tokio::test]
async fn interrupting_a_turn_ends_it_and_the_next_turn_resumes() {
    let root = std::env::temp_dir().join(format!("vults-interrupt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for (name, script) in [("claude", CLAUDE), ("codex", CODEX)] {
        std::fs::write(bin.join(name), script).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(bin.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    // SAFETY: this binary's only test, before any other thread reads the environment.
    unsafe { std::env::set_var("PATH", path) };

    // Claude, mid-tool: the interrupt ends the tool and the turn; the session goes on.
    let work = folder(&root, "talk", "talk");
    let driver = drivers::for_cli(Provider::Claude, Spec::in_dir(work.clone())).unwrap();
    let sink = Arc::new(Record::default());
    let (mut driver, result) = stopped_turn(driver, sink.clone(), |r| {
        r.events().iter().any(|e| matches!(e, Event::ToolStart { .. }))
    })
    .await;
    assert_eq!(result, Ok(()));
    let events = sink.events();
    assert_eq!(events.last(), Some(&Event::TurnEnd(EndReason::Interrupted)));
    assert!(events.contains(&Event::ToolEnd {
        id: "t1".into(),
        ok: false
    }));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Usage(u) if u.output == 2)),
        "its usage still counts"
    );
    assert_eq!(driver.session(), Some("sess-1"));
    resumes(&mut driver).await;
    let args = std::fs::read_to_string(work.join("args")).unwrap();
    assert!(args.lines().nth(1).unwrap().contains("--resume sess-1"), "{args}");

    // Claude, a card waiting: Stop answers it no, before the interrupt goes out.
    let work = folder(&root, "card", "card");
    let driver = drivers::for_cli(Provider::Claude, Spec::in_dir(work.clone())).unwrap();
    let sink = Arc::new(Record::default());
    let (_, result) = stopped_turn(driver, sink.clone(), |r| r.asked.load(Ordering::SeqCst)).await;
    assert_eq!(result, Ok(()));
    assert_eq!(
        sink.events().first(),
        Some(&Event::Session("sess-1".into())),
        "the driver must know the CLI session before showing its permission"
    );
    assert_eq!(
        sink.events().last(),
        Some(&Event::TurnEnd(EndReason::Interrupted))
    );
    let got = std::fs::read_to_string(work.join("got")).unwrap();
    let mut lines = got.lines();
    let first = lines.next().unwrap();
    assert!(
        first.contains(r#""request_id":"r1""#) && first.contains(r#""behavior":"deny""#),
        "{got}"
    );
    assert!(
        lines.next().unwrap().contains(r#""subtype":"interrupt""#),
        "{got}"
    );

    // Claude, deaf to the interrupt, and the turn dropped (as Chat does past its grace): the
    // press is released and the session id it heard is kept.
    let work = folder(&root, "deaf", "deaf");
    let mut driver = drivers::for_cli(Provider::Claude, Spec::in_dir(work.clone())).unwrap();
    let stop = driver.interrupter();
    let sink = Record::default();
    let sing = input("sing");
    {
        let turn = driver.send(&sing, &sink);
        tokio::pin!(turn);
        let gone = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                tokio::select! {
                    _ = &mut turn => panic!("a deaf turn never ends by itself"),
                    () = tokio::time::sleep(Duration::from_millis(20)) => {}
                }
                if sink.events().contains(&Event::Text("la la".into())) {
                    break;
                }
            }
        });
        gone.await.unwrap();
        stop.interrupt().await.unwrap();
        let _ = tokio::time::timeout(Duration::from_millis(300), &mut turn).await;
    }
    assert_eq!(driver.session(), Some("sess-1"));
    resumes(&mut driver).await;

    // Codex: Stop before the turn has an id waits for it, then turn/interrupt; the server stays up.
    let work = folder(&root, "codex", "");
    let driver = drivers::for_cli(Provider::Codex, Spec::in_dir(work.clone())).unwrap();
    let sink = Arc::new(Record::default());
    let (mut driver, result) = stopped_turn(driver, sink.clone(), |r| {
        r.events().iter().any(|e| matches!(e, Event::Text(_)))
    })
    .await;
    assert_eq!(result, Ok(()));
    assert_eq!(
        sink.events().last(),
        Some(&Event::TurnEnd(EndReason::Interrupted))
    );
    let got = std::fs::read_to_string(work.join("got")).unwrap();
    assert!(
        got.contains(r#""method":"turn/interrupt","params":{"threadId":"th1","turnId":"u1"}"#),
        "{got}"
    );
    resumes(&mut driver).await;
    let got = std::fs::read_to_string(work.join("got")).unwrap();
    assert_eq!(
        got.matches(r#""method":"initialize""#).count(),
        1,
        "the same server: {got}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
