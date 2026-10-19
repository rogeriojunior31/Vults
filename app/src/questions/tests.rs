use super::*;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::process::{Command as ProcessCommand, Stdio};
use tokio::sync::mpsc;

const FIXTURE: &str = "questions::tests::question_process_fixture";
#[test]
fn question_process_fixture() {
    let Ok(role) = std::env::var("VULTS_QUESTION_TEST_ROLE") else {
        return;
    };
    if role == "agent" || role == "external-wrapper" {
        let status = ProcessCommand::new(std::env::current_exe().unwrap())
            .args(["--exact", FIXTURE])
            .env(
                "VULTS_QUESTION_TEST_ROLE",
                if role == "external-wrapper" {
                    "agent"
                } else {
                    "relay"
                },
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        std::process::exit(i32::from(!status.success()));
    }
    let stream =
        std::os::unix::net::UnixStream::connect(std::env::var_os("VULTS_QUESTION_TEST_MCP").unwrap())
            .unwrap();
    let (tx, rx) = std::sync::mpsc::sync_channel(16);
    let input = stream.try_clone().unwrap();
    std::thread::spawn(move || crate::mcp::stdio::read(std::io::BufReader::new(input), tx));
    crate::mcp::stdio::serve(
        rx,
        stream,
        std::path::Path::new("/"),
        Some(std::env::var_os("VULTS_QUESTION_TEST_SOCKET").unwrap().into()),
        || true,
        std::env::var("VULTS_QUESTION_TEST_EXTERNAL").is_ok_and(|value| value == "yes"),
    );
    std::process::exit(0);
}
struct Fixture {
    dir: std::path::PathBuf,
    child: std::process::Child,
    socket: std::os::unix::net::UnixStream,
    reader: std::io::BufReader<std::os::unix::net::UnixStream>,
    rx: mpsc::Receiver<super::Command>,
    server: tokio::task::JoinHandle<std::io::Result<()>>,
}
impl Fixture {
    async fn new() -> Self {
        Self::with_external(false).await
    }
    async fn with_external(external: bool) -> Self {
        use std::os::unix::fs::DirBuilderExt;
        let dir = std::env::temp_dir().join(format!(
            "vults-q-{}-{}",
            std::process::id(),
            crate::traces::new_id()
        ));
        std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        vults_store::Store::open(&dir).unwrap();
        let question_path = dir.join("questions.sock");
        let mcp_path = dir.join("mcp.sock");
        let listener = std::os::unix::net::UnixListener::bind(&mcp_path).unwrap();
        let (tx, rx) = mpsc::channel(16);
        let server = tokio::spawn(transport::serve(question_path.clone(), tx));
        let child = ProcessCommand::new(std::env::current_exe().unwrap())
            .args(["--exact", FIXTURE])
            .env(
                "VULTS_QUESTION_TEST_ROLE",
                if external { "external-wrapper" } else { "agent" },
            )
            .env("VULTS_QUESTION_TEST_EXTERNAL", if external { "yes" } else { "" })
            .env("VULTS_QUESTION_TEST_MCP", mcp_path)
            .env("VULTS_QUESTION_TEST_SOCKET", &question_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let (socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
        socket.set_write_timeout(Some(Duration::from_secs(4))).unwrap();
        let reader = std::io::BufReader::new(socket.try_clone().unwrap());
        let mut fixture = Self {
            dir,
            child,
            socket,
            reader,
            rx,
            server,
        };
        fixture.send(
            json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}),
        );
        assert_eq!(fixture.read()["id"], 0);
        fixture.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        fixture
    }
    fn send(&mut self, value: Value) {
        writeln!(self.socket, "{value}").unwrap();
        self.socket.flush().unwrap();
    }
    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    async fn next(&mut self) -> super::Command {
        tokio::time::timeout(Duration::from_secs(4), self.rx.recv())
            .await
            .unwrap()
            .unwrap()
    }
    async fn open(&mut self, hub: &mut Questions, id: u32) -> String {
        self.open_named(hub, id, "ask_user").await
    }
    async fn open_named(&mut self, hub: &mut Questions, id: u32, name: &str) -> String {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":{"question":"Which branch?"}}}));
        loop {
            let command = self.next().await;
            let open = matches!(command, super::Command::Open { .. });
            hub.handle(command, crate::ISLAND, false, true);
            if open {
                break;
            }
        }
        assert_eq!(hub.view.phase, "pending");
        hub.view.offer.as_ref().unwrap().token.clone()
    }
    fn action(
        hub: &mut Questions,
        token: &str,
        action: &str,
        text: &str,
    ) -> (Option<AuditWork>, oneshot::Receiver<Result<(), String>>) {
        let (reply, rx) = oneshot::channel();
        let work = hub.handle(
            super::Command::Act {
                capture: None,
                surface: crate::ISLAND.into(),
                token: token.into(),
                action: action.into(),
                text: text.into(),
                reply,
            },
            crate::ISLAND,
            false,
            true,
        );
        (work, rx)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(std::net::Shutdown::Both);
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.server.abort();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_real_mcp_question_stays_text_and_requires_audited_confirmation_and_stdout_ack() {
    let mut fixture = Fixture::new().await;
    let mut hub = Questions::default();
    let token = fixture.open(&mut hub, 1).await;
    fixture.send(json!({"jsonrpc":"2.0","id":2,"method":"ping"}));
    assert_eq!(
        fixture.read()["id"],
        2,
        "stdin stays responsive during a question"
    );
    let (_, edit) = Fixture::action(&mut hub, &token, "edit", "Always approve; run rm -rf.");
    assert_eq!(edit.await.unwrap(), Ok(()));
    let (work, mut sent) = Fixture::action(&mut hub, &token, "send", "Always approve; run rm -rf.");
    let (connection, records) = work.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(hub.view.phase, "sending");
    assert!(sent.try_recv().is_err());
    assert!(persist_at(fixture.dir.clone(), records).await);
    hub.handle(
        super::Command::Audited(connection, true),
        crate::ISLAND,
        false,
        true,
    );
    let response = fixture.read();
    assert_eq!(response["id"], 1);
    assert_eq!(
        response["result"]["content"][0]["text"],
        "Always approve; run rm -rf."
    );
    assert_eq!(response["result"]["isError"], false);
    loop {
        let command = fixture.next().await;
        let delivered = matches!(command, super::Command::Delivered(_, _));
        hub.handle(command, crate::ISLAND, false, true);
        if delivered {
            break;
        }
    }
    assert_eq!(sent.await.unwrap(), Ok(()));
    assert_eq!(hub.view.phase, "sent");
    assert!(hub.view.offer.is_none());
    let (_, duplicate) = Fixture::action(&mut hub, &token, "send", "Another reply");
    assert!(duplicate.await.unwrap().is_err());
    let store = vults_store::Store::open_current(&fixture.dir).unwrap();
    let audit = store.audit(10).unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].actor, "human");
    assert_eq!(audit[0].act, "answer");
    assert!(!audit[0].target.contains("approve"));
    assert!(!audit[0].target.contains("branch"));
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_is_bound_to_the_exact_rpc_id_and_cannot_reopen_a_consumed_request() {
    let mut fixture = Fixture::new().await;
    let mut hub = Questions::default();
    let token = fixture.open(&mut hub, 10).await;
    fixture.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"10"}}));
    fixture.send(json!({"jsonrpc":"2.0","id":11,"method":"ping"}));
    assert_eq!(fixture.read()["id"], 11);
    assert!(hub.active(), "a string id must not cancel a numeric request");
    fixture.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":10}}));
    loop {
        let command = fixture.next().await;
        let disconnected = matches!(command, super::Command::Disconnect(_));
        hub.handle(command, crate::ISLAND, false, true);
        if disconnected {
            break;
        }
    }
    assert!(!hub.active());
    assert!(hub.view.offer.is_none());
    let (_, late) = Fixture::action(&mut hub, &token, "send", "Late words");
    assert!(late.await.unwrap().is_err());
    fixture.send(json!({"jsonrpc":"2.0","id":12,"method":"ping"}));
    assert_eq!(
        fixture.read()["id"],
        12,
        "cancelled request emits no MCP response"
    );
    fixture.send(json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"Replay"}}}));
    assert_eq!(fixture.read()["result"]["isError"], true);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lock_departure_expiry_and_failed_audit_revoke_the_deferred_send() {
    let mut fixture = Fixture::new().await;
    let mut hub = Questions::default();
    for n in 0..4 {
        let token = fixture.open(&mut hub, 20 + n).await;
        let (work, action) = Fixture::action(&mut hub, &token, "send", "Private final text");
        let (connection, _) = work.unwrap();
        match n {
            0 => {
                hub.handle(super::Command::Tick, crate::ISLAND, true, true);
            }
            1 => {
                hub.handle(
                    super::Command::Depart(crate::ISLAND.into()),
                    crate::ISLAND,
                    false,
                    true,
                );
            }
            2 => {
                hub.delivery.as_mut().unwrap().pending.expires = Instant::now();
            }
            _ => {}
        }
        hub.handle(
            super::Command::Audited(connection, n != 3),
            crate::ISLAND,
            false,
            true,
        );
        assert!(action.await.unwrap().is_err());
        assert!(!hub.active());
        assert!(hub.view.offer.is_none());
        assert_eq!(fixture.read()["result"]["isError"], true);
        assert!(
            hub.records
                .iter()
                .all(|record| !record.target.contains("Private"))
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lost_stdout_ack_is_consumed_and_never_reported_as_sent() {
    let mut fixture = Fixture::new().await;
    let mut hub = Questions::default();
    let token = fixture.open(&mut hub, 30).await;
    let (work, action) = Fixture::action(&mut hub, &token, "send", "Consumed reply");
    let (connection, records) = work.unwrap();
    assert!(persist_at(fixture.dir.clone(), records).await);
    fixture.socket.shutdown(std::net::Shutdown::Read).unwrap();
    hub.handle(
        super::Command::Audited(connection, true),
        crate::ISLAND,
        false,
        true,
    );
    while hub.active() {
        let command = fixture.next().await;
        hub.handle(command, crate::ISLAND, false, true);
    }
    assert!(action.await.unwrap().is_err());
    assert_eq!(hub.view.phase, "failed");
    let (_, retry) = Fixture::action(&mut hub, &token, "send", "Consumed reply");
    assert!(retry.await.unwrap().is_err());
    assert!(
        hub.records
            .iter()
            .any(|record| record.actor == vults_core::audit::Actor::System)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ownership_surface_and_page_lease_are_checked_before_any_draft_mutation() {
    let mut hub = Questions::default();
    let proof = Requester::stdio().unwrap();
    assert!(!proof.launched_by(std::process::id()));
    let (reply, rx) = oneshot::channel();
    hub.handle(
        super::Command::Open {
            connection: 0,
            proof,
            request: "id".into(),
            question: "Forged owner".into(),
            received: Instant::now(),
            reply,
        },
        crate::ISLAND,
        false,
        true,
    );
    assert!(rx.await.unwrap().is_err());
    assert!(!hub.active());
    let mut fixture = Fixture::new().await;
    let token = fixture.open(&mut hub, 40).await;
    let (reply, rx) = oneshot::channel();
    hub.handle(
        super::Command::Act {
            capture: None,
            surface: crate::NEST.into(),
            token: token.clone(),
            action: "send".into(),
            text: "Wrong surface".into(),
            reply,
        },
        crate::ISLAND,
        false,
        true,
    );
    assert!(rx.await.unwrap().is_err());
    assert_eq!(hub.view.offer.as_ref().unwrap().draft, "");
    hub.pending.as_mut().unwrap().touched = Instant::now() - LEASE;
    let (reply, rx) = oneshot::channel();
    hub.handle(
        super::Command::Status {
            surface: crate::ISLAND.into(),
            token: Some(token),
            reply,
        },
        crate::ISLAND,
        false,
        true,
    );
    assert!(
        rx.await.unwrap().offer.is_none(),
        "A heartbeat cannot revive an expired page lease"
    );
    assert!(!hub.active());
    assert_eq!(fixture.read()["result"]["isError"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dictation_stays_a_draft_until_matching_apply_and_separate_confirmation() {
    let mut fixture = Fixture::new().await;
    let mut hub = Questions::default();
    let token = fixture.open_named(&mut hub, 50, "vults_dictate").await;
    let (_, edited) = Fixture::action(&mut hub, &token, "edit", "Manual");
    assert!(edited.await.unwrap().is_ok());
    let early = capture::Press::new(token.clone());
    hub.handle(Command::VoiceStart(early.clone()), crate::ISLAND, false, true);
    assert!(early.cancelled.load(std::sync::atomic::Ordering::Acquire));
    assert!(
        hub.recording.is_none(),
        "A question must be displayed before a key can select it"
    );
    let (reply, displayed) = oneshot::channel();
    hub.handle(
        Command::Status {
            surface: crate::ISLAND.into(),
            token: Some(token.clone()),
            reply,
        },
        crate::ISLAND,
        false,
        true,
    );
    assert!(displayed.await.unwrap().offer.is_some());
    let press = capture::Press::new(token.clone());
    hub.handle(Command::VoiceStart(press), crate::ISLAND, false, true);
    let id = hub.recording.as_ref().unwrap().id;
    assert_eq!(
        hub.view.offer.as_ref().unwrap().dictation.as_ref().unwrap().phase,
        "listening"
    );
    let (_, premature) = Fixture::action(&mut hub, &token, "send", "Do not deliver");
    assert!(premature.await.unwrap().is_err());
    assert_eq!(hub.view.offer.as_ref().unwrap().draft, "Manual");
    hub.handle(
        Command::VoiceResult {
            token: token.clone(),
            id,
            result: None,
        },
        crate::ISLAND,
        false,
        true,
    );
    let (_, edited) = Fixture::action(&mut hub, &token, "edit", "Manual edit during decode");
    assert!(edited.await.unwrap().is_ok());
    hub.handle(
        Command::VoiceResult {
            token: token.clone(),
            id,
            result: Some(Ok("spoken send".into())),
        },
        crate::ISLAND,
        false,
        true,
    );
    hub.handle(
        Command::VoiceResult {
            token: token.clone(),
            id,
            result: None,
        },
        crate::ISLAND,
        false,
        true,
    );
    assert_eq!(
        hub.view.offer.as_ref().unwrap().dictation.as_ref().unwrap().phase,
        "ready"
    );
    assert_eq!(
        hub.view.offer.as_ref().unwrap().draft,
        "Manual edit during decode"
    );
    let (_, premature) = Fixture::action(&mut hub, &token, "send", "Do not deliver");
    assert!(premature.await.unwrap().is_err());
    for capture in [Some(id + 1), Some(id)] {
        let (reply, rx) = oneshot::channel();
        hub.handle(
            Command::Act {
                surface: crate::ISLAND.into(),
                token: token.clone(),
                action: "apply-dictation".into(),
                text: "Manual edit during decode spoken send".into(),
                capture,
                reply,
            },
            crate::ISLAND,
            false,
            true,
        );
        assert_eq!(rx.await.unwrap().is_ok(), capture == Some(id));
    }
    assert!(hub.recording.is_none());
    let (work, action) = Fixture::action(&mut hub, &token, "send", "Manual edit during decode spoken send");
    let (connection, records) = work.unwrap();
    assert!(persist_at(fixture.dir.clone(), records).await);
    hub.handle(Command::Audited(connection, true), crate::ISLAND, false, true);
    let answer = fixture.read();
    assert_eq!(
        answer["result"]["content"][0]["text"],
        "Manual edit during decode spoken send"
    );
    while hub.active() {
        hub.handle(fixture.next().await, crate::ISLAND, false, true);
    }
    assert!(action.await.unwrap().is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_and_replaced_captures_cannot_mutate_or_send_another_question() {
    let mut fixture = Fixture::new().await;
    let mut hub = Questions::default();
    let old = fixture.open(&mut hub, 60).await;
    let (reply, displayed) = oneshot::channel();
    hub.handle(
        Command::Status {
            surface: crate::ISLAND.into(),
            token: Some(old.clone()),
            reply,
        },
        crate::ISLAND,
        false,
        true,
    );
    assert!(displayed.await.unwrap().offer.is_some());
    let press = capture::Press::new(old.clone());
    hub.handle(Command::VoiceStart(press.clone()), crate::ISLAND, false, true);
    let id = hub.recording.as_ref().unwrap().id;
    hub.handle(Command::VoiceStop(old.clone()), crate::ISLAND, false, true);
    assert!(press.cancelled.load(std::sync::atomic::Ordering::Acquire));
    hub.handle(
        Command::VoiceResult {
            token: old.clone(),
            id,
            result: Some(Ok("late".into())),
        },
        crate::ISLAND,
        false,
        true,
    );
    assert!(hub.view.offer.as_ref().unwrap().dictation.is_none());
    let next_press = capture::Press::new(old.clone());
    hub.handle(
        Command::VoiceStart(next_press.clone()),
        crate::ISLAND,
        false,
        true,
    );
    let next_id = hub.recording.as_ref().unwrap().id;
    hub.finish(Revocation::Cancelled, "cancelled");
    assert_eq!(fixture.read()["result"]["isError"], true);
    let new = fixture.open(&mut hub, 61).await;
    assert_ne!(new, old);
    hub.handle(
        Command::VoiceResult {
            token: old,
            id: next_id,
            result: Some(Ok("late private words".into())),
        },
        crate::ISLAND,
        false,
        true,
    );
    assert!(next_press.cancelled.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(hub.view.offer.as_ref().unwrap().draft, "");
    assert!(hub.view.offer.as_ref().unwrap().dictation.is_none());
    let (reply, displayed) = oneshot::channel();
    hub.handle(
        Command::Status {
            surface: crate::ISLAND.into(),
            token: Some(new.clone()),
            reply,
        },
        crate::ISLAND,
        false,
        true,
    );
    assert!(displayed.await.unwrap().offer.is_some());
    hub.handle(
        Command::VoiceStart(capture::Press::new(new)),
        crate::ISLAND,
        false,
        true,
    );
    let active = hub.recording.as_ref().unwrap().press.clone();
    hub.handle(Command::Tick, crate::ISLAND, true, true);
    assert!(active.cancelled.load(std::sync::atomic::Ordering::Acquire));
    assert!(!hub.active());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_questions_require_current_consent_and_do_not_inherit_reenabled_access() {
    let mut fixture = Fixture::with_external(true).await;
    let mut hub = Questions::default();
    fixture.send(json!({"jsonrpc":"2.0","id":70,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"Initially off"}}}));
    let open = fixture.next().await;
    if let Command::Open { ref proof, .. } = open {
        assert!(!proof.launched_by(std::process::id()));
    } else {
        panic!("expected real external proof");
    }
    hub.handle(open, crate::ISLAND, false, true);
    assert_eq!(fixture.read()["result"]["isError"], true);
    assert!(!hub.active());
    hub.allow_external(true, None);
    let token = fixture.open_named(&mut hub, 71, "vults_dictate").await;
    assert!(hub.pending.as_ref().unwrap().external);
    let (_, edited) = Fixture::action(&mut hub, &token, "edit", "Private external draft");
    assert!(edited.await.unwrap().is_ok());
    // A fast off/on cannot resurrect the pending request, even if the runtime sees only the last choice.
    let changed = Instant::now();
    hub.allow_external(true, Some(changed));
    assert!(!hub.active());
    assert_eq!(fixture.read()["result"]["isError"], true);
    fixture.send(json!({"jsonrpc":"2.0","id":72,"method":"tools/call","params":{"name":"ask_user","arguments":{"question":"Queued before the new grant"}}}));
    let mut stale = loop {
        let command = fixture.next().await;
        if matches!(command, Command::Open { .. }) {
            break command;
        }
        hub.handle(command, crate::ISLAND, false, true);
    };
    if let Command::Open { ref mut received, .. } = stale {
        *received = changed - Duration::from_millis(1);
    }
    hub.handle(stale, crate::ISLAND, false, true);
    assert_eq!(fixture.read()["result"]["isError"], true);
    assert!(!hub.active());
    let token = fixture.open(&mut hub, 73).await;
    let (work, action) = Fixture::action(&mut hub, &token, "send", "Confirmed external text");
    let (connection, records) = work.unwrap();
    assert!(persist_at(fixture.dir.clone(), records).await);
    hub.allow_external(false, Some(Instant::now()));
    hub.handle(Command::Audited(connection, true), crate::ISLAND, false, true);
    assert!(action.await.unwrap().is_err());
    assert_eq!(fixture.read()["result"]["isError"], true);
    assert_eq!(hub.view.phase, "failed");
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_mode_lists_only_question_tools_and_delivers_only_confirmed_text() {
    let mut fixture = Fixture::with_external(true).await;
    fixture.send(json!({"jsonrpc":"2.0","id":80,"method":"tools/list"}));
    let list = fixture.read();
    let names: Vec<_> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["ask_user", "vults_dictate"]);
    fixture.send(
        json!({"jsonrpc":"2.0","id":81,"method":"tools/call","params":{"name":"flock_list","arguments":{}}}),
    );
    assert_eq!(fixture.read()["result"]["isError"], true);
    let mut hub = Questions::default();
    hub.allow_external(true, None);
    let token = fixture.open(&mut hub, 82).await;
    let (work, action) = Fixture::action(&mut hub, &token, "send", "Exact external reply");
    let (connection, records) = work.unwrap();
    assert!(persist_at(fixture.dir.clone(), records).await);
    hub.handle(Command::Audited(connection, true), crate::ISLAND, false, true);
    assert_eq!(
        fixture.read()["result"]["content"][0]["text"],
        "Exact external reply"
    );
    while hub.active() {
        hub.handle(fixture.next().await, crate::ISLAND, false, true);
    }
    assert!(action.await.unwrap().is_ok());
    assert_eq!(hub.view.phase, "sent");
    let token = fixture.open(&mut hub, 83).await;
    hub.allow_external(false, Some(Instant::now()));
    assert_eq!(fixture.read()["result"]["isError"], true);
    let (_, late) = Fixture::action(&mut hub, &token, "send", "Late reply");
    assert!(late.await.unwrap().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_revocation_cannot_race_between_the_consent_read_and_reply_handoff() {
    use std::sync::{Arc, Mutex, mpsc as channel};
    let mut fixture = Fixture::with_external(true).await;
    let mut hub = Questions::default();
    hub.allow_external(true, None);
    let token = fixture.open(&mut hub, 90).await;
    let (work, action) = Fixture::action(&mut hub, &token, "send", "Already confirmed text");
    let (connection, records) = work.unwrap();
    assert!(persist_at(fixture.dir.clone(), records).await);
    let settings = Arc::new(Mutex::new(crate::settings::Settings {
        question_agents: true,
        ..Default::default()
    }));
    let changed = Arc::new(Mutex::new(None));
    let (go, started) = channel::channel();
    let (checked, check) = channel::channel();
    let setter_settings = settings.clone();
    let setter_changed = changed.clone();
    let setter = std::thread::spawn(move || {
        started.recv().unwrap();
        checked.send(setter_settings.try_lock().is_err()).unwrap();
        let mut saved = setter_settings.lock().unwrap();
        let mut epoch = setter_changed.lock().unwrap();
        saved.question_agents = false;
        *epoch = Some(Instant::now());
    });
    with_consent(&settings, &changed, |enabled, epoch| {
        go.send(()).unwrap();
        assert!(
            check.recv_timeout(Duration::from_secs(2)).unwrap(),
            "Revocation cannot cross the admitted handoff"
        );
        hub.allow_external(enabled, epoch);
        hub.handle(Command::Audited(connection, true), crate::ISLAND, false, true);
    });
    setter.join().unwrap();
    with_consent(&settings, &changed, |enabled, epoch| {
        hub.allow_external(enabled, epoch);
    });
    // The handoff won the ordering: later revocation must report consumed/uncertain, never recall.
    assert!(action.await.unwrap().is_err());
    assert_eq!(hub.view.phase, "failed");
    assert_eq!(
        fixture.read()["result"]["content"][0]["text"],
        "Already confirmed text"
    );
}
