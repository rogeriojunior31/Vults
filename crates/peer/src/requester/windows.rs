//! A process as Windows reports it through one query-only handle: a PID cannot be reused while the
//! handle is open, so its birth, parent and image are read from one and the same process.
use super::{Process, Requester, Snapshot};
use std::os::windows::io::AsRawHandle;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::Pipes::GetNamedPipeClientProcessId;
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::core::PWSTR;

/// winternl.h's PROCESS_BASIC_INFORMATION.
#[repr(C)]
#[derive(Default)]
struct BasicInformation {
    exit_status: i32,
    peb: usize,
    affinity: usize,
    priority: i32,
    pid: usize,
    parent: usize,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryInformationProcess(
        process: HANDLE,
        class: u32,
        information: *mut BasicInformation,
        length: u32,
        returned: *mut u32,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
}

impl Snapshot {
    pub(super) fn read(pid: u32) -> Option<Self> {
        // 0 is the idle process, 4 the kernel: never a requester.
        if pid <= 4 {
            return None;
        }
        let mine = crate::current_user_sid()?;
        // SAFETY: query-only handle, every out-buffer sized for its call, closed on every path.
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let snapshot = (|| {
                if crate::windows::token_sid(process).as_deref() != Some(mine.as_str()) {
                    return None;
                }
                let (mut creation, mut exit) = (FILETIME::default(), FILETIME::default());
                let (mut kernel, mut user) = (FILETIME::default(), FILETIME::default());
                GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user).ok()?;
                if exit.dwLowDateTime != 0 || exit.dwHighDateTime != 0 {
                    return None;
                }
                let mut basic = BasicInformation::default();
                let size = std::mem::size_of::<BasicInformation>() as u32;
                if NtQueryInformationProcess(process, 0, &mut basic, size, std::ptr::null_mut()) < 0 {
                    return None;
                }
                let mut path = vec![0u16; 32_768];
                let mut length = path.len() as u32;
                QueryFullProcessImageNameW(
                    process,
                    PROCESS_NAME_WIN32,
                    PWSTR(path.as_mut_ptr()),
                    &mut length,
                )
                .ok()?;
                // Windows paths ignore case.
                let image = String::from_utf16_lossy(&path[..length as usize]).to_lowercase();
                let name = image.rsplit('\\').next().unwrap_or_default().to_owned();
                let started = (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime);
                Some(Self {
                    process: Process { pid, started },
                    parent: u32::try_from(basic.parent).ok()?,
                    name,
                    executable: image,
                })
            })();
            let _ = CloseHandle(process);
            snapshot
        }
    }

    /// A shell only from the system's own folders, which only an administrator can write: a
    /// program named `cmd.exe` anywhere else hides nobody.
    pub(super) fn is_shell(&self) -> bool {
        let Some(system) = system_directory() else {
            return false;
        };
        let programs = std::env::var("ProgramW6432")
            .or_else(|_| std::env::var("ProgramFiles"))
            .unwrap_or_default()
            .to_lowercase();
        match self.name.as_str() {
            "cmd.exe" => self.executable == format!(r"{system}\cmd.exe"),
            "powershell.exe" => self.executable == format!(r"{system}\windowspowershell\v1.0\powershell.exe"),
            "pwsh.exe" => {
                !programs.is_empty()
                    && self
                        .executable
                        .strip_prefix(&format!(r"{programs}\powershell\"))
                        .is_some_and(|rest| rest.split('\\').count() == 2)
            }
            _ => false,
        }
    }
}

impl Snapshot {
    /// mise's shims stay alive as parents of the tool they start (`shims\claude.exe` runs
    /// `mise.exe`, which runs the real `claude.exe`). Not a proof of anything by itself: the chain
    /// still has to end at whoever started it, as `launched_by` checks.
    pub(super) fn is_launcher(&self) -> bool {
        self.name == "mise.exe" || self.executable.contains(r"\mise\shims\")
    }
}

fn system_directory() -> Option<String> {
    let mut buffer = [0u16; 260];
    // SAFETY: the buffer's length is passed; the result is the number of characters written.
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    (length > 0 && length < buffer.len()).then(|| String::from_utf16_lossy(&buffer[..length]).to_lowercase())
}

impl Requester {
    /// The pipe's connected relay must run this same executable as this same user.
    pub fn pipe(pipe: &impl AsRawHandle) -> Option<Self> {
        if !crate::pipe_client_is_same_user(pipe) {
            return None;
        }
        let mut relay = 0u32;
        // SAFETY: `pipe` is an open pipe handle and `relay` is valid for writes.
        unsafe { GetNamedPipeClientProcessId(HANDLE(pipe.as_raw_handle()), &mut relay) }.ok()?;
        let proof = Self::capture(relay)?;
        let own = Snapshot::read(std::process::id())?;
        (proof.chain.first()?.executable == own.executable).then_some(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_process_and_its_parent_are_read_from_windows_itself() {
        let own = Snapshot::read(std::process::id()).unwrap();
        let exe = std::env::current_exe().unwrap();
        assert_eq!(own.executable, exe.to_string_lossy().to_lowercase());
        assert!(!own.is_shell());
        let proof = Requester::stdio().unwrap();
        assert_eq!(proof.relay().pid, std::process::id());
        assert!(proof.is_current());
        let mut reused = proof.clone();
        reused.chain[0].process.started += 1;
        assert!(!reused.is_current());
        assert!(Snapshot::read(0).is_none() && Snapshot::read(4).is_none());
    }

    #[test]
    fn only_the_systems_own_shells_are_shells() {
        let system = system_directory().unwrap();
        let shell = |name: &str, executable: String| Snapshot {
            process: Process { pid: 9, started: 1 },
            parent: 1,
            name: name.into(),
            executable,
        };
        assert!(shell("cmd.exe", format!(r"{system}\cmd.exe")).is_shell());
        assert!(
            shell(
                "powershell.exe",
                format!(r"{system}\windowspowershell\v1.0\powershell.exe")
            )
            .is_shell()
        );
        assert!(!shell("cmd.exe", r"c:\users\me\cmd.exe".into()).is_shell());
        assert!(!shell("node.exe", format!(r"{system}\node.exe")).is_shell());
    }

    #[test]
    fn a_cmd_shim_above_the_client_still_traces_to_whoever_started_it() {
        let system = system_directory().unwrap();
        let process = |pid: u32, parent: u32, started: u64, image: String| Snapshot {
            process: Process { pid, started },
            parent,
            name: image.rsplit('\\').next().unwrap().into(),
            executable: image,
        };
        let cmd = format!(r"{system}\cmd.exe");
        let read = |born: u64| {
            let cmd = cmd.clone();
            move |pid| match pid {
                30 => Some(process(30, 20, 300, r"c:\vults\vults.exe".into())),
                20 => Some(process(20, 10, 200, r"c:\node\node.exe".into())),
                10 => Some(process(10, 5, born, cmd.clone())),
                _ => None,
            }
        };
        let proof = Requester::with(30, read(100)).unwrap();
        assert_eq!(proof.client().pid, 20);
        assert_eq!(proof.above.last().map(|s| s.parent), Some(5));
        // A shell born after the client reused a dead parent's id: not its parent.
        let late = Requester::with(30, read(250)).unwrap();
        assert!(late.above.is_empty());
    }

    #[test]
    fn a_mise_shim_and_mise_itself_lead_back_to_the_app() {
        let process = |pid: u32, parent: u32, image: &str| Snapshot {
            process: Process {
                pid,
                started: u64::from(pid),
            },
            parent,
            name: image.rsplit('\\').next().unwrap().into(),
            executable: image.into(),
        };
        let proof = Requester::with(50, |pid| match pid {
            50 => Some(process(50, 40, r"c:\vults\vults.exe")),
            40 => Some(process(40, 30, r"c:\mise\installs\claude\latest\claude.exe")),
            30 => Some(process(30, 20, r"c:\scoop\apps\mise\current\bin\mise.exe")),
            20 => Some(process(20, 7, r"c:\users\me\appdata\local\mise\shims\claude.exe")),
            _ => None,
        })
        .unwrap();
        assert_eq!(proof.client().pid, 40);
        assert_eq!(proof.above.len(), 2);
        assert_eq!(proof.above.last().map(|s| s.parent), Some(7));
    }
}
