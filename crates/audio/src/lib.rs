//! Shared microphone capture, held in memory; no transcription or network access.
//! The optional Linux meeting recording check alone uses private, temporary WAVs.

mod capture;
mod meter;
mod resample;

#[cfg(all(target_os = "linux", feature = "meeting-capture"))]
pub mod meeting;

pub use capture::{Audio, Capture, CaptureOptions, InputInfo, input_info, microphones};
pub use meter::Meter;
pub use resample::{Converter as MonoResampler, resample};

pub const SAMPLE_RATE: u32 = 16_000;
