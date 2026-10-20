use super::*;

fn ms(at: u64) -> Duration {
    Duration::from_millis(at)
}
fn send(end: &mut EndOfTurn, event: Event<'_>, at: u64) -> Decision {
    end.handle(end.generation.unwrap(), event, ms(at))
}
fn paused(semantic: bool) -> EndOfTurn {
    let mut end = EndOfTurn::new(semantic);
    assert!(end.begin(Generation(1), Language::English, ms(0)));
    assert_eq!(send(&mut end, Event::SpeechStarted, 100), Decision::Wait);
    assert_eq!(
        send(&mut end, Event::Transcript("check the build"), 200),
        Decision::Wait
    );
    assert_eq!(send(&mut end, Event::SpeechEnded, 1000), Decision::Wait);
    end
}
fn ticket(decision: Decision) -> Ticket {
    let Decision::Score(ticket) = decision else {
        panic!("expected a score request, got {decision:?}")
    };
    ticket
}

#[test]
fn english_portuguese_and_other_language_policy_table() {
    for (language, text, expected) in [
        (Language::English, "check the build", BASE_SILENCE),
        (Language::English, "check the", EXTENDED_SILENCE),
        (Language::English, "and then OR...", EXTENDED_SILENCE),
        (Language::English, "well, um!", EXTENDED_SILENCE),
        (Language::English, "another", BASE_SILENCE),
        (Language::English, "this is formal", BASE_SILENCE),
        (Language::Portuguese, "verifique o build", BASE_SILENCE),
        (Language::Portuguese, "verifique o", EXTENDED_SILENCE),
        (Language::Portuguese, "sim, MAS...", EXTENDED_SILENCE),
        (Language::Portuguese, "ate", EXTENDED_SILENCE),
        (Language::Portuguese, "então!", EXTENDED_SILENCE), // check-english:allow
        (Language::Portuguese, "hã...", EXTENDED_SILENCE),  // check-english:allow
        (Language::Portuguese, "comando", BASE_SILENCE),
        (Language::Other, "and", BASE_SILENCE),
        (Language::Other, "你好", BASE_SILENCE),
        (Language::English, " ", BASE_SILENCE),
        (Language::Portuguese, "...", BASE_SILENCE),
    ] {
        assert_eq!(silence_delay(text, language), expected, "{language:?}: {text}");
    }
}

#[test]
fn silence_before_words_and_ongoing_voice_never_finish_a_turn() {
    let mut end = EndOfTurn::new(false);
    end.begin(Generation(1), Language::English, ms(0));
    assert_eq!(send(&mut end, Event::Tick, 5000), Decision::Wait);
    send(&mut end, Event::SpeechEnded, 5100);
    send(&mut end, Event::Transcript("..."), 5200);
    assert_eq!(send(&mut end, Event::Tick, 9000), Decision::Wait);
    send(&mut end, Event::SpeechStarted, 9100);
    send(&mut end, Event::Transcript("hello"), 9200);
    assert_eq!(send(&mut end, Event::Tick, 20000), Decision::Wait);
    send(&mut end, Event::SpeechEnded, 20100);
    assert_eq!(send(&mut end, Event::Tick, 21699), Decision::Wait);
    assert_eq!(send(&mut end, Event::Tick, 21700), Decision::End(Reason::Silence));
    assert_eq!(send(&mut end, Event::Tick, 23000), Decision::Wait);
}

#[test]
fn continuation_words_wait_longer_and_duplicate_edges_do_not_restart_silence() {
    let mut end = paused(false);
    send(&mut end, Event::Transcript("check it and"), 1100);
    send(&mut end, Event::SpeechEnded, 2000);
    assert_eq!(send(&mut end, Event::Tick, 3599), Decision::Wait);
    assert_eq!(send(&mut end, Event::Tick, 3600), Decision::End(Reason::Silence));
}

#[test]
fn partial_rewrites_do_not_keep_resetting_the_timer_but_actual_word_growth_does() {
    let mut end = paused(false);
    send(&mut end, Event::Transcript("check"), 1200);
    send(&mut end, Event::Transcript("check this build"), 1500);
    send(&mut end, Event::Transcript("check this build!"), 2000);
    assert_eq!(send(&mut end, Event::Tick, 2599), Decision::Wait);
    assert_eq!(send(&mut end, Event::Tick, 2600), Decision::End(Reason::Silence));

    let mut end = paused(false);
    send(&mut end, Event::Transcript("check this build again"), 2000);
    assert_eq!(send(&mut end, Event::Tick, 2600), Decision::Wait);
    assert_eq!(send(&mut end, Event::Tick, 3600), Decision::End(Reason::Silence));
}

#[test]
fn semantic_requests_are_paced_bound_to_tickets_and_accept_the_exact_threshold() {
    let mut end = paused(true);
    let first = ticket(send(&mut end, Event::Tick, 2600));
    assert_eq!(
        send(&mut end, Event::Score(first, Ok(0.49)), 2650),
        Decision::Wait
    );
    assert_eq!(send(&mut end, Event::Tick, 2799), Decision::Wait);
    let second = ticket(send(&mut end, Event::Tick, 2800));
    assert_ne!(first, second);
    assert_eq!(send(&mut end, Event::Score(first, Err(())), 2810), Decision::Wait);
    assert!(end.semantic_enabled());
    assert_eq!(send(&mut end, Event::Score(first, Ok(1.0)), 2820), Decision::Wait);
    assert_eq!(
        send(&mut end, Event::Score(second, Ok(0.5)), 2850),
        Decision::End(Reason::Semantic)
    );
    assert_eq!(
        send(&mut end, Event::Score(second, Ok(0.9)), 2860),
        Decision::Wait
    );
}

#[test]
fn unanswered_or_repeated_low_scores_cannot_extend_the_absolute_hold_limit() {
    for hung in [false, true] {
        let mut end = paused(true);
        let mut pending = ticket(send(&mut end, Event::Tick, 2600));
        if !hung {
            for at in [2800, 3000, 3200, 3400, 3600] {
                pending = ticket(send(&mut end, Event::Score(pending, Ok(0.0)), at));
            }
        }
        assert_eq!(send(&mut end, Event::Tick, 3799), Decision::Wait);
        assert_eq!(
            send(&mut end, Event::Score(pending, Ok(1.0)), 3800),
            Decision::End(Reason::HoldLimit)
        );
        assert_eq!(send(&mut end, Event::Tick, 4000), Decision::Wait);
    }
}

#[test]
fn renewed_speech_and_new_turns_revoke_old_results_without_disabling_the_model() {
    let mut end = paused(true);
    let old = ticket(send(&mut end, Event::Tick, 2600));
    send(&mut end, Event::SpeechStarted, 2700);
    assert_eq!(send(&mut end, Event::Score(old, Ok(1.0)), 2800), Decision::Wait);
    assert_eq!(send(&mut end, Event::Tick, 5000), Decision::Wait);
    send(&mut end, Event::SpeechEnded, 5100);
    let current = ticket(send(&mut end, Event::Tick, 6700));
    assert_ne!(old, current);
    assert_eq!(send(&mut end, Event::Score(old, Err(())), 6800), Decision::Wait);
    assert!(end.semantic_enabled());
    assert!(end.begin(Generation(2), Language::Portuguese, ms(6900)));
    assert_eq!(
        end.handle(Generation(1), Event::Score(current, Ok(1.0)), ms(7000)),
        Decision::Wait
    );
    assert!(!end.begin(Generation(1), Language::English, ms(7001)));
    assert!(!end.begin(Generation(2), Language::English, ms(7002)));
    assert_eq!(send(&mut end, Event::Tick, 10000), Decision::Wait);
}

#[test]
fn one_failure_or_invalid_score_falls_back_and_stays_disabled_for_the_conversation() {
    for result in [Err(()), Ok(f32::NAN), Ok(f32::INFINITY), Ok(-0.1), Ok(1.1)] {
        let mut end = paused(true);
        let pending = ticket(send(&mut end, Event::Tick, 2600));
        assert_eq!(
            send(&mut end, Event::Score(pending, result), 2700),
            Decision::End(Reason::Silence)
        );
        assert!(!end.semantic_enabled());
        assert!(end.begin(Generation(2), Language::English, ms(2800)));
        send(&mut end, Event::SpeechStarted, 2900);
        send(&mut end, Event::Transcript("hello"), 3000);
        send(&mut end, Event::SpeechEnded, 3100);
        assert_eq!(send(&mut end, Event::Tick, 4700), Decision::End(Reason::Silence));
        assert!(!end.semantic_enabled());
    }
}

#[test]
fn cancellation_clock_order_and_ticket_exhaustion_never_revive_or_wrap() {
    let mut end = paused(true);
    let old = ticket(send(&mut end, Event::Tick, 2600));
    assert_eq!(send(&mut end, Event::Score(old, Ok(1.0)), 2500), Decision::Wait);
    assert_eq!(send(&mut end, Event::Cancel, 2700), Decision::Wait);
    assert_eq!(send(&mut end, Event::Score(old, Ok(1.0)), 2800), Decision::Wait);
    assert!(!end.begin(Generation(2), Language::English, ms(2600)));
    assert!(end.begin(Generation(2), Language::English, ms(2900)));
    end.serial = u64::MAX;
    send(&mut end, Event::SpeechStarted, 3000);
    send(&mut end, Event::Transcript("hello"), 3100);
    send(&mut end, Event::SpeechEnded, 3200);
    assert_eq!(send(&mut end, Event::Tick, 4800), Decision::End(Reason::Silence));
    assert!(!end.semantic_enabled());
}

#[test]
fn scripted_detector_drives_the_real_turn_machine_once_after_voice_and_text_settle() {
    use crate::turn::{Effect, Event as TurnEvent, Phase, Turn};
    let mut turn = Turn::default();
    let generation = turn.start(ms(0)).unwrap().generation;
    let mut end = EndOfTurn::new(false);
    end.begin(generation, Language::Portuguese, ms(0));
    send(&mut end, Event::SpeechStarted, 100);
    send(&mut end, Event::Transcript("verifique o"), 200);
    send(&mut end, Event::SpeechEnded, 500);
    assert_eq!(send(&mut end, Event::Tick, 3000), Decision::Wait);
    send(&mut end, Event::SpeechStarted, 3050);
    send(&mut end, Event::Transcript("verifique o build"), 3200);
    send(&mut end, Event::SpeechEnded, 3500);
    if send(&mut end, Event::Tick, 5100) == Decision::End(Reason::Silence) {
        let actions = turn.handle(generation, TurnEvent::InputCommitted, ms(5100));
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].effect, Effect::Generate);
    } else {
        panic!("settled text should commit");
    }
    assert_eq!(turn.phase(), Phase::Thinking);
    assert_eq!(send(&mut end, Event::Tick, 6000), Decision::Wait);
}

#[test]
fn delayed_scheduler_and_revised_text_cannot_bypass_silence_or_extend_semantic_hold() {
    let mut end = paused(true);
    assert_eq!(
        send(&mut end, Event::Tick, 5000),
        Decision::End(Reason::HoldLimit)
    );

    let mut end = paused(true);
    let pending = ticket(send(&mut end, Event::Tick, 2600));
    send(&mut end, Event::Transcript("check the build and"), 2700);
    assert_eq!(
        send(&mut end, Event::Score(pending, Ok(1.0)), 2800),
        Decision::Wait
    );
    assert_eq!(send(&mut end, Event::Tick, 5299), Decision::Wait);
    assert_eq!(
        send(&mut end, Event::Tick, 5300),
        Decision::End(Reason::HoldLimit)
    );

    let mut end = paused(true);
    let pending = ticket(send(&mut end, Event::Tick, 2600));
    send(&mut end, Event::Transcript("check it and"), 2700);
    assert_eq!(
        send(&mut end, Event::Score(pending, Ok(0.9)), 2800),
        Decision::Wait
    );
    assert_eq!(send(&mut end, Event::Tick, 3599), Decision::Wait);
    let current = ticket(send(&mut end, Event::Tick, 3600));
    assert_eq!(
        send(&mut end, Event::Score(current, Ok(0.5)), 3650),
        Decision::End(Reason::Semantic)
    );
}

#[test]
fn worker_deadline_keeps_nominal_hold_and_disappears_when_ticket_is_revoked() {
    let mut end = paused(true);
    let pending = ticket(send(&mut end, Event::Tick, 2700));
    assert_eq!(end.semantic_deadline(pending), Some(ms(3800)));
    send(&mut end, Event::SpeechStarted, 2800);
    assert_eq!(end.semantic_deadline(pending), None);
}
