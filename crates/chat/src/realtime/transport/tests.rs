use super::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::protocol::Role;

#[derive(Debug, Default)]
struct Mock {
    wait_finish: AtomicBool,
    finishing: tokio::sync::Notify,
    release_finish: tokio::sync::Notify,
    slow_finish: AtomicBool,
    off: AtomicBool,
    spent: AtomicBool,
    no_key: AtomicBool,
    keys: AtomicUsize,
    attempts: AtomicUsize,
    settled: Mutex<Vec<(String, Option<u64>, bool)>>,
}
impl Gate for Mock {
    fn active(&self) -> bool {
        !self.off.load(Ordering::SeqCst)
    }
    fn now_ms(&self) -> i64 {
        1_792_368_000_000
    }
    fn reserve(&self, ceiling: u64) -> BoxFuture<'_, Option<Admission>> {
        Box::pin(async move {
            assert_eq!(ceiling, 179_200);
            if self.spent.load(Ordering::SeqCst) {
                return None;
            }
            Some(Admission::new(
                format!("attempt_{}", self.attempts.fetch_add(1, Ordering::SeqCst)),
                || {},
            ))
        })
    }
    fn key(&self) -> BoxFuture<'_, Option<String>> {
        Box::pin(async move {
            self.keys.fetch_add(1, Ordering::SeqCst);
            (!self.no_key.load(Ordering::SeqCst)).then(|| "synthetic-key".into())
        })
    }
    fn finish<'a>(&'a self, attempt: &'a str, actual: Option<u64>, ok: bool) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            self.settled.lock().unwrap().push((attempt.into(), actual, ok));
            self.finishing.notify_one();
            if self.wait_finish.load(Ordering::SeqCst) {
                self.release_finish.notified().await;
            }
            if self.slow_finish.load(Ordering::SeqCst) {
                std::future::pending::<()>().await;
            }
            true
        })
    }
}
fn plan() -> Plan {
    Plan::new("gpt-realtime-2.1-mini", 8192, 1024).unwrap()
}
fn channels(
    capacity: usize,
) -> (
    Channels,
    mpsc::Sender<Command>,
    mpsc::Receiver<Event>,
    watch::Sender<bool>,
) {
    let (input_tx, input) = mpsc::channel(8);
    let (output, output_rx) = mpsc::channel(capacity);
    let (cancel_tx, cancel) = watch::channel(false);
    (
        Channels {
            input,
            output,
            cancel,
        },
        input_tx,
        output_rx,
        cancel_tx,
    )
}
async fn read(socket: &mut WebSocketStream<DuplexStream>) -> Value {
    loop {
        let msg = timeout(Duration::from_secs(2), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if matches!(msg, Message::Ping(_) | Message::Pong(_)) {
            continue;
        }
        return serde_json::from_str(msg.to_text().unwrap()).unwrap();
    }
}
async fn write(socket: &mut WebSocketStream<DuplexStream>, mut event: Value) {
    if event["type"]
        .as_str()
        .is_some_and(|kind| kind.starts_with("response.output_audio"))
    {
        let response = event["response_id"].as_str().unwrap().to_owned();
        if event.get("item_id").is_none() {
            event["item_id"] = json!(format!("item_{response}"));
        }
        if event.get("output_index").is_none() {
            event["output_index"] = json!(0);
        }
        if event.get("content_index").is_none() {
            event["content_index"] = json!(0);
        }
    }
    socket
        .send(Message::Text(event.to_string().into()))
        .await
        .unwrap();
}
async fn config(socket: &mut WebSocketStream<DuplexStream>) {
    let update = read(socket).await;
    assert_eq!(update["type"], "session.update");
    socket.send(Message::Ping(vec![1].into())).await.unwrap();
    let mut session = update["session"].clone();
    session["id"] = json!("sess_test");
    session["model"] = json!("gpt-realtime-2.1-mini");
    write(socket, json!({"type":"session.updated","session":session})).await;
}
async fn request(socket: &mut WebSocketStream<DuplexStream>, response: &str) -> String {
    let mut frames = 0;
    loop {
        let event = read(socket).await;
        if event["type"] == "input_audio_buffer.commit" {
            break;
        }
        assert_eq!(event["type"], "input_audio_buffer.append");
        let audio = STANDARD.decode(event["audio"].as_str().unwrap()).unwrap();
        assert_eq!(&audio[..4], &[1, 0, 255, 255]);
        frames += audio.len() / 2;
    }
    assert_eq!(frames, 2400);
    let create = read(socket).await;
    assert_eq!(create["type"], "response.create");
    assert_eq!(create["response"]["max_output_tokens"], 1024);
    assert!(
        create["response"].get("instructions").is_none(),
        "repair must be consumed once"
    );
    let attempt = create["response"]["metadata"]["vults_attempt"]
        .as_str()
        .unwrap()
        .to_owned();
    write(socket,json!({"type":"response.created","response":{"id":response,"status":"in_progress","metadata":{"vults_attempt":attempt}}})).await;
    attempt
}
async fn response(socket: &mut WebSocketStream<DuplexStream>, response: &str, unknown: bool) {
    write(socket,json!({"type":"response.output_audio.delta","response_id":response,"delta":STANDARD.encode([1,0,255,255])})).await;
    finish_response(socket, response, "completed", unknown).await;
}
async fn finish_response(
    socket: &mut WebSocketStream<DuplexStream>,
    response: &str,
    status: &str,
    unknown: bool,
) {
    write(
        socket,
        json!({"type":"response.output_audio.done", "response_id":response}),
    )
    .await;
    write(socket, json!({"type":"response.output_audio_transcript.done", "response_id":response,"transcript":"One two three four five."})).await;
    let mut done: Value = serde_json::from_str(
        include_str!("../../../tests/fixtures/realtime-session.jsonl")
            .lines()
            .nth(3)
            .unwrap(),
    )
    .unwrap();
    done["response"]["id"] = json!(response);
    done["response"]["status"] = json!(status);
    done["response"]["output"] = json!([{"id":format!("item_{response}"),"type":"message","role":"assistant","content":[{"type":"audio","transcript":"One two three four five."}]}]);
    if unknown {
        done["response"]["usage"] = Value::Null;
    }
    write(socket, done).await;
}
fn pcm() -> Vec<i16> {
    [1_i16, -1].repeat(1200)
}

#[tokio::test]
async fn interrupt_before_or_after_production_truncates_the_exact_item_and_waits_for_ack() {
    for finished in [false, true] {
        let gate = Arc::new(Mock::default());
        let worker_gate = gate.clone();
        let (client, mut server) = socket_pair().await;
        let (io, tx, mut rx, _cancel) = channels(16);
        let worker =
            tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
        let (ack, wait_ack) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            config(&mut server).await;
            request(&mut server, "resp_0").await;
            write(&mut server,json!({"type":"response.output_audio.delta","response_id":"resp_0","delta":STANDARD.encode(vec![0_u8;9600])})).await;
            if finished {
                finish_response(&mut server, "resp_0", "completed", false).await;
            }
            let event = read(&mut server).await;
            if !finished {
                assert_eq!(event["type"], "response.cancel");
                assert_eq!(event["response_id"], "resp_0");
                finish_response(&mut server, "resp_0", "cancelled", false).await;
            } else {
                assert_eq!(event["type"], "conversation.item.truncate");
            }
            let truncate = if finished { event } else { read(&mut server).await };
            assert_eq!(truncate["type"], "conversation.item.truncate");
            assert_eq!(truncate["item_id"], "item_resp_0");
            assert_eq!(truncate["audio_end_ms"], 70);
            assert_eq!(truncate["content_index"], 0);
            wait_ack.await.unwrap();
            write(&mut server,json!({"type":"conversation.item.truncated","item_id":"item_resp_0","content_index":0,"audio_end_ms":70})).await;
            assert!(server.next().await.is_none_or(|event| event.is_err()));
        });
        next(&mut rx).await;
        turn(&tx).await;
        let Event::Audio { identity, .. } = next(&mut rx).await else {
            panic!("audio identity");
        };
        if finished {
            assert!(matches!(next(&mut rx).await, Event::Finished(_)));
            assert!(matches!(next(&mut rx).await, Event::Ready));
        }
        tx.send(Command::Interrupt {
            identity: identity.clone(),
            played_frames: 1681,
        })
        .await
        .unwrap();
        if !finished {
            assert!(matches!(next(&mut rx).await,Event::Finished(receipt) if !receipt.ok));
        }
        tokio::task::yield_now().await;
        assert!(
            rx.try_recv().is_err(),
            "no Ready before matching truncation acknowledgement"
        );
        ack.send(()).unwrap();
        assert!(matches!(next(&mut rx).await,Event::Truncated(got) if got==identity));
        assert!(matches!(next(&mut rx).await, Event::Ready));
        drop(tx);
        assert_eq!(worker.await.unwrap(), Ok(()));
        server.await.unwrap();
        assert_eq!(gate.settled.lock().unwrap().len(), 1);
        assert_eq!(gate.settled.lock().unwrap()[0].1, Some(2059));
    }
}

#[tokio::test(start_paused = true)]
async fn rejected_or_missing_truncation_ack_never_readies_or_refunds_a_settled_turn() {
    for bad in [false, true] {
        let gate = Arc::new(Mock::default());
        let worker_gate = gate.clone();
        let (client, mut server) = socket_pair().await;
        let (io, tx, mut rx, _cancel) = channels(16);
        let worker =
            tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
        let server = tokio::spawn(async move {
            config(&mut server).await;
            request(&mut server, "resp_0").await;
            response(&mut server, "resp_0", false).await;
            assert_eq!(read(&mut server).await["type"], "conversation.item.truncate");
            if bad {
                write(&mut server,json!({"type":"conversation.item.truncated","item_id":"wrong_item","content_index":0,"audio_end_ms":0})).await;
            }
            let _ = server.next().await;
        });
        next(&mut rx).await;
        turn(&tx).await;
        let Event::Audio { identity, .. } = next(&mut rx).await else {
            panic!("audio");
        };
        next(&mut rx).await;
        next(&mut rx).await;
        tx.send(Command::Interrupt {
            identity,
            played_frames: 0,
        })
        .await
        .unwrap();
        assert_eq!(
            worker.await.unwrap(),
            Err(if bad { Failure::Protocol } else { Failure::Timeout })
        );
        assert!(rx.try_recv().is_err());
        server.await.unwrap();
        assert_eq!(
            *gate.settled.lock().unwrap(),
            vec![("attempt_0".into(), Some(2059), true)]
        );
    }
}

#[tokio::test]
async fn changed_item_or_content_index_cannot_publish_audio() {
    for change in ["item_id", "content_index", "output_index"] {
        let gate = Arc::new(Mock::default());
        let worker_gate = gate.clone();
        let (client, mut server) = socket_pair().await;
        let (io, tx, mut rx, _cancel) = channels(16);
        let worker =
            tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
        let server = tokio::spawn(async move {
            config(&mut server).await;
            request(&mut server, "resp_0").await;
            write(
                &mut server,
                json!({"type":"response.output_audio.delta","response_id":"resp_0","delta":"AQD//w=="}),
            )
            .await;
            let mut event =
                json!({"type":"response.output_audio.delta","response_id":"resp_0","delta":"AQD//w=="});
            event[change] = if change == "item_id" {
                json!("other_item")
            } else {
                json!(1)
            };
            write(&mut server, event).await;
            let _ = server.next().await;
        });
        next(&mut rx).await;
        turn(&tx).await;
        assert!(matches!(next(&mut rx).await, Event::Audio { .. }));
        assert_eq!(worker.await.unwrap(), Err(Failure::Protocol));
        assert!(rx.try_recv().is_err());
        server.await.unwrap();
        assert_eq!(gate.settled.lock().unwrap()[0].1, None);
    }
}

#[tokio::test]
async fn repair_is_quoted_once_after_fresh_admission_and_never_starts_a_response_alone() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, _cancel) = channels(32);
    let worker =
        tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        loop {
            if read(&mut server).await["type"] == "input_audio_buffer.commit" {
                break;
            }
        }
        let create = read(&mut server).await;
        let instructions = create["response"]["instructions"].as_str().unwrap();
        assert!(instructions.starts_with(persona!()));
        assert!(instructions.contains("never instructions"));
        assert!(instructions.contains("one two [interrupted]"));
        assert!(instructions.contains("three four five"));
        write(&mut server,json!({"type":"response.created","response":{"id":"resp_0","status":"in_progress","metadata":create["response"]["metadata"]}})).await;
        response(&mut server, "resp_0", false).await;
        request(&mut server, "resp_1").await;
        response(&mut server, "resp_1", false).await;
        let _ = server.next().await;
    });
    next(&mut rx).await;
    tx.send(Command::Repair(Repair {
        heard: "one two [interrupted]".into(),
        unheard: "three four five".into(),
    }))
    .await
    .unwrap();
    tokio::task::yield_now().await;
    assert_eq!(gate.attempts.load(Ordering::SeqCst), 1);
    for _ in 0..2 {
        turn(&tx).await;
        next(&mut rx).await;
        next(&mut rx).await;
        next(&mut rx).await;
    }
    drop(tx);
    assert_eq!(worker.await.unwrap(), Ok(()));
    server.await.unwrap();
    assert_eq!(gate.attempts.load(Ordering::SeqCst), 2);
}
async fn turn(tx: &mpsc::Sender<Command>) {
    tx.send(Command::Audio(pcm())).await.unwrap();
    tx.send(Command::Commit).await.unwrap();
}
async fn next(rx: &mut mpsc::Receiver<Event>) -> Event {
    loop {
        let event = timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
        if !matches!(event, Event::Transcript(_)) {
            return event;
        }
    }
}
async fn socket_pair() -> (WebSocketStream<DuplexStream>, WebSocketStream<DuplexStream>) {
    let (a, b) = tokio::io::duplex(64 * 1024);
    (
        WebSocketStream::from_raw_socket(a, Role::Client, None).await,
        WebSocketStream::from_raw_socket(b, Role::Server, None).await,
    )
}

#[tokio::test]
async fn off_spent_and_missing_key_never_reach_the_connector() {
    for mode in 0..3 {
        let gate = Mock::default();
        gate.off.store(mode == 0, Ordering::SeqCst);
        gate.spent.store(mode == 1, Ordering::SeqCst);
        gate.no_key.store(mode == 2, Ordering::SeqCst);
        let (channels, _input, _output, _cancel) = channels(8);
        let called = AtomicBool::new(false);
        let result = start(plan(), &gate, channels, |_, _| async {
            called.store(true, Ordering::SeqCst);
            Err::<WebSocketStream<DuplexStream>, _>(Failure::Transport)
        })
        .await;
        assert_eq!(result, Err([Failure::Off, Failure::Budget, Failure::Key][mode]));
        assert!(!called.load(Ordering::SeqCst));
        assert_eq!(gate.keys.load(Ordering::SeqCst), usize::from(mode == 2));
        if mode == 2 {
            assert_eq!(gate.settled.lock().unwrap()[0].1, Some(0));
        }
    }
}

#[tokio::test]
async fn duplex_socket_streams_two_turns_and_settles_each_exact_attempt() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, _cancel) = channels(16);
    let worker = tokio::spawn(async move {
        start(plan(), &*worker_gate, io, |model, key| async move {
            assert_eq!(model, "gpt-realtime-2.1-mini");
            assert_eq!(key, "synthetic-key");
            Ok(client)
        })
        .await
    });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        for n in 0..2 {
            request(&mut server, &format!("resp_{n}")).await;
            response(&mut server, &format!("resp_{n}"), false).await;
        }
        let _ = server.next().await;
    });
    assert!(matches!(next(&mut rx).await, Event::Ready));
    for _ in 0..2 {
        turn(&tx).await;
        let Event::Audio { pcm, .. } = next(&mut rx).await else {
            panic!("Expected audio")
        };
        assert_eq!(pcm, vec![1, -1]);
        let Event::Finished(receipt) = next(&mut rx).await else {
            panic!("Expected settlement")
        };
        assert_eq!(receipt.actual_microusd, Some(2059));
        assert!(matches!(next(&mut rx).await, Event::Ready));
    }
    drop(tx);
    assert_eq!(worker.await.unwrap(), Ok(()));
    server.await.unwrap();
    assert_eq!(
        *gate.settled.lock().unwrap(),
        vec![
            ("attempt_0".into(), Some(2059), true),
            ("attempt_1".into(), Some(2059), true)
        ]
    );
}

#[tokio::test]
async fn revocation_and_cancel_interrupt_a_pending_response_and_keep_unknown_cost() {
    for revoke in [false, true] {
        let gate = Arc::new(Mock::default());
        let worker_gate = gate.clone();
        let (client, mut server) = socket_pair().await;
        let (io, tx, mut rx, cancel) = channels(8);
        let worker =
            tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            config(&mut server).await;
            request(&mut server, "resp_0").await;
            let _ = started_tx.send(());
            let _ = server.next().await;
        });
        next(&mut rx).await;
        turn(&tx).await;
        started_rx.await.unwrap();
        if revoke {
            gate.off.store(true, Ordering::SeqCst);
        } else {
            cancel.send(true).unwrap();
        }
        assert_eq!(
            timeout(Duration::from_secs(2), worker).await.unwrap().unwrap(),
            Err(if revoke { Failure::Off } else { Failure::Cancelled })
        );
        server.await.unwrap();
        assert_eq!(gate.settled.lock().unwrap()[0].1, None);
    }
}

#[tokio::test]
async fn unknown_usage_closes_transport_after_one_conservative_settlement() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, _cancel) = channels(8);
    let worker =
        tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        request(&mut server, "resp_0").await;
        response(&mut server, "resp_0", true).await;
        let _ = server.next().await;
    });
    next(&mut rx).await;
    turn(&tx).await;
    assert_eq!(worker.await.unwrap(), Err(Failure::Protocol));
    server.await.unwrap();
    assert_eq!(
        *gate.settled.lock().unwrap(),
        vec![("attempt_0".into(), None, true)]
    );
}

#[tokio::test]
async fn next_turn_cannot_send_audio_after_budget_is_spent() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, _cancel) = channels(8);
    let worker =
        tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        request(&mut server, "resp_0").await;
        response(&mut server, "resp_0", false).await;
        assert!(server.next().await.is_none_or(|v| v.is_err()));
    });
    next(&mut rx).await;
    turn(&tx).await;
    next(&mut rx).await;
    next(&mut rx).await;
    next(&mut rx).await;
    gate.spent.store(true, Ordering::SeqCst);
    tx.send(Command::Audio(pcm())).await.unwrap();
    assert_eq!(worker.await.unwrap(), Err(Failure::Budget));
    server.await.unwrap();
    assert_eq!(gate.settled.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn output_backpressure_stops_the_socket_instead_of_growing_a_queue() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, tx, _rx, _cancel) = channels(1);
    let worker =
        tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        request(&mut server, "resp_0").await;
        write(
            &mut server,
            json!({"type":"response.output_audio.delta","response_id":"resp_0","delta":"AQD//w=="}),
        )
        .await;
        let _ = server.next().await;
    });
    turn(&tx).await;
    assert_eq!(worker.await.unwrap(), Err(Failure::Output));
    server.await.unwrap();
    assert_eq!(gate.settled.lock().unwrap()[0].1, None);
}

#[tokio::test(start_paused = true)]
async fn stalled_configuration_expires_and_proven_unsent_inference_costs_zero() {
    let gate = Arc::new(Mock::default());
    let (client, mut server) = socket_pair().await;
    let (io, _tx, _rx, _cancel) = channels(8);
    let server = tokio::spawn(async move {
        read(&mut server).await;
        std::future::pending::<()>().await;
    });
    assert_eq!(
        start(plan(), &*gate, io, |_, _| async { Ok(client) }).await,
        Err(Failure::Timeout)
    );
    server.abort();
    assert_eq!(gate.settled.lock().unwrap()[0].1, Some(0));
}

#[tokio::test(start_paused = true)]
async fn a_delayed_known_receipt_is_never_replaced_by_a_weaker_unknown_settlement() {
    let gate = Arc::new(Mock::default());
    gate.slow_finish.store(true, Ordering::SeqCst);
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, _cancel) = channels(8);
    let worker =
        tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        request(&mut server, "resp_0").await;
        response(&mut server, "resp_0", false).await;
        let _ = server.next().await;
    });
    next(&mut rx).await;
    turn(&tx).await;
    assert_eq!(worker.await.unwrap(), Err(Failure::Ledger));
    server.await.unwrap();
    assert_eq!(
        *gate.settled.lock().unwrap(),
        vec![("attempt_0".into(), Some(2059), true)]
    );
}

#[tokio::test]
async fn unexpected_inference_before_manual_commit_cannot_be_refunded_as_zero() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, _tx, _rx, _cancel) = channels(8);
    let worker =
        tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        write(&mut server,json!({"type":"response.created","response":{"id":"unexpected","status":"in_progress","metadata":null}})).await;
        let _ = server.next().await;
    });
    assert_eq!(worker.await.unwrap(), Err(Failure::Protocol));
    server.await.unwrap();
    assert_eq!(gate.settled.lock().unwrap()[0].1, None);
}

#[tokio::test]
async fn cancellation_during_connect_refunds_only_proven_unsent_work() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let (io, _tx, _rx, cancel) = channels(8);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        start(plan(), &*worker_gate, io, |_, _| async move {
            let _ = entered_tx.send(());
            std::future::pending::<Result<WebSocketStream<DuplexStream>, Failure>>().await
        })
        .await
    });
    entered_rx.await.unwrap();
    cancel.send(true).unwrap();
    assert_eq!(worker.await.unwrap(), Err(Failure::Cancelled));
    assert_eq!(gate.settled.lock().unwrap()[0].1, Some(0));
}

#[tokio::test]
async fn malformed_and_unowned_audio_never_reaches_the_consumer() {
    for (response_id, delta, expected) in [
        ("wrong", "AQD//w==", Failure::Protocol),
        ("resp_0", "invalid base64", Failure::Protocol),
        ("resp_0", "AQ==", Failure::Output),
    ] {
        let gate = Arc::new(Mock::default());
        let worker_gate = gate.clone();
        let (client, mut server) = socket_pair().await;
        let (io, tx, mut rx, _cancel) = channels(8);
        let worker =
            tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
        let server = tokio::spawn(async move {
            config(&mut server).await;
            request(&mut server, "resp_0").await;
            write(
                &mut server,
                json!({"type":"response.output_audio.delta","response_id":response_id,"delta":delta}),
            )
            .await;
            let _ = server.next().await;
        });
        next(&mut rx).await;
        turn(&tx).await;
        assert_eq!(worker.await.unwrap(), Err(expected));
        server.await.unwrap();
        assert!(rx.try_recv().is_err());
        assert_eq!(gate.settled.lock().unwrap()[0].1, None);
    }
}

#[tokio::test]
async fn revocation_during_known_settlement_keeps_billing_but_publishes_no_late_ready() {
    let gate = Arc::new(Mock::default());
    gate.wait_finish.store(true, Ordering::SeqCst);
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, _cancel) = channels(8);
    let worker =
        tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        request(&mut server, "resp_0").await;
        response(&mut server, "resp_0", false).await;
        let _ = server.next().await;
    });
    next(&mut rx).await;
    turn(&tx).await;
    assert!(matches!(next(&mut rx).await, Event::Audio { .. }));
    gate.finishing.notified().await;
    gate.off.store(true, Ordering::SeqCst);
    gate.release_finish.notify_one();
    assert_eq!(worker.await.unwrap(), Err(Failure::Off));
    server.await.unwrap();
    assert!(rx.try_recv().is_err());
    assert_eq!(
        *gate.settled.lock().unwrap(),
        vec![("attempt_0".into(), Some(2059), true)]
    );
}

#[test]
fn abandoned_handoff_refunds_only_before_ownership_is_claimed_and_debug_redacts_pcm() {
    let dropped = Arc::new(AtomicUsize::new(0));
    let first = dropped.clone();
    drop(Admission::new("unused".into(), move || {
        first.fetch_add(1, Ordering::SeqCst);
    }));
    let second = dropped.clone();
    assert_eq!(
        Admission::new("claimed".into(), move || {
            second.fetch_add(1, Ordering::SeqCst);
        })
        .claim(),
        "claimed"
    );
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert!(!format!("{:?}", Command::Audio(vec![31415])).contains("31415"));
    assert!(
        !format!(
            "{:?}",
            Event::Audio {
                identity: Identity {
                    attempt: "opaque".into(),
                    response: "response".into(),
                    item: "item".into()
                },
                pcm: vec![31415]
            }
        )
        .contains("31415")
    );
}

#[tokio::test]
async fn abort_drops_the_transport_without_inventing_a_settlement() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, _cancel) = channels(8);
    let worker =
        tokio::spawn(async move { start(plan(), &*worker_gate, io, |_, _| async { Ok(client) }).await });
    config(&mut server).await;
    assert!(matches!(next(&mut rx).await, Event::Ready));
    turn(&tx).await;
    request(&mut server, "resp_0").await;
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let closed = timeout(Duration::from_secs(2), server.next()).await.unwrap();
    assert!(closed.is_none() || closed.is_some_and(|m| m.is_err()));
    assert!(gate.settled.lock().unwrap().is_empty());
}

#[tokio::test]
async fn buffered_audio_bursts_yield_to_an_inline_bounded_consumer() {
    let gate = Mock::default();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, cancel) = channels(4);
    turn(&tx).await;
    let server = tokio::spawn(async move {
        config(&mut server).await;
        request(&mut server, "resp_0").await;
        for _ in 0..10 {
            write(&mut server, json!({"type":"response.output_audio.delta","response_id":"resp_0","delta":STANDARD.encode([1,0,255,255])})).await;
        }
        response(&mut server, "resp_0", false).await;
        let _ = server.next().await;
    });
    let driver = start(plan(), &gate, io, |_, _| async { Ok(client) });
    tokio::pin!(driver);
    let mut audio = 0;
    let result = loop {
        tokio::select! { biased;
            event = rx.recv() => match event.unwrap() {
                Event::Audio { .. } => audio += 1,
                Event::Finished(_) => { cancel.send(true).unwrap(); },
                Event::Ready | Event::Transcript(_) => {},
                Event::Truncated(_) | Event::ToolStep(_) => panic!("unexpected non-audio result"),
            },
            result = &mut driver => break result,
        }
    };
    assert_eq!(result, Err(Failure::Cancelled));
    assert_eq!(audio, 11);
    assert_eq!(gate.settled.lock().unwrap()[0].1, Some(2059));
    server.await.unwrap();
}

#[derive(Debug)]
struct ReadTool {
    calls: Arc<AtomicUsize>,
    gate: Arc<Mock>,
    mode: u8,
}
impl crate::tools::Tool for ReadTool {
    fn name(&self) -> &str {
        "flock_list"
    }
    fn description(&self) -> &str {
        "Reads a cached snapshot."
    }
    fn schema(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            assert!(
                !self.gate.settled.lock().unwrap().is_empty(),
                "settlement precedes the read"
            );
            assert!(
                self.gate.attempts.load(Ordering::SeqCst) >= 2,
                "next admission precedes the read"
            );
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.mode == 2 {
                return std::future::pending().await;
            }
            if self.mode == 1 {
                return Err("x".repeat(crate::tools::MAX_RESULT + 50));
            }
            Ok(r#"{"sessions":[]}"#.into())
        })
    }
}
async fn tool_response(
    socket: &mut WebSocketStream<DuplexStream>,
    response: &str,
    call: &str,
    args: &str,
    status: &str,
) {
    let item = json!({"id":format!("item_{call}"),"type":"function_call","status":"completed","name":"flock_list","call_id":call,"arguments":args});
    let mut added = item.clone();
    added["status"] = json!("in_progress");
    added["arguments"] = json!("");
    write(
        socket,
        json!({"type":"response.output_item.added","response_id":response,"output_index":0,"item":added}),
    )
    .await;
    write(socket,json!({"type":"response.function_call_arguments.delta","response_id":response,"output_index":0,"item_id":item["id"],"call_id":call,"delta":args})).await;
    write(socket,json!({"type":"response.function_call_arguments.done","response_id":response,"output_index":0,"item_id":item["id"],"call_id":call,"name":"flock_list","arguments":args})).await;
    write(socket,json!({"type":"response.output_item.done","response_id":response,"output_index":0,"item":item.clone()})).await;
    let mut done: Value = serde_json::from_str(
        include_str!("../../../tests/fixtures/realtime-session.jsonl")
            .lines()
            .nth(3)
            .unwrap(),
    )
    .unwrap();
    done["response"]["id"] = json!(response);
    done["response"]["status"] = json!(status);
    done["response"]["output"] = json!([item]);
    write(socket, done).await;
}
async fn continuation(socket: &mut WebSocketStream<DuplexStream>, response: &str) -> String {
    let create = read(socket).await;
    assert_eq!(create["type"], "response.create");
    let attempt = create["response"]["metadata"]["vults_attempt"]
        .as_str()
        .unwrap()
        .to_owned();
    write(socket,json!({"type":"response.created","response":{"id":response,"status":"in_progress","metadata":{"vults_attempt":attempt}}})).await;
    attempt
}
#[tokio::test]
async fn completed_tool_steps_settle_reserve_read_once_and_continue_without_another_upload() {
    for mode in [0, 1] {
        let gate = Arc::new(Mock::default());
        let worker_gate = gate.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let plan = plan()
            .with_tools(vec![Arc::new(ReadTool {
                calls: calls.clone(),
                gate: gate.clone(),
                mode,
            })])
            .unwrap();
        let (client, mut server) = socket_pair().await;
        let (io, tx, mut rx, _cancel) = channels(16);
        let worker =
            tokio::spawn(async move { start(plan, &*worker_gate, io, |_, _| async { Ok(client) }).await });
        let server = tokio::spawn(async move {
            config(&mut server).await;
            request(&mut server, "resp_tool").await;
            tool_response(&mut server, "resp_tool", "call_one", "{}", "completed").await;
            let result = read(&mut server).await;
            assert_eq!(result["type"], "conversation.item.create");
            assert_eq!(result["item"]["type"], "function_call_output");
            assert_eq!(result["item"]["call_id"], "call_one");
            assert!(result["item"]["output"].as_str().unwrap().len() <= crate::tools::MAX_RESULT);
            let payload: Value = serde_json::from_str(result["item"]["output"].as_str().unwrap()).unwrap();
            assert_eq!(payload["is_error"], mode == 1);
            assert_eq!(payload["outside"], false);
            assert!(payload["text"].as_str().unwrap().len() <= crate::tools::MAX_RESULT + 64);
            assert_eq!(continuation(&mut server, "resp_audio").await, "attempt_1");
            response(&mut server, "resp_audio", false).await;
            let _ = server.next().await;
        });
        assert!(matches!(next(&mut rx).await, Event::Ready));
        turn(&tx).await;
        assert!(
            matches!(next(&mut rx).await,Event::ToolStep(ref receipt) if receipt.attempt=="attempt_0" && receipt.ok)
        );
        assert!(matches!(next(&mut rx).await, Event::Audio { .. }));
        assert!(matches!(next(&mut rx).await,Event::Finished(ref receipt) if receipt.attempt=="attempt_1"));
        assert!(matches!(next(&mut rx).await, Event::Ready));
        drop(tx);
        assert_eq!(worker.await.unwrap(), Ok(()));
        server.await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(gate.settled.lock().unwrap().len(), 2);
        assert!(
            gate.settled
                .lock()
                .unwrap()
                .iter()
                .all(|(_, actual, _)| *actual == Some(2059))
        );
    }
}
#[tokio::test]
async fn incomplete_invalid_duplicate_or_unaffordable_calls_cannot_execute_or_replay() {
    for mode in 0..5 {
        let gate = Arc::new(Mock::default());
        let worker_gate = gate.clone();
        let server_gate = gate.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let plan = plan()
            .with_tools(vec![Arc::new(ReadTool {
                calls: calls.clone(),
                gate: gate.clone(),
                mode: 0,
            })])
            .unwrap();
        let (client, mut server) = socket_pair().await;
        let (io, tx, mut rx, _cancel) = channels(16);
        let worker =
            tokio::spawn(async move { start(plan, &*worker_gate, io, |_, _| async { Ok(client) }).await });
        let server = tokio::spawn(async move {
            config(&mut server).await;
            request(&mut server, "resp_tool").await;
            if mode == 2 {
                server_gate.spent.store(true, Ordering::SeqCst);
            }
            tool_response(
                &mut server,
                "resp_tool",
                "call_one",
                if mode == 1 { r#"{"unexpected":"x"}"# } else { "{}" },
                if mode == 0 { "cancelled" } else { "completed" },
            )
            .await;
            if mode >= 3 {
                let result = read(&mut server).await;
                assert_eq!(result["item"]["call_id"], "call_one");
                continuation(&mut server, "resp_next").await;
                if mode == 4 {
                    server.close(None).await.unwrap();
                    return;
                }
                if mode == 3 {
                    // Same call ID under a fresh response/attempt is still consumed.
                    let item = json!({"id":"different_item","type":"function_call","name":"flock_list","call_id":"call_one","arguments":"","status":"in_progress"});
                    write(&mut server,json!({"type":"response.output_item.added","response_id":"resp_next","output_index":0,"item":item})).await;
                }
            }
            let _ = server.next().await;
        });
        next(&mut rx).await;
        turn(&tx).await;
        let result = worker.await.unwrap();
        assert!(result.is_err());
        if mode != 1 {
            server.await.unwrap();
        } else {
            server.abort();
            let _ = server.await;
        }
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(mode >= 3));
        assert_eq!(gate.settled.lock().unwrap().len(), if mode >= 3 { 2 } else { 1 });
        if mode >= 3 {
            assert_eq!(
                gate.settled.lock().unwrap()[1].1,
                None,
                "uncertain continuation stays charged"
            );
        }
    }
}
#[tokio::test]
async fn revocation_during_a_read_cancels_it_and_refunds_only_unsent_continuation() {
    let gate = Arc::new(Mock::default());
    let worker_gate = gate.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let plan = plan()
        .with_tools(vec![Arc::new(ReadTool {
            calls: calls.clone(),
            gate: gate.clone(),
            mode: 2,
        })])
        .unwrap();
    let (client, mut server) = socket_pair().await;
    let (io, tx, mut rx, cancel) = channels(16);
    let worker =
        tokio::spawn(async move { start(plan, &*worker_gate, io, |_, _| async { Ok(client) }).await });
    let server = tokio::spawn(async move {
        config(&mut server).await;
        request(&mut server, "resp_tool").await;
        tool_response(&mut server, "resp_tool", "call_one", "{}", "completed").await;
        let _ = server.next().await;
    });
    next(&mut rx).await;
    turn(&tx).await;
    assert!(matches!(next(&mut rx).await, Event::ToolStep(_)));
    timeout(Duration::from_secs(2), async {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    cancel.send(true).unwrap();
    assert_eq!(worker.await.unwrap(), Err(Failure::Cancelled));
    server.await.unwrap();
    let settled = gate.settled.lock().unwrap();
    assert_eq!(settled.len(), 2);
    assert_eq!(settled[0].1, Some(2059));
    assert_eq!(settled[1].1, Some(0));
}
