//! Optional, bounded local diarization. No capture, storage, downloads or app activation.

mod cluster;
mod control;
mod inference;

use std::io::{Read, Seek};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, TryLockError};

use super::{speaker_models::VerifiedModels, speakers::Timeline, wav};
pub use crate::onnx::Runtime;
pub use crate::vad::SpeechProfile;

const RATE: usize = crate::SAMPLE_RATE as usize;
const MAX_SAMPLES: usize = RATE * 30 * 60;
const MAX_DIAGNOSTIC_SAMPLES: usize = RATE * 180;
static ACTIVE: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Reading,
    Loading,
    Analyzing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub phase: Phase,
    pub completed_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug)]
pub struct Analysis {
    pub timeline: Timeline,
    pub speakers: usize,
    pub observations: usize,
}

/// Blocking: call on the recording worker while its consent/cleanup lock remains held.
/// Read one already-open source, normalize to 16 kHz mono, and analyze at most thirty minutes.
/// Only one analysis is admitted; competing jobs fail before audio reads or session allocation.
/// Sessions run CPU-only with one thread. Interactive speech preempts native inference; the exact
/// operation retries afterward with fresh options. Cancellation/failure returns no partial turns.
/// Session construction cannot abort; cancellation discards it before further work/publication.
/// Real-meeting quality and hardware acceptance remain release gates.
pub fn analyze_wav(
    _runtime: &Runtime,
    artifacts: VerifiedModels,
    input: impl Read + Seek,
    stop: &AtomicBool,
    mut progress: impl FnMut(Progress),
) -> Result<Option<Analysis>, String> {
    let Some(_job) = admit(stop)? else { return Ok(None) };
    let mut chunks = wav::Chunks::new(input)?;
    if chunks.total > MAX_SAMPLES as u64 {
        return Err("speaker analysis is limited to thirty minutes per source".into());
    }
    let duration_ms = chunks.total * 1000 / RATE as u64;
    progress(Progress {
        phase: Phase::Reading,
        completed_ms: 0,
        total_ms: duration_ms,
    });
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    progress(Progress {
        phase: Phase::Loading,
        completed_ms: 0,
        total_ms: duration_ms,
    });
    let Some(mut models) = inference::Models::load(&artifacts, stop)? else {
        return Ok(None);
    };
    drop(artifacts);
    let Some(result) = models.run(&mut chunks, stop, |completed_ms| {
        progress(Progress {
            phase: Phase::Analyzing,
            completed_ms,
            total_ms: duration_ms,
        })
    })?
    else {
        return Ok(None);
    };
    drop(models);
    Ok((!stop.load(Ordering::SeqCst)).then_some(Analysis {
        timeline: Timeline {
            duration_ms,
            turns: result.turns,
        },
        speakers: result.speakers,
        observations: result.observations,
    }))
}

#[cfg(feature = "meeting-assets")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribution {
    Analyzed,
    Unavailable,
}

#[cfg(feature = "meeting-assets")]
#[derive(Debug)]
pub struct IdentifiedTranscript {
    pub transcript: super::speakers::AttributedTranscript,
    pub attribution: Attribution,
}

/// Blocking source-local handoff: ASR and analysis share the same owned WAV descriptor.
/// An absent bundle or failed analysis keeps the transcript unassigned. No download or repair.
/// Analysis failure preserves all original text/times as unknown; cancellation publishes nothing.
/// This composes the production primitives without qualifying automatic labels in the app.
#[cfg(feature = "meeting-assets")]
pub fn transcribe_wav(
    transcriber: &crate::Transcriber,
    bundle: Option<&super::assets::Bundle>,
    mut input: impl Read + Seek,
    language: Option<&str>,
    stop: &AtomicBool,
    transcription_progress: impl FnMut(super::Progress),
    analysis_progress: impl FnMut(Progress),
) -> Result<Option<IdentifiedTranscript>, String> {
    let Some(transcript) =
        super::transcribe_words_wav(transcriber, &mut input, language, stop, transcription_progress)?
    else {
        return Ok(None);
    };
    let result = if let Some(bundle) = bundle {
        (|| {
            input.rewind().map_err(|e| e.to_string())?;
            let Some(models) =
                super::speaker_models::read_models(&bundle.segmentation(), &bundle.embedding(), stop)?
            else {
                return Ok(None);
            };
            let Some(runtime) = Runtime::load(&bundle.runtime(), super::assets::RUNTIME_SHA256, stop)? else {
                return Ok(None);
            };
            analyze_wav(&runtime, models, &mut input, stop, analysis_progress)
        })()
    } else {
        Ok(None)
    };
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    finish_transcript(transcript, result, stop)
}

#[cfg(feature = "meeting-assets")]
fn finish_transcript(
    transcript: super::Transcript,
    analysis: Result<Option<Analysis>, String>,
    stop: &AtomicBool,
) -> Result<Option<IdentifiedTranscript>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if let Ok(Some(analysis)) = analysis {
        match super::speakers::associate(transcript.clone(), &analysis.timeline, stop) {
            Ok(Some(transcript)) => {
                return Ok(Some(IdentifiedTranscript {
                    transcript,
                    attribution: Attribution::Analyzed,
                }));
            }
            Ok(None) => return Ok(None),
            Err(_) => {}
        }
    }
    let timeline = Timeline {
        duration_ms: transcript.duration_ms,
        turns: Vec::new(),
    };
    let Some(transcript) = super::speakers::associate(transcript, &timeline, stop)? else {
        return Ok(None);
    };
    Ok((!stop.load(Ordering::SeqCst)).then_some(IdentifiedTranscript {
        transcript,
        attribution: Attribution::Unavailable,
    }))
}

/// Offline diagnostic only: select a Silero profile with a 200 ms speech margin.
/// The caller supplies a trusted immutable model. No download, recording or app activation.
/// Admission waits behind interactive speech; Silero has no native abort callback, so this
/// bounded call (at most three minutes of audio) finishes before cancellation discards its result.
pub fn speech_activity_wav(
    input: impl Read + Seek,
    model: &std::path::Path,
    stop: &AtomicBool,
    profile: SpeechProfile,
) -> Result<Option<Timeline>, String> {
    let Some(_job) = admit(stop)? else { return Ok(None) };
    let Some(pcm) = read_audio(input, stop, &mut |_| {})? else {
        return Ok(None);
    };
    let Some(regions) = crate::vad::Vad::speech_background(model, &pcm, stop, profile)? else {
        return Ok(None);
    };
    let mut turns: Vec<super::speakers::Turn> = Vec::new();
    for region in regions {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let start_ms = region.start.saturating_sub(crate::vad::MARGIN) as u64 * 1000 / RATE as u64;
        let end_ms = (region.end + crate::vad::MARGIN).min(pcm.len()) as u64 * 1000 / RATE as u64;
        if let Some(last) = turns.last_mut()
            && last.end_ms >= start_ms
        {
            last.end_ms = last.end_ms.max(end_ms);
        } else if start_ms < end_ms {
            turns.push(super::speakers::Turn {
                start_ms,
                end_ms,
                speaker: None,
            });
        }
    }
    Ok((!stop.load(Ordering::SeqCst)).then_some(Timeline {
        duration_ms: pcm.len() as u64 * 1000 / RATE as u64,
        turns,
    }))
}

fn admit(stop: &AtomicBool) -> Result<Option<MutexGuard<'static, ()>>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    match ACTIVE.try_lock() {
        Ok(guard) => Ok(Some(guard)),
        Err(TryLockError::WouldBlock) => Err("speaker analysis is already running".into()),
        // This mutex owns only admission, not reusable model state.
        Err(TryLockError::Poisoned(error)) => Ok(Some(error.into_inner())),
    }
}

fn read_audio(
    input: impl Read + Seek,
    stop: &AtomicBool,
    progress: &mut impl FnMut(Progress),
) -> Result<Option<Vec<f32>>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let mut chunks = wav::Chunks::new(input)?;
    if chunks.total > MAX_DIAGNOSTIC_SAMPLES as u64 {
        return Err("speech diagnostic is limited to three minutes per source".into());
    }
    let total_ms = chunks.total * 1000 / RATE as u64;
    let mut pcm = Vec::with_capacity(chunks.total as usize);
    progress(Progress {
        phase: Phase::Reading,
        completed_ms: 0,
        total_ms,
    });
    while let Some(chunk) = chunks.next(stop)? {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if pcm.len() + chunk.len() > MAX_DIAGNOSTIC_SAMPLES {
            return Err("speaker audio exceeds its sample budget".into());
        }
        pcm.extend(chunk);
        progress(Progress {
            phase: Phase::Reading,
            completed_ms: chunks.offset * 1000 / RATE as u64,
            total_ms,
        });
    }
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if chunks.offset != chunks.total || pcm.is_empty() {
        return Err("speaker audio has an incomplete timeline".into());
    }
    Ok(Some(pcm))
}

#[cfg(test)]
mod native_tests;
#[cfg(test)]
mod tests;
