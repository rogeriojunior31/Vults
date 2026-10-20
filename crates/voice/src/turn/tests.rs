use super::*;
fn confirmed(turn: &mut Turn, at: u64) -> Vec<Effect> {
    send(turn, Event::InterruptConfirmed(turn.probe), at)
}
fn rejected(turn: &mut Turn, at: u64) -> Vec<Effect> {
    send(turn, Event::InterruptRejected(turn.probe), at)
}

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}
fn send(turn: &mut Turn, event: Event, at: u64) -> Vec<Effect> {
    turn.handle(turn.generation(), event, ms(at))
        .into_iter()
        .map(|a| a.effect)
        .collect()
}
fn speaking(mode: BargeIn) -> Turn {
    let mut turn = Turn::new(mode);
    assert_eq!(
        turn.start(ms(0)).unwrap().effect,
        Effect::Listen {
            replay: Duration::ZERO
        }
    );
    assert_eq!(send(&mut turn, Event::InputCommitted, 10), [Effect::Generate]);
    assert!(send(&mut turn, Event::FirstAudioPlayed { at: ms(2000) }, 2000).is_empty());
    turn
}

#[test]
fn playback_lock_starts_at_consumption_and_retains_only_live_suppressed_speech() {
    for (mode, lock) in [(BargeIn::Headphones, 500), (BargeIn::EchoCancelled, 1000)] {
        let mut turn = speaking(mode);
        send(&mut turn, Event::SpeechStarted, 2010);
        assert!(send(&mut turn, Event::Tick, 2000 + lock - 1).is_empty());
        assert_eq!(turn.phase(), Phase::SpeakingLocked);
        assert_eq!(
            send(&mut turn, Event::Tick, 2000 + lock),
            [Effect::ProbeInterruption {
                probe: Probe(1),
                replay: REPLAY
            }]
        );
        assert_eq!(turn.phase(), Phase::InterruptPending);
        assert!(send(&mut turn, Event::FirstAudioPlayed { at: ms(2000) }, 2001 + lock).is_empty());

        let mut turn = speaking(mode);
        send(&mut turn, Event::SpeechStarted, 2010);
        assert!(send(&mut turn, Event::SpeechEnded, 2000 + lock).is_empty());
        assert_eq!(turn.phase(), Phase::Armed);
        assert!(send(&mut turn, Event::Tick, 4000).is_empty());
    }
}

#[test]
fn speakers_disable_barge_in_and_pending_noise_times_out_without_cancelling() {
    let mut turn = speaking(BargeIn::Disabled);
    assert!(send(&mut turn, Event::SpeechStarted, 3000).is_empty());
    assert!(confirmed(&mut turn, 3100).is_empty());
    assert!(send(&mut turn, Event::Tick, 5000).is_empty());
    assert_eq!(turn.phase(), Phase::SpeakingLocked);

    let mut turn = speaking(BargeIn::Headphones);
    send(&mut turn, Event::SpeechStarted, 2600);
    send(&mut turn, Event::SpeechEnded, 2700);
    assert_eq!(confirmed(&mut turn, 4100), [Effect::ResetVad]);
    assert_eq!(turn.phase(), Phase::Armed);
    send(&mut turn, Event::SpeechStarted, 4200);
    assert_eq!(rejected(&mut turn, 4250), [Effect::ResetVad]);
    assert_eq!(turn.phase(), Phase::Armed);
}

#[test]
fn scripted_stt_model_and_output_drive_real_turns_and_reject_old_callbacks() {
    let mut turn = speaking(BargeIn::Headphones);
    let old = turn.generation();
    assert_eq!(
        send(&mut turn, Event::SpeechStarted, 2600),
        [Effect::ProbeInterruption {
            probe: Probe(1),
            replay: ms(260)
        }]
    );
    let actions = turn.handle(old, Event::InterruptConfirmed(turn.probe), ms(2700));
    assert_eq!(
        actions.iter().map(|a| a.effect).collect::<Vec<_>>(),
        [
            Effect::CancelResponse,
            Effect::ClearPlayback,
            Effect::ResetVad,
            Effect::Listen { replay: ms(260) }
        ]
    );
    assert!(actions[..3].iter().all(|a| a.generation == old));
    let new = actions[3].generation;
    assert_ne!(new, old);
    assert_eq!(turn.phase(), Phase::Listening);
    for event in [
        Event::ResponseFinished,
        Event::FirstAudioPlayed { at: ms(2000) },
        Event::PlaybackDrained,
        Event::InputCommitted,
        Event::Cancel,
    ] {
        assert!(turn.handle(old, event, ms(2800)).is_empty());
    }
    // Fake STT commit -> fake model completion -> output acknowledges the requested drain.
    assert_eq!(send(&mut turn, Event::InputCommitted, 2900), [Effect::Generate]);
    assert_eq!(
        send(&mut turn, Event::ResponseFinished, 3000),
        [Effect::DrainPlayback]
    );
    assert!(send(&mut turn, Event::FirstAudioPlayed { at: ms(3100) }, 3100).is_empty());
    assert_eq!(turn.phase(), Phase::Draining);
    assert_eq!(send(&mut turn, Event::PlaybackDrained, 4000), [Effect::ResetVad]);
    assert_eq!(turn.phase(), Phase::TailGrace);
    assert!(send(&mut turn, Event::Tick, 4449).is_empty());
    assert_eq!(
        send(&mut turn, Event::Tick, 4450),
        [Effect::ResetVad, Effect::Finished]
    );
    assert_eq!(turn.phase(), Phase::Idle);
    assert!(send(&mut turn, Event::SpeechStarted, 4500).is_empty());
}

#[test]
fn generation_end_is_not_playback_end_and_drain_remains_interruptible() {
    let mut turn = speaking(BargeIn::Headphones);
    assert!(send(&mut turn, Event::PlaybackDrained, 2100).is_empty());
    assert_eq!(
        send(&mut turn, Event::ResponseFinished, 2200),
        [Effect::DrainPlayback]
    );
    assert_eq!(turn.phase(), Phase::Draining);
    assert!(send(&mut turn, Event::ResponseFinished, 2300).is_empty());
    send(&mut turn, Event::SpeechStarted, 2400);
    assert_eq!(
        send(&mut turn, Event::Tick, 2500),
        [Effect::ProbeInterruption {
            probe: Probe(1),
            replay: REPLAY
        }]
    );
    assert_eq!(rejected(&mut turn, 2510), [Effect::ResetVad]);
    assert_eq!(turn.phase(), Phase::Draining);
    send(&mut turn, Event::SpeechStarted, 2520);
    let old = turn.generation();
    assert_eq!(
        send(&mut turn, Event::PlaybackDrained, 2550),
        [Effect::ResetVad, Effect::Listen { replay: REPLAY }]
    );
    assert_ne!(turn.generation(), old);
    assert_eq!(turn.phase(), Phase::Listening);
}

#[test]
fn speech_in_tail_grace_is_a_new_turn_without_cancelling_or_replaying_agent_tail() {
    let mut turn = speaking(BargeIn::Headphones);
    send(&mut turn, Event::ResponseFinished, 3000);
    send(&mut turn, Event::PlaybackDrained, 4000);
    let old = turn.generation();
    let actions = turn.handle(old, Event::SpeechStarted, ms(4449));
    assert_eq!(
        actions.iter().map(|a| a.effect).collect::<Vec<_>>(),
        [
            Effect::ResetVad,
            Effect::Listen {
                replay: Duration::ZERO
            }
        ]
    );
    assert_ne!(turn.generation(), old);
    assert!(turn.handle(old, Event::Tick, ms(4450)).is_empty());
    assert_eq!(turn.phase(), Phase::Listening);
}

#[test]
fn empty_output_finishes_only_after_drain_and_never_starts_capture() {
    let mut turn = Turn::default();
    assert!(
        turn.handle(turn.generation(), Event::SpeechStarted, ms(0))
            .is_empty()
    );
    turn.start(ms(1)).unwrap();
    send(&mut turn, Event::InputCommitted, 2);
    assert_eq!(
        send(&mut turn, Event::ResponseFinished, 3),
        [Effect::DrainPlayback]
    );
    assert_eq!(
        send(&mut turn, Event::PlaybackDrained, 4),
        [Effect::ResetVad, Effect::Finished]
    );
    assert_eq!(turn.phase(), Phase::Idle);
}

#[test]
fn cancellation_failure_and_restart_invalidate_every_phase() {
    for phase in [
        Phase::Listening,
        Phase::Thinking,
        Phase::SpeakingLocked,
        Phase::Armed,
        Phase::InterruptPending,
        Phase::Draining,
        Phase::TailGrace,
    ] {
        for event in [Event::Cancel, Event::Failed] {
            let mut turn = Turn::default();
            turn.start(ms(1)).unwrap();
            turn.phase = phase;
            let old = turn.generation();
            assert_eq!(
                send(&mut turn, event, 2),
                [
                    Effect::CancelResponse,
                    Effect::ClearPlayback,
                    Effect::ResetVad,
                    Effect::Finished
                ]
            );
            assert!(send(&mut turn, event, 3).is_empty());
            let new = turn.start(ms(4)).unwrap().generation;
            assert_ne!(new, old);
            assert!(turn.handle(old, Event::InputCommitted, ms(5)).is_empty());
        }
    }
}

#[test]
fn nonmonotonic_events_and_generation_exhaustion_fail_closed() {
    let mut turn = speaking(BargeIn::Headphones);
    assert!(send(&mut turn, Event::ResponseFinished, 1999).is_empty());
    assert!(turn.start(ms(2001)).is_none());
    send(&mut turn, Event::Cancel, 3000);
    assert!(turn.start(ms(2999)).is_none());
    turn.generation = Generation(u64::MAX);
    assert!(turn.start(ms(3001)).is_none());
    assert_eq!(turn.phase(), Phase::Idle);
}

#[test]
fn late_probe_results_cannot_confirm_or_reject_a_new_probe_in_the_same_turn() {
    let mut turn = speaking(BargeIn::Headphones);
    send(&mut turn, Event::SpeechStarted, 2600);
    let old = turn.probe;
    send(&mut turn, Event::Tick, 4100);
    send(&mut turn, Event::SpeechStarted, 4200);
    let current = turn.probe;
    assert_ne!(old, current);
    assert!(send(&mut turn, Event::InterruptConfirmed(old), 4250).is_empty());
    assert!(send(&mut turn, Event::InterruptRejected(old), 4260).is_empty());
    assert_eq!(turn.phase(), Phase::InterruptPending);
    assert!(confirmed(&mut turn, 4300).contains(&Effect::Listen { replay: REPLAY }));
    assert_eq!(turn.phase(), Phase::Listening);
}

#[test]
fn delayed_consumption_ack_uses_playback_time_and_retains_live_onsets() {
    for ended in [false, true] {
        let mut turn = Turn::new(BargeIn::Headphones);
        turn.start(ms(0)).unwrap();
        send(&mut turn, Event::InputCommitted, 10);
        assert!(send(&mut turn, Event::FirstAudioPlayed { at: ms(3001) }, 2700).is_empty());
        assert_eq!(turn.phase(), Phase::Thinking, "future playback times are invalid");
        send(&mut turn, Event::SpeechStarted, 2800);
        if ended {
            send(&mut turn, Event::SpeechEnded, 2900);
        }
        send(&mut turn, Event::Tick, 3100);
        let effects = send(&mut turn, Event::FirstAudioPlayed { at: ms(2000) }, 3200);
        if ended {
            assert!(effects.is_empty());
            assert_eq!(turn.phase(), Phase::Armed);
        } else {
            assert_eq!(
                effects,
                [Effect::ProbeInterruption {
                    probe: Probe(1),
                    replay: REPLAY
                }]
            );
            assert_eq!(turn.phase(), Phase::InterruptPending);
        }
    }
}

#[test]
fn unverified_input_clears_suppressed_onsets_and_pending_probes() {
    let mut turn = speaking(BargeIn::Headphones);
    send(&mut turn, Event::SpeechStarted, 2100);
    assert_eq!(send(&mut turn, Event::InputUnavailable, 2200), [Effect::ResetVad]);
    assert!(send(&mut turn, Event::Tick, 3000).is_empty());
    send(&mut turn, Event::SpeechStarted, 3100);
    let stale = turn.probe;
    assert_eq!(turn.phase(), Phase::InterruptPending);
    assert_eq!(send(&mut turn, Event::InputUnavailable, 3150), [Effect::ResetVad]);
    assert!(send(&mut turn, Event::InterruptConfirmed(stale), 3200).is_empty());
    assert_eq!(turn.phase(), Phase::Armed);
}
