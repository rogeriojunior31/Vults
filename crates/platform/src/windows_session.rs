//! Read the current logon session without opening or changing the desktop.
use windows::Win32::System::RemoteDesktop::{
    WTS_CURRENT_SESSION, WTS_SESSIONSTATE_LOCK, WTS_SESSIONSTATE_UNLOCK, WTSFreeMemory, WTSINFOEXW,
    WTSQuerySessionInformationW, WTSSessionInfoEx,
};
use windows::core::PWSTR;

/// Unknown or inaccessible state is never reported as an unlock.
pub fn locked() -> Option<bool> {
    let mut buffer = PWSTR::null();
    let mut length = 0;
    // SAFETY: WTS owns the output until WTSFreeMemory; size and level are checked before access.
    unsafe {
        WTSQuerySessionInformationW(
            None,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut buffer,
            &mut length,
        )
        .ok()?;
        let state = if !buffer.is_null() && length as usize >= std::mem::size_of::<WTSINFOEXW>() {
            let info = &*buffer.0.cast::<WTSINFOEXW>();
            if info.Level == 1 {
                Some(info.Data.WTSInfoExLevel1.SessionFlags as u32)
            } else {
                None
            }
        } else {
            None
        };
        WTSFreeMemory(buffer.0.cast());
        match state {
            Some(WTS_SESSIONSTATE_LOCK) => Some(true),
            Some(WTS_SESSIONSTATE_UNLOCK) => Some(false),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires an interactive Windows logon session"]
    fn current_interactive_session_can_be_queried_without_changing_it() {
        assert!(super::locked().is_some());
    }
}
