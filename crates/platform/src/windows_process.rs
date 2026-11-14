//! A bounded diagnostic owns its entire process tree, including npm and Python launchers.
use std::{
    io,
    os::windows::{io::AsRawHandle, process::CommandExt},
    process::{Child, Command},
};
use vults_peer::job::{SUSPENDED_NO_WINDOW, TreeJob, resume_suspended};

/// Only for short, non-interactive probes. Dropping it also stops descendants after a successful exit.
#[derive(Debug)]
pub struct DiagnosticProcess {
    child: Child,
    job: Option<TreeJob>,
}
impl DiagnosticProcess {
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        let job = TreeJob::new()?;
        let child = command.creation_flags(SUSPENDED_NO_WINDOW).spawn()?;
        let owned = Self {
            child,
            job: Some(job),
        };
        // Assignment happens before any launcher code can run and spawn a child outside the job.
        if let Some(job) = &owned.job {
            job.assign(owned.child.as_raw_handle())?;
        }
        resume_suspended(owned.child.id())?;
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
