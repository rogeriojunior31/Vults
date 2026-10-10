//! Where the app's surfaces live on each OS. Takes the toolkit's window, never Tauri itself.

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub mod jump;

#[cfg(target_os = "linux")]
pub mod shortcuts;

#[cfg(target_os = "linux")]
pub mod tray;

#[cfg(target_os = "linux")]
pub mod save;

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

/// The language the desktop session reads, as a tag or POSIX locale (`pt_BR.UTF-8`); none when it
/// says nothing (`C`, `POSIX`, unset).
pub fn system_language() -> Option<String> {
    #[cfg(windows)]
    return windows_time::locale();
    #[cfg(not(windows))]
    {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|v| std::env::var(v).ok())
            .find(|v| !v.is_empty())
            .filter(|v| v != "C" && v != "POSIX" && !v.starts_with("C."))
    }
}
