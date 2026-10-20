use super::*;
fn segment(text: &str, start_frame: u64, end_frame: u64) -> Segment<'_> {
    Segment {
        text,
        start_frame,
        end_frame,
    }
}

#[test]
fn splits_complete_and_inflight_segments_at_whole_words_without_losing_source_text() {
    let segments = [
        segment("First sentence. ", 0, 100),
        segment("  one two three four.\n", 100, 500),
        segment("Last words.", 600, 800),
    ];
    let mut heard = split(&segments, 300, 800, true).unwrap();
    assert_eq!(heard.prefix, "First sentence.   one two");
    assert_eq!(heard.unheard, " three four.\nLast words.");
    assert_eq!(
        heard.prefix.clone() + &heard.unheard,
        segments.iter().map(|s| s.text).collect::<String>()
    );
    assert_eq!(heard.history_text(), "First sentence.   one two [interrupted]");
    assert_eq!(heard.unheard_words, 4);
    assert_eq!(heard.take_nudge().as_deref(), Some(" three four.\nLast words."));
    assert!(heard.take_nudge().is_none());
}

#[test]
fn zero_played_never_claims_text_and_complete_drain_keeps_the_original() {
    let text = "  The complete response.\n";
    let segments = [segment(text, 0, 2400)];
    let zero = split(&segments, 0, 2400, true).unwrap();
    assert_eq!(zero.prefix, "");
    assert_eq!(zero.unheard, text);
    assert_eq!(zero.history_text(), "[interrupted]");
    let full = split(&segments, 2400, 2400, false).unwrap();
    assert_eq!(full.prefix, text);
    assert_eq!(full.history_text(), text);
    assert!(full.unheard.is_empty() && !full.interrupted);
    assert_eq!(split(&[], 0, 0, false).unwrap().history_text(), "");
}

#[test]
fn word_floor_unicode_and_nudge_threshold_are_conservative() {
    for (text, played, expected) in [
        ("alpha beta gamma", 33, ""),
        ("alpha beta gamma", 34, "alpha"),
        ("ol\u{e1}  caf\u{e9} amanh\u{e3}", 67, "ol\u{e1}  caf\u{e9}"),
        ("你好世界", 99, ""),
    ] {
        let segments = [segment(text, 0, 100)];
        let heard = split(&segments, played, 100, true).unwrap();
        assert_eq!(heard.prefix, expected);
        assert_eq!(heard.prefix.clone() + &heard.unheard, text);
        assert!(!format!("{heard:?}").contains(text));
    }
    for (text, nudge) in [("one two", false), ("one two three", true)] {
        assert_eq!(
            split(&[segment(text, 0, 100)], 0, 100, true)
                .unwrap()
                .take_nudge()
                .is_some(),
            nudge
        );
    }
}

#[test]
fn missing_or_invalid_timing_cannot_rewrite_history_as_a_full_response() {
    for (segments, played, total) in [
        (vec![segment("secret", 0, 0)], 0, 0),
        (vec![segment("secret", 10, 5)], 0, 20),
        (vec![segment("secret", 0, 30)], 0, 20),
        (vec![segment("a", 0, 20), segment("b", 10, 30)], 10, 30),
        (vec![segment("a", 0, 20)], 21, 20),
    ] {
        assert!(matches!(
            split(&segments, played, total, true),
            Err(Failure::Bounds)
        ));
    }
    assert!(matches!(
        split(&[segment("bad\0text", 0, 10)], 0, 10, true),
        Err(Failure::Text)
    ));
    assert!(matches!(
        split(&[segment(&"x".repeat(65537), 0, 10)], 0, 10, true),
        Err(Failure::Text)
    ));
}

#[test]
fn explicit_interruption_marks_a_producer_stopped_after_received_audio_drained() {
    let mut heard = split(&[segment("Already heard.", 0, 100)], 100, 100, true).unwrap();
    assert_eq!(heard.history_text(), "Already heard. [interrupted]");
    assert!(heard.take_nudge().is_none());
}
