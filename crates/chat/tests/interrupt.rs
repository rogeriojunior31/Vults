//! Stop in the middle of a Claude Code turn (`docs/dev/plan-zeca.md` S11), against a stand-in CLI
//! on the PATH that speaks stream-json as Claude Code 2.1.295 does: an `interrupt` control request
//! ends the turn with an `error_during_execution` result, and the session goes on. Its own binary:
//! it changes PATH.

#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use vults_agents::driver::{Answer, Ask, BoxFuture, EndReason, Event, Input, Sink, Spec};
use vults_chat::{Provider, drivers};

const STAND_IN: &str = r#"#!/usr/bin/env bash
# The first turn talks until it is interrupted; a resumed one answers at once.
IFS= read -r -t 10 _message
echo "$*" >>"$PWD/args"
echo '{"type":"system","subtype":"init","session_id":"sess-1"}'
case " $* " in
  *" --resume sess-1 "*)
    echo '{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"again"}}}'
    echo '{"type":"result","subtype":"success","is_error":false,"result":"again"}'
    exit 0 ;;
esac
echo '{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"a long "}}}'
echo '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"sleep 60"}}]}}'
while IFS= read -r -t 20 line; do
  case $line in
    *'"subtype":"interrupt"'*)
      echo '{"type":"control_response","response":{"subtype":"success","request_id":"vults-interrupt","response":{}}}'
      echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","is_error":true}]}}'
      echo '{"type":"result","subtype":"error_during_execution","is_error":true,"session_id":"sess-1"}'
      exit 1 ;;
  esac
done
echo '{"type":"result","subtype":"success","is_error":false,"result":"never interrupted"}'
"#;

#[derive(Default)]
struct Record(Mutex<Vec<Event>>);

impl Sink for Record {
    fn event(&self, event: Event) -> BoxFuture<'_, ()> {
        self.0.lock().unwrap().push(event);
        Box::pin(async {})
    }

    fn ask(&self, _: Ask) -> BoxFuture<'_, Answer> {
        Box::pin(async { Answer::Deny })
    }
}

#[tokio::test]
async fn interrupting_a_turn_ends_it_and_the_next_turn_resumes() {
    let root = std::env::temp_dir().join(format!("vults-interrupt-{}", std::process::id()));
    let (bin, work) = (root.join("bin"), root.join("work"));
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let claude = bin.join("claude");
    std::fs::write(&claude, STAND_IN).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    // SAFETY: this binary's only test, before any other thread reads the environment.
    unsafe { std::env::set_var("PATH", path) };

    let mut driver = drivers::for_cli(Provider::Claude, Spec::in_dir(work.clone())).unwrap();
    let stop = driver.interrupter();
    let sink = Arc::new(Record::default());
    let turn = {
        let sink = sink.clone();
        tokio::spawn(async move {
            let input = Input {
                text: "write a long poem".into(),
                files: Vec::new(),
            };
            let result = driver.send(&input, &*sink).await;
            (driver, result)
        })
    };
    // Stop once the tool is running, as a press on Stop would.
    tokio::time::timeout(Duration::from_secs(10), async {
        while !sink
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, Event::ToolStart { .. }))
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the turn started its tool");
    stop.interrupt().await.unwrap();
    let (mut driver, result) = tokio::time::timeout(Duration::from_secs(10), turn)
        .await
        .expect("the turn ended soon after the interrupt")
        .unwrap();
    assert_eq!(result, Ok(()));
    let events = sink.0.lock().unwrap().clone();
    assert_eq!(events.last(), Some(&Event::TurnEnd(EndReason::Interrupted)));
    assert!(events.contains(&Event::ToolEnd {
        id: "t1".into(),
        ok: false
    }));
    assert_eq!(driver.session(), Some("sess-1"));

    // The next turn resumes the same conversation, and the press is gone.
    let again = Record::default();
    let input = Input {
        text: "go on".into(),
        files: Vec::new(),
    };
    assert_eq!(driver.send(&input, &again).await, Ok(()));
    let events = again.0.lock().unwrap().clone();
    assert_eq!(events.last(), Some(&Event::TurnEnd(EndReason::Done)));
    assert!(events.contains(&Event::Text("again".into())));
    let args = std::fs::read_to_string(work.join("args")).unwrap();
    assert!(args.lines().nth(1).unwrap().contains("--resume sess-1"), "{args}");
    let _ = std::fs::remove_dir_all(&root);
}
