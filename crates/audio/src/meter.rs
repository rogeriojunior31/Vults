use std::f32::consts::TAU;
use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex};

use crate::SAMPLE_RATE;

/// Normalized loudness and nine logarithmic speech bands, all in 0..1.
#[derive(Debug, Clone, Copy, Default)]
pub struct Meter {
    pub level: f32,
    pub bands: [f32; 9],
}

pub(crate) struct Metering {
    fft: Arc<dyn Fft<f32>>,
    ring: [f32; 512],
    window: [f32; 512],
    spectrum: [Complex<f32>; 512],
    scratch: Vec<Complex<f32>>,
    position: usize,
    frames: usize,
    squares: f32,
    bands: [f32; 9],
}

impl Metering {
    pub fn new() -> Self {
        let fft = FftPlanner::new().plan_fft_forward(512);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        Self {
            fft,
            scratch,
            ring: [0.0; 512],
            window: std::array::from_fn(|i| 0.5 - 0.5 * (TAU * i as f32 / 512.0).cos()),
            spectrum: [Complex::default(); 512],
            position: 0,
            frames: 0,
            squares: 0.0,
            bands: [0.0; 9],
        }
    }

    pub fn push(&mut self, sample: f32) -> Option<Meter> {
        self.ring[self.position] = sample;
        self.position = (self.position + 1) % 512;
        self.frames += 1;
        self.squares += sample * sample;
        if self.frames < SAMPLE_RATE as usize / 30 {
            return None;
        }
        let level = (self.squares / self.frames as f32).sqrt() * 6.0;
        self.frames = 0;
        self.squares = 0.0;
        for i in 0..512 {
            self.spectrum[i] = Complex::new(self.ring[(self.position + i) % 512] * self.window[i], 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.spectrum, &mut self.scratch);
        let mut peaks = [0.0_f32; 9];
        for (bin, value) in self.spectrum.iter().enumerate().take(257).skip(1) {
            let hz = bin as f32 * SAMPLE_RATE as f32 / 512.0;
            if !(90.0..=7_000.0).contains(&hz) {
                continue;
            }
            let band = ((hz / 90.0).ln() / (7_000.0_f32 / 90.0).ln() * 9.0) as usize;
            peaks[band.min(8)] = peaks[band.min(8)].max(value.norm() / 128.0);
        }
        for (i, (previous, peak)) in self.bands.iter_mut().zip(peaks).enumerate() {
            let db = 20.0 * peak.max(1e-12).log10() + i as f32 * 2.2;
            let target = ((db + 78.0) / 48.0).clamp(0.0, 1.0);
            let alpha = if target > *previous { 0.6 } else { 0.25 };
            *previous += alpha * (target - *previous);
        }
        Some(Meter {
            level: level.min(1.0),
            bands: self.bands,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tone_lights_its_band_and_silence_releases_it() {
        let mut meter = Metering::new();
        let mut updates = Vec::new();
        for i in 0..16_000 {
            if let Some(m) = meter.push(0.0003 * (TAU * 750.0 * i as f32 / 16_000.0).sin()) {
                updates.push(m);
            }
        }
        assert_eq!(updates.len(), 30);
        let last = updates.last().expect("meter");
        assert!(last.bands[4] > 0.3, "{last:?}");
        assert!(
            last.bands.iter().enumerate().all(|(i, v)| i == 4 || *v < 0.05),
            "{last:?}"
        );
        let mut last = *last;
        for _ in 0..16_000 {
            if let Some(m) = meter.push(0.0) {
                last = m;
            }
        }
        assert!(last.bands.iter().all(|v| *v < 0.001));
        assert_eq!(last.level, 0.0);
    }
}
