//! Where the app's surfaces live on each OS. Takes the toolkit's window, never Tauri itself.

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub mod jump;

#[cfg(target_os = "linux")]
pub mod shortcuts;

#[cfg(target_os = "linux")]
pub mod tray;

#[cfg(windows)]
mod windows_time;

/// The local UTC offset at `unix` (seconds east of UTC), from the desktop's time zone; none when
/// it can't be read. Daylight saving included: it is the offset of that instant, not of today.
pub fn utc_offset(unix: i64) -> Option<i32> {
    #[cfg(target_os = "linux")]
    return linux::utc_offset(unix);
    #[cfg(windows)]
    return windows_time::utc_offset(unix);
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = unix;
        None
    }
}
