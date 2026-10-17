//! The ACP driver (`docs/dev/plan-zeca.md` W10) against a stand-in agent on the PATH that speaks
//! as OpenCode 1.18.35 does over ACP: a turn that asks before a command, a turn stopped midway, and
//! a new process resuming the session. Each scenario has its own folder, whose `mode` file picks
//! the stand-in's behavior. One test: it changes PATH. The stand-in is a bash script: Unix only.
#![cfg(unix)]
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vults_agents::driver::{Answer, Ask, BoxFuture, EndReason, Event, Input, Sink, Spec};
use vults_chat::drivers;

const AGENT: &str = r#"#!/usr/bin/env bash
prompt=
while IFS= read -r -t 20 line; do
  printf '%s\n' "$line" >>"$PWD/got"
  id=$(printf '%s' "$line" | sed -n 's/^{"jsonrpc":"2.0","id":\([0-9]*\).*/\1/p')
  case $line in
    *'"method":"initialize"'*)
      echo '{"jsonrpc":"2.0","id":'"$id"',"result":{"protocolVersion":1,"agentCapabilities":{"loadSession":true}}}' ;;
    *'"method":"session/new"'*)
      echo '{"jsonrpc":"2.0","id":'"$id"',"result":{"sessionId":"ses_1"}}' ;;
    *'"method":"session/load"'*)
      echo '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"ses_1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"old history"}}}}'
      echo '{"jsonrpc":"2.0","id":'"$id"',"result":{}}' ;;
    *'"method":"session/prompt"'*)
      prompt=$id
      echo '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"ses_1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"hello"}}}}'
      case $(cat "$PWD/mode") in
        card)
          echo '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"ses_1","update":{"sessionUpdate":"tool_call","toolCallId":"call_b","title":"bash","kind":"execute","status":"pending"}}}'
          echo '{"jsonrpc":"2.0","id":900,"method":"session/request_permission","params":{"sessionId":"ses_1","toolCall":{"toolCallId":"call_b","title":"rm -rf build","kind":"execute","rawInput":{"command":"rm -rf build"}},"options":[{"optionId":"once","kind":"allow_once"},{"optionId":"always","kind":"allow_always"},{"optionId":"reject","kind":"reject_once"}]}}' ;;
        talk) ;;
        sneaky)
          echo '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"ses_1","update":{"sessionUpdate":"tool_call","toolCallId":"call_s","title":"rm -rf build","kind":"execute","status":"completed","rawInput":{"command":"rm -rf build"}}}}' ;;
        *)
          echo '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"ses_1","update":{"sessionUpdate":"usage_update","cost":{"amount":0.75,"currency":"USD"}}}}'
          echo '{"jsonrpc":"2.0","id":'"$prompt"',"result":{"stopReason":"end_turn"}}' ;;
      esac ;;
    *'"id":900,"result"'*)
      echo '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"ses_1","update":{"sessionUpdate":"tool_call_update","toolCallId":"call_b","status":"completed"}}}'
      echo '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"ses_1","update":{"sessionUpdate":"usage_update","cost":{"amount":0.5,"currency":"USD"}}}}'
      echo '{"jsonrpc":"2.0","id":'"$prompt"',"result":{"stopReason":"end_turn","usage":{"inputTokens":3,"outputTokens":2,"cachedReadTokens":10}}}' ;;
    *'"method":"session/cancel"'*)
      echo '{"jsonrpc":"2.0","id":'"$prompt"',"result":{"stopReason":"cancelled"}}' ;;
  esac
done
"#;

#[derive(Default)]
struct Record {
    events: Mutex<Vec<Event>>,
    asks: Mutex<Vec<Ask>>,
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

    /// A human's yes, and the yes for the rest of the conversation: neither is ever "always".
    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        self.asks.lock().unwrap().push(ask);
        Box::pin(async { Answer::AllowInConversation })
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

fn got(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("got")).unwrap_or_default()
}

#[tokio::test]
async fn an_acp_agent_asks_stops_and_resumes() {
    let root = std::env::temp_dir().join(format!("vults-acp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("opencode"), AGENT).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(bin.join("opencode"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    // SAFETY: this binary's only test, before any other thread reads the environment.
    unsafe { std::env::set_var("PATH", path) };

    // A command asks; the yes is the agent's "once", never its "always".
    let work = folder(&root, "card", "card");
    let spec = Spec {
        system_append: Some("Keep edits in this folder.".into()),
        role: vults_agents::driver::Role::Worker,
        ..Spec::in_dir(work.clone())
    };
    let mut driver = drivers::acp("opencode", spec, None).unwrap();
    let sink = Record::default();
    assert_eq!(driver.send(&input("clean the build"), &sink).await, Ok(()));
    assert_eq!(driver.session(), Some("ses_1"));
    let asks = sink.asks.lock().unwrap().clone();
    assert_eq!(asks.len(), 1);
    assert_eq!(
        (asks[0].tool.as_str(), asks[0].input["command"].as_str()),
        ("Bash", Some("rm -rf build"))
    );
    let sent = got(&work);
    assert!(sent.contains(r#""optionId":"once""#), "{sent}");
    assert!(!sent.contains(r#""optionId":"always""#), "{sent}");
    // No file system or terminal of ours; the brief leads the first prompt.
    assert!(sent.contains(r#""terminal":false"#), "{sent}");
    assert!(
        sent.contains(r"Keep edits in this folder.\n\nclean the build"),
        "{sent}"
    );
    let events = sink.events();
    assert!(events.contains(&Event::Text("hello".into())));
    assert!(events.contains(&Event::ToolEnd {
        id: "call_b".into(),
        ok: true
    }));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ToolStart { id, .. } if id == "call_b"))
    );
    let usage = events.iter().find_map(|e| match e {
        Event::Usage(u) => Some(u.clone()),
        _ => None,
    });
    let usage = usage.unwrap();
    assert_eq!(
        (usage.input, usage.output, usage.cost_usd, usage.session_cost_usd),
        (13, 2, Some(0.5), Some(0.5))
    );
    assert_eq!(events.last(), Some(&Event::TurnEnd(EndReason::Done)));

    // The next turn in the same process: the brief is not sent again, the cost is its own share.
    std::fs::write(work.join("mode"), "done").unwrap();
    let again = Record::default();
    assert_eq!(driver.send(&input("and now?"), &again).await, Ok(()));
    let sent = got(&work);
    assert_eq!(sent.matches("Keep edits in this folder.").count(), 1, "{sent}");
    let share = again.events().iter().find_map(|e| match e {
        Event::Usage(u) => u.cost_usd,
        _ => None,
    });
    assert!((share.unwrap() - 0.25).abs() < 1e-9);

    // Stopped midway: the agent is told to cancel, and the turn ends as interrupted.
    let work = folder(&root, "talk", "talk");
    let mut driver = drivers::acp("opencode", Spec::in_dir(work.clone()), None).unwrap();
    let stop = driver.interrupter();
    let sink = Arc::new(Record::default());
    let turn = {
        let sink = sink.clone();
        tokio::spawn(async move {
            let result = driver.send(&input("write a long poem"), &*sink).await;
            (driver, result)
        })
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        while !sink.events().contains(&Event::Text("hello".into())) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    stop.interrupt().await.unwrap();
    let (_driver, result) = tokio::time::timeout(Duration::from_secs(10), turn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(
        sink.events().last(),
        Some(&Event::TurnEnd(EndReason::Interrupted))
    );
    assert!(got(&work).contains(r#""method":"session/cancel""#));

    // A command that ran without asking (the user's own settings let it): the turn stops.
    let work = folder(&root, "sneaky", "sneaky");
    let mut driver = drivers::acp("opencode", Spec::in_dir(work.clone()), None).unwrap();
    let sink = Record::default();
    let said = driver.send(&input("clean"), &sink).await;
    // Its process (and anything it started) goes with the failed turn.
    assert!(
        said.as_ref().is_err_and(|e| e.contains("without asking")),
        "{said:?}"
    );

    // A new process resumes the session; its history is not news, and no brief is sent again.
    let work = folder(&root, "resume", "done");
    let mut driver = drivers::acp("opencode", Spec::in_dir(work.clone()), Some("ses_1".into())).unwrap();
    let sink = Record::default();
    assert_eq!(driver.send(&input("go on"), &sink).await, Ok(()));
    let sent = got(&work);
    assert!(sent.contains(r#""method":"session/load""#), "{sent}");
    let events = sink.events();
    assert!(!events.contains(&Event::Text("old history".into())), "{events:?}");
    // Its earlier cost is not this turn's: only the running total is told.
    let usage = events.iter().find_map(|e| match e {
        Event::Usage(u) => Some(u.clone()),
        _ => None,
    });
    let usage = usage.unwrap();
    assert_eq!((usage.cost_usd, usage.session_cost_usd), (None, Some(0.75)));
    let _ = std::fs::remove_dir_all(&root);
}

/// Against the real OpenCode on this computer (its own account and model): `cargo test -p
/// vults-chat --test acp -- --ignored`. It writes one file in a folder of its own, and asks first.
#[tokio::test]
#[ignore = "runs the real OpenCode"]
async fn opencode_writes_a_file_after_asking() {
    let home = std::env::var("HOME").unwrap();
    let work = PathBuf::from(home).join(".cache/vults-acp-live");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let mut driver = drivers::acp("opencode", Spec::in_dir(work.clone()), None).unwrap();
    let sink = Record::default();
    let said = driver
        .send(&input("Create a file named hello.txt containing the word hi, then run `cat hello.txt`. Then say done."), &sink)
        .await;
    assert_eq!(said, Ok(()));
    let asks = sink.asks.lock().unwrap().clone();
    let events = sink.events();
    eprintln!("asks: {asks:#?}\nevents: {events:#?}");
    assert!(asks.iter().any(|a| a.tool == "Edit"), "the write asked first");
    assert!(asks.iter().any(|a| a.tool == "Bash"), "the command asked first");
    assert_eq!(
        std::fs::read_to_string(work.join("hello.txt")).unwrap().trim(),
        "hi"
    );
    assert_eq!(events.last(), Some(&Event::TurnEnd(EndReason::Done)));
    let _ = std::fs::remove_dir_all(&work);
}
