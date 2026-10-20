use super::*;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn reply(n: u64, text: &str) -> Reply<'_> {
    Reply {
        generation: Generation(n),
        source: Source::SpokenResponse,
        text,
        played_frames: 4800,
        total_frames: 4800,
        interrupted: false,
        synchronized: true,
    }
}
#[test]
fn default_off_and_only_complete_audible_questions_can_propose() {
    assert!(!format!("{:?}", reply(1, "Private follow-up words?")).contains("Private follow-up words"));
    let mut off = FollowUp::default();
    assert!(!off.completed(reply(1, "Continue?"), ms(100), ms(100)));
    assert!(off.take(Generation(2), ms(101)).is_none());
    for text in [
        "Continue?",
        "Tudo certo?",
        "\u{bf}Continuamos?",
        "\u{7ee7}\u{7eed}\u{5417}\u{ff1f}",
        "Continue? \n",
    ] {
        assert!(
            FollowUp::new(true).completed(reply(1, text), ms(100), ms(100)),
            "{text}"
        );
    }
    for text in [
        "",
        "?",
        "...",
        "Done.",
        "Question? Answer.",
        "A quoted \"question?\"",
        "Continue? [interrupted]",
    ] {
        assert!(
            !FollowUp::new(true).completed(reply(1, text), ms(100), ms(100)),
            "{text}"
        );
    }
    for source in [Source::Tool, Source::Notification, Source::Permission] {
        let mut event = reply(1, "Continue?");
        event.source = source;
        assert!(!FollowUp::new(true).completed(event, ms(100), ms(100)));
    }
    for (played, total, interrupted, synchronized) in [
        (0, 0, false, true),
        (4799, 4800, false, true),
        (4801, 4800, false, true),
        (4800, 4800, true, true),
        (4800, 4800, false, false),
    ] {
        let mut event = reply(1, "Continue?");
        event.played_frames = played;
        event.total_frames = total;
        event.interrupted = interrupted;
        event.synchronized = synchronized;
        assert!(!FollowUp::new(true).completed(event, ms(100), ms(100)));
    }
}
#[test]
fn exact_next_generation_and_original_deadline_survive_delayed_claims() {
    let mut policy = FollowUp::new(true);
    assert!(policy.completed(reply(7, "Continue?"), ms(1000), ms(1500)));
    assert!(policy.take(Generation(7), ms(2000)).is_none());
    let window = policy.take(Generation(8), ms(8000)).unwrap();
    assert_eq!(window.generation(), Generation(8));
    assert_eq!(window.remaining(ms(8000)), Some(ms(1000)));
    assert_eq!(window.remaining(ms(8999)), Some(ms(1)));
    assert!(window.remaining(ms(9000)).is_none());
    assert!(window.remaining(ms(7999)).is_none());
    assert!(policy.take(Generation(8), ms(8000)).is_none());
    assert!(!policy.completed(reply(7, "Continue?"), ms(8000), ms(8000)));
}
#[test]
fn repeated_stale_or_non_spoken_events_cannot_extend_or_replace_a_window() {
    let mut policy = FollowUp::new(true);
    assert!(policy.completed(reply(4, "Continue?"), ms(1000), ms(1000)));
    assert!(!policy.completed(reply(4, "A repeated question?"), ms(5000), ms(5000)));
    let mut tool = reply(4, "Tool question?");
    tool.source = Source::Tool;
    assert!(!policy.completed(tool, ms(6000), ms(6000)));
    assert!(!policy.completed(reply(3, "An old question?"), ms(7000), ms(7000)));
    assert_eq!(
        policy.take(Generation(5), ms(8000)).unwrap().remaining(ms(8000)),
        Some(ms(1000))
    );
}
#[test]
fn expiry_new_turns_and_revocation_discard_proposals() {
    for now in [ms(9000), ms(9001)] {
        let mut policy = FollowUp::new(true);
        assert!(policy.completed(reply(1, "Continue?"), ms(1000), ms(1000)));
        assert!(policy.take(Generation(2), now).is_none());
    }
    let mut policy = FollowUp::new(true);
    assert!(!policy.completed(reply(1, "Continue?"), ms(1000), ms(9000)));
    assert!(policy.completed(reply(2, "Continue?"), ms(10000), ms(10000)));
    assert!(!policy.completed(reply(3, "Done."), ms(11000), ms(11000)));
    assert!(policy.take(Generation(3), ms(11000)).is_none());
    assert!(policy.completed(reply(4, "Continue?"), ms(12000), ms(12000)));
    assert!(policy.take(Generation(6), ms(12000)).is_none());
    assert!(policy.take(Generation(5), ms(12000)).is_none());
    assert!(policy.completed(reply(6, "Continue?"), ms(13000), ms(13000)));
    policy.cancel();
    assert!(policy.take(Generation(7), ms(13000)).is_none());
    assert!(!policy.completed(reply(7, "Continue?"), ms(14000), ms(14000)));
}
#[test]
fn invalid_clocks_and_overflow_cannot_create_time_or_authority() {
    let mut policy = FollowUp::new(true);
    assert!(!policy.completed(reply(1, "Continue?"), ms(1001), ms(1000)));
    assert!(policy.completed(reply(1, "Continue?"), ms(1000), ms(1000)));
    assert!(policy.take(Generation(2), ms(999)).is_none());
    assert_eq!(
        policy.take(Generation(2), ms(1001)).unwrap().remaining(ms(1001)),
        Some(ms(7999))
    );
    assert!(!FollowUp::new(true).completed(reply(u64::MAX, "Continue?"), ms(1), ms(1)));
    assert!(!FollowUp::new(true).completed(reply(1, "Continue?"), Duration::MAX, Duration::MAX));
}

#[test]
fn a_later_listen_retires_missing_or_delayed_replies_before_they_can_propose() {
    for armed in [false, true] {
        let mut policy = FollowUp::new(true);
        if armed {
            assert!(policy.completed(reply(1, "Continue?"), ms(1000), ms(1000)));
        }
        assert!(policy.take(Generation(4), ms(1000)).is_none());
        assert!(!policy.completed(reply(2, "A delayed question?"), ms(1000), ms(1000)));
        assert!(!policy.completed(reply(3, "Another delayed question?"), ms(1000), ms(1000)));
        assert!(policy.take(Generation(3), ms(1000)).is_none());
        assert!(policy.completed(reply(4, "A current question?"), ms(1000), ms(1000)));
        assert!(policy.take(Generation(5), ms(1000)).is_some());
    }
    assert!(!FollowUp::new(true).completed(reply(0, "No started turn?"), ms(1), ms(1)));
}
