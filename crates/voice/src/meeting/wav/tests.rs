use super::*;
use std::io::Cursor;

pub(crate) fn wav(samples: &[f32], rate: u32, channels: u16, format: SampleFormat, bits: u16) -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(
        &mut buffer,
        hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: bits,
            sample_format: format,
        },
    )
    .expect("fixture writer");
    for &sample in samples {
        if format == SampleFormat::Float {
            writer.write_sample(sample).expect("float sample");
        } else {
            writer
                .write_sample((sample * ((1_u64 << (bits - 1)) - 1) as f32) as i32)
                .expect("integer sample");
        }
    }
    writer.finalize().expect("fixture WAV");
    buffer.into_inner()
}

#[test]
fn wav_admission_requires_a_representable_clock_across_formats_and_rates() {
    for rate in [8_000_u32, 8_001, 16_000, 22_050, 44_100, 48_000, 192_000] {
        for channels in [1_u16, 2] {
            for (format, bits) in [
                (SampleFormat::Float, 32),
                (SampleFormat::Int, 16),
                (SampleFormat::Int, 24),
                (SampleFormat::Int, 32),
            ] {
                let frames = rate.div_ceil(1000) as usize;
                let below = wav(
                    &vec![0.1; (frames - 1) * usize::from(channels)],
                    rate,
                    channels,
                    format,
                    bits,
                );
                assert!(
                    Chunks::new(Cursor::new(below)).is_err(),
                    "rate={rate}, channels={channels}, bits={bits}"
                );
                let input = wav(
                    &vec![0.1; frames * usize::from(channels)],
                    rate,
                    channels,
                    format,
                    bits,
                );
                let mut chunks = Chunks::new(Cursor::new(input)).expect("first representable duration");
                assert_eq!(chunks.total * 1000 / u64::from(SAMPLE_RATE), 1);
                let pcm = chunks
                    .next(&AtomicBool::new(false))
                    .expect("read")
                    .expect("audio");
                assert_eq!(pcm.len() as u64, chunks.total);
                assert_eq!(chunks.offset, chunks.total);
                assert!(chunks.next(&AtomicBool::new(false)).expect("EOF").is_none());
            }
        }
    }
}

#[test]
fn quiet_cuts_cover_every_sample_and_never_leave_a_tiny_tail() {
    let stop = AtomicBool::new(false);
    for length in [
        SAMPLE_RATE as usize / 1000,
        MAX - 1,
        MAX,
        MAX + 1,
        MAX + WINDOW,
        MAX * 2 + WINDOW,
        MAX * 3,
    ] {
        let mut samples: Vec<f32> = (0..length).map(|i| (i as f32 * 0.037).sin() * 0.3).collect();
        if length > MAX {
            samples[MAX - SEARCH + WINDOW * 3..MAX - SEARCH + WINDOW * 4].fill(0.0);
        }
        let mut chunks = Chunks::new(Cursor::new(wav(
            &samples,
            SAMPLE_RATE,
            1,
            SampleFormat::Float,
            32,
        )))
        .expect("chunks");
        let mut recovered = Vec::new();
        while let Some(chunk) = chunks.next(&stop).expect("read") {
            assert!(chunk.len() <= MAX);
            if length > MAX {
                assert!(chunk.len() >= SAMPLE_RATE as usize / 2);
            }
            if recovered.is_empty() && length > MAX {
                assert_eq!(chunk.len(), MAX - SEARCH + WINDOW * 3 + WINDOW / 2);
            }
            recovered.extend(chunk);
        }
        assert_eq!(recovered, samples);
        assert_eq!(chunks.offset, chunks.total);
    }
}

#[test]
fn file_reads_are_batched_without_changing_normalized_audio_or_its_clock() {
    use std::cell::Cell;
    use std::io::{Result as IoResult, SeekFrom};

    struct CountReads<'a> {
        input: Cursor<Vec<u8>>,
        calls: &'a Cell<usize>,
    }
    impl Read for CountReads<'_> {
        fn read(&mut self, bytes: &mut [u8]) -> IoResult<usize> {
            self.calls.set(self.calls.get() + 1);
            self.input.read(bytes)
        }
    }
    impl Seek for CountReads<'_> {
        fn seek(&mut self, position: SeekFrom) -> IoResult<u64> {
            self.input.seek(position)
        }
    }

    let samples: Vec<f32> = (0..48_000)
        .flat_map(|i| {
            let sample = (i as f32 * 0.05).sin() * 0.3;
            [sample, sample * 0.5]
        })
        .collect();
    let expected = vults_audio::resample(&samples, 2, 48_000).expect("reference");
    let bytes = wav(&samples, 48_000, 2, SampleFormat::Float, 32);
    let max_calls = bytes.len() / 1024 + 32;
    let calls = Cell::new(0);
    let mut chunks = Chunks::new(CountReads {
        input: Cursor::new(bytes),
        calls: &calls,
    })
    .expect("chunks");
    let stop = AtomicBool::new(false);
    let mut recovered = Vec::new();
    while let Some(chunk) = chunks.next(&stop).expect("read") {
        recovered.extend(chunk);
    }
    assert_eq!(recovered, expected);
    assert_eq!(chunks.offset, 16_000);
    assert_eq!(chunks.offset, chunks.total);
    assert!(
        calls.get() <= max_calls,
        "too many backing reads: {} > {max_calls}",
        calls.get()
    );
}

#[test]
fn continuous_resampling_matches_the_original_and_stays_bounded() {
    let stop = AtomicBool::new(false);
    let frames = 48_000 * 29 + 137;
    let samples: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let sample = (i as f32 * 0.05).sin() * 0.3;
            [sample, sample * 0.5]
        })
        .collect();
    let expected = vults_audio::resample(&samples, 2, 48_000).expect("reference");
    let mut chunks =
        Chunks::new(Cursor::new(wav(&samples, 48_000, 2, SampleFormat::Float, 32))).expect("chunks");
    let mut recovered = Vec::new();
    while let Some(chunk) = chunks.next(&stop).expect("read") {
        assert!(chunks.pcm.len() <= SEARCH + SAMPLE_RATE as usize / 5);
        recovered.extend(chunk);
    }
    assert_eq!(recovered, expected);
    assert_eq!(chunks.offset, frames as u64 / 3);
}

#[test]
fn integer_widths_and_nonstandard_rates_keep_the_original_duration() {
    let stop = AtomicBool::new(false);
    for bits in [16, 24, 32] {
        for rate in [8_000, 16_000, 44_100, 192_000] {
            let samples: Vec<f32> = (0..rate / 5 * 2)
                .map(|i| (i as f32 * 0.05).sin() * 0.25)
                .collect();
            let mut chunks =
                Chunks::new(Cursor::new(wav(&samples, rate, 2, SampleFormat::Int, bits))).expect("chunks");
            let pcm = chunks.next(&stop).expect("read").expect("audio");
            assert_eq!(pcm.len(), SAMPLE_RATE as usize / 5);
            assert!(pcm.iter().all(|s| s.is_finite()));
            assert!(pcm.iter().any(|s| s.abs() > 0.1));
            assert!(chunks.next(&stop).expect("EOF").is_none());
        }
    }
}

#[test]
fn malformed_truncated_and_invalid_samples_fail_without_silent_repair() {
    let stop = AtomicBool::new(false);
    assert!(Chunks::new(Cursor::new(b"not a WAV")).is_err());
    assert!(Chunks::new(Cursor::new(wav(&[], SAMPLE_RATE, 1, SampleFormat::Int, 16))).is_err());
    assert!(Chunks::new(Cursor::new(wav(&[0.0; 10], 4_000, 1, SampleFormat::Int, 16))).is_err());
    assert!(Chunks::new(Cursor::new(wav(&[0.0; 10], SAMPLE_RATE, 1, SampleFormat::Int, 8))).is_err());
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut chunks = Chunks::new(Cursor::new(wav(
            &[bad; SAMPLE_RATE as usize / 1000],
            SAMPLE_RATE,
            1,
            SampleFormat::Float,
            32,
        )))
        .expect("header");
        assert!(chunks.next(&stop).is_err());
    }
    let mut truncated = wav(&[0.2; 1600], SAMPLE_RATE, 1, SampleFormat::Int, 16);
    truncated.truncate(truncated.len() - 10);
    let mut chunks = Chunks::new(Cursor::new(truncated)).expect("header");
    assert!(chunks.next(&stop).is_err());
    let mut partial = wav(&[0.0; 10], SAMPLE_RATE, 2, SampleFormat::Int, 16);
    partial[40..44].copy_from_slice(&18_u32.to_le_bytes());
    assert!(Chunks::new(Cursor::new(partial)).is_err());
    let mut overlong = wav(&[0.0; 10], 8_000, 1, SampleFormat::Int, 16);
    overlong[40..44].copy_from_slice(&(8_000_u32 * (4 * 60 * 60 + 1) * 2).to_le_bytes());
    assert!(Chunks::new(Cursor::new(overlong)).is_err());
}

#[test]
fn cancellation_does_not_read_the_next_block() {
    let stop = AtomicBool::new(true);
    let mut chunks = Chunks::new(Cursor::new(wav(
        &[0.3; 1000],
        SAMPLE_RATE,
        1,
        SampleFormat::Float,
        32,
    )))
    .expect("chunks");
    assert!(chunks.next(&stop).expect("cancelled").is_none());
    assert_eq!(chunks.read, 0);
    assert_eq!(chunks.offset, 0);
}
