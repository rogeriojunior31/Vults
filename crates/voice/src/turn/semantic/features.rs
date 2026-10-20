//! Whisper-compatible eight-second features; fixed shapes keep memory and work bounded.
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

pub const SAMPLES: usize = 8 * 16000;
pub const MELS: usize = 80;
pub const FRAMES: usize = 800;
const FFT: usize = 400;
const BINS: usize = FFT / 2 + 1;

pub(super) struct Features {
    fft: Arc<dyn Fft<f64>>,
    window: Vec<f64>,
    filters: Vec<Vec<f64>>,
}
impl Features {
    pub fn new() -> Self {
        let to_hz = |mel: f64| {
            if mel < 15.0 {
                mel * 200.0 / 3.0
            } else {
                1000.0 * ((mel - 15.0) * 6.4_f64.ln() / 27.0).exp()
            }
        };
        let max_mel = 15.0 + (8000.0_f64 / 1000.0).ln() * 27.0 / 6.4_f64.ln();
        let edges: Vec<_> = (0..MELS + 2)
            .map(|i| to_hz(max_mel * i as f64 / (MELS + 1) as f64))
            .collect();
        let filters = (0..MELS)
            .map(|mel| {
                (0..BINS)
                    .map(|bin| {
                        let hz = bin as f64 * 40.0;
                        let up = (hz - edges[mel]) / (edges[mel + 1] - edges[mel]);
                        let down = (edges[mel + 2] - hz) / (edges[mel + 2] - edges[mel + 1]);
                        up.min(down).max(0.0) * 2.0 / (edges[mel + 2] - edges[mel])
                    })
                    .collect()
            })
            .collect();
        Self {
            fft: FftPlanner::new().plan_fft_forward(FFT),
            window: (0..FFT)
                .map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / FFT as f64).cos())
                .collect(),
            filters,
        }
    }
    pub fn extract(
        &self,
        audio: &[f32],
        stop: &AtomicBool,
        deadline: Option<Instant>,
    ) -> Result<Option<Vec<f32>>, String> {
        if cancelled(stop, deadline)? {
            return Ok(None);
        }
        let audio = &audio[audio.len().saturating_sub(SAMPLES)..];
        if audio.iter().any(|s| !s.is_finite()) {
            return Err("semantic turn audio contains a nonfinite sample".into());
        }
        let mut padded = vec![0.0_f32; SAMPLES];
        padded[SAMPLES - audio.len()..].copy_from_slice(audio);
        let mean = (padded.iter().map(|&x| f64::from(x)).sum::<f64>() / SAMPLES as f64) as f32;
        let variance = (padded
            .iter()
            .map(|&x| f64::from((x - mean) * (x - mean)))
            .sum::<f64>()
            / SAMPLES as f64) as f32;
        let scale = (variance + 1e-7).sqrt();
        if !scale.is_finite() {
            return Err("semantic turn audio normalization overflowed".into());
        }
        for x in &mut padded {
            *x = (*x - mean) / scale;
        }
        let mut frame = vec![Complex::default(); FFT];
        let mut scratch = vec![Complex::default(); self.fft.get_inplace_scratch_len()];
        let mut power = vec![0.0; BINS];
        let mut result = vec![0.0_f32; MELS * FRAMES];
        for time in 0..FRAMES {
            if cancelled(stop, deadline)? {
                return Ok(None);
            }
            for (i, sample) in frame.iter_mut().enumerate() {
                let index = (time * 160 + i) as isize - (FFT / 2) as isize;
                // Reflect around the edge sample, matching centered STFT's reflect padding.
                let index = if index < 0 {
                    -index
                } else if index >= SAMPLES as isize {
                    2 * SAMPLES as isize - 2 - index
                } else {
                    index
                } as usize;
                *sample = Complex::new(f64::from(padded[index]) * self.window[i], 0.0);
            }
            self.fft.process_with_scratch(&mut frame, &mut scratch);
            for (power, bin) in power.iter_mut().zip(&frame) {
                // The reference stores complex64 FFT values before its float64 power/mel reduction.
                *power = f64::from(bin.re as f32).powi(2) + f64::from(bin.im as f32).powi(2);
            }
            for (mel, filter) in self.filters.iter().enumerate() {
                let energy: f64 = filter
                    .iter()
                    .zip(&power)
                    .map(|(weight, power)| weight * power)
                    .sum();
                result[mel * FRAMES + time] = energy.max(1e-10).log10() as f32;
            }
        }
        let floor = result.iter().copied().fold(f32::NEG_INFINITY, f32::max) - 8.0;
        for value in &mut result {
            *value = (value.max(floor) + 4.0) / 4.0;
        }
        if result.iter().any(|v| !v.is_finite()) {
            return Err("semantic turn features are invalid".into());
        }
        Ok((!cancelled(stop, deadline)?).then_some(result))
    }
}

fn cancelled(stop: &AtomicBool, deadline: Option<Instant>) -> Result<bool, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(true);
    }
    if deadline.is_some_and(|at| Instant::now() >= at) {
        return Err("semantic turn inference budget expired".into());
    }
    Ok(false)
}
