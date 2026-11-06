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

/// A same-user process's creation time; missing/exited is distinct from inaccessible.
pub fn process_birth(pid: u32) -> Option<Option<u64>> {
    use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, FILETIME};
    use windows::Win32::System::Threading::GetProcessTimes;
    if pid <= 1 {
        return None;
    }
    let mine = current_user_sid()?;
    // SAFETY: query-only handle, valid output buffers, and every opened handle is closed.
    unsafe {
        let process = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(process) => process,
            Err(error) if error.code() == ERROR_INVALID_PARAMETER.to_hresult() => return Some(None),
            Err(_) => return None,
        };
        let result = (|| {
            if token_sid(process).as_deref() != Some(mine.as_str()) {
                return None;
            }
            let mut creation = FILETIME::default();
            let mut exit = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user).ok()?;
            if exit.dwLowDateTime != 0 || exit.dwHighDateTime != 0 {
                return Some(None);
            }
            Some(Some(
                (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime),
            ))
        })();
        let _ = CloseHandle(process);
        result
    }
}

/// Skip hook shells without attributing a session to a recycled or foreign parent PID.
pub fn agent_pid() -> u32 {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    // SAFETY: the snapshot is closed after enumeration and the structure has its required size.
    let processes = unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return 0;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut rows = Vec::new();
        let mut next = Process32FirstW(snapshot, &mut entry);
        while next.is_ok() && rows.len() < 65_536 {
            let end = entry
                .szExeFile
                .iter()
                .position(|ch| *ch == 0)
                .unwrap_or(entry.szExeFile.len());
            rows.push((
                entry.th32ProcessID,
                entry.th32ParentProcessID,
                String::from_utf16_lossy(&entry.szExeFile[..end]),
            ));
            next = Process32NextW(snapshot, &mut entry);
        }
        let _ = CloseHandle(snapshot);
        rows
    };
    let mut pid = std::process::id();
    let Some(Some(mut birth)) = process_birth(pid) else {
        return 0;
    };
    for _ in 0..8 {
        let Some((_, parent, _)) = processes.iter().find(|(id, _, _)| *id == pid) else {
            return 0;
        };
        let Some(Some(parent_birth)) = process_birth(*parent) else {
            return 0;
        };
        if parent_birth > birth || *parent == pid {
            return 0;
        }
        pid = *parent;
        birth = parent_birth;
        let Some((_, _, name)) = processes.iter().find(|(id, _, _)| *id == pid) else {
            return 0;
        };
        if !matches!(
            name.to_ascii_lowercase().as_str(),
            "cmd.exe"
                | "powershell.exe"
                | "pwsh.exe"
                | "sh.exe"
                | "bash.exe"
                | "dash.exe"
                | "zsh.exe"
                | "fish.exe"
                | "busybox.exe"
        ) {
            return pid;
        }
    }
    0
}

/// A same-user executable path for foreground application identity, never command execution.
pub fn process_image(pid: u32) -> Option<std::path::PathBuf> {
    use windows::Win32::System::Threading::{PROCESS_NAME_WIN32, QueryFullProcessImageNameW};
    if !process_is_same_user(pid) {
        return None;
    }
    // SAFETY: bounded output buffer, query-only handle, closed on every return path.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut path = vec![0u16; 32_768];
        let mut length = path.len() as u32;
        let result =
            QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(path.as_mut_ptr()), &mut length)
                .ok()
                .map(|_| std::path::PathBuf::from(String::from_utf16_lossy(&path[..length as usize])));
        let _ = CloseHandle(process);
        result
    }
}

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

#[cfg(test)]
mod process_tests {
    #[test]
    fn hook_shells_resolve_to_the_long_lived_owner() {
        if let Ok(expected) = std::env::var("VULTS_TEST_OWNER_PID") {
            assert_eq!(super::agent_pid(), expected.parse::<u32>().unwrap());
            return;
        }
        use std::os::windows::process::CommandExt;
        for (shell, args) in [
            (
                "cmd.exe",
                vec![
                    "/d",
                    "/c",
                    "\"%VULTS_TEST_CHILD_EXE%\" --exact windows::process_tests::hook_shells_resolve_to_the_long_lived_owner",
                ],
            ),
            (
                "powershell.exe",
                vec![
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "& $env:VULTS_TEST_CHILD_EXE --exact windows::process_tests::hook_shells_resolve_to_the_long_lived_owner; exit $LASTEXITCODE",
                ],
            ),
        ] {
            let mut command = std::process::Command::new(shell);
            if shell == "cmd.exe" {
                command.raw_arg(r#"/d /s /c ""%VULTS_TEST_CHILD_EXE%" --exact windows::process_tests::hook_shells_resolve_to_the_long_lived_owner""#);
            } else {
                command.args(args);
            }
            let output = command
                .env("VULTS_TEST_OWNER_PID", std::process::id().to_string())
                .env("VULTS_TEST_CHILD_EXE", std::env::current_exe().unwrap())
                .creation_flags(0x0800_0000)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{shell}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn current_process_has_a_stable_birth_and_unknown_pids_are_not_alive() {
        let birth = super::process_birth(std::process::id());
        assert!(matches!(birth, Some(Some(_))));
        assert_eq!(birth, super::process_birth(std::process::id()));
        assert_eq!(super::process_birth(0), None);
        assert_eq!(super::process_birth(u32::MAX), Some(None));
        assert!(super::agent_pid() > 1);
    }
}
