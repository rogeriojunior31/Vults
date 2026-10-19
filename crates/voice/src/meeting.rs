//! Bounded, local WAV transcription for the future meeting owner. No capture, storage or network.

#[cfg(feature = "meeting-assets")]
pub mod assets;
#[cfg(feature = "diarization")]
pub mod diarization;
pub mod speaker_models;
pub mod speakers;
mod wav;

use std::io::{Read, Seek};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{SAMPLE_RATE, Transcriber, WHISPER, gate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Transcript {
    pub duration_ms: u64,
    pub segments: Vec<Segment>,
}

/// Content-free progress at admission and after each completed chunk, including silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub completed_ms: u64,
    pub total_ms: u64,
    pub segments: usize,
}

/// Read one already-open WAV with bounded PCM memory, preserving its original timeline.
/// `None` means cancelled; neither cancellation nor failure returns a partial transcript.
/// The caller owns consent, worker lifetime and the temporary audio's cleanup lock.
/// This does not delete, write, reopen or retain the input, or apply dictation transformations.
pub fn transcribe_wav(
    transcriber: &Transcriber,
    input: impl Read + Seek,
    language: Option<&str>,
    stop: &AtomicBool,
    progress: impl FnMut(Progress),
) -> Result<Option<Transcript>, String> {
    run(input, stop, progress, |pcm| {
        decode(transcriber, pcm, language, stop, false)
    })
}

#[cfg(all(feature = "diarization", feature = "meeting-assets"))]
fn transcribe_words_wav(
    transcriber: &Transcriber,
    input: impl Read + Seek,
    language: Option<&str>,
    stop: &AtomicBool,
    progress: impl FnMut(Progress),
) -> Result<Option<Transcript>, String> {
    run(input, stop, progress, |pcm| {
        decode(transcriber, pcm, language, stop, true)
    })
}

fn run(
    input: impl Read + Seek,
    stop: &AtomicBool,
    mut progress: impl FnMut(Progress),
    mut decode: impl FnMut(&[f32]) -> Result<Option<Vec<Segment>>, String>,
) -> Result<Option<Transcript>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let mut chunks = wav::Chunks::new(input)?;
    let mut transcript = Transcript {
        duration_ms: millis(chunks.total),
        segments: Vec::new(),
    };
    let mut text_bytes = 0;
    progress(Progress {
        completed_ms: 0,
        total_ms: transcript.duration_ms,
        segments: 0,
    });
    while let Some(pcm) = chunks.next(stop)? {
        let offset = millis(chunks.offset - pcm.len() as u64);
        let Some(segments) = decode(&pcm)? else {
            return Ok(None);
        };
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        for segment in segments {
            if let Some(segment) = place(segment, offset, millis(chunks.offset), transcript.segments.last()) {
                text_bytes += segment.text.len();
                if transcript.segments.len() >= 100_000 || text_bytes > 16 * 1024 * 1024 {
                    return Err("meeting transcript exceeds the local text limit".into());
                }
                transcript.segments.push(segment);
            }
        }
        progress(Progress {
            completed_ms: millis(chunks.offset),
            total_ms: transcript.duration_ms,
            segments: transcript.segments.len(),
        });
    }
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if chunks.offset != chunks.total {
        return Err("meeting WAV returned an incomplete timeline".into());
    }
    Ok(Some(transcript))
}

fn millis(samples: u64) -> u64 {
    samples * 1000 / u64::from(SAMPLE_RATE)
}

fn place(mut segment: Segment, offset: u64, end: u64, previous: Option<&Segment>) -> Option<Segment> {
    segment.start_ms = offset
        .saturating_add(segment.start_ms)
        .min(end)
        .max(previous.map_or(offset, |previous| previous.end_ms));
    segment.end_ms = offset.saturating_add(segment.end_ms).min(end);
    (segment.start_ms < segment.end_ms && !segment.text.is_empty()).then_some(segment)
}

struct Abort<'a> {
    stop: &'a AtomicBool,
    scheduler: &'a crate::scheduler::Scheduler,
    yielded: AtomicBool,
}

unsafe extern "C" fn abort(data: *mut std::ffi::c_void) -> bool {
    // SAFETY: borrowed only during `full`, before its caller drops the Abort.
    let request = unsafe { &*data.cast::<Abort<'_>>() };
    if request.stop.load(Ordering::SeqCst) {
        return true;
    }
    let waiting = request.scheduler.foreground_waiting();
    if waiting {
        request.yielded.store(true, Ordering::SeqCst);
    }
    waiting
}

fn decode(
    transcriber: &Transcriber,
    pcm: &[f32],
    language: Option<&str>,
    stop: &AtomicBool,
    words: bool,
) -> Result<Option<Vec<Segment>>, String> {
    let audible = speech_range(pcm);
    if audible.len() < SAMPLE_RATE as usize / 2 {
        return Ok(Some(Vec::new()));
    }
    // Whisper can anchor its first segment at zero despite leading silence. Trim only the
    // outer silence, retaining its offset; internal pauses stay on the original timeline.
    let offset = millis(audible.start as u64);
    let pcm = &pcm[audible];
    loop {
        let Some(_busy) = WHISPER.background(stop) else {
            return Ok(None);
        };
        let mut state = transcriber.ctx.create_state().map_err(|e| e.to_string())?;
        let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_n_threads(crate::threads());
        params.set_no_context(true);
        params.set_no_timestamps(false);
        params.set_token_timestamps(words);
        params.set_suppress_blank(true);
        params.set_no_speech_thold(crate::NO_SPEECH);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        let request = Abort {
            stop,
            scheduler: &WHISPER,
            yielded: AtomicBool::new(false),
        };
        // The safe setter's trampoline has a pointer-type mismatch; share the raw callback used
        // by interactive decoding, with this request borrowed until `full` returns.
        unsafe {
            params.set_abort_callback(Some(abort));
            params.set_abort_callback_user_data(std::ptr::from_ref(&request).cast_mut().cast());
        }
        let result = state.full(params, pcm);
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if request.yielded.load(Ordering::SeqCst) {
            // Retry this same chunk, never publishing the interrupted model's partial segments.
            continue;
        }
        result.map_err(|e| format!("meeting transcription failed: {e}"))?;
        let mut segments = Vec::new();
        for segment in state
            .as_iter()
            .filter(|segment| segment.no_speech_probability() < crate::NO_SPEECH)
        {
            let (Some(start), Some(end)) = (
                u64::try_from(segment.start_timestamp())
                    .ok()
                    .and_then(|t| t.checked_mul(10)),
                u64::try_from(segment.end_timestamp())
                    .ok()
                    .and_then(|t| t.checked_mul(10)),
            ) else {
                continue;
            };
            let text = segment
                .to_str_lossy()
                .map_err(|e| format!("could not read meeting text: {e}"))?;
            let decoded = Segment {
                start_ms: offset.saturating_add(start),
                end_ms: offset.saturating_add(end),
                text: gate::measured(&text, true).text,
            };
            if words {
                let tokens = (0..segment.n_tokens()).filter_map(|at| {
                    let token = segment.get_token(at)?;
                    if token.token_id() >= transcriber.ctx.token_eot() {
                        return None;
                    }
                    let data = token.token_data();
                    Some((
                        transcriber.ctx.token_to_bytes(token.token_id()).ok()?.to_vec(),
                        data.t0,
                        data.t1,
                    ))
                });
                segments.extend(timed_words(decoded, offset, tokens));
            } else {
                segments.push(decoded);
            }
        }
        return Ok(Some(segments));
    }
}

fn timed_words(
    segment: Segment,
    offset: u64,
    tokens: impl Iterator<Item = (Vec<u8>, i64, i64)>,
) -> Vec<Segment> {
    let split = || -> Option<Vec<Segment>> {
        let mut pieces: Vec<(Vec<u8>, u64, u64)> = Vec::new();
        for (bytes, start, end) in tokens {
            let start = offset.checked_add(u64::try_from(start).ok()?.checked_mul(10)?)?;
            let end = offset.checked_add(u64::try_from(end).ok()?.checked_mul(10)?)?;
            let punctuation = |bytes: &[u8]| {
                bytes
                    .iter()
                    .all(|b| b.is_ascii_whitespace() || b.is_ascii_punctuation())
            };
            let word_start = bytes.first().is_some_and(u8::is_ascii_whitespace) && !punctuation(&bytes);
            let prefix = pieces.last().is_some_and(|piece| punctuation(&piece.0));
            if pieces.is_empty() || word_start && !prefix {
                pieces.push((bytes, start, end));
            } else {
                let last = pieces.last_mut()?;
                last.0.extend(bytes);
                last.2 = last.2.max(end);
            }
        }
        let mut words = Vec::new();
        let mut previous = segment.start_ms;
        for (bytes, start, end) in pieces {
            let text = String::from_utf8(bytes).ok()?.trim().to_owned();
            if text.is_empty() {
                continue;
            }
            let (start, end) = (start.max(segment.start_ms), end.min(segment.end_ms));
            if start < previous || start >= end {
                return None;
            }
            previous = end;
            words.push(Segment {
                start_ms: start,
                end_ms: end,
                text,
            });
        }
        // Filtered text, invalid UTF-8 or unreliable token clocks keep the original whole segment.
        if words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            != segment.text
        {
            return None;
        }
        (!words.is_empty()).then_some(words)
    };
    split().unwrap_or_else(|| vec![segment])
}

fn speech_range(pcm: &[f32]) -> std::ops::Range<usize> {
    const FRAME: usize = SAMPLE_RATE as usize / 50;
    const MARGIN: usize = SAMPLE_RATE as usize / 4;
    let rms: Vec<f64> = pcm
        .chunks(FRAME)
        .map(|frame| (frame.iter().map(|&s| f64::from(s).powi(2)).sum::<f64>() / frame.len() as f64).sqrt())
        .collect();
    // A full meeting chunk may contain continuous speech: its quietest fifth is not room noise.
    // Whisper's no-speech probability and text filter still apply after this outer amplitude trim.
    let peak = rms.iter().copied().fold(0.0, f64::max);
    let threshold = (peak * 0.1).max(0.002);
    let (Some(first), Some(last)) = (
        rms.iter().position(|&r| r > threshold),
        rms.iter().rposition(|&r| r > threshold),
    ) else {
        return 0..0;
    };
    (first * FRAME).saturating_sub(MARGIN)..((last + 1) * FRAME + MARGIN).min(pcm.len())
}

#[cfg(test)]
mod tests;
