#![allow(clippy::unwrap_used)]
use super::*;

fn reference(turns: &[(u64, u64, u16)]) -> Reference {
    Reference {
        schema: 1,
        clock: Clock {
            recording: "fixture".into(),
            source: "microphone".into(),
            wav_sha256: "a".repeat(64),
            duration_ms: 10_000,
        },
        language: Language::En,
        annotation: Annotation::Synthetic,
        split: Split::Evaluation,
        turns: turns
            .iter()
            .map(|&(start_ms, end_ms, speaker)| Turn {
                start_ms,
                end_ms,
                speaker: Some(speaker),
            })
            .collect(),
    }
}
fn prediction(reference: &Reference, turns: &[(u64, u64, Option<u16>)]) -> Prediction {
    Prediction {
        schema: 1,
        clock: reference.clock.clone(),
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

#[test]
fn permuted_labels_and_duplicate_activity_have_perfect_global_mapping() {
    let r = reference(&[(0, 2000, 8), (2000, 4000, 42), (4000, 6000, 8)]);
    let p = prediction(
        &r,
        &[
            (0, 2000, Some(99)),
            (2000, 4000, Some(7)),
            (4000, 6000, Some(99)),
            (4000, 6000, Some(99)),
        ],
    );
    let s = score(&r, &p).unwrap();
    assert_eq!(s.der, Some(0.0));
    assert_eq!(s.correct_speaker_ms, 6000);
    assert_eq!((s.reference_speakers, s.predicted_speakers), (2, 2));
    assert!(!s.production_qualified);
}
#[test]
fn optimal_mapping_beats_greedy_and_equal_speaker_counts_do_not_hide_confusion() {
    let r = reference(&[(0, 3000, 0), (3000, 5000, 1)]);
    let p = prediction(
        &r,
        &[(0, 2000, Some(0)), (2000, 3000, Some(1)), (3000, 5000, Some(0))],
    );
    let s = score(&r, &p).unwrap();
    assert_eq!(s.correct_speaker_ms, 3000);
    assert_eq!(s.confused_speaker_ms, 2000);
    assert_eq!(s.der, Some(0.4));
}
#[test]
fn loss_false_alarm_and_confusion_are_reported_separately() {
    let r = reference(&[(0, 3000, 0), (3000, 5000, 1)]);
    let s = score(&r, &prediction(&r, &[(0, 2000, Some(7)), (3000, 6000, Some(7))])).unwrap();
    assert_eq!(
        (
            s.missed_speaker_ms,
            s.false_alarm_speaker_ms,
            s.confused_speaker_ms
        ),
        (1000, 1000, 2000)
    );
    assert_eq!(s.der, Some(0.8));
}
#[test]
fn false_alarms_distinguish_reference_absence_from_excess_overlap() {
    let r = reference(&[(1000, 3000, 0), (2000, 3000, 1)]);
    let p = prediction(
        &r,
        &[
            (0, 4000, Some(7)),
            (0, 2000, Some(9)),
            (0, 2000, Some(9)),
            (1000, 2000, None),
        ],
    );
    let s = score(&r, &p).unwrap();
    assert_eq!(s.false_alarm_without_reference_speaker_ms, 3000);
    assert_eq!(s.excess_overlap_speaker_ms, 2000);
    assert_eq!(s.false_alarm_speaker_ms, 5000);
    assert_eq!(
        s.false_alarm_speaker_ms,
        s.false_alarm_without_reference_speaker_ms + s.excess_overlap_speaker_ms
    );
}
#[test]
fn overlap_counts_speaker_time_and_unknown_activity_never_matches_a_person() {
    let r = reference(&[(0, 3000, 0), (1000, 2000, 1)]);
    let missing = score(&r, &prediction(&r, &[(0, 3000, Some(0))])).unwrap();
    assert_eq!(
        (missing.reference_speaker_ms, missing.missed_speaker_ms),
        (4000, 1000)
    );
    assert_eq!(missing.der, Some(0.25));
    let unknown = score(
        &r,
        &prediction(&r, &[(0, 3000, Some(0)), (1000, 2000, None), (1000, 2000, None)]),
    )
    .unwrap();
    assert_eq!(
        (
            unknown.unknown_activity_ms,
            unknown.confused_speaker_ms,
            unknown.missed_speaker_ms
        ),
        (1000, 1000, 0)
    );
    assert_eq!(unknown.der, Some(0.25));
    assert_eq!(unknown.predicted_speakers, 1);
}
#[test]
fn fixed_zero_collar_keeps_boundary_errors_and_half_open_turns() {
    let r = reference(&[(1000, 2000, 0)]);
    let s = score(&r, &prediction(&r, &[(1100, 2100, Some(9))])).unwrap();
    assert_eq!(
        (
            s.missed_speaker_ms,
            s.false_alarm_speaker_ms,
            s.confused_speaker_ms
        ),
        (100, 100, 0)
    );
    assert_eq!(s.der, Some(0.2));
    assert_eq!(s.collar_ms, 0);
    assert!(s.includes_overlap);
}
#[test]
fn silence_has_no_invented_rate_and_false_alarms_can_exceed_one_hundred_percent() {
    let r = reference(&[]);
    let s = score(&r, &prediction(&r, &[(0, 1000, Some(0))])).unwrap();
    assert_eq!(s.der, None);
    assert_eq!(s.false_alarm_speaker_ms, 1000);
    let r = reference(&[(0, 1000, 0)]);
    let s = score(&r, &prediction(&r, &[(0, 8000, Some(0))])).unwrap();
    assert_eq!(s.der, Some(7.0));
}
#[test]
fn source_recording_hash_duration_and_schema_must_all_match() {
    let r = reference(&[(0, 1000, 0)]);
    for kind in 0..5 {
        let mut p = prediction(&r, &[(0, 1000, Some(0))]);
        match kind {
            0 => p.clock.source = "system".into(),
            1 => p.clock.recording = "another".into(),
            2 => p.clock.wav_sha256 = "b".repeat(64),
            3 => p.clock.duration_ms += 1,
            _ => p.schema = 2,
        }
        assert!(
            score(&r, &p).is_err(),
            "Mismatch {kind} must fail, even when all turns fit"
        );
    }
}
#[test]
fn invalid_annotations_budgets_and_metadata_are_rejected() {
    for kind in 0..10 {
        let mut r = reference(&[(0, 1000, 0)]);
        match kind {
            0 => r.turns[0].speaker = None,
            1 => r.turns[0].end_ms = 0,
            2 => r.turns[0].end_ms = 10_001,
            3 => r.turns = vec![r.turns[0].clone(); MAX_TURNS + 1],
            4 => r.clock.duration_ms = MAX_DURATION_MS + 1,
            5 => r.clock.wav_sha256 = "A".repeat(64),
            6 => r.clock.source = "private path/with spaces".into(),
            7 => r.clock.recording = "x".repeat(65),
            8 => {
                r.turns = (0..9)
                    .map(|speaker| Turn {
                        start_ms: 0,
                        end_ms: 1000,
                        speaker: Some(speaker),
                    })
                    .collect()
            }
            _ => r.schema = 0,
        }
        assert!(score(&r, &prediction(&r, &[])).is_err(), "Invalid input {kind}");
    }
    let mut r = reference(&[(0, 1000, 0)]);
    r.clock.duration_ms = 0;
    assert!(score(&r, &prediction(&r, &[])).is_err());
}
#[test]
fn annotation_language_split_and_canonical_fingerprints_remain_explicit() {
    let mut r = reference(&[(0, 1000, 0)]);
    let p = prediction(&r, &[(0, 1000, Some(2))]);
    let before = score(&r, &p).unwrap();
    r.language = Language::Pt;
    r.split = Split::Calibration;
    r.annotation = Annotation::Human;
    let after = score(&r, &p).unwrap();
    assert_ne!(before.reference_sha256, after.reference_sha256);
    assert_eq!(before.prediction_sha256, after.prediction_sha256);
    assert_eq!(
        (after.language, after.annotation, after.split),
        (Language::Pt, Annotation::Human, Split::Calibration)
    );
    assert!(
        !after.production_qualified,
        "An asserted human annotation is no release approval"
    );
    let mut value = serde_json::to_value(&r).unwrap();
    value["collarMs"] = serde_json::json!(250);
    assert!(
        serde_json::from_value::<Reference>(value).is_err(),
        "Protocol changes cannot be silently ignored"
    );
}

// Compare the event sweep and global matching against an independent per-millisecond oracle.
#[test]
fn exhaustive_small_tapes_match_framewise_permutation_oracle() {
    let tapes = [
        vec![],
        vec![(0, 3, 0)],
        vec![(0, 2, 0), (2, 4, 1)],
        vec![(0, 3, 0), (1, 4, 1)],
        vec![(0, 1, 1), (3, 4, 0)],
        vec![(0, 4, 0), (0, 4, 0)],
    ];
    for a in &tapes {
        for b in &tapes {
            let r = reference(a);
            let p = prediction(
                &r,
                &b.iter().map(|&(s, e, k)| (s, e, Some(k))).collect::<Vec<_>>(),
            );
            let s = score(&r, &p).unwrap();
            let (mut total, mut missed, mut fa, mut best_correct) = (0, 0, 0, 0);
            for t in 0..4 {
                let nr = a
                    .iter()
                    .filter(|&&(start, end, _)| start <= t && t < end)
                    .map(|x| x.2)
                    .collect::<BTreeSet<_>>()
                    .len() as u64;
                let nh = b
                    .iter()
                    .filter(|&&(start, end, _)| start <= t && t < end)
                    .map(|x| x.2)
                    .collect::<BTreeSet<_>>()
                    .len() as u64;
                total += nr;
                missed += nr.saturating_sub(nh);
                fa += nh.saturating_sub(nr);
            }
            for flip in [false, true] {
                let mut correct = 0;
                for t in 0..4 {
                    let ra = a
                        .iter()
                        .filter(|&&(start, end, _)| start <= t && t < end)
                        .map(|x| x.2)
                        .collect::<BTreeSet<_>>();
                    let hb = b
                        .iter()
                        .filter(|&&(start, end, _)| start <= t && t < end)
                        .map(|x| if flip { 1 - x.2 } else { x.2 })
                        .collect::<BTreeSet<_>>();
                    correct += ra.intersection(&hb).count() as u64;
                }
                best_correct = best_correct.max(correct);
            }
            assert_eq!(
                (
                    s.reference_speaker_ms,
                    s.missed_speaker_ms,
                    s.false_alarm_speaker_ms,
                    s.correct_speaker_ms,
                    s.confused_speaker_ms
                ),
                (total, missed, fa, best_correct, total - missed - best_correct)
            );
        }
    }
}
#[test]
fn matching_agrees_with_brute_force_on_rectangular_matrices() {
    for encoded in 0..729 {
        let mut weights = [[0; MAX_SPEAKERS]; MAX_SPEAKERS];
        let mut digits = encoded;
        for row in weights.iter_mut().take(2) {
            for v in row.iter_mut().take(3) {
                *v = digits % 3;
                digits /= 3;
            }
        }
        let mut expected = 0;
        for a in 0..3 {
            for b in 0..3 {
                if a != b {
                    expected = expected.max(weights[0][a] + weights[1][b]);
                }
            }
        }
        assert_eq!(optimal_overlap(&weights, 2, 3), expected);
    }
}
#[test]
fn json_reader_rejects_oversized_files_and_non_regular_inputs() {
    let dir = std::env::temp_dir().join(format!("vults-meeting-eval-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("large.json");
    File::create(&path).unwrap().set_len(MAX_JSON_BYTES + 1).unwrap();
    assert!(read_json::<Reference>(&path).unwrap_err().contains("one MiB"));
    assert!(read_json::<Reference>(&dir).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&path, dir.join("link.json")).unwrap();
        assert!(read_json::<Reference>(&dir.join("link.json")).is_err());
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let fifo = dir.join("fifo");
        let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        // SAFETY: a valid, terminated path in this private fixture directory.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(read_json::<Reference>(&fifo).is_err());
    }
    std::fs::remove_dir_all(dir).unwrap();
}
