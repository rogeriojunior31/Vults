use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::SAMPLE_RATE;

/// Down-mix complete interleaved frames, rejecting invalid samples rather than inventing silence.
pub fn resample(samples: &[f32], channels: u16, rate: u32) -> Result<Vec<f32>, String> {
    if channels == 0 || !samples.len().is_multiple_of(usize::from(channels)) {
        return Err("invalid audio channel count or incomplete frame".into());
    }
    if samples.iter().any(|s| !s.is_finite()) {
        return Err("audio contains a non-finite sample".into());
    }
    let mono: Vec<f32> = samples
        .chunks_exact(usize::from(channels))
        .map(|frame| frame.iter().map(|s| s / f32::from(channels)).sum())
        .collect();
    let mut converter = Converter::new(rate)?;
    let mut out = converter.push(&mono)?;
    out.extend(converter.finish()?);
    Ok(out)
}

/// Stateful mono conversion to 16 kHz, retaining filter history across input blocks.
/// Call `finish` once at EOF to flush the exact remaining sample count.
pub struct Converter {
    fft: Option<Fft<f32>>,
    pending: Vec<f32>,
    skip: usize,
    rate: u32,
    input: usize,
    output: usize,
}

impl std::fmt::Debug for Converter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MonoResampler")
            .field("rate", &self.rate)
            .field("input", &self.input)
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}

impl Converter {
    pub fn new(rate: u32) -> Result<Self, String> {
        if !(8_000..=384_000).contains(&rate) {
            return Err("the microphone sample rate is outside 8-384 kHz".into());
        }
        let fft = if rate == SAMPLE_RATE {
            None
        } else {
            Some(
                Fft::new(rate as usize, SAMPLE_RATE as usize, 1024, 1, FixedSync::Both)
                    .map_err(|e| format!("can't prepare audio resampling: {e}"))?,
            )
        };
        let skip = fft.as_ref().map_or(0, Resampler::output_delay);
        Ok(Self {
            fft,
            pending: Vec::new(),
            skip,
            rate,
            input: 0,
            output: 0,
        })
    }

    pub fn push(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err("audio contains a non-finite sample".into());
        }
        self.input += samples.len();
        let Some(fft) = self.fft.as_ref() else {
            self.output += samples.len();
            return Ok(samples.to_vec());
        };
        let frames = fft.input_frames_next();
        self.pending.extend_from_slice(samples);
        let mut out = Vec::new();
        let mut consumed = 0;
        while self.pending.len() - consumed >= frames {
            let input = InterleavedSlice::new(&self.pending[consumed..consumed + frames], 1, frames)
                .map_err(|e| e.to_string())?;
            let block = self
                .fft
                .as_mut()
                .ok_or("missing audio resampler")?
                .process(&input, None)
                .map_err(|e| format!("can't resample audio: {e}"))?
                .take_data();
            let skip = self.skip.min(block.len());
            self.skip -= skip;
            out.extend_from_slice(&block[skip..]);
            consumed += frames;
        }
        self.pending.drain(..consumed);
        self.output += out.len();
        Ok(out)
    }

    pub fn finish(mut self) -> Result<Vec<f32>, String> {
        let wanted = (self.input as u64 * u64::from(SAMPLE_RATE) / u64::from(self.rate)) as usize;
        let mut out = Vec::new();
        while self.output < wanted {
            let frames = self
                .fft
                .as_ref()
                .ok_or("missing audio resampler")?
                .input_frames_next();
            let padding = frames - self.pending.len();
            let remaining = wanted - self.output;
            let block = self.push(&vec![0.0; padding])?;
            out.extend_from_slice(&block[..remaining.min(block.len())]);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rejected_block_does_not_advance_the_stream() {
        let mut stream = Converter::new(48_000).expect("resampler");
        assert!(stream.push(&[f32::NAN]).is_err());
        assert!(stream.push(&[f32::INFINITY]).is_err());
        assert!(stream.finish().expect("no input admitted").is_empty());
    }

    #[test]
    fn callback_boundaries_do_not_change_the_signal() {
        let input: Vec<f32> = (0..48_137).map(|i| (i as f32 * 0.1).sin() * 0.1).collect();
        let expected = resample(&input, 1, 48_000).expect("clip");
        let mut stream = Converter::new(48_000).expect("stream");
        let mut out = Vec::new();
        for chunk in input.chunks(137) {
            out.extend(stream.push(chunk).expect("chunk"));
        }
        out.extend(stream.finish().expect("flush"));
        assert_eq!(out, expected);
        for len in [0, 1, 20, 511, 1023] {
            assert_eq!(resample(&input[..len], 1, 48_000).expect("short").len(), len / 3);
        }
    }
}
