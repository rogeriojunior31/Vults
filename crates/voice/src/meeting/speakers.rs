//! Associate anonymous turns with one source's text without inventing word timing or identity.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Segment, Transcript};

const MAX_ITEMS: usize = 100_000;
const MAX_DURATION_MS: u64 = 4 * 60 * 60 * 1000;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker: Option<usize>,
}

/// Anonymous speaker IDs belong to this source/result only, never to a person or another track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    pub duration_ms: u64,
    pub turns: Vec<Turn>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AttributedSegment {
    pub segment: Segment,
    pub speaker: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AttributedTranscript {
    pub duration_ms: u64,
    pub segments: Vec<AttributedSegment>,
}

/// Blocking, bounded association on one already-aligned source clock. No IO or model calls.
/// A whole segment gets a label only when one known voice covers it continuously and alone.
/// Preserve the text/times exactly; speaker changes, overlap, unknown turns and gaps stay unknown.
/// Cancellation or invalid input yields no partial result. Call separately for each audio track.
pub fn associate(
    transcript: Transcript,
    timeline: &Timeline,
    stop: &AtomicBool,
) -> Result<Option<AttributedTranscript>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if transcript.duration_ms == 0
        || transcript.duration_ms > MAX_DURATION_MS
        || transcript.duration_ms != timeline.duration_ms
        || transcript.segments.len() > MAX_ITEMS
        || timeline.turns.len() > MAX_ITEMS
    {
        return Err("speaker and transcript timelines must match within the meeting limits".into());
    }
    let mut previous_end = 0;
    let mut text_bytes = 0;
    for segment in &transcript.segments {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        text_bytes += segment.text.len();
        if segment.start_ms < previous_end
            || segment.start_ms >= segment.end_ms
            || segment.end_ms > transcript.duration_ms
            || text_bytes > MAX_TEXT_BYTES
        {
            return Err("invalid meeting text timeline or size".into());
        }
        previous_end = segment.end_ms;
    }
    let mut events = Vec::with_capacity(timeline.turns.len() * 2);
    for turn in &timeline.turns {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if turn.start_ms >= turn.end_ms || turn.end_ms > timeline.duration_ms {
            return Err("invalid speaker turn timeline".into());
        }
        events.push((turn.start_ms, turn.speaker, true));
        events.push((turn.end_ms, turn.speaker, false));
    }
    events.sort_unstable();
    let mut active: BTreeMap<Option<usize>, usize> = BTreeMap::new();
    let mut spans: Vec<Turn> = Vec::new();
    let mut at = 0;
    for (until, speaker, entering) in events {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if until > at {
            let voice = if active.len() == 1 {
                active.first_key_value().and_then(|(&speaker, _)| speaker)
            } else {
                None
            };
            if let Some(last) = spans.last_mut()
                && last.speaker == voice
            {
                last.end_ms = until;
            } else {
                spans.push(Turn {
                    start_ms: at,
                    end_ms: until,
                    speaker: voice,
                });
            }
            at = until;
        }
        if entering {
            *active.entry(speaker).or_default() += 1;
        } else if let Some(count) = active.get_mut(&speaker) {
            *count -= 1;
            if *count == 0 {
                active.remove(&speaker);
            }
        }
    }
    let mut segments = Vec::with_capacity(transcript.segments.len());
    for segment in transcript.segments {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let index = spans.partition_point(|turn| turn.end_ms <= segment.start_ms);
        let speaker = spans
            .get(index)
            .filter(|turn| turn.start_ms <= segment.start_ms && segment.end_ms <= turn.end_ms)
            .and_then(|turn| turn.speaker);
        segments.push(AttributedSegment { segment, speaker });
    }
    Ok((!stop.load(Ordering::SeqCst)).then_some(AttributedTranscript {
        duration_ms: transcript.duration_ms,
        segments,
    }))
}

#[cfg(test)]
mod tests;
