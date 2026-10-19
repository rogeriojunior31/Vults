use std::io::{BufReader, Read, Seek};
use std::sync::atomic::{AtomicBool, Ordering};

use hound::{SampleFormat, WavReader};
use vults_audio::MonoResampler;

use crate::SAMPLE_RATE;

pub(super) const MAX: usize = SAMPLE_RATE as usize * 28;
const WINDOW: usize = SAMPLE_RATE as usize / 10;
const SEARCH: usize = SAMPLE_RATE as usize * 2;

pub(super) struct Chunks<R> {
    reader: WavReader<BufReader<R>>,
    resampler: Option<MonoResampler>,
    pcm: Vec<f32>,
    read: u64,
    pub total: u64,
    pub offset: u64,
}

impl<R: Read + Seek> Chunks<R> {
    pub fn new(input: R) -> Result<Self, String> {
        // Hound reads individual samples; buffer raw files to avoid a syscall per sample.
        let reader =
            WavReader::new(BufReader::new(input)).map_err(|e| format!("invalid meeting WAV: {e}"))?;
        let spec = reader.spec();
        if !(1..=32).contains(&spec.channels)
            || !(8_000..=192_000).contains(&spec.sample_rate)
            || !reader.len().is_multiple_of(u32::from(spec.channels))
            || !matches!(
                (spec.sample_format, spec.bits_per_sample),
                (SampleFormat::Float, 32) | (SampleFormat::Int, 16 | 24 | 32)
            )
        {
            return Err("unsupported meeting WAV format".into());
        }
        let frames = u64::from(reader.len()) / u64::from(spec.channels);
        if frames == 0 || frames > u64::from(spec.sample_rate) * 4 * 60 * 60 {
            return Err("meeting WAV must contain audio and be at most four hours long".into());
        }
        let total = frames * u64::from(SAMPLE_RATE) / u64::from(spec.sample_rate);
        if total < u64::from(SAMPLE_RATE) / 1000 {
            return Err("meeting WAV must contain at least one millisecond of audio".into());
        }
        Ok(Self {
            reader,
            resampler: Some(MonoResampler::new(spec.sample_rate)?),
            pcm: Vec::with_capacity(MAX + SAMPLE_RATE as usize / 5),
            read: 0,
            total,
            offset: 0,
        })
    }

    pub fn next(&mut self, stop: &AtomicBool) -> Result<Option<Vec<f32>>, String> {
        while self.pcm.len() < MAX && self.resampler.is_some() {
            if stop.load(Ordering::SeqCst) {
                return Ok(None);
            }
            let spec = self.reader.spec();
            let left = u64::from(self.reader.len()).saturating_sub(self.read);
            if left == 0 {
                let resampler = self.resampler.take().ok_or("missing meeting resampler")?;
                self.pcm.extend(resampler.finish()?);
                break;
            }
            let count = left.min(u64::from(spec.sample_rate / 10) * u64::from(spec.channels)) as usize;
            let samples: Vec<f32> = if spec.sample_format == SampleFormat::Float {
                self.reader.samples::<f32>().take(count).collect::<Result<_, _>>()
            } else {
                let scale = (1_u64 << (spec.bits_per_sample - 1)) as f32;
                self.reader
                    .samples::<i32>()
                    .take(count)
                    .map(|sample| sample.map(|sample| sample as f32 / scale))
                    .collect::<Result<_, _>>()
            }
            .map_err(|e| format!("could not read meeting WAV: {e}"))?;
            if samples.len() != count || samples.iter().any(|sample| !sample.is_finite()) {
                return Err("meeting WAV is truncated or contains non-finite samples".into());
            }
            self.read += count as u64;
            let mono: Vec<f32> = samples
                .chunks_exact(usize::from(spec.channels))
                .map(|frame| frame.iter().map(|sample| sample / f32::from(spec.channels)).sum())
                .collect();
            self.pcm.extend(
                self.resampler
                    .as_mut()
                    .ok_or("missing meeting resampler")?
                    .push(&mono)?,
            );
        }
        if stop.load(Ordering::SeqCst) || self.pcm.is_empty() {
            return Ok(None);
        }
        if self.pcm.iter().any(|sample| !sample.is_finite()) {
            return Err("meeting WAV resampling returned non-finite samples".into());
        }
        let end = if self.total.saturating_sub(self.offset) <= MAX as u64 {
            self.pcm.len().min(MAX)
        } else {
            // Leave enough audio for a final decode, even when the WAV barely exceeds 28 s.
            let latest = (self.total - self.offset - u64::from(SAMPLE_RATE) / 2).min(MAX as u64) as usize;
            boundary(&self.pcm[..latest])
        };
        self.offset += end as u64;
        Ok(Some(self.pcm.drain(..end).collect()))
    }
}

/// Cut at the middle of the quietest 100 ms window in the last two seconds.
/// Equal-energy windows prefer the latest cut so silence does not create tiny chunks.
fn boundary(pcm: &[f32]) -> usize {
    let end = pcm.len().min(MAX);
    let start = end.saturating_sub(SEARCH);
    let square = |sample: f32| f64::from(sample).powi(2);
    let mut energy = pcm[start..start + WINDOW].iter().map(|&s| square(s)).sum::<f64>();
    let mut lowest = energy;
    let mut best = start + WINDOW / 2;
    for at in start + 1..=end - WINDOW {
        energy += square(pcm[at + WINDOW - 1]) - square(pcm[at - 1]);
        if energy <= lowest {
            lowest = energy;
            best = at + WINDOW / 2;
        }
    }
    best
}

#[cfg(test)]
mod tests;
