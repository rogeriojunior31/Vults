//! Named pipes live in a machine-wide namespace, so any account could create ours first.
//! The pipe name carries our SID, and both ends check the other process runs as us.

use std::os::windows::io::AsRawHandle;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::core::PWSTR;

/// The SID this process runs as, as `S-1-5-21-…`.
pub fn current_user_sid() -> Option<String> {
    // SAFETY: the pseudo-handle from GetCurrentProcess is always valid and needs no close.
    unsafe { token_sid(GetCurrentProcess()) }
}

/// Client side: the process serving `pipe` runs as us.
pub fn pipe_server_is_same_user(pipe: &impl AsRawHandle) -> bool {
    let mut pid = 0u32;
    // SAFETY: `pipe` is an open pipe handle and `pid` is valid for writes.
    let ok = unsafe { GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid) }.is_ok();
    ok && process_is_same_user(pid)
}

/// Server side: the process connected to `pipe` runs as us.
pub fn pipe_client_is_same_user(pipe: &impl AsRawHandle) -> bool {
    let mut pid = 0u32;
    // SAFETY: `pipe` is an open pipe handle and `pid` is valid for writes.
    let ok = unsafe { GetNamedPipeClientProcessId(HANDLE(pipe.as_raw_handle()), &mut pid) }.is_ok();
    ok && process_is_same_user(pid)
}

fn process_is_same_user(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let Some(mine) = current_user_sid() else {
        return false;
    };
    // SAFETY: the handle is closed below and never escapes.
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let theirs = token_sid(process);
        let _ = CloseHandle(process);
        theirs.as_deref() == Some(mine.as_str())
    }
}

/// The user SID behind a process handle. `process` is borrowed, never closed.
unsafe fn token_sid(process: HANDLE) -> Option<String> {
    let mut token = HANDLE::default();
    // SAFETY (whole body): every out-pointer is valid, the buffer is sized by the first
    // GetTokenInformation call, and every handle and allocation is released before returning.
    unsafe {
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        if needed == 0 {
            let _ = CloseHandle(token);
            return None;
        }
        let mut buf = vec![0u8; needed as usize];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )
        .is_ok();
        let _ = CloseHandle(token);
        if !ok {
            return None;
        }
        let user = &*(buf.as_ptr() as *const TOKEN_USER);
        let mut text = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut text).ok()?;
        let sid = text.to_string().ok();
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        sid
    }
}
