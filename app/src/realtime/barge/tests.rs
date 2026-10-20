use super::super::State;
use super::*;
use std::sync::atomic::AtomicUsize;
fn machine() -> vults_voice::turn::Turn {
    let mut turn = vults_voice::turn::Turn::new(vults_voice::turn::BargeIn::Headphones);
    turn.start(Duration::ZERO).unwrap();
    turn
}
fn fixture(
    analyze: impl FnMut(Kind, &[i16], &AtomicBool) -> std::result::Result<Evidence, ()> + Send + 'static,
) -> (Detector, Arc<State>) {
    let state = Arc::new(State::default());
    let (lease, epoch) = state.begin().unwrap();
    let check = state.clone();
    (
        Detector::start(Arc::new(lease), Arc::new(move || check.current(epoch)), analyze).unwrap(),
        state,
    )
}
fn result(detector: &mut Detector, send: &mpsc::Sender<Command>) {
    let until = Instant::now() + Duration::from_secs(1);
    while detector.job.is_some() && Instant::now() < until {
        detector.tick(send).unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(detector.job.is_none());
}
#[test]
fn routed_evidence_is_bounded_generation_bound_and_never_returns_probe_words() {
    let seen = Arc::new(AtomicUsize::new(0));
    let count = seen.clone();
    let (mut detector, _state) = fixture(move |kind, pcm, _| {
        assert!(pcm.len() <= WINDOW);
        let observation = count.fetch_add(1, Ordering::SeqCst);
        Ok(match kind {
            Kind::Speech => Evidence::Speech((observation == 0).then_some(0)),
            Kind::Words(_) => Evidence::Words(true),
        })
    });
    let mut turn = machine();
    let generation = turn.generation();
    let (send, mut receive) = mpsc::channel(8);
    detector.input(&vec![1; RATE]).unwrap();
    detector.tick(&send).unwrap();
    assert_eq!(seen.load(Ordering::SeqCst), 0);
    detector.speaking(generation);
    for _ in 0..10 {
        detector.input(&vec![1; RATE]).unwrap();
    }
    assert_eq!(detector.ring.len(), WINDOW);
    detector.tick(&send).unwrap();
    result(&mut detector, &send);
    assert!(
        matches!(receive.try_recv().unwrap(), Command::Signal { generation: g, signal: Signal::SpeechStarted } if g==generation)
    );
    turn.handle(
        generation,
        vults_voice::turn::Event::InputCommitted,
        Duration::from_millis(10),
    );
    turn.handle(
        generation,
        vults_voice::turn::Event::FirstAudioPlayed {
            at: Duration::from_millis(20),
        },
        Duration::from_millis(20),
    );
    let actions = turn.handle(
        generation,
        vults_voice::turn::Event::SpeechStarted,
        Duration::from_millis(600),
    );
    let action = *actions
        .iter()
        .find(|action| matches!(action.effect, Effect::ProbeInterruption { .. }))
        .unwrap();
    let Effect::ProbeInterruption { probe, .. } = action.effect else {
        unreachable!()
    };
    // A final VAD pass may observe the short phrase ending while its probe is pending.
    detector.input(&vec![0; RATE / 2]).unwrap();
    detector.next = Instant::now();
    detector.tick(&send).unwrap();
    detector.action(action);
    result(&mut detector, &send);
    assert!(matches!(
        receive.try_recv().unwrap(),
        Command::Signal {
            signal: Signal::SpeechEnded,
            ..
        }
    ));
    turn.handle(
        generation,
        vults_voice::turn::Event::SpeechEnded,
        Duration::from_millis(620),
    );
    detector.next = Instant::now();
    detector.tick(&send).unwrap();
    result(&mut detector, &send);
    assert!(
        matches!(receive.try_recv().unwrap(), Command::Signal { signal: Signal::Confirm(value), .. } if value==probe)
    );
    let actions = turn.handle(
        generation,
        vults_voice::turn::Event::InterruptConfirmed(probe),
        Duration::from_millis(700),
    );
    let mut replay = None;
    for action in actions {
        replay = detector.action(action).or(replay);
    }
    assert!(
        matches!(replay, Some(Event::Replay { generation: next, input }) if next != generation && !input.samples.is_empty() && input.samples.len()<=WINDOW)
    );
    assert!(detector.generation.is_none());
    assert!(receive.try_recv().is_err());
}
#[test]
fn cancelled_native_work_cannot_publish_and_retains_the_real_lease_until_exit() {
    let (entered, waiting) = sync::channel();
    let (release, released) = sync::channel();
    let (mut detector, state) = fixture(move |_, _, _| {
        entered.send(()).unwrap();
        released.recv().unwrap();
        Ok(Evidence::Speech(Some(0)))
    });
    let turn = machine();
    detector.speaking(turn.generation());
    detector.input(&vec![1; RATE]).unwrap();
    let (send, mut output) = mpsc::channel(4);
    detector.tick(&send).unwrap();
    waiting.recv_timeout(Duration::from_secs(1)).unwrap();
    let (closed, results) = mpsc::channel(1);
    drop(closed);
    detector.results = results;
    assert!(
        matches!(detector.tick(&send), Err(Failure::Input)),
        "A disconnected analysis owner fails closed"
    );
    drop(detector);
    assert_eq!(state.begin().err(), Some(Failure::Busy));
    release.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(1);
    while state.running.load(Ordering::SeqCst) && Instant::now() < until {
        std::thread::yield_now();
    }
    assert!(!state.running.load(Ordering::SeqCst));
    assert!(output.try_recv().is_err());
}

#[test]
fn changed_routes_stale_generations_and_expired_probes_cannot_confirm() {
    for mode in 0..3 {
        let (entered, waiting) = sync::channel();
        let (release, released) = sync::channel();
        let mut calls = 0;
        let (mut detector, state) = fixture(move |kind, _, _| {
            calls += 1;
            if calls > 1 {
                entered.send(()).unwrap();
                released.recv().unwrap();
            }
            Ok(match kind {
                Kind::Speech => Evidence::Speech(Some(0)),
                Kind::Words(_) => Evidence::Words(true),
            })
        });
        let mut turn = machine();
        let generation = turn.generation();
        let (send, mut signals) = mpsc::channel(8);
        detector.speaking(generation);
        detector.input(&vec![1; RATE]).unwrap();
        detector.tick(&send).unwrap();
        result(&mut detector, &send);
        signals.try_recv().unwrap();
        turn.handle(
            generation,
            vults_voice::turn::Event::InputCommitted,
            Duration::from_millis(10),
        );
        turn.handle(
            generation,
            vults_voice::turn::Event::FirstAudioPlayed {
                at: Duration::from_millis(20),
            },
            Duration::from_millis(20),
        );
        let probe = turn
            .handle(
                generation,
                vults_voice::turn::Event::SpeechStarted,
                Duration::from_millis(600),
            )
            .into_iter()
            .find(|action| matches!(action.effect, Effect::ProbeInterruption { .. }))
            .unwrap();
        detector.action(probe);
        detector.next = Instant::now();
        detector.tick(&send).unwrap();
        waiting.recv_timeout(Duration::from_secs(1)).unwrap();
        match mode {
            0 => {
                state.epoch.fetch_add(1, Ordering::SeqCst);
            }
            1 => {
                let actions = turn.handle(
                    generation,
                    vults_voice::turn::Event::InterruptRequested,
                    Duration::from_millis(610),
                );
                for action in actions {
                    detector.action(action);
                }
            }
            _ => {
                detector.pending.as_mut().unwrap().1 = Instant::now();
                detector.job.as_mut().unwrap().until = Instant::now();
            }
        }
        release.send(()).unwrap();
        if mode == 0 {
            assert!(matches!(detector.tick(&send), Err(Failure::Cancelled)));
        } else {
            result(&mut detector, &send);
            assert!(signals.try_recv().is_err());
        }
    }
}

#[tokio::test]
async fn the_native_pump_delivers_signals_and_generation_bound_replay_without_uploading() {
    let (detector, _state) = fixture(|kind, _, _| {
        Ok(match kind {
            Kind::Speech => Evidence::Speech(Some(0)),
            Kind::Words(_) => Evidence::Words(true),
        })
    });
    let (external, input) = mpsc::channel(4);
    let (output, mut events) = mpsc::channel(64);
    let (_cancel, cancel) = tokio::sync::watch::channel(false);
    let ready = Arc::new(AtomicBool::new(false));
    let supply = ready.clone();
    let (admit, wait) = tokio::sync::oneshot::channel();
    let mut supplied = false;
    let worker = tokio::spawn(pump(
        Channels {
            input,
            output,
            cancel,
        },
        detector,
        move || {
            if supply.load(Ordering::SeqCst) && !supplied {
                supplied = true;
                Ok(Some(vec![1; RATE]))
            } else {
                Ok(None)
            }
        },
        move |mut io| async move {
            let mut turn = machine();
            let generation = turn.generation();
            turn.handle(
                generation,
                vults_voice::turn::Event::InputCommitted,
                Duration::from_millis(10),
            );
            turn.handle(
                generation,
                vults_voice::turn::Event::FirstAudioPlayed {
                    at: Duration::from_millis(20),
                },
                Duration::from_millis(20),
            );
            io.output.send(Event::Speaking(generation)).await.unwrap();
            ready.store(true, Ordering::SeqCst);
            let Command::Signal {
                signal: Signal::SpeechStarted,
                ..
            } = io.input.recv().await.unwrap()
            else {
                panic!("Only detected speech is expected")
            };
            let actions = turn.handle(
                generation,
                vults_voice::turn::Event::SpeechStarted,
                Duration::from_millis(600),
            );
            for action in actions {
                io.output.send(Event::Action(action)).await.unwrap();
            }
            let Command::Signal {
                signal: Signal::Confirm(probe),
                ..
            } = io.input.recv().await.unwrap()
            else {
                panic!("Only a local word confirmation is expected")
            };
            for action in turn.handle(
                generation,
                vults_voice::turn::Event::InterruptConfirmed(probe),
                Duration::from_millis(700),
            ) {
                io.output.send(Event::Action(action)).await.unwrap();
            }
            wait.await.unwrap();
            assert!(
                io.input.try_recv().is_err(),
                "A probe cannot create/upload a Turn"
            );
            Ok(None)
        },
    ));
    let mut listened = None;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match events.recv().await.unwrap() {
                Event::Action(Action {
                    generation,
                    effect: Effect::Listen { .. },
                }) => listened = Some(generation),
                Event::Replay { generation, input } => {
                    assert_eq!(listened, Some(generation));
                    assert!(!input.samples.is_empty());
                    break;
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    admit.send(()).unwrap();
    assert!(worker.await.unwrap().unwrap().is_none());
    drop(external);
}

#[test]
fn empty_words_and_local_decode_errors_reject_without_commands_or_turns() {
    for fail in [false, true] {
        let (mut detector, _state) = fixture(move |kind, _, _| match kind {
            Kind::Speech => Ok(Evidence::Speech(Some(0))),
            Kind::Words(_) => {
                if fail {
                    Err(())
                } else {
                    Ok(Evidence::Words(false))
                }
            }
        });
        let mut turn = machine();
        let generation = turn.generation();
        let (send, mut signals) = mpsc::channel(8);
        detector.speaking(generation);
        detector.input(&vec![1; RATE]).unwrap();
        detector.tick(&send).unwrap();
        result(&mut detector, &send);
        signals.try_recv().unwrap();
        turn.handle(
            generation,
            vults_voice::turn::Event::InputCommitted,
            Duration::from_millis(10),
        );
        turn.handle(
            generation,
            vults_voice::turn::Event::FirstAudioPlayed {
                at: Duration::from_millis(20),
            },
            Duration::from_millis(20),
        );
        let action = turn
            .handle(
                generation,
                vults_voice::turn::Event::SpeechStarted,
                Duration::from_millis(600),
            )
            .into_iter()
            .find(|action| matches!(action.effect, Effect::ProbeInterruption { .. }))
            .unwrap();
        let Effect::ProbeInterruption { probe, .. } = action.effect else {
            unreachable!()
        };
        detector.action(action);
        detector.next = Instant::now();
        detector.tick(&send).unwrap();
        result(&mut detector, &send);
        assert!(
            matches!(signals.try_recv().unwrap(),Command::Signal {signal:Signal::Reject(p),..} if p==probe)
        );
        assert!(signals.try_recv().is_err());
    }
}

#[tokio::test]
async fn closed_native_input_finishes_the_driver_without_starting_detection() {
    let (detector, _state) = fixture(|_, _, _| panic!("A closed owner cannot analyze audio"));
    let (send, input) = mpsc::channel(1);
    drop(send);
    let (output, _events) = mpsc::channel(8);
    let (_cancel, cancel) = tokio::sync::watch::channel(false);
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        pump(
            Channels {
                input,
                output,
                cancel,
            },
            detector,
            || Ok(None),
            |mut io| async move {
                assert!(io.input.recv().await.is_none());
                Ok(None)
            },
        ),
    )
    .await
    .unwrap();
    assert!(result.unwrap().is_none());
}
