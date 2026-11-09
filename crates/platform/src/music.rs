//! Temporary local media changes during independent dictation. No track metadata is read.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Lower,
    Pause,
    Leave,
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{Listening, wait_for_restore};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{Listening, shutdown, wait_for_restore_until};
