use super::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::io::DuplexStream;
use tokio_tungstenite::tungstenite::protocol::Role;

#[derive(Default)]
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
async fn write(socket: &mut WebSocketStream<DuplexStream>, event: Value) {
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
    let attempt = create["response"]["metadata"]["vults_attempt"]
        .as_str()
        .unwrap()
        .to_owned();
    write(socket,json!({"type":"response.created","response":{"id":response,"status":"in_progress","metadata":{"vults_attempt":attempt}}})).await;
    attempt
}
async fn response(socket: &mut WebSocketStream<DuplexStream>, response: &str, unknown: bool) {
    write(socket,json!({"type":"response.output_audio.delta","response_id":response,"delta":STANDARD.encode([1,0,255,255])})).await;
    let mut done: Value = serde_json::from_str(
        include_str!("../../../tests/fixtures/realtime-session.jsonl")
            .lines()
            .nth(3)
            .unwrap(),
    )
    .unwrap();
    done["response"]["id"] = json!(response);
    if unknown {
        done["response"]["usage"] = Value::Null;
    }
    write(socket, done).await;
}
fn pcm() -> Vec<i16> {
    [1_i16, -1].repeat(1200)
}
async fn turn(tx: &mpsc::Sender<Command>) {
    tx.send(Command::Audio(pcm())).await.unwrap();
    tx.send(Command::Commit).await.unwrap();
}
async fn next(rx: &mut mpsc::Receiver<Event>) -> Event {
    timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap()
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
                attempt: "opaque".into(),
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
                Event::Ready => {},
            },
            result = &mut driver => break result,
        }
    };
    assert_eq!(result, Err(Failure::Cancelled));
    assert_eq!(audio, 11);
    assert_eq!(gate.settled.lock().unwrap()[0].1, Some(2059));
    server.await.unwrap();
}
