//! A process tree the app owns on Windows: a job that ends every process in it when its last
//! handle closes, on a stop, a drop or the app's own death. The process starts suspended and joins
//! the job before it runs, so nothing it starts (the node behind an npm shim) can escape it.

use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};

use windows::Win32::Foundation::{ERROR_NO_MORE_FILES, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_BASIC_LIMIT_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject,
};
use windows::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, GetProcessIdOfThread, OpenThread, ResumeThread,
    THREAD_QUERY_LIMITED_INFORMATION, THREAD_SUSPEND_RESUME,
};
use windows::core::PCWSTR;

/// The creation flags of a process that joins a [`TreeJob`]: no console window, not running yet.
pub const SUSPENDED_NO_WINDOW: u32 = CREATE_NO_WINDOW.0 | CREATE_SUSPENDED.0;

/// Anonymous and never inherited: closing it ends the whole tree.
#[derive(Debug)]
pub struct TreeJob(OwnedHandle);

impl TreeJob {
    pub fn new() -> io::Result<Self> {
        // SAFETY: an anonymous non-inheritable job, owned from here on.
        let raw = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(io::Error::other)?;
        // SAFETY: CreateJobObjectW returned this unique owned handle.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(raw.0) });
        let limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
                LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                ..Default::default()
            },
            ..Default::default()
        };
        // SAFETY: the structure and its size match the selected information class.
        unsafe {
            SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        }
        .map_err(io::Error::other)?;
        Ok(job)
    }

    /// Puts `process`, started with [`SUSPENDED_NO_WINDOW`], in the job.
    pub fn assign(&self, process: RawHandle) -> io::Result<()> {
        // SAFETY: both handles are open for the call; the job keeps no reference to `process`.
        unsafe { AssignProcessToJobObject(HANDLE(self.0.as_raw_handle()), HANDLE(process)) }
            .map_err(io::Error::other)
    }
}

/// Lets a process started suspended run: its one thread, checked to still be its own.
pub fn resume_suspended(pid: u32) -> io::Result<()> {
    // Rust's Child API does not expose the initial thread handle. A suspended process has one
    // thread: refuse anything else, and check its owner again on the opened handle.
    // SAFETY: the snapshot owns no process and is closed through OwnedHandle.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }.map_err(io::Error::other)?;
    // SAFETY: this unique snapshot handle was returned by CreateToolhelp32Snapshot.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw.0) };
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut thread_id = None;
    // SAFETY: `entry` has its required size and the snapshot stays open while it is walked.
    unsafe {
        Thread32First(HANDLE(snapshot.as_raw_handle()), &mut entry).map_err(io::Error::other)?;
        loop {
            if entry.th32OwnerProcessID == pid && thread_id.replace(entry.th32ThreadID).is_some() {
                return Err(io::Error::other(
                    "The suspended process has an unexpected thread layout.",
                ));
            }
            if let Err(error) = Thread32Next(HANDLE(snapshot.as_raw_handle()), &mut entry) {
                if error.code() == windows::core::HRESULT::from_win32(ERROR_NO_MORE_FILES.0) {
                    break;
                }
                return Err(io::Error::other(error));
            }
        }
    }
    let id = thread_id.ok_or_else(|| io::Error::other("Could not find the suspended process's thread."))?;
    // SAFETY: access limited to querying the owner and resuming this thread.
    let raw = unsafe {
        OpenThread(
            THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
            false,
            id,
        )
    }
    .map_err(io::Error::other)?;
    // SAFETY: OpenThread returned this unique owned handle.
    let thread = unsafe { OwnedHandle::from_raw_handle(raw.0) };
    // SAFETY: the owner is checked on the opened handle, so a reused thread id resumes nothing else.
    unsafe {
        if GetProcessIdOfThread(HANDLE(thread.as_raw_handle())) != pid {
            return Err(io::Error::other("The suspended thread's owner changed."));
        }
        if ResumeThread(HANDLE(thread.as_raw_handle())) != 1 {
            return Err(io::Error::other("Could not resume the suspended process."));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::time::{Duration, Instant};

    /// A shell that starts a grandchild, as an npm shim starts node: closing the job ends both.
    #[test]
    fn closing_the_job_ends_the_whole_tree() {
        let job = TreeJob::new().unwrap();
        let marker = std::env::temp_dir().join(format!("vults-job-tree-{}", std::process::id()));
        let _ = std::fs::remove_file(&marker);
        // The grandchild writes its pid, then waits long enough to outlive a plain kill.
        let script = format!(
            "start /b \"\" powershell.exe -NoProfile -Command \"$PID | Out-File -Encoding ascii '{}'; Start-Sleep 60\" & ping -n 60 127.0.0.1 >nul",
            marker.display()
        );
        let mut shell = std::process::Command::new("cmd.exe");
        shell.raw_arg(format!("/d /s /c \"{script}\""));
        let mut child = shell.creation_flags(SUSPENDED_NO_WINDOW).spawn().unwrap();
        job.assign(child.as_raw_handle()).unwrap();
        resume_suspended(child.id()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let grandchild: u32 = loop {
            if let Some(pid) = std::fs::read_to_string(&marker)
                .ok()
                .and_then(|s| s.trim().parse().ok())
            {
                break pid;
            }
            assert!(Instant::now() < deadline, "the grandchild never started");
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(matches!(crate::process_birth(grandchild), Some(Some(_))));
        drop(job);
        let deadline = Instant::now() + Duration::from_secs(10);
        while crate::process_birth(grandchild) != Some(None) {
            assert!(Instant::now() < deadline, "the grandchild outlived its job");
            std::thread::sleep(Duration::from_millis(50));
        }
        child.wait().unwrap();
        let _ = std::fs::remove_file(&marker);
    }
}
