//! Where the island lives on each OS. Takes the toolkit's window, never Tauri itself.

#[cfg(target_os = "linux")]
pub mod linux;
