//! Explicit offline V5.4 experiment; never linked to the app. No recording or downloads.

use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use vults_voice::meeting::diarization::{Runtime, SpeechProfile, analyze_wav, speech_activity_wav};
use vults_voice::meeting::speaker_models::{
    EMBEDDING_SHA256 as EMBEDDING, SEGMENTATION_SHA256 as SEGMENTATION, read_models,
};
use vults_voice::meeting::speakers::Timeline;

use serde_json::json;
use sha2::{Digest, Sha256};
use vults_spike::meeting_eval::{Prediction, Reference, Report, Turn, read_json, score};

fn evaluate(
    reference: Option<Reference>,
    wav_hash: &str,
    timeline: &Timeline,
) -> Result<Option<Report>, String> {
    reference
        .map(|reference| {
            if reference.clock.wav_sha256 != wav_hash {
                return Err("annotation WAV hash does not match the analyzed source".into());
            }
            let mut clock = reference.clock.clone();
            clock.duration_ms = timeline.duration_ms;
            let prediction = Prediction {
                schema: 1,
                clock,
                turns: timeline
                    .turns
                    .iter()
                    .map(|t| {
                        Ok(Turn {
                            start_ms: t.start_ms,
                            end_ms: t.end_ms,
                            speaker: t
                                .speaker
                                .map(u16::try_from)
                                .transpose()
                                .map_err(|_| "speaker label exceeds evaluation limit")?,
                        })
                    })
                    .collect::<Result<_, String>>()?,
            };
            score(&reference, &prediction)
        })
        .transpose()
}

fn open_audio(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let input = options.open(path).map_err(|e| e.to_string())?;
    if !input.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("the spike requires regular files, not symlinks or devices".into());
    }
    Ok(input)
}

fn fingerprint(input: &mut File) -> Result<String, String> {
    input.rewind().map_err(|e| e.to_string())?;
    let mut file = BufReader::new(input);
    let mut hash = Sha256::new();
    let mut bytes = [0; 64 * 1024];
    loop {
        let len = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if len == 0 {
            break;
        }
        hash.update(&bytes[..len]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn intersect_speech(timeline: &Timeline, speech: &Timeline) -> Result<Timeline, String> {
    if timeline.duration_ms != speech.duration_ms {
        return Err("speech diagnostic and speaker source durations differ".into());
    }
    let mut turns = Vec::new();
    for turn in &timeline.turns {
        let first = speech.turns.partition_point(|span| span.end_ms <= turn.start_ms);
        for span in &speech.turns[first..] {
            if span.start_ms >= turn.end_ms {
                break;
            }
            let start_ms = turn.start_ms.max(span.start_ms);
            let end_ms = turn.end_ms.min(span.end_ms);
            if start_ms < end_ms {
                if turns.len() == 4096 {
                    return Err("speech diagnostic exceeds the evaluation turn budget".into());
                }
                turns.push(vults_voice::meeting::speakers::Turn {
                    start_ms,
                    end_ms,
                    speaker: turn.speaker,
                });
            }
        }
    }
    turns.sort_by_key(|t| (t.start_ms, t.end_ms, t.speaker));
    Ok(Timeline {
        duration_ms: timeline.duration_ms,
        turns,
    })
}

fn main() -> Result<(), String> {
    // SAFETY: synchronous process entry, before native initialization or worker creation.
    unsafe { std::env::set_var("ORT_DISABLE_TELEMETRY", "1") };
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let advanced = args.last().is_some_and(|arg| arg == "--advanced-embedding");
    if advanced {
        args.pop();
    }
    if !matches!(args.len(), 10 | 12 | 14)
        || args[..10]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| p[0].as_str())
            .collect::<Vec<_>>()
            != [
                "--runtime",
                "--runtime-sha256",
                "--segmentation",
                "--embedding",
                "--wav",
            ]
        || args.len() >= 12 && args[10] != "--reference"
        || args.len() == 14 && !matches!(args[12].as_str(), "--vad" | "--vad-frames")
    {
        return Err("usage: vults-diarization-spike --runtime ABSOLUTE_LIBRARY --runtime-sha256 SHA256 --segmentation MODEL --embedding MODEL --wav CONSENTED_WAV [--reference ANNOTATION_JSON [--vad TRUSTED_IMMUTABLE_MODEL | --vad-frames TRUSTED_IMMUTABLE_MODEL]] [--advanced-embedding]".into());
    }
    if !Path::new(&args[1]).is_absolute() {
        return Err("provide an explicit absolute runtime path".into());
    }
    let stop = AtomicBool::new(false);
    let read = if advanced {
        vults_voice::meeting::speaker_models::read_advanced_models
    } else {
        read_models
    };
    let artifacts = read(Path::new(&args[5]), Path::new(&args[7]), &stop)?
        .ok_or("speaker model admission was cancelled")?;
    // Keep the hashed descriptor through inference; reopening a replaced path changes its clock.
    let mut input = open_audio(Path::new(&args[9]))?;
    let wav_hash = fingerprint(&mut input)?;
    let reference: Option<Reference> = args.get(11).map(|path| read_json(Path::new(path))).transpose()?;
    let mut vad_file = args
        .get(13)
        .map(|path| {
            if !Path::new(path).is_absolute() {
                return Err("provide an absolute VAD model path".into());
            }
            let mut file = open_audio(Path::new(path))?;
            if file.metadata().map_err(|e| e.to_string())?.len() != vults_voice::VAD.size
                || fingerprint(&mut file)? != vults_voice::VAD.sha256
            {
                return Err("VAD model fingerprint mismatch; no diagnostic was run".into());
            }
            Ok::<_, String>(file)
        })
        .transpose()?;
    if let Some(reference) = &reference {
        if reference.clock.wav_sha256 != wav_hash {
            return Err("annotation WAV hash does not match the analyzed source".into());
        }
        score(
            reference,
            &Prediction {
                schema: 1,
                clock: reference.clock.clone(),
                turns: Vec::new(),
            },
        )?;
    }
    input.rewind().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let runtime =
        Runtime::load(Path::new(&args[1]), &args[3], &stop)?.ok_or("runtime admission cancelled")?;
    let result =
        analyze_wav(&runtime, artifacts, &mut input, &stop, |_| {})?.ok_or("speaker analysis cancelled")?;
    if fingerprint(&mut input)? != wav_hash {
        return Err("meeting source changed during analysis; no result was published".into());
    }
    let quality = evaluate(reference.clone(), &wav_hash, &result.timeline)?;
    let baseline_ms = started.elapsed().as_millis();
    let vad_diagnostic = if let Some(file) = &mut vad_file {
        #[cfg(target_os = "linux")]
        let model = {
            use std::os::fd::AsRawFd;
            std::path::PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))
        };
        #[cfg(not(target_os = "linux"))]
        let model = std::path::PathBuf::from(&args[13]);
        input.rewind().map_err(|e| e.to_string())?;
        let vad_started = Instant::now();
        let profile = if args[12] == "--vad-frames" {
            SpeechProfile::MeetingFrames
        } else {
            SpeechProfile::Dictation
        };
        let speech =
            speech_activity_wav(&mut input, &model, &stop, profile)?.ok_or("speech diagnostic cancelled")?;
        let gated = intersect_speech(&result.timeline, &speech)?;
        if fingerprint(&mut input)? != wav_hash || fingerprint(file)? != vults_voice::VAD.sha256 {
            return Err("diagnostic source or VAD model changed; no result was published".into());
        }
        Some(json!({
            "experimental": true, "productionQualified": false,
            "profile": if profile == SpeechProfile::MeetingFrames { "meeting-frames" } else { "dictation" },
            "modelSha256": vults_voice::VAD.sha256, "marginMs": 200,
            "totalMs": vad_started.elapsed().as_millis(),
            "quality": evaluate(reference, &wav_hash, &gated)?,
            "speech": speech.turns.iter().map(|t| json!({"startMs": t.start_ms, "endMs": t.end_ms})).collect::<Vec<_>>(),
            "turns": gated.turns.iter().map(|t| json!({"startMs": t.start_ms, "endMs": t.end_ms, "speaker": t.speaker})).collect::<Vec<_>>(),
        }))
    } else {
        None
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "experimental": true, "threshold": 0.7, "distance": "average-linkage cosine distance",
            "coactivityConstraints": true,
            "fillsSpeechGaps": false,
            "argmaxTieBreak": "first",
            "runtimeSha256": args[3].to_ascii_lowercase(), "segmentationSha256": SEGMENTATION, "embeddingSha256": if advanced { vults_voice::meeting::speaker_models::ADVANCED_EMBEDDING_SHA256 } else { EMBEDDING },
            "wavSha256": wav_hash, "durationMs": result.timeline.duration_ms,
            "totalMs": baseline_ms,
            "speakers": result.speakers, "observations": result.observations,
            "quality": quality,
            "vadDiagnostic": vad_diagnostic,
            "turns": result.timeline.turns.iter().map(|t| json!({"startMs": t.start_ms, "endMs": t.end_ms, "speaker": t.speaker})).collect::<Vec<_>>(),
        }))
        .map_err(|e| e.to_string())?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vults_voice::meeting::{
        Segment, Transcript,
        speakers::{Timeline, Turn, associate},
    };

    #[test]
    fn speech_intersection_preserves_source_gaps_overlap_unknown_and_short_turns() {
        let timeline = Timeline {
            duration_ms: 1000,
            turns: vec![
                Turn {
                    start_ms: 0,
                    end_ms: 400,
                    speaker: Some(0),
                },
                Turn {
                    start_ms: 100,
                    end_ms: 500,
                    speaker: None,
                },
                Turn {
                    start_ms: 300,
                    end_ms: 900,
                    speaker: Some(1),
                },
                Turn {
                    start_ms: 700,
                    end_ms: 701,
                    speaker: Some(0),
                },
            ],
        };
        let speech = Timeline {
            duration_ms: 1000,
            turns: vec![
                Turn {
                    start_ms: 100,
                    end_ms: 200,
                    speaker: None,
                },
                Turn {
                    start_ms: 400,
                    end_ms: 800,
                    speaker: None,
                },
            ],
        };
        let gated = intersect_speech(&timeline, &speech).expect("same source");
        assert_eq!(gated.duration_ms, 1000);
        assert_eq!(
            gated
                .turns
                .iter()
                .map(|t| (t.start_ms, t.end_ms, t.speaker))
                .collect::<Vec<_>>(),
            [
                (100, 200, None),
                (100, 200, Some(0)),
                (400, 500, None),
                (400, 800, Some(1)),
                (700, 701, Some(0))
            ]
        );
        assert!(
            intersect_speech(
                &timeline,
                &Timeline {
                    duration_ms: 999,
                    turns: vec![]
                }
            )
            .is_err()
        );
        assert!(
            intersect_speech(
                &timeline,
                &Timeline {
                    duration_ms: 1000,
                    turns: vec![]
                }
            )
            .expect("empty mask")
            .turns
            .is_empty()
        );
        assert!(
            intersect_speech(
                &Timeline {
                    duration_ms: 1000,
                    turns: vec![timeline.turns[0].clone(); 4097]
                },
                &speech
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn hashing_and_analysis_keep_one_descriptor_and_detect_changed_content() {
        use std::fs;
        let directory = std::env::temp_dir().join(format!("vults-spike-clock-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("private fixture directory");
        let path = directory.join("audio.wav");
        fs::write(&path, b"first fixture").expect("fixture");
        let mut opened = open_audio(&path).expect("regular file");
        let before = fingerprint(&mut opened).expect("hash");
        fs::rename(&path, directory.join("original.wav")).expect("rename fixture");
        fs::write(&path, b"other fixture").expect("replacement");
        assert_eq!(fingerprint(&mut opened).expect("same descriptor"), before);
        assert_ne!(
            fingerprint(&mut open_audio(&path).expect("replacement")).expect("new hash"),
            before
        );
        fs::write(directory.join("original.wav"), b"edited fixture").expect("in-place mutation");
        assert_ne!(fingerprint(&mut opened).expect("changed bytes"), before);
        std::os::unix::fs::symlink(&path, directory.join("link.wav")).expect("symlink");
        assert!(open_audio(&directory.join("link.wav")).is_err());
        assert!(open_audio(&directory).is_err());
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn native_output_scoring_checks_actual_wav_and_normalized_duration() {
        let reference: Reference =
            serde_json::from_str(include_str!("../../fixtures/meeting-eval/en-reference.json"))
                .expect("synthetic reference");
        let timeline = Timeline {
            duration_ms: reference.clock.duration_ms,
            turns: vec![
                vults_voice::meeting::speakers::Turn {
                    start_ms: 0,
                    end_ms: 3000,
                    speaker: Some(7),
                },
                vults_voice::meeting::speakers::Turn {
                    start_ms: 3000,
                    end_ms: 5000,
                    speaker: Some(9),
                },
            ],
        };
        assert_eq!(
            evaluate(Some(reference.clone()), &"0".repeat(64), &timeline)
                .expect("aligned")
                .expect("report")
                .der,
            Some(0.0)
        );
        assert!(evaluate(Some(reference.clone()), &"1".repeat(64), &timeline).is_err());
        let shifted = Timeline {
            duration_ms: timeline.duration_ms + 1,
            ..timeline
        };
        assert!(evaluate(Some(reference), &"0".repeat(64), &shifted).is_err());
        assert!(
            evaluate(None, &"0".repeat(64), &shifted)
                .expect("optional")
                .is_none()
        );
    }

    #[test]
    fn recorded_spike_turns_keep_known_speech_and_overlap_uncertainty() {
        let evidence: serde_json::Value =
            serde_json::from_str(include_str!("../../fixtures/diarization-2026-10-18.json"))
                .expect("recorded evidence");
        for (fixture, start, end, expected) in [
            ("single", 1500, 2500, Some(0)),
            ("alternating", 14_000, 15_000, Some(1)),
            ("alternating", 20_200, 21_200, Some(0)),
            ("overlap", 14_000, 15_000, None),
            ("overlap", 21_000, 22_000, Some(1)),
            ("silence", 1000, 2000, None),
        ] {
            let value = &evidence["results"][fixture];
            let duration_ms = value["durationMs"].as_u64().expect("duration");
            let timeline = Timeline {
                duration_ms,
                turns: value["turns"]
                    .as_array()
                    .expect("turns")
                    .iter()
                    .map(|t| Turn {
                        start_ms: t["startMs"].as_u64().expect("start"),
                        end_ms: t["endMs"].as_u64().expect("end"),
                        speaker: t["speaker"].as_u64().map(|n| n as usize),
                    })
                    .collect(),
            };
            let text = Transcript {
                duration_ms,
                segments: vec![Segment {
                    start_ms: start,
                    end_ms: end,
                    text: "Disposable association probe, not recognized speech".into(),
                }],
            };
            let result = associate(text, &timeline, &AtomicBool::new(false))
                .expect("valid clocks")
                .expect("not cancelled");
            assert_eq!(result.segments[0].speaker, expected, "{fixture}");
            assert_eq!(
                (
                    result.segments[0].segment.start_ms,
                    result.segments[0].segment.end_ms
                ),
                (start, end)
            );
        }
    }
}
