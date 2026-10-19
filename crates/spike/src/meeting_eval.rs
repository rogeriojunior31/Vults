//! Strict per-source diarization scoring. No audio, native runtime, network or model calls.
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_TURNS: usize = 4096;
const MAX_SPEAKERS: usize = 8;
const MAX_DURATION_MS: u64 = 180_000;
const MAX_JSON_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Clock {
    pub recording: String,
    pub source: String,
    pub wav_sha256: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Pt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Annotation {
    Human,
    Controlled,
    Synthetic,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Split {
    Calibration,
    Evaluation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Turn {
    pub start_ms: u64,
    pub end_ms: u64,
    /// None is one unassigned activity stream, never an identity eligible for matching.
    pub speaker: Option<u16>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Reference {
    pub schema: u8,
    pub clock: Clock,
    pub language: Language,
    pub annotation: Annotation,
    pub split: Split,
    pub turns: Vec<Turn>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Prediction {
    pub schema: u8,
    pub clock: Clock,
    pub turns: Vec<Turn>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub schema: u8,
    pub reference_sha256: String,
    pub prediction_sha256: String,
    pub clock: Clock,
    pub language: Language,
    pub annotation: Annotation,
    pub split: Split,
    pub collar_ms: u64,
    pub includes_overlap: bool,
    pub reference_speaker_ms: u64,
    pub missed_speaker_ms: u64,
    pub false_alarm_speaker_ms: u64,
    /// Reference absence is not necessarily acoustic silence (for example, unannotated laughter).
    pub false_alarm_without_reference_speaker_ms: u64,
    pub excess_overlap_speaker_ms: u64,
    pub confused_speaker_ms: u64,
    pub correct_speaker_ms: u64,
    pub unknown_activity_ms: u64,
    pub reference_speakers: usize,
    pub predicted_speakers: usize,
    /// Undefined on a silence-only reference; false alarms still remain explicit above.
    pub der: Option<f64>,
    pub production_qualified: bool,
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file: File = options.open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("meeting evaluation requires a regular JSON file".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_JSON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_JSON_BYTES {
        return Err("meeting evaluation JSON exceeds one MiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn validate(clock: &Clock, turns: &[Turn], reference: bool) -> Result<BTreeMap<u16, usize>, String> {
    let identifier = |s: &str| {
        !s.is_empty()
            && s.len() <= 64
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    };
    if !identifier(&clock.recording)
        || !identifier(&clock.source)
        || clock.wav_sha256.len() != 64
        || !clock
            .wav_sha256
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        || clock.duration_ms == 0
        || clock.duration_ms > MAX_DURATION_MS
        || turns.len() > MAX_TURNS
    {
        return Err("invalid meeting evaluation clock, hash or budget".into());
    }
    let mut speakers = BTreeSet::new();
    for turn in turns {
        if turn.start_ms >= turn.end_ms
            || turn.end_ms > clock.duration_ms
            || reference && turn.speaker.is_none()
        {
            return Err("invalid or unannotated meeting evaluation turn".into());
        }
        if let Some(speaker) = turn.speaker {
            speakers.insert(speaker);
        }
    }
    if speakers.len() > MAX_SPEAKERS {
        return Err("meeting evaluation supports at most eight speakers per source".into());
    }
    Ok(speakers
        .into_iter()
        .enumerate()
        .map(|(i, speaker)| (speaker, i))
        .collect())
}

/// Score the full source with a fixed zero collar, overlap retained and one global optimal
/// one-to-one label mapping. Equal duration alone is never evidence of a matching source clock.
pub fn score(reference: &Reference, prediction: &Prediction) -> Result<Report, String> {
    if reference.schema != 1 || prediction.schema != 1 || reference.clock != prediction.clock {
        return Err(
            "meeting evaluation requires the same schema, recording, source, WAV hash and duration".into(),
        );
    }
    let refs = validate(&reference.clock, &reference.turns, true)?;
    let hyps = validate(&prediction.clock, &prediction.turns, false)?;
    let mut report = Report {
        schema: 1,
        reference_sha256: fingerprint(reference)?,
        prediction_sha256: fingerprint(prediction)?,
        clock: reference.clock.clone(),
        language: reference.language,
        annotation: reference.annotation,
        split: reference.split,
        collar_ms: 0,
        includes_overlap: true,
        reference_speaker_ms: 0,
        missed_speaker_ms: 0,
        false_alarm_speaker_ms: 0,
        false_alarm_without_reference_speaker_ms: 0,
        excess_overlap_speaker_ms: 0,
        confused_speaker_ms: 0,
        correct_speaker_ms: 0,
        unknown_activity_ms: 0,
        reference_speakers: refs.len(),
        predicted_speakers: hyps.len(),
        der: None,
        production_qualified: false,
    };
    let mut events = Vec::with_capacity((reference.turns.len() + prediction.turns.len()) * 2);
    for (side, turns) in [&reference.turns, &prediction.turns].into_iter().enumerate() {
        for turn in turns {
            events.push((turn.start_ms, side, turn.speaker, true));
            events.push((turn.end_ms, side, turn.speaker, false));
        }
    }
    events.sort_unstable();
    let mut active: [BTreeMap<Option<u16>, usize>; 2] = Default::default();
    let mut weights = [[0u64; MAX_SPEAKERS]; MAX_SPEAKERS];
    let mut at = 0;
    let mut paired_ms = 0;
    for (until, side, speaker, entering) in events {
        let dt = until - at;
        if dt > 0 {
            let nr = active[0].len() as u64;
            let nh = active[1].len() as u64;
            report.reference_speaker_ms += dt * nr;
            report.missed_speaker_ms += dt * nr.saturating_sub(nh);
            let false_alarm_ms = dt * nh.saturating_sub(nr);
            report.false_alarm_speaker_ms += false_alarm_ms;
            if nr == 0 {
                report.false_alarm_without_reference_speaker_ms += false_alarm_ms;
            } else {
                report.excess_overlap_speaker_ms += false_alarm_ms;
            }
            paired_ms += dt * nr.min(nh);
            if active[1].contains_key(&None) {
                report.unknown_activity_ms += dt;
            }
            for r in active[0].keys().flatten() {
                for h in active[1].keys().flatten() {
                    weights[refs[r]][hyps[h]] += dt;
                }
            }
        }
        at = until;
        if entering {
            *active[side].entry(speaker).or_default() += 1;
        } else if let Some(count) = active[side].get_mut(&speaker) {
            *count -= 1;
            if *count == 0 {
                active[side].remove(&speaker);
            }
        }
    }
    report.correct_speaker_ms = optimal_overlap(&weights, refs.len(), hyps.len());
    report.confused_speaker_ms = paired_ms - report.correct_speaker_ms;
    if report.reference_speaker_ms > 0 {
        report.der = Some(
            (report.missed_speaker_ms + report.false_alarm_speaker_ms + report.confused_speaker_ms) as f64
                / report.reference_speaker_ms as f64,
        );
    }
    Ok(report)
}

fn fingerprint(value: &impl Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

// Eight speakers bound exact subset matching to 256 states; no greedy label-order dependence.
fn optimal_overlap(weights: &[[u64; MAX_SPEAKERS]; MAX_SPEAKERS], nr: usize, nh: usize) -> u64 {
    let mut best = vec![0; 1 << nh];
    for row in weights.iter().take(nr) {
        let mut next = best.clone();
        for (mask, &total) in best.iter().enumerate() {
            for (h, &overlap) in row.iter().enumerate().take(nh) {
                if mask & (1 << h) == 0 {
                    next[mask | (1 << h)] = next[mask | (1 << h)].max(total + overlap);
                }
            }
        }
        best = next;
    }
    best.into_iter().max().unwrap_or(0)
}

#[cfg(test)]
mod tests;
