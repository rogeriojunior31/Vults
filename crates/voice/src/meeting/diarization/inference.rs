use crate::meeting::{speaker_models::VerifiedModels, speakers::Turn};
use crate::onnx::NativeSession;
use ort::session::Session;
use ort::value::Tensor;
use std::collections::BTreeSet;
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{RATE, cluster, control};

const WINDOW: usize = RATE * 10;
const HOP: usize = RATE * 5;
const FRAME: usize = 270;
const OFFSET: usize = (991 - FRAME) / 2;
const POWERSET: [&[usize]; 7] = [&[], &[0], &[1], &[2], &[0, 1], &[0, 2], &[1, 2]];

fn frame_class(frame: &[f32; 7]) -> usize {
    // Match the original first-maximum decoder, including equal signed zeros.
    (1..frame.len()).fold(
        0,
        |best, next| if frame[next] > frame[best] { next } else { best },
    )
}

pub struct Models {
    segmentation: NativeSession,
    embedding: NativeSession,
}

pub struct ResultSet {
    pub speakers: usize,
    pub observations: usize,
    pub turns: Vec<Turn>,
}

struct Frame {
    start: usize,
    end: usize,
    observation: Option<usize>,
}

fn session(bytes: &[u8], stop: &AtomicBool) -> Result<Option<NativeSession>, String> {
    let Some(_busy) = crate::WHISPER.background(stop) else {
        return Ok(None);
    };
    let session = Session::builder()
        .map_err(|e| e.to_string())?
        .with_intra_threads(1)
        .map_err(|e| e.to_string())?
        .with_inter_threads(1)
        .map_err(|e| e.to_string())?
        .with_memory_pattern(false)
        .map_err(|e| e.to_string())?
        .commit_from_memory(bytes)
        .map_err(|e| e.to_string())?;
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    Ok(Some(NativeSession(ManuallyDrop::new(session))))
}

impl Models {
    pub fn load(artifacts: &VerifiedModels, stop: &AtomicBool) -> Result<Option<Self>, String> {
        let Some(segmentation) = session(artifacts.segmentation(), stop)? else {
            return Ok(None);
        };
        let Some(embedding) = session(artifacts.embedding(), stop)? else {
            return Ok(None);
        };
        for (key, expected) in [
            ("sample_rate", "16000"),
            ("window_size", "160000"),
            ("receptive_field_size", "991"),
            ("receptive_field_shift", "270"),
            ("num_classes", "7"),
            ("num_speakers", "3"),
        ] {
            if segmentation
                .0
                .metadata()
                .map_err(|e| e.to_string())?
                .custom(key)
                .as_deref()
                != Some(expected)
            {
                return Err(format!("unsupported segmentation metadata: {key}"));
            }
        }
        for (key, expected) in [
            ("sample_rate", "16000"),
            ("normalize_samples", "1"),
            ("feature_normalize_type", "global-mean"),
            ("output_dim", "192"),
        ] {
            if embedding
                .0
                .metadata()
                .map_err(|e| e.to_string())?
                .custom(key)
                .as_deref()
                != Some(expected)
            {
                return Err(format!("unsupported embedding metadata: {key}"));
            }
        }
        Ok((!stop.load(Ordering::SeqCst)).then_some(Self {
            segmentation,
            embedding,
        }))
    }

    fn segment(&mut self, pcm: &[f32], stop: &AtomicBool) -> Result<Option<Vec<usize>>, String> {
        control::native(stop, |options| {
            let mut padded = vec![0.0; WINDOW];
            padded[..pcm.len()].copy_from_slice(pcm);
            let input = Tensor::from_array(([1, 1, WINDOW], padded)).map_err(|e| e.to_string())?;
            let output = self
                .segmentation
                .0
                .run_with_options(ort::inputs![input], options)
                .map_err(|e| e.to_string())?;
            let (shape, data) = output[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
            if shape.as_ref() != [1, 589, 7] || data.iter().any(|v| !v.is_finite()) {
                return Err("unexpected segmentation output".into());
            }
            Ok(data.as_chunks::<7>().0.iter().map(frame_class).collect())
        })
    }

    fn embed(&mut self, samples: &[f32], stop: &AtomicBool) -> Result<Option<Vec<f32>>, String> {
        control::native(stop, |options| {
            // knf-rs matches the export's 80-bin, 16 kHz, no-dither, snip-edges/global-mean input.
            let features = knf_rs::compute_fbank(samples).map_err(|e| e.to_string())?;
            if features.ncols() != 80 || features.iter().any(|v| !v.is_finite()) {
                return Err("invalid filterbank features".into());
            }
            let input = Tensor::from_array(([1, features.nrows(), 80], features.into_raw_vec_and_offset().0))
                .map_err(|e| e.to_string())?;
            let output = self
                .embedding
                .0
                .run_with_options(ort::inputs![input], options)
                .map_err(|e| e.to_string())?;
            let (shape, data) = output[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
            if shape.as_ref() != [1, 192] || data.iter().any(|v| !v.is_finite()) {
                return Err("unexpected embedding output".into());
            }
            Ok(data.to_vec())
        })
    }

    pub fn run<R: std::io::Read + std::io::Seek>(
        &mut self,
        chunks: &mut crate::meeting::wav::Chunks<R>,
        stop: &AtomicBool,
        mut progress: impl FnMut(u64),
    ) -> Result<Option<ResultSet>, String> {
        if chunks.total == 0 || chunks.total > super::MAX_SAMPLES as u64 {
            return Err("speaker input exceeds the finite audio budget".into());
        }
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        progress(0);
        let total = chunks.total as usize;
        let windows = windows(total);
        let mut audio = AudioWindow::default();
        let mut embeddings = Vec::new();
        let mut frames = Vec::new();
        let mut cannot_link = BTreeSet::new();
        for (start, left, right) in windows {
            let end = (start + WINDOW).min(total);
            let Some(samples) = audio.read(chunks, start, end, stop)? else {
                return Ok(None);
            };
            let Some(classes) = self.segment(samples, stop)? else {
                return Ok(None);
            };
            let mut observations = [None; 3];
            for (slot, observation) in observations.iter_mut().enumerate() {
                let mut pure = Vec::new();
                for (frame, &class) in classes.iter().enumerate() {
                    if POWERSET[class] == [slot] {
                        let at = frame * FRAME + OFFSET;
                        let until = (at + FRAME).min(samples.len());
                        if at < until {
                            pure.extend_from_slice(&samples[at..until]);
                        }
                    }
                }
                // A short/overlap-only voice stays Unknown; no name or identity is guessed.
                if pure.len() >= RATE {
                    *observation = Some(embeddings.len());
                    let Some(embedding) = self.embed(&pure, stop)? else {
                        return Ok(None);
                    };
                    embeddings.push(embedding);
                }
            }
            for (frame, &class) in classes.iter().enumerate() {
                let at = (start + frame * FRAME + OFFSET).max(left);
                let until = (start + frame * FRAME + OFFSET + FRAME).min(right);
                if at >= until {
                    continue;
                }
                if let [a, b] = POWERSET[class]
                    && let (Some(a), Some(b)) = (observations[*a], observations[*b])
                {
                    cannot_link.insert((a, b));
                }
                for &slot in POWERSET[class] {
                    frames.push(Frame {
                        start: at,
                        end: until,
                        observation: observations[slot],
                    });
                }
            }
            progress(right as u64 * 1000 / RATE as u64);
            if stop.load(Ordering::SeqCst) {
                return Ok(None);
            }
        }
        if chunks.offset != chunks.total {
            return Err("speaker audio has an incomplete timeline".into());
        }
        let Some(labels) = cluster::assign(&embeddings, 0.7, &cannot_link, stop)? else {
            return Ok(None);
        };
        let speakers = labels.iter().max().map_or(0, |n| n + 1);
        let turns = merge_turns(frames, &labels);
        Ok((!stop.load(Ordering::SeqCst)).then_some(ResultSet {
            speakers,
            observations: embeddings.len(),
            turns,
        }))
    }
}

#[derive(Default)]
struct AudioWindow {
    pcm: Vec<f32>,
    offset: usize,
}
impl AudioWindow {
    fn read<R: std::io::Read + std::io::Seek>(
        &mut self,
        chunks: &mut crate::meeting::wav::Chunks<R>,
        start: usize,
        end: usize,
        stop: &AtomicBool,
    ) -> Result<Option<&[f32]>, String> {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let discard = start
            .checked_sub(self.offset)
            .ok_or("speaker windows must be ordered")?;
        if discard > self.pcm.len() {
            return Err("speaker windows lost their source clock".into());
        }
        self.pcm.drain(..discard);
        self.offset = start;
        while self.pcm.len() < end - start {
            let Some(chunk) = chunks.next(stop)? else {
                return if stop.load(Ordering::SeqCst) {
                    Ok(None)
                } else {
                    Err("speaker audio has an incomplete timeline".into())
                };
            };
            self.pcm.extend(chunk);
        }
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        Ok(Some(&self.pcm[..end - start]))
    }
}

fn windows(len: usize) -> Vec<(usize, usize, usize)> {
    if len == 0 {
        return Vec::new();
    }
    let mut starts = vec![0];
    while starts.last().copied().unwrap_or(0) + WINDOW < len {
        starts.push(starts.last().copied().unwrap_or(0) + HOP);
    }
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let left = if i == 0 { 0 } else { start + HOP / 2 };
            let right = if i + 1 == starts.len() {
                len
            } else {
                start + WINDOW - HOP / 2
            };
            (start, left, right)
        })
        .collect()
}

fn merge_turns(frames: Vec<Frame>, labels: &[usize]) -> Vec<Turn> {
    let mut turns: Vec<Turn> = frames
        .into_iter()
        .map(|f| Turn {
            start_ms: f.start as u64 * 1000 / RATE as u64,
            end_ms: f.end as u64 * 1000 / RATE as u64,
            speaker: f.observation.map(|i| labels[i]),
        })
        // Clipped edges can collapse on the millisecond clock; rounding them up invents activity.
        .filter(|turn| turn.start_ms < turn.end_ms)
        .collect();
    turns.sort_by_key(|t| (t.speaker, t.start_ms, t.end_ms));
    let mut merged: Vec<Turn> = Vec::new();
    for turn in turns {
        if let Some(last) = merged.last_mut()
            && last.speaker == turn.speaker
            // Filling even a one-frame pause invents activity and continuous text coverage.
            && last.end_ms >= turn.start_ms
        {
            last.end_ms = last.end_ms.max(turn.end_ms);
        } else {
            merged.push(turn);
        }
    }
    merged.sort_by_key(|t| (t.start_ms, t.end_ms, t.speaker));
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_windows_preserve_samples_across_decode_chunks_with_bounded_pcm() {
        use std::io::Cursor;
        for seconds in [1, 10, 11, 29, 181] {
            let total = RATE * seconds + 17;
            let mut output = Cursor::new(Vec::new());
            let mut writer = hound::WavWriter::new(
                &mut output,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: RATE as u32,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .expect("header");
            for at in 0..total {
                writer.write_sample((at % 32767) as i16).expect("sample");
            }
            writer.finalize().expect("WAV");
            let mut chunks =
                crate::meeting::wav::Chunks::new(Cursor::new(output.into_inner())).expect("reader");
            let mut audio = AudioWindow::default();
            let stop = AtomicBool::new(false);
            for (start, _, _) in windows(total) {
                let end = (start + WINDOW).min(total);
                let samples = audio
                    .read(&mut chunks, start, end, &stop)
                    .expect("clock")
                    .expect("active");
                assert_eq!(samples.len(), end - start);
                for (index, sample) in samples.iter().enumerate() {
                    assert!((*sample - ((start + index) % 32767) as f32 / 32768.0).abs() < 0.000001);
                }
                assert!(audio.pcm.len() <= crate::meeting::wav::MAX + WINDOW);
            }
            assert_eq!(chunks.offset, total as u64);
            stop.store(true, Ordering::SeqCst);
            assert!(audio.read(&mut chunks, 0, 1, &stop).expect("cancel").is_none());
        }
    }

    #[test]
    fn powerset_argmax_keeps_the_first_exact_maximum_without_rounding_near_ties() {
        assert_eq!(frame_class(&[0.0; 7]), 0);
        assert_eq!(frame_class(&[-0.0, 0.0, -1.0, -1.0, -1.0, -1.0, -1.0]), 0);
        for first in 0..7 {
            let mut frame = [-3.0; 7];
            frame[first] = 2.0;
            assert_eq!(frame_class(&frame), first);
            for later in first + 1..7 {
                frame[later] = 2.0;
                assert_eq!(frame_class(&frame), first);
                frame[later] = f32::from_bits(2.0_f32.to_bits() + 1);
                assert_eq!(frame_class(&frame), later);
                frame[later] = -3.0;
            }
        }
    }

    #[test]
    fn central_windows_own_each_sample_once_at_short_and_overlap_boundaries() {
        for len in [
            1,
            WINDOW - 1,
            WINDOW,
            WINDOW + 1,
            WINDOW + HOP,
            RATE * 180,
            RATE * 1800,
        ] {
            let ranges = windows(len);
            assert_eq!(ranges.first().expect("first").1, 0);
            assert_eq!(ranges.last().expect("last").2, len);
            for &(start, left, right) in &ranges {
                assert!(start <= left && left < right && right <= len && right <= start + WINDOW);
            }
            for pair in ranges.windows(2) {
                assert_eq!(pair[0].2, pair[1].1);
            }
        }
        assert!(windows(0).is_empty());
    }

    #[test]
    fn overlap_and_unknown_keep_their_original_clocks_across_local_labels() {
        let frames = vec![
            Frame {
                start: 16000,
                end: 32000,
                observation: Some(0),
            },
            Frame {
                start: 32000,
                end: 48000,
                observation: Some(2),
            },
            Frame {
                start: 24000,
                end: 40000,
                observation: Some(1),
            },
            Frame {
                start: 48000,
                end: 64000,
                observation: None,
            },
        ];
        let turns = merge_turns(frames, &[0, 1, 0]);
        assert_eq!(turns.len(), 3);
        assert_eq!(
            (turns[0].start_ms, turns[0].end_ms, turns[0].speaker),
            (1000, 3000, Some(0))
        );
        assert_eq!(
            (turns[1].start_ms, turns[1].end_ms, turns[1].speaker),
            (1500, 2500, Some(1))
        );
        assert_eq!(
            (turns[2].start_ms, turns[2].end_ms, turns[2].speaker),
            (3000, 4000, None)
        );
    }

    #[test]
    fn clipped_submillisecond_frames_cannot_invalidate_the_text_timeline() {
        use crate::meeting::{
            Segment, Transcript,
            speakers::{Timeline, associate},
        };

        for known in [false, true] {
            let observation = known.then_some(0);
            let turns = merge_turns(
                vec![
                    Frame {
                        start: OFFSET,
                        end: OFFSET + 1,
                        observation,
                    },
                    Frame {
                        start: RATE / 2,
                        end: RATE,
                        observation,
                    },
                    Frame {
                        start: RATE + 16,
                        end: RATE + 32,
                        observation,
                    },
                    Frame {
                        start: 2 * RATE + 1,
                        end: 2 * RATE + 15,
                        observation,
                    },
                ],
                &[7],
            );
            assert_eq!(
                turns,
                vec![
                    Turn {
                        start_ms: 500,
                        end_ms: 1000,
                        speaker: known.then_some(7)
                    },
                    Turn {
                        start_ms: 1001,
                        end_ms: 1002,
                        speaker: known.then_some(7)
                    },
                ]
            );
            let result = associate(
                Transcript {
                    duration_ms: 2000,
                    segments: vec![Segment {
                        start_ms: 1001,
                        end_ms: 1002,
                        text: "Short edge".into(),
                    }],
                },
                &Timeline {
                    duration_ms: 2000,
                    turns,
                },
                &AtomicBool::new(false),
            )
            .expect("representable turns")
            .expect("active");
            assert_eq!(result.segments[0].speaker, known.then_some(7));
        }
    }

    #[test]
    fn speech_gaps_are_not_filled_when_merging_turns() {
        use crate::meeting::{
            Segment, Transcript,
            speakers::{Timeline, associate},
        };

        for gap_ms in [1, 16, 17, 1000] {
            for known in [false, true] {
                let start = RATE + RATE * gap_ms / 1000;
                let turns = merge_turns(
                    vec![
                        Frame {
                            start: 0,
                            end: RATE,
                            observation: known.then_some(0),
                        },
                        Frame {
                            start,
                            end: start + RATE / 2,
                            observation: known.then_some(1),
                        },
                        Frame {
                            start: start + RATE / 2,
                            end: start + RATE,
                            observation: known.then_some(1),
                        },
                    ],
                    &[0, 0],
                );
                assert_eq!(turns.len(), 2, "gap={gap_ms}, known={known}");
                assert_eq!((turns[0].start_ms, turns[0].end_ms), (0, 1000));
                assert_eq!(
                    (turns[1].start_ms, turns[1].end_ms),
                    (1000 + gap_ms as u64, 2000 + gap_ms as u64)
                );
                let duration_ms = turns[1].end_ms;
                let result = associate(
                    Transcript {
                        duration_ms,
                        segments: vec![
                            Segment {
                                start_ms: 900,
                                end_ms: 1100 + gap_ms as u64,
                                text: "Across a gap".into(),
                            },
                            Segment {
                                start_ms: 1200 + gap_ms as u64,
                                end_ms: 1300 + gap_ms as u64,
                                text: "Continuous coverage".into(),
                            },
                        ],
                    },
                    &Timeline { duration_ms, turns },
                    &AtomicBool::new(false),
                )
                .expect("valid source clock")
                .expect("active");
                assert_eq!(result.segments[0].speaker, None);
                assert_eq!(result.segments[1].speaker, known.then_some(0));
            }
        }
    }
}
