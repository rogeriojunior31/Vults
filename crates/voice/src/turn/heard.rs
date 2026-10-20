//! Frame-based transcript repair. Proportional words are an estimate, not acoustic alignment.

#[derive(Clone, Copy)]
pub struct Segment<'a> {
    pub text: &'a str,
    pub start_frame: u64,
    pub end_frame: u64,
}
impl std::fmt::Debug for Segment<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Segment")
            .field("bytes", &self.text.len())
            .field("start_frame", &self.start_frame)
            .field("end_frame", &self.end_frame)
            .finish()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Bounds,
    Text,
}

pub struct Heard {
    pub prefix: String,
    pub unheard: String,
    pub unheard_words: usize,
    pub interrupted: bool,
    nudge: Option<String>,
}
impl std::fmt::Debug for Heard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Heard")
            .field("prefix_bytes", &self.prefix.len())
            .field("unheard_words", &self.unheard_words)
            .field("interrupted", &self.interrupted)
            .finish()
    }
}
impl Heard {
    /// Replacement for model context, not a localized UI label. Caller owns response identity.
    pub fn history_text(&self) -> String {
        if !self.interrupted {
            return self.prefix.clone();
        }
        let prefix = self.prefix.trim_end();
        if prefix.is_empty() {
            "[interrupted]".into()
        } else {
            format!("{prefix} [interrupted]")
        }
    }
    /// A one-shot repair hint for the next authorized turn; this never sends or executes it.
    pub fn take_nudge(&mut self) -> Option<String> {
        self.nudge.take()
    }
}

/// Segments concatenate verbatim and have finalized, ordered, nonoverlapping audio extents.
/// All frame counts use 24 kHz mono audio. Explicit interruption also covers a producer
/// stopped after its currently received audio drained. Word timing is proportional.
pub fn split(
    segments: &[Segment<'_>],
    played_frames: u64,
    total_frames: u64,
    interrupted: bool,
) -> Result<Heard, Failure> {
    if played_frames > total_frames || segments.len() > 4096 || total_frames > 24_000 * 120 {
        return Err(Failure::Bounds);
    }
    let (mut bytes, mut previous) = (0_usize, 0);
    for segment in segments {
        bytes = bytes.checked_add(segment.text.len()).ok_or(Failure::Text)?;
        if bytes > 64 * 1024 || segment.text.contains('\0') {
            return Err(Failure::Text);
        }
        if segment.start_frame < previous
            || segment.end_frame <= segment.start_frame
            || segment.end_frame > total_frames
        {
            return Err(Failure::Bounds);
        }
        previous = segment.end_frame;
    }
    let (mut prefix, mut unheard) = (String::new(), String::new());
    for segment in segments {
        let cut = if played_frames <= segment.start_frame {
            0
        } else if played_frames >= segment.end_frame {
            segment.text.len()
        } else {
            let words = segment.text.split_whitespace().count();
            let count = ((words as u128 * u128::from(played_frames - segment.start_frame))
                / u128::from(segment.end_frame - segment.start_frame)) as usize;
            word_end(segment.text, count)
        };
        prefix.push_str(&segment.text[..cut]);
        unheard.push_str(&segment.text[cut..]);
    }
    let unheard_words = unheard.split_whitespace().count();
    let nudge = (unheard_words >= 3).then(|| unheard.clone());
    Ok(Heard {
        prefix,
        unheard,
        unheard_words,
        interrupted: interrupted || played_frames < total_frames,
        nudge,
    })
}

fn word_end(text: &str, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let (mut words, mut in_word) = (0, false);
    for (at, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if in_word {
                words += 1;
                if words == count {
                    return at;
                }
            }
            in_word = false;
        } else {
            in_word = true;
        }
    }
    text.len()
}

#[cfg(test)]
mod tests;
