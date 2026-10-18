use std::f32::consts::TAU;

use vults_audio::resample;

fn tone(hz: f32, rate: u32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|i| 0.2 * (TAU * hz * i as f32 / rate as f32).sin())
        .collect()
}

#[test]
fn a_resampled_tone_keeps_its_pitch_level_and_tail() {
    for rate in [8_000, 44_100, 48_000, 96_000] {
        let input = tone(1_000.0, rate, rate as usize + 137);
        let out = resample(&input, 1, rate).expect("resampling");
        assert_eq!(out.len(), input.len() * 16_000 / rate as usize);
        let middle = &out[512..out.len() - 512];
        let rms = (middle.iter().map(|x| x * x).sum::<f32>() / middle.len() as f32).sqrt();
        assert!((rms - 0.2 / 2.0_f32.sqrt()).abs() < 0.003, "rate {rate}: {rms}");
        let crossings = middle.windows(2).filter(|p| p[0] <= 0.0 && p[1] > 0.0).count();
        let hz = crossings as f32 * 16_000.0 / middle.len() as f32;
        assert!((hz - 1_000.0).abs() < 2.0, "rate {rate}: {hz}");
        let last = out.len() - 128;
        assert!(out[last..].iter().any(|x| x.abs() > 0.05), "tail was not flushed");
    }
}

#[test]
fn downsampling_rejects_out_of_band_sound() {
    let out = resample(&tone(12_000.0, 48_000, 48_000), 1, 48_000).expect("resampling");
    let middle = &out[512..out.len() - 512];
    let rms = (middle.iter().map(|x| x * x).sum::<f32>() / middle.len() as f32).sqrt();
    assert!(rms < 0.002, "aliased tone: {rms}");
}

#[test]
fn stereo_is_mixed_and_invalid_formats_are_refused() {
    let stereo: Vec<f32> = (0..48_000).flat_map(|_| [0.5, -0.5]).collect();
    assert!(
        resample(&stereo, 2, 48_000)
            .expect("stereo")
            .iter()
            .all(|x| x.abs() < 1e-6)
    );
    assert_eq!(resample(&[0.1, 0.2], 1, 16_000).expect("native"), [0.1, 0.2]);
    assert!(resample(&[], 1, 48_000).expect("empty").is_empty());
    assert!(resample(&[0.1], 0, 48_000).is_err());
    assert!(resample(&[0.1], 1, 0).is_err());
    assert!(resample(&[f32::NAN], 1, 48_000).is_err());
    assert!(resample(&[0.1], 2, 48_000).is_err());
}
