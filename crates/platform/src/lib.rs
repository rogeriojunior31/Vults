//! Where the app's surfaces live on each OS. Takes the toolkit's window, never Tauri itself.

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub mod jump;

#[cfg(target_os = "linux")]
pub mod shortcuts;
