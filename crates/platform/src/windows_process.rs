//! A bounded diagnostic owns its entire process tree, including npm and Python launchers.
use std::{
    io,
    os::windows::{
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::CommandExt,
    },
    process::{Child, Command},
};
use windows::{
    Win32::{
        Foundation::{ERROR_NO_MORE_FILES, HANDLE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
            Threading::{
                CREATE_NO_WINDOW, CREATE_SUSPENDED, GetProcessIdOfThread, OpenThread, ResumeThread,
                THREAD_QUERY_LIMITED_INFORMATION, THREAD_SUSPEND_RESUME,
            },
        },
    },
    core::PCWSTR,
};

/// Only for short, non-interactive probes. Dropping it also stops descendants after a successful exit.
#[derive(Debug)]
pub struct DiagnosticProcess {
    child: Child,
    job: Option<OwnedHandle>,
}
impl DiagnosticProcess {
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        // SAFETY: an anonymous non-inheritable job is owned by this function until returned.
        let raw = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(io::Error::other)?;
        // SAFETY: CreateJobObjectW returned this unique owned handle.
        let job = unsafe { OwnedHandle::from_raw_handle(raw.0) };
        let limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: windows::Win32::System::JobObjects::JOBOBJECT_BASIC_LIMIT_INFORMATION {
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
        let child = command
            .creation_flags((CREATE_NO_WINDOW | CREATE_SUSPENDED).0)
            .spawn()?;
        let owned = Self {
            child,
            job: Some(job),
        };
        // Assignment happens before any launcher code can run and spawn a child outside the job.
        // SAFETY: both handles remain owned and live through assignment.
        unsafe { AssignProcessToJobObject(raw, HANDLE(owned.child.as_raw_handle())) }
            .map_err(io::Error::other)?;
        resume_owned_thread(owned.child.id())?;
        Ok(owned)
    }
    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}
impl Drop for DiagnosticProcess {
    fn drop(&mut self) {
        // No child inherits the job handle; closing our last handle terminates its entire tree.
        drop(self.job.take());
        // This also covers an assignment failure, while the initial process is still suspended.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn resume_owned_thread(pid: u32) -> io::Result<()> {
    // Rust's stable Child API does not expose its initial thread handle. The suspended process
    // has one thread; refuse ambiguity and verify ownership again after opening the handle.
    // SAFETY: the snapshot owns no process and is closed below through OwnedHandle.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }.map_err(io::Error::other)?;
    // SAFETY: this unique snapshot handle was returned by CreateToolhelp32Snapshot.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw.0) };
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut thread_id = None;
    // SAFETY: entry has the required initialized size and snapshot stays open for enumeration.
    unsafe {
        Thread32First(HANDLE(snapshot.as_raw_handle()), &mut entry).map_err(io::Error::other)?;
        loop {
            if entry.th32OwnerProcessID == pid && thread_id.replace(entry.th32ThreadID).is_some() {
                return Err(io::Error::other(
                    "The diagnostic process has an unexpected thread layout.",
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
    let id = thread_id.ok_or_else(|| io::Error::other("Could not find the diagnostic process thread."))?;
    // SAFETY: access is limited to querying ownership and resuming this thread.
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
    // SAFETY: the process id is checked on the opened handle, so thread-id reuse cannot resume another process.
    unsafe {
        if GetProcessIdOfThread(HANDLE(thread.as_raw_handle())) != pid {
            return Err(io::Error::other("The diagnostic thread owner changed."));
        }
        if ResumeThread(HANDLE(thread.as_raw_handle())) != 1 {
            return Err(io::Error::other(
                "Could not resume the suspended diagnostic process.",
            ));
        }
    }
    Ok(())
}
