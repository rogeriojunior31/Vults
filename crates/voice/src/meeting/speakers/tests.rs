use super::*;

fn transcript(ranges: &[(u64, u64)]) -> Transcript {
    Transcript {
        duration_ms: 10_000,
        segments: ranges
            .iter()
            .map(|&(start_ms, end_ms)| Segment {
                start_ms,
                end_ms,
                text: format!("Original text {start_ms}"),
            })
            .collect(),
    }
}
fn timeline(turns: &[(u64, u64, Option<usize>)]) -> Timeline {
    Timeline {
        duration_ms: 10_000,
        turns: turns
            .iter()
            .map(|&(start_ms, end_ms, speaker)| Turn {
                start_ms,
                end_ms,
                speaker,
            })
            .collect(),
    }
}
fn labels(text: Transcript, voices: &Timeline) -> Vec<Option<usize>> {
    associate(text, voices, &AtomicBool::new(false))
        .expect("valid")
        .expect("not cancelled")
        .segments
        .into_iter()
        .map(|segment| segment.speaker)
        .collect()
}

#[test]
fn whole_segment_coverage_retains_text_timestamps_and_returning_voice() {
    let text = transcript(&[(100, 500), (2000, 3000), (4000, 5000)]);
    let originals: Vec<_> = text
        .segments
        .iter()
        .map(|s| (s.start_ms, s.end_ms, s.text.clone()))
        .collect();
    let result = associate(
        text,
        &timeline(&[(0, 1000, Some(4)), (1500, 3500, Some(2)), (3500, 6000, Some(4))]),
        &AtomicBool::new(false),
    )
    .expect("valid")
    .expect("not cancelled");
    assert_eq!(result.duration_ms, 10_000);
    assert_eq!(
        result.segments.iter().map(|s| s.speaker).collect::<Vec<_>>(),
        [Some(4), Some(2), Some(4)]
    );
    for (result, original) in result.segments.into_iter().zip(originals) {
        assert_eq!(
            (
                result.segment.start_ms,
                result.segment.end_ms,
                result.segment.text
            ),
            original
        );
    }
}

#[test]
fn change_overlap_unknown_or_a_one_millisecond_gap_never_guesses_words() {
    for turns in [
        vec![(0, 1500, Some(0)), (1500, 4000, Some(1))],
        vec![(0, 4000, Some(0)), (1500, 2000, Some(1))],
        vec![(0, 4000, Some(0)), (1500, 2000, None)],
        vec![(0, 1500, Some(0)), (1501, 4000, Some(0))],
        vec![(1501, 4000, Some(0))],
        vec![(0, 2999, Some(0))],
        vec![],
    ] {
        assert_eq!(labels(transcript(&[(1000, 3000)]), &timeline(&turns)), [None]);
    }
}

#[test]
fn duplicate_same_voice_and_abutting_turns_do_not_create_false_overlap() {
    assert_eq!(
        labels(
            transcript(&[(500, 1500), (1500, 2500)]),
            &timeline(&[(1000, 3000, Some(8)), (0, 1000, Some(8)), (500, 2000, Some(8)),])
        ),
        [Some(8), Some(8)]
    );
}

#[test]
fn half_open_boundaries_do_not_leak_the_next_or_previous_speaker() {
    assert_eq!(
        labels(
            transcript(&[(0, 1000), (1000, 2000), (2000, 3000)]),
            &timeline(&[(0, 1000, Some(0)), (1000, 2000, Some(1)), (2000, 3000, None),])
        ),
        [Some(0), Some(1), None]
    );
}

#[test]
fn track_labels_are_local_and_durations_are_not_silently_rescaled() {
    let a = timeline(&[(0, 10_000, Some(0))]);
    let b = timeline(&[(0, 10_000, Some(99))]);
    assert_eq!(labels(transcript(&[(0, 500)]), &a), [Some(0)]);
    assert_eq!(labels(transcript(&[(0, 500)]), &b), [Some(99)]);
    let mut mismatched = a;
    mismatched.duration_ms -= 1;
    assert!(associate(transcript(&[(0, 500)]), &mismatched, &AtomicBool::new(false)).is_err());
}

#[test]
fn invalid_ranges_order_duration_and_item_budgets_fail_without_a_result() {
    let stop = AtomicBool::new(false);
    for ranges in [
        vec![(500, 500)],
        vec![(3000, 2000)],
        vec![(0, 10_001)],
        vec![(1000, 3000), (2000, 4000)],
    ] {
        assert!(associate(transcript(&ranges), &timeline(&[]), &stop).is_err());
    }
    for turns in [
        vec![(0, 10_001, Some(0))],
        vec![(500, 500, None)],
        vec![(1000, 0, None)],
    ] {
        assert!(associate(transcript(&[(0, 500)]), &timeline(&turns), &stop).is_err());
    }
    let mut voices = timeline(&[]);
    voices.turns = vec![
        Turn {
            start_ms: 0,
            end_ms: 1,
            speaker: None
        };
        MAX_ITEMS + 1
    ];
    assert!(associate(transcript(&[]), &voices, &stop).is_err());
    let mut text = transcript(&[]);
    text.duration_ms = MAX_DURATION_MS + 1;
    assert!(associate(text, &timeline(&[]), &stop).is_err());
    assert!(associate(transcript(&vec![(0, 1); MAX_ITEMS + 1]), &timeline(&[]), &stop).is_err());
}

#[test]
fn text_budget_and_pre_cancelled_work_never_return_a_partial_transcript() {
    let mut text = transcript(&[(0, 500)]);
    text.segments[0].text = "x".repeat(MAX_TEXT_BYTES + 1);
    assert!(associate(text, &timeline(&[]), &AtomicBool::new(false)).is_err());
    assert!(
        associate(
            transcript(&[(0, 500)]),
            &timeline(&[(0, 1000, Some(0))]),
            &AtomicBool::new(true)
        )
        .expect("cancelled")
        .is_none()
    );
    assert!(associate(Transcript::default(), &timeline(&[]), &AtomicBool::new(false)).is_err());
}

#[test]
fn event_sweep_matches_a_small_exhaustive_coverage_oracle() {
    let choices = [
        (0, 3, Some(0)),
        (2, 6, Some(0)),
        (3, 6, Some(1)),
        (1, 4, None),
        (0, 1, Some(1)),
        (5, 6, Some(0)),
    ];
    for a in choices {
        for b in choices {
            for c in choices {
                let turns = [a, b, c];
                for start in 0..6 {
                    for end in start + 1..=6 {
                        let mut expected = None;
                        let mut certain = true;
                        for at in start..end {
                            let active: std::collections::BTreeSet<_> = turns
                                .iter()
                                .filter(|&&(left, right, _)| left <= at && at < right)
                                .map(|&(_, _, speaker)| speaker)
                                .collect();
                            let voice = if active.len() == 1 {
                                active.first().copied().flatten()
                            } else {
                                None
                            };
                            if voice.is_none() || (at > start && voice != expected) {
                                certain = false;
                            }
                            expected = voice;
                        }
                        if !certain {
                            expected = None;
                        }
                        assert_eq!(
                            labels(transcript(&[(start, end)]), &timeline(&turns)),
                            [expected],
                            "{turns:?} [{start}, {end})"
                        );
                    }
                }
            }
        }
    }
}
