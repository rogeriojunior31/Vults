use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::AppHandle;
use vults_audio::meeting::Tracks;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    start_ms: u64,
    end_ms: u64,
    text: String,
    speaker: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    duration_ms: u64,
    segments: Vec<Segment>,
    attribution: Option<&'static str>,
}
impl From<vults_voice::meeting::Transcript> for Track {
    fn from(value: vults_voice::meeting::Transcript) -> Self {
        Self {
            duration_ms: value.duration_ms,
            attribution: None,
            segments: value
                .segments
                .into_iter()
                .map(|s| Segment {
                    start_ms: s.start_ms,
                    end_ms: s.end_ms,
                    text: s.text,
                    speaker: None,
                })
                .collect(),
        }
    }
}

impl Track {
    pub(super) fn archived(&self) -> Result<vults_store::meetings::Track, String> {
        let invalid = |_| "Meeting timing or speaker identity is invalid.".to_owned();
        Ok(vults_store::meetings::Track {
            duration_ms: self.duration_ms.try_into().map_err(invalid)?,
            segments: self
                .segments
                .iter()
                .map(|s| {
                    Ok(vults_store::meetings::Segment {
                        start_ms: s.start_ms.try_into().map_err(invalid)?,
                        end_ms: s.end_ms.try_into().map_err(invalid)?,
                        text: s.text.clone(),
                        speaker: s.speaker.map(u32::try_from).transpose().map_err(invalid)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        })
    }

    pub(super) fn summary_source(&self) -> String {
        self.segments
            .iter()
            .map(|segment| {
                let voice = segment.speaker.map_or_else(
                    || "Unknown voice".into(),
                    |id| format!("Voice {}", id.saturating_add(1)),
                );
                format!(
                    "[{}-{} ms] {voice}: {}\n",
                    segment.start_ms, segment.end_ms, segment.text
                )
            })
            .collect()
    }
}

#[cfg(feature = "meeting-speakers")]
impl From<vults_voice::meeting::diarization::IdentifiedTranscript> for Track {
    fn from(value: vults_voice::meeting::diarization::IdentifiedTranscript) -> Self {
        Self {
            duration_ms: value.transcript.duration_ms,
            attribution: Some(match value.attribution {
                vults_voice::meeting::diarization::Attribution::Analyzed => "analyzed",
                vults_voice::meeting::diarization::Attribution::Unavailable => "unavailable",
            }),
            segments: value
                .transcript
                .segments
                .into_iter()
                .map(|s| Segment {
                    start_ms: s.segment.start_ms,
                    end_ms: s.segment.end_ms,
                    text: s.segment.text,
                    speaker: s.speaker,
                })
                .collect(),
        }
    }
}

pub struct Options {
    pub model_id: String,
    pub language: Option<String>,
    pub identify_speakers: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub id: u64,
    pub microphone: Track,
    pub system: Track,
}

#[cfg(test)]
mod summary_tests {
    use super::*;

    #[test]
    fn archive_conversion_keeps_unknowns_and_rejects_unrepresentable_timing() {
        let mut track = Track {
            duration_ms: 300,
            attribution: Some("analyzed"),
            segments: vec![
                Segment {
                    start_ms: 0,
                    end_ms: 100,
                    text: "Unknown words".into(),
                    speaker: None,
                },
                Segment {
                    start_ms: 100,
                    end_ms: 300,
                    text: "Known voice".into(),
                    speaker: Some(3),
                },
            ],
        };
        let archived = track.archived().unwrap();
        assert_eq!(archived.segments[0].speaker, None);
        assert_eq!(archived.segments[1].speaker, Some(3));
        assert_eq!(archived.segments[1].start_ms, 100);
        track.duration_ms = u64::from(u32::MAX) + 1;
        assert!(track.archived().is_err());
    }

    #[test]
    fn canonical_source_text_keeps_local_clocks_and_unknown_speakers() {
        let track = Track {
            duration_ms: 300,
            attribution: None,
            segments: vec![
                Segment {
                    start_ms: 0,
                    end_ms: 100,
                    text: "Original words.".into(),
                    speaker: None,
                },
                Segment {
                    start_ms: 100,
                    end_ms: 300,
                    text: "Other words.".into(),
                    speaker: Some(0),
                },
            ],
        };
        assert_eq!(
            track.summary_source(),
            "[0-100 ms] Unknown voice: Original words.\n[100-300 ms] Voice 1: Other words.\n"
        );
        assert_eq!(track.segments[0].text, "Original words.");
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub track: &'static str,
    pub completed_ms: u64,
    pub total_ms: u64,
    pub stage: &'static str,
}

#[derive(Debug, Default)]
pub struct Job {
    pub progress: Option<Progress>,
    pub result: Option<Transcript>,
    pub failed: bool,
}

pub fn run(
    app: &AppHandle,
    id: u64,
    options: Options,
    tracks: Tracks,
    stop: &AtomicBool,
    shared: &Mutex<Job>,
) -> Result<(), String> {
    let result = transcribe(app, id, &options, tracks, stop, shared);
    let mut job = shared.lock().unwrap_or_else(|e| e.into_inner());
    match result {
        Ok(transcript) if !stop.load(Ordering::SeqCst) => job.result = transcript,
        Err(_) => job.failed = true,
        _ => {}
    }
    if job.failed {
        Err("meeting transcription failed".into())
    } else {
        Ok(())
    }
}

fn transcribe(
    app: &AppHandle,
    id: u64,
    options: &Options,
    tracks: Tracks,
    stop: &AtomicBool,
    shared: &Mutex<Job>,
) -> Result<Option<Transcript>, String> {
    let model_id = options.model_id.as_str();
    let language = options.language.as_deref();
    let model = vults_voice::model(model_id).ok_or("unknown meeting model")?;
    let transcriber = if let Some(cached) = crate::voice::cached_transcriber(app, model_id) {
        cached
    } else {
        let Some(loaded) = vults_voice::Transcriber::load_background(
            &vults_voice::model_path(&crate::voice::models_dir(), model),
            stop,
        )?
        else {
            return Ok(None);
        };
        Arc::new(loaded)
    };
    let report = |track, stage, completed_ms, total_ms| {
        let mut job = shared.lock().unwrap_or_else(|e| e.into_inner());
        job.progress = Some(Progress {
            track,
            stage,
            completed_ms,
            total_ms,
        });
    };
    #[cfg(feature = "meeting-speakers")]
    let bundle = if options.identify_speakers {
        vults_voice::meeting::assets::installed_for_analysis(
            &crate::voice::models_dir().join("meetings"),
            stop,
        )
        .ok()
        .flatten()
    } else {
        None
    };
    let source = |input, track| -> Result<Option<Track>, String> {
        let progress =
            |p: vults_voice::meeting::Progress| report(track, "transcription", p.completed_ms, p.total_ms);
        #[cfg(feature = "meeting-speakers")]
        if options.identify_speakers {
            return vults_voice::meeting::diarization::transcribe_wav(
                &transcriber,
                bundle.as_ref(),
                input,
                language,
                stop,
                progress,
                |p| report(track, "speakers", p.completed_ms, p.total_ms),
            )
            .map(|result| result.map(Track::from));
        }
        #[cfg(not(feature = "meeting-speakers"))]
        let _ = options.identify_speakers;
        vults_voice::meeting::transcribe_wav(&transcriber, input, language, stop, progress)
            .map(|result| result.map(Track::from))
    };
    let Some(microphone) = source(tracks.microphone, "microphone")? else {
        return Ok(None);
    };
    let Some(system) = source(tracks.system, "system")? else {
        return Ok(None);
    };
    Ok(Some(Transcript {
        id,
        microphone,
        system,
    }))
}
