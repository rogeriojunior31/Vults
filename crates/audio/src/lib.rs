//! Shared microphone capture, held in memory; no transcription, disk or network access.

mod capture;
mod meter;
mod resample;

pub use capture::{Audio, Capture, CaptureOptions, InputInfo, input_info, microphones};
pub use meter::Meter;
pub use resample::resample;

pub const SAMPLE_RATE: u32 = 16_000;
