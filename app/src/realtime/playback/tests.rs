use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

async fn without_commands<P: Output, F: Future<Output = Result<(), Failure>>>(
    channels: Channels,
    active: &(dyn Fn() -> bool + Send + Sync),
    mode: turn::BargeIn,
    open: impl FnMut(Arc<AtomicBool>) -> Result<P, Failure>,
    remote: impl FnOnce(transport::Channels) -> F,
) -> Result<Option<(Turn, Failure)>, Failure> {
    super::drive(None, channels, active, mode, open, remote, |_, _| {
        std::future::ready(Ok(None))
    })
    .await
}

struct Device {
    cursor: Cursor,
    automatic: bool,
    stopped: bool,
    verified: bool,
}
fn device(automatic: bool) -> Arc<Mutex<Device>> {
    Arc::new(Mutex::new(Device {
        cursor: Cursor {
            phase: OutputPhase::Playing,
            total_frames: 0,
            submitted_frames: 0,
            played_frames: 0,
            first_played: None,
            failure: None,
        },
        automatic,
        stopped: false,
        verified: true,
    }))
}
struct Fake(Arc<Mutex<Device>>);
impl Output for Fake {
    fn route_verified(&self) -> bool {
        self.0.lock().unwrap().verified
    }
    fn push(&self, pcm: &[i16]) -> Result<(), Failure> {
        let mut state = self.0.lock().unwrap();
        state.cursor.total_frames += pcm.len() as u64;
        state.cursor.submitted_frames = state.cursor.total_frames;
        state
            .cursor
            .first_played
            .get_or_insert_with(std::time::Instant::now);
        Ok(())
    }
    fn finish(&self) -> Result<(), Failure> {
        let mut state = self.0.lock().unwrap();
        state.cursor.phase = OutputPhase::Draining;
        if state.automatic {
            state.cursor.phase = OutputPhase::Finished;
            state.cursor.played_frames = state.cursor.total_frames;
        }
        Ok(())
    }
    fn cursor(&self) -> Cursor {
        self.0.lock().unwrap().cursor
    }
    fn stop(&self) -> Cursor {
        let mut state = self.0.lock().unwrap();
        state.stopped = true;
        if state.cursor.phase != OutputPhase::Finished {
            state.cursor.phase = OutputPhase::Stopped;
        }
        state.cursor
    }
}
impl Drop for Fake {
    fn drop(&mut self) {
        self.stop();
    }
}
fn channels(
    capacity: usize,
) -> (
    Channels,
    mpsc::Sender<Command>,
    mpsc::Receiver<Event>,
    watch::Sender<bool>,
) {
    let (send, input) = mpsc::channel(4);
    let (output, receive) = mpsc::channel(capacity);
    let (stop, cancel) = watch::channel(false);
    (
        Channels {
            input,
            output,
            cancel,
        },
        send,
        receive,
        stop,
    )
}
fn generation() -> Generation {
    turn::Turn::default().start(Duration::ZERO).unwrap().generation
}
fn input(generation: Generation, value: i16) -> Command {
    Command::Turn {
        generation,
        turn: Turn::new(vec![value; 4800]).unwrap(),
    }
}
fn identity(n: usize) -> Identity {
    Identity {
        attempt: format!("attempt_{n}"),
        response: format!("response_{n}"),
        item: format!("item_{n}"),
    }
}

#[tokio::test]
async fn prepared_native_capture_uses_the_current_generation_and_existing_local_command_gate() {
    for local in [false, true] {
        let (io, send, mut receive, _stop) = channels(64);
        let captured = Turn::from_capture(&vec![0.1; 3200], "A finalized native capture.".into()).unwrap();
        let expected = captured.samples();
        let uploaded = expected.clone();
        let worker = tokio::spawn(async move {
            super::drive(
                Some(captured),
                io,
                &|| true,
                turn::BargeIn::Disabled,
                |_| Ok(Fake(device(true))),
                move |mut remote| async move {
                    assert!(!local, "A consumed local capture cannot open a provider");
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    let (pcm, _) = upload(&mut remote).await;
                    assert_eq!(
                        pcm.as_slice(),
                        uploaded.as_ref(),
                        "The native capture reaches the existing upload path unchanged"
                    );
                    Err(Failure::Transport)
                },
                move |_, _| {
                    std::future::ready(Ok(local.then(|| LocalReply {
                        feedback: crate::voice_commands::Feedback::Done,
                        text: "Local command completed.".into(),
                        choices: Vec::new(),
                    })))
                },
            )
            .await
        });
        if local {
            let mut listening = 0;
            let mut completed = 0;
            loop {
                match next(&mut receive).await {
                    Event::Local { .. } => completed += 1,
                    Event::Action(action) if matches!(action.effect, Effect::Listen { .. }) => {
                        listening += 1;
                        if listening == 2 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            assert_eq!(completed, 1);
            drop(send);
            assert!(worker.await.unwrap().unwrap().is_none());
        } else {
            drop(send);
            let (original, reason) = worker.await.unwrap().unwrap().unwrap();
            assert_eq!(reason, Failure::Transport);
            assert!(Arc::ptr_eq(&original.samples(), &expected));
        }
    }
}

#[tokio::test]
async fn a_closed_consumer_before_generate_never_offers_local_recovery() {
    let (io, send, mut receive, _stop) = channels(64);
    let called = Arc::new(AtomicBool::new(false));
    let remote_called = called.clone();
    let worker = tokio::spawn(async move {
        without_commands(
            io,
            &|| true,
            turn::BargeIn::Disabled,
            |_| Ok(Fake(device(true))),
            move |_| async move {
                remote_called.store(true, Ordering::SeqCst);
                Err(Failure::Off)
            },
        )
        .await
    });
    assert!(matches!(
        next(&mut receive).await,
        Event::Action(Action {
            effect: Effect::Listen { .. },
            ..
        })
    ));
    drop(receive);
    send.send(input(generation(), 99)).await.unwrap();
    assert!(matches!(worker.await.unwrap(), Err(Failure::Output)));
    assert!(!called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn temporary_native_teardown_busy_retries_before_any_upload() {
    let (io, send, mut receive, _stop) = channels(64);
    send.send(input(generation(), 99)).await.unwrap();
    drop(send);
    let attempts = Arc::new(AtomicUsize::new(0));
    let opened = attempts.clone();
    let worker = tokio::spawn(async move {
        without_commands(
            io,
            &|| true,
            turn::BargeIn::Disabled,
            move |_| {
                if opened.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(Failure::Busy)
                } else {
                    Ok(Fake(device(true)))
                }
            },
            |mut remote| async move {
                remote.output.send(transport::Event::Ready).await.unwrap();
                upload(&mut remote).await;
                audio(&remote, 0).await;
                done(&remote, 0, true).await;
                assert!(remote.input.recv().await.is_none());
                Ok(())
            },
        )
        .await
    });
    assert_eq!(history(&mut receive).await.played_frames, 4800);
    assert!(worker.await.unwrap().unwrap().is_none());
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn terminal_or_unacknowledged_interruption_keeps_only_unsynchronized_heard_history() {
    for terminal_first in [false, true] {
        let (io, send, mut receive, _stop) = channels(128);
        send.send(input(generation(), 99)).await.unwrap();
        let state = device(false);
        let output = state.clone();
        let worker = tokio::spawn(async move {
            without_commands(
                io,
                &|| true,
                turn::BargeIn::Headphones,
                move |_| Ok(Fake(output.clone())),
                |mut remote| async move {
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    upload(&mut remote).await;
                    audio(&remote, 0).await;
                    done(&remote, 0, true).await;
                    if !terminal_first {
                        assert!(matches!(
                            remote.input.recv().await,
                            Some(transport::Command::Interrupt { .. })
                        ));
                    }
                    Err(Failure::Timeout)
                },
            )
            .await
        });
        loop {
            if matches!(next(&mut receive).await, Event::Settled(_)) {
                break;
            }
        }
        state.lock().unwrap().cursor.played_frames = 1681;
        tokio::time::sleep(Duration::from_millis(100)).await;
        send.send(Command::Signal {
            generation: generation(),
            signal: Signal::SpeechStarted,
        })
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        state.lock().unwrap().verified = false;
        send.send(Command::Signal {
            generation: generation(),
            signal: Signal::SpeechEnded,
        })
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(420)).await;
        while let Ok(event) = receive.try_recv() {
            assert!(!matches!(
                event,
                Event::Action(Action {
                    effect: Effect::ProbeInterruption { .. } | Effect::ClearPlayback,
                    ..
                })
            ));
        }
        assert!(!state.lock().unwrap().stopped);
        state.lock().unwrap().verified = true;
        send.send(Command::Signal {
            generation: generation(),
            signal: Signal::SpeechStarted,
        })
        .await
        .unwrap();
        let probe = loop {
            if let Event::Action(Action {
                effect: Effect::ProbeInterruption { probe, .. },
                ..
            }) = next(&mut receive).await
            {
                break probe;
            }
        };
        send.send(Command::Signal {
            generation: generation(),
            signal: Signal::Confirm(probe),
        })
        .await
        .unwrap();
        let record = history(&mut receive).await;
        assert_eq!(record.identity, identity(0));
        assert_eq!(record.text, "one two [interrupted]");
        assert_eq!(record.played_frames, 1680);
        assert!(!record.server_synchronized);
        assert!(matches!(worker.await.unwrap(), Err(Failure::Timeout)));
        assert!(state.lock().unwrap().stopped);
    }
}
async fn upload(io: &mut transport::Channels) -> (Vec<i16>, Option<Repair>) {
    let (mut samples, mut repair) = (Vec::new(), None);
    loop {
        match io.input.recv().await.unwrap() {
            transport::Command::Audio(pcm) => {
                assert!(pcm.len() <= 2400);
                samples.extend(pcm);
            }
            transport::Command::Repair(hint) => {
                assert!(repair.is_none());
                repair = Some(hint);
            }
            transport::Command::Commit => return (samples, repair),
            _ => panic!("unexpected upload command"),
        }
    }
}
async fn audio(io: &transport::Channels, n: usize) {
    io.output
        .send(transport::Event::Audio {
            identity: identity(n),
            pcm: vec![1; 4800],
        })
        .await
        .unwrap();
}
async fn done(io: &transport::Channels, n: usize, ok: bool) {
    io.output
        .send(transport::Event::Transcript(Transcript {
            identity: identity(n),
            text: "one two three four five six".into(),
            frames: 4800,
        }))
        .await
        .unwrap();
    io.output
        .send(transport::Event::Finished(Settlement {
            attempt: identity(n).attempt,
            actual_microusd: Some(10),
            ok,
            within_bounds: true,
        }))
        .await
        .unwrap();
    io.output.send(transport::Event::Ready).await.unwrap();
}
async fn next(rx: &mut mpsc::Receiver<Event>) -> Event {
    tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .unwrap()
        .unwrap()
}
async fn history(rx: &mut mpsc::Receiver<Event>) -> Record {
    loop {
        if let Event::History(record) = next(rx).await {
            return record;
        }
    }
}

#[tokio::test]
async fn admission_refusal_keeps_the_complete_original_and_opens_no_output() {
    let (io, send, _receive, _stop) = channels(32);
    send.send(input(generation(), 31415)).await.unwrap();
    let opened = AtomicUsize::new(0);
    let result = without_commands(
        io,
        &|| true,
        turn::BargeIn::Disabled,
        |_| {
            opened.fetch_add(1, Ordering::SeqCst);
            Ok(Fake(device(true)))
        },
        |mut remote| async move {
            assert!(remote.input.try_recv().is_err());
            Err(Failure::Budget)
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.0.samples().as_ref(), vec![31415; 4800]);
    assert_eq!(result.1, Failure::Budget);
    assert_eq!(opened.load(Ordering::SeqCst), 0);
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "requires VULTS_PLAYBACK_NULL_TEST and a private ALSA null configuration"]
async fn native_null_conversation_drains_cpal_into_matching_memory_history() {
    assert_eq!(std::env::var("VULTS_PLAYBACK_NULL_TEST").as_deref(), Ok("1"));
    let config = std::env::var_os("ALSA_CONFIG_PATH").expect("private ALSA configuration");
    assert!(std::fs::read_to_string(config).unwrap().contains("type null"));
    let (io, send, mut receive, _stop) = channels(64);
    send.send(input(generation(), 99)).await.unwrap();
    drop(send);
    let worker = tokio::spawn(async move {
        without_commands(
            io,
            &|| true,
            turn::BargeIn::Disabled,
            |cancel| {
                Playback::start(cancel).map_err(|error| match error {
                    vults_audio::playback::Failure::Busy => Failure::Busy,
                    _ => Failure::Output,
                })
            },
            |mut remote| async move {
                remote.output.send(transport::Event::Ready).await.unwrap();
                upload(&mut remote).await;
                audio(&remote, 0).await;
                done(&remote, 0, true).await;
                assert!(remote.input.recv().await.is_none());
                Ok(())
            },
        )
        .await
    });
    let record = history(&mut receive).await;
    assert_eq!(record.identity, identity(0));
    assert_eq!(record.played_frames, 4800);
    assert_eq!(record.text, "one two three four five six");
    assert!(record.server_synchronized && !record.interrupted);
    assert!(worker.await.unwrap().unwrap().is_none());
}

#[tokio::test]
async fn one_connection_releases_successful_audio_and_recovers_only_the_later_failed_turn() {
    let (io, send, mut receive, _stop) = channels(64);
    let Command::Turn { turn, .. } = input(generation(), 123) else {
        unreachable!()
    };
    let weak = Arc::downgrade(&turn.samples());
    send.send(Command::Turn {
        generation: generation(),
        turn,
    })
    .await
    .unwrap();
    let worker = tokio::spawn(async move {
        without_commands(
            io,
            &|| true,
            turn::BargeIn::Disabled,
            |_| Ok(Fake(device(true))),
            |mut remote| async move {
                remote.output.send(transport::Event::Ready).await.unwrap();
                assert_eq!(upload(&mut remote).await.0, vec![123; 4800]);
                audio(&remote, 0).await;
                done(&remote, 0, true).await;
                assert_eq!(upload(&mut remote).await.0, vec![456; 4800]);
                assert!(weak.upgrade().is_none());
                Err(Failure::Transport)
            },
        )
        .await
    });
    let record = history(&mut receive).await;
    assert!(!record.interrupted);
    assert_eq!(record.text, "one two three four five six");
    let next_generation = loop {
        if let Event::Action(Action {
            generation,
            effect: Effect::Listen { .. },
        }) = next(&mut receive).await
        {
            break generation;
        }
    };
    send.send(input(next_generation, 456)).await.unwrap();
    let (turn, reason) = worker.await.unwrap().unwrap().unwrap();
    assert_eq!(turn.samples().as_ref(), vec![456; 4800]);
    assert_eq!(reason, Failure::Transport);
}

#[tokio::test]
async fn completion_waits_for_device_drain_and_does_not_recover_a_completed_input() {
    let (io, send, mut receive, _stop) = channels(64);
    send.send(input(generation(), 123)).await.unwrap();
    let control = device(false);
    let output = control.clone();
    let worker = tokio::spawn(async move {
        without_commands(
            io,
            &|| true,
            turn::BargeIn::Disabled,
            move |_| Ok(Fake(output.clone())),
            |mut remote| async move {
                remote.output.send(transport::Event::Ready).await.unwrap();
                upload(&mut remote).await;
                audio(&remote, 0).await;
                done(&remote, 0, true).await;
                assert!(remote.input.recv().await.is_none());
                Err(Failure::Timeout)
            },
        )
        .await
    });
    loop {
        if matches!(next(&mut receive).await, Event::Settled(_)) {
            break;
        }
    }
    assert_eq!(control.lock().unwrap().cursor.phase, OutputPhase::Draining);
    assert!(!matches!(receive.try_recv(), Ok(Event::History(_))));
    {
        let mut state = control.lock().unwrap();
        state.cursor.played_frames = 4800;
        state.cursor.phase = OutputPhase::Finished;
    }
    assert_eq!(history(&mut receive).await.played_frames, 4800);
    drop(send);
    assert!(matches!(worker.await.unwrap(), Err(Failure::Timeout)));
}

#[tokio::test]
async fn cancellation_revocation_and_abort_stop_native_output_without_text_recovery() {
    for mode in 0..3 {
        let (io, send, _receive, stop) = channels(64);
        send.send(input(generation(), 99)).await.unwrap();
        let state = device(false);
        let output = state.clone();
        let enabled = Arc::new(AtomicBool::new(true));
        let active = enabled.clone();
        let worker = tokio::spawn(async move {
            without_commands(
                io,
                &move || active.load(Ordering::SeqCst),
                turn::BargeIn::Disabled,
                move |_| Ok(Fake(output.clone())),
                |mut remote| async move {
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    upload(&mut remote).await;
                    audio(&remote, 0).await;
                    remote.cancel.changed().await.unwrap();
                    Err(Failure::Cancelled)
                },
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            while state.lock().unwrap().cursor.total_frames == 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        match mode {
            0 => stop.send(true).unwrap(),
            1 => enabled.store(false, Ordering::SeqCst),
            _ => worker.abort(),
        }
        let result = worker.await;
        if mode < 2 {
            assert!(matches!(result.unwrap(), Err(Failure::Cancelled | Failure::Off)));
        } else {
            assert!(result.unwrap_err().is_cancelled());
        }
        assert!(state.lock().unwrap().stopped);
    }
}

#[tokio::test]
async fn failed_receipts_and_output_saturation_retain_the_original() {
    for saturation in [false, true] {
        let (io, send, _receive, _stop) = channels(if saturation { 2 } else { 64 });
        send.send(input(generation(), 42)).await.unwrap();
        let (turn, reason) = without_commands(
            io,
            &|| true,
            turn::BargeIn::Disabled,
            |_| Ok(Fake(device(true))),
            |mut remote| async move {
                remote.output.send(transport::Event::Ready).await.unwrap();
                if !saturation {
                    upload(&mut remote).await;
                    audio(&remote, 0).await;
                    done(&remote, 0, false).await;
                }
                remote.cancel.changed().await.unwrap();
                Err(Failure::Transport)
            },
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(turn.samples().as_ref(), vec![42; 4800]);
        assert_eq!(
            reason,
            if saturation {
                Failure::Output
            } else {
                Failure::Response
            }
        );
    }
}

#[tokio::test]
async fn interruption_freezes_once_waits_for_ack_and_repairs_only_the_next_authorized_turn() {
    for finished in [false, true] {
        let (io, send, mut receive, _stop) = channels(128);
        let capture = Turn::new(vec![123; 4800]).unwrap();
        send.send(Command::Turn {
            generation: generation(),
            turn: capture.clone(),
        })
        .await
        .unwrap();
        let state = device(false);
        let output = state.clone();
        let (ack, wait_ack) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(async move {
            without_commands(
                io,
                &|| true,
                turn::BargeIn::Headphones,
                move |_| {
                    let fresh = device(false).lock().unwrap().cursor;
                    output.lock().unwrap().cursor = fresh;
                    Ok(Fake(output.clone()))
                },
                |mut remote| async move {
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    upload(&mut remote).await;
                    audio(&remote, 0).await;
                    if finished {
                        done(&remote, 0, true).await;
                    }
                    let transport::Command::Interrupt {
                        identity: got,
                        played_frames,
                    } = remote.input.recv().await.unwrap()
                    else {
                        panic!("interrupt");
                    };
                    assert_eq!(got, identity(0));
                    assert_eq!(played_frames, 1680);
                    if !finished {
                        done(&remote, 0, false).await;
                    }
                    wait_ack.await.unwrap();
                    assert!(
                        remote.input.try_recv().is_err(),
                        "no next turn before server acknowledgement"
                    );
                    remote
                        .output
                        .send(transport::Event::Truncated(identity(0)))
                        .await
                        .unwrap();
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    let (pcm, hint) = upload(&mut remote).await;
                    assert_eq!(pcm, vec![456; 4800]);
                    let hint = hint.unwrap();
                    assert_eq!(hint.heard, "one two [interrupted]");
                    assert_eq!(hint.unheard, " three four five six");
                    Err(Failure::Transport)
                },
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            while state.lock().unwrap().cursor.total_frames == 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        state.lock().unwrap().cursor.played_frames = 1680;
        tokio::time::sleep(Duration::from_millis(520)).await;
        send.send(Command::Signal {
            generation: generation(),
            signal: Signal::SpeechStarted,
        })
        .await
        .unwrap();
        let probe = loop {
            if let Event::Action(Action {
                effect: Effect::ProbeInterruption { probe, .. },
                ..
            }) = next(&mut receive).await
            {
                break probe;
            }
        };
        send.send(Command::Signal {
            generation: generation(),
            signal: Signal::Confirm(probe),
        })
        .await
        .unwrap();
        let next_generation = loop {
            if let Event::Action(Action {
                generation,
                effect: Effect::Listen { .. },
            }) = next(&mut receive).await
                && generation != self::generation()
            {
                break generation;
            }
        };
        assert!(
            capture.handled(),
            "confirmed interruption consumes retained clones before producer completion"
        );
        send.send(Command::Turn {
            generation: next_generation,
            turn: capture,
        })
        .await
        .unwrap();
        send.send(input(next_generation, 456)).await.unwrap();
        send.send(Command::Signal {
            generation: generation(),
            signal: Signal::Confirm(probe),
        })
        .await
        .unwrap();
        send.send(input(generation(), 999)).await.unwrap();
        assert!(state.lock().unwrap().stopped);
        while let Ok(event) = receive.try_recv() {
            assert!(!matches!(event, Event::History(_)));
        }
        ack.send(()).unwrap();
        let record = history(&mut receive).await;
        assert_eq!(record.text, "one two [interrupted]");
        assert_eq!(record.identity, identity(0));
        assert!(record.interrupted);
        assert_eq!(record.played_frames, 1680);
        assert!(!format!("{record:?}").contains("one two"));
        let (original, reason) = worker.await.unwrap().unwrap().unwrap();
        assert_eq!(original.samples().as_ref(), vec![456; 4800]);
        assert_eq!(reason, Failure::Transport);
    }
}

#[tokio::test]
async fn local_commands_retire_generations_before_lazy_provider_admission() {
    let (io, send, mut receive, _stop) = channels(128);
    let prepared = Arc::new(AtomicUsize::new(0));
    let count = prepared.clone();
    let opened = Arc::new(AtomicBool::new(false));
    let remote_opened = opened.clone();
    let worker = tokio::spawn(async move {
        super::drive(
            None,
            io,
            &|| true,
            turn::BargeIn::Disabled,
            |_| Ok(Fake(device(true))),
            move |mut remote| {
                remote_opened.store(true, Ordering::SeqCst);
                async move {
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    let (pcm, _) = upload(&mut remote).await;
                    assert_eq!(pcm, vec![77; 4800], "only the unrecognized turn uploads");
                    Err(Failure::Transport)
                }
            },
            move |turn, _| {
                count.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Ok(if turn.samples()[0] == 77 {
                    None
                } else {
                    Some(LocalReply {
                        feedback: crate::voice_commands::Feedback::Done,
                        text: "Command completed.".into(),
                        choices: Vec::new(),
                    })
                }))
            },
        )
        .await
    });
    let first = match next(&mut receive).await {
        Event::Action(a) if matches!(a.effect, Effect::Listen { .. }) => a.generation,
        other => panic!("{other:?}"),
    };
    let capture = Turn::new(vec![11; 4800]).unwrap();
    send.send(Command::Turn {
        generation: first,
        turn: capture.clone(),
    })
    .await
    .unwrap();
    // A queued duplicate may never execute, upload or masquerade as the next input.
    send.send(input(first, 22)).await.unwrap();
    let mut local = 0;
    let second = loop {
        match next(&mut receive).await {
            Event::Local { generation, reply } => {
                assert_eq!(generation, first);
                assert_eq!(reply.feedback, crate::voice_commands::Feedback::Done);
                local += 1;
            }
            Event::Action(a) if matches!(a.effect, Effect::Listen { .. }) => break a.generation,
            Event::Action(a) => assert_ne!(a.effect, Effect::Generate),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(local, 1);
    assert_ne!(first, second);
    assert!(!opened.load(Ordering::SeqCst));
    assert!(capture.handled());
    // Relabelling the exact same capture with a fresh generation cannot replay it either.
    send.send(Command::Turn {
        generation: second,
        turn: capture,
    })
    .await
    .unwrap();
    send.send(input(second, 77)).await.unwrap();
    let (turn, reason) = worker.await.unwrap().unwrap().unwrap();
    assert_eq!(reason, Failure::Transport);
    assert_eq!(turn.samples()[0], 77);
    assert_eq!(prepared.load(Ordering::SeqCst), 2);
    assert!(opened.load(Ordering::SeqCst));
}

#[tokio::test]
async fn an_established_connection_never_receives_a_locally_consumed_followup() {
    let (io, send, mut receive, _stop) = channels(128);
    send.send(input(generation(), 77)).await.unwrap();
    let prepared = Arc::new(AtomicUsize::new(0));
    let count = prepared.clone();
    let worker = tokio::spawn(async move {
        super::drive(
            None,
            io,
            &|| true,
            turn::BargeIn::Disabled,
            |_| Ok(Fake(device(true))),
            |mut remote| async move {
                remote.output.send(transport::Event::Ready).await.unwrap();
                let (pcm, _) = upload(&mut remote).await;
                assert_eq!(pcm, vec![77; 4800]);
                audio(&remote, 0).await;
                done(&remote, 0, true).await;
                assert!(
                    remote.input.recv().await.is_none(),
                    "local command may not upload or retry"
                );
                Ok(())
            },
            move |turn, _| {
                count.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Ok(if turn.samples()[0] == 77 {
                    None
                } else {
                    Some(LocalReply {
                        feedback: crate::voice_commands::Feedback::Requested,
                        text: "Command requested.".into(),
                        choices: Vec::new(),
                    })
                }))
            },
        )
        .await
    });
    let second = loop {
        if let Event::Action(a) = next(&mut receive).await
            && matches!(a.effect, Effect::Listen { .. })
            && a.generation != generation()
        {
            break a.generation;
        }
    };
    send.send(input(second, 11)).await.unwrap();
    let third = loop {
        match next(&mut receive).await {
            Event::Action(a) if matches!(a.effect, Effect::Listen { .. }) => break a.generation,
            Event::Action(a) => assert_ne!(a.effect, Effect::Generate),
            Event::Local { reply, .. } => {
                assert_eq!(reply.feedback, crate::voice_commands::Feedback::Requested)
            }
            other => panic!("{other:?}"),
        }
    };
    assert_ne!(third, second);
    drop(send);
    assert!(worker.await.unwrap().unwrap().is_none());
    assert_eq!(prepared.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn pending_command_admission_is_cancellable_and_never_constructs_a_provider() {
    let (io, send, mut receive, stop) = channels(64);
    send.send(input(generation(), 11)).await.unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let mut entered = Some(entered);
    let called = Arc::new(AtomicBool::new(false));
    let remote_called = called.clone();
    let worker = tokio::spawn(async move {
        super::drive(
            None,
            io,
            &|| true,
            turn::BargeIn::Disabled,
            |_| Ok(Fake(device(true))),
            move |_| {
                remote_called.store(true, Ordering::SeqCst);
                std::future::ready(Ok(()))
            },
            move |_, _| {
                let entered = entered.take();
                async move {
                    let _ = entered.unwrap().send(());
                    std::future::pending().await
                }
            },
        )
        .await
    });
    assert!(matches!(
        next(&mut receive).await,
        Event::Action(Action {
            effect: Effect::Listen { .. },
            ..
        })
    ));
    tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .unwrap();
    stop.send(true).unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), worker)
            .await
            .unwrap()
            .unwrap(),
        Err(Failure::Cancelled)
    ));
    assert!(!called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn a_committed_local_action_never_recovers_after_failure_before_or_after_connection() {
    for connected in [false, true] {
        let (io, send, mut receive, _stop) = channels(128);
        let capture = Turn::new(vec![11; 4800]).unwrap();
        let original = capture.clone();
        let worker = tokio::spawn(async move {
            super::drive(
                None,
                io,
                &|| true,
                turn::BargeIn::Disabled,
                |_| Ok(Fake(device(true))),
                move |mut remote| async move {
                    assert!(connected, "consumed input cannot open the provider");
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    upload(&mut remote).await;
                    audio(&remote, 0).await;
                    done(&remote, 0, true).await;
                    assert!(remote.input.recv().await.is_none());
                    Ok(())
                },
                |turn, _| {
                    std::future::ready(if turn.samples()[0] == 77 {
                        Ok(None)
                    } else {
                        // The state owner committed; its execution acknowledgement was then lost.
                        turn.consume();
                        Err(Failure::Timeout)
                    })
                },
            )
            .await
        });
        let first = match next(&mut receive).await {
            Event::Action(a) => a.generation,
            other => panic!("{other:?}"),
        };
        let selected = if connected {
            send.send(input(first, 77)).await.unwrap();
            loop {
                if let Event::Action(a) = next(&mut receive).await
                    && a.generation != first
                    && matches!(a.effect, Effect::Listen { .. })
                {
                    break a.generation;
                }
            }
        } else {
            first
        };
        send.send(Command::Turn {
            generation: selected,
            turn: capture,
        })
        .await
        .unwrap();
        assert!(matches!(worker.await.unwrap(), Err(Failure::Timeout)));
        assert!(original.handled());
        while let Ok(event) = receive.try_recv() {
            assert!(
                !matches!(event, Event::Local { .. }),
                "no invented Done after lost receipt"
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn unresolved_effect_receipt_publishes_requested_before_the_native_waiter_expires() {
    use crate::realtime::routing::{Completion, Router};
    use crate::voice_commands::{Feedback, Report};
    let owner = Arc::new(crate::realtime::State::default());
    let (lease, _) = owner.begin().unwrap();
    let router = Router::new(Arc::new(lease), Arc::new(|_| true));
    let (io, send, mut receive, _stop) = channels(64);
    send.send(input(generation(), 11)).await.unwrap();
    let worker = tokio::spawn(async move {
        super::drive(
            None,
            io,
            &|| true,
            turn::BargeIn::Disabled,
            |_| Ok(Fake(device(true))),
            |_| async { panic!("a consumed native action must never construct the provider") },
            move |turn, deadline| {
                let router = router.clone();
                async move {
                    turn.consume();
                    let (reply, receive) = tokio::sync::oneshot::channel();
                    let report = Report {
                        feedback: Feedback::Requested,
                        text: String::new(),
                        choices: vec![],
                        effects: vec![],
                    };
                    let mut completion = Completion::new(
                        reply,
                        report,
                        router,
                        deadline - Duration::from_millis(50),
                        Arc::new(AtomicBool::new(true)),
                    );
                    let held = completion.work();
                    completion.finish(vults_core::i18n::Lang::En);
                    let report = receive.await.unwrap().unwrap().unwrap();
                    drop(held);
                    Ok(Some(LocalReply {
                        feedback: report.feedback,
                        text: report.text,
                        choices: report.choices,
                    }))
                }
            },
        )
        .await
    });
    let result = tokio::time::timeout(LOCAL_TIMEOUT, async {
        loop {
            match receive.recv().await.unwrap() {
                Event::Local { reply, .. } => break reply,
                Event::Action(a) => assert_ne!(a.effect, Effect::Generate),
                other => panic!("{other:?}"),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(result.feedback, Feedback::Requested);
    drop(send);
    assert!(worker.await.unwrap().unwrap().is_none());
}

#[tokio::test]
async fn tool_receipts_keep_thinking_and_final_audio_without_recovering_consumed_input() {
    for mode in 0..4 {
        let (io, send, mut receive, _stop) = channels(128);
        send.send(input(generation(), 77)).await.unwrap();
        let control = device(true);
        let output = control.clone();
        let (release, wait) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(async move {
            without_commands(
                io,
                &|| true,
                turn::BargeIn::Disabled,
                move |_| Ok(Fake(output.clone())),
                |mut remote| async move {
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    upload(&mut remote).await;
                    let receipt = || Settlement {
                        attempt: "tool_attempt".into(),
                        actual_microusd: Some(5),
                        ok: true,
                        within_bounds: true,
                    };
                    remote
                        .output
                        .send(transport::Event::ToolStep(receipt()))
                        .await
                        .unwrap();
                    wait.await.unwrap();
                    if mode == 1 {
                        return Err(Failure::Transport);
                    }
                    if mode == 2 {
                        remote
                            .output
                            .send(transport::Event::ToolStep(receipt()))
                            .await
                            .unwrap();
                        remote.cancel.changed().await.unwrap();
                        return Err(Failure::Transport);
                    }
                    let mut spoken = identity(1);
                    if mode == 3 {
                        spoken.attempt = "tool_attempt".into();
                    }
                    remote
                        .output
                        .send(transport::Event::Audio {
                            identity: spoken.clone(),
                            pcm: vec![1; 4800],
                        })
                        .await
                        .unwrap();
                    remote
                        .output
                        .send(transport::Event::Transcript(Transcript {
                            identity: spoken.clone(),
                            text: "one two three four five six".into(),
                            frames: 4800,
                        }))
                        .await
                        .unwrap();
                    remote
                        .output
                        .send(transport::Event::Finished(Settlement {
                            attempt: spoken.attempt,
                            actual_microusd: Some(8),
                            ok: true,
                            within_bounds: true,
                        }))
                        .await
                        .unwrap();
                    if mode == 3 {
                        remote.cancel.changed().await.unwrap();
                        return Err(Failure::Transport);
                    }
                    remote.output.send(transport::Event::Ready).await.unwrap();
                    assert!(remote.input.recv().await.is_none());
                    Ok(())
                },
            )
            .await
        });
        loop {
            match next(&mut receive).await {
                Event::Settled(receipt) => {
                    assert_eq!(receipt.attempt, "tool_attempt");
                    break;
                }
                Event::Action(a) => assert!(!matches!(a.effect, Effect::DrainPlayback | Effect::Finished)),
                Event::History(_) => panic!("a read is not spoken history"),
                _ => {}
            }
        }
        assert!(
            receive.try_recv().is_err(),
            "tool completion must not emit drain, history or a new Listen"
        );
        release.send(()).unwrap();
        if mode == 0 {
            let record = history(&mut receive).await;
            assert_eq!(record.identity, identity(1));
            assert_eq!(record.played_frames, 4800);
            drop(send);
            assert!(worker.await.unwrap().unwrap().is_none());
        } else {
            let result = worker.await.unwrap();
            assert!(
                matches!(result, Err(Failure::Transport)) && mode == 1
                    || matches!(result, Err(Failure::Protocol)) && mode >= 2,
                "no original audio/text recovery after a consumed read: {result:?}"
            );
            if mode == 3 {
                assert_eq!(control.lock().unwrap().cursor.total_frames, 0);
            }
            while let Ok(event) = receive.try_recv() {
                assert!(!matches!(event, Event::History(_)));
            }
        }
    }
}
