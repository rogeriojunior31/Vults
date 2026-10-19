use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const FILES: [&str; 2] = ["mic.wav", "system.wav"];

#[derive(Debug)]
pub(super) struct TemporaryAudio {
    root: PathBuf,
    // The lock must outlive removal, including on error and unwind.
    _lock: File,
}

impl TemporaryAudio {
    pub fn open(root: &Path) -> io::Result<Self> {
        match DirBuilder::new().mode(0o700).create(root) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        let meta = fs::symlink_metadata(root)?;
        // SAFETY: geteuid has no preconditions.
        let uid = unsafe { libc::geteuid() };
        if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o777 != 0o700 {
            return Err(io::Error::other(
                "meeting temporary directory must be owned by this user and mode 0700",
            ));
        }
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(root.join(".lock"))?;
        let meta = lock.metadata()?;
        if !meta.is_file() || meta.uid() != uid || meta.nlink() != 1 || meta.mode() & 0o777 != 0o600 {
            return Err(io::Error::other("unsafe meeting temporary lock"));
        }
        // SAFETY: lock is a live descriptor; flock does not retain a Rust pointer.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::other(
                "another meeting capture owns this temporary directory",
            ));
        }
        let temporary = Self {
            root: root.into(),
            _lock: lock,
        };
        temporary.cleanup()?;
        Ok(temporary)
    }

    pub fn file(&self, name: &str) -> Result<File, String> {
        if !FILES.contains(&name) {
            return Err("unknown meeting temporary file".into());
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(self.root.join(name))
            .map_err(|e| e.to_string())
    }

    pub fn reader(&self, name: &str) -> Result<File, String> {
        if !FILES.contains(&name) {
            return Err("unknown meeting temporary file".into());
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(self.root.join(name))
            .map_err(|e| e.to_string())?;
        let meta = file.metadata().map_err(|e| e.to_string())?;
        // SAFETY: geteuid has no preconditions.
        let uid = unsafe { libc::geteuid() };
        if !meta.is_file() || meta.uid() != uid || meta.nlink() != 1 || meta.mode() & 0o777 != 0o600 {
            return Err("unsafe meeting temporary audio".into());
        }
        Ok(file)
    }

    fn cleanup(&self) -> io::Result<()> {
        let mut failed = None;
        for name in FILES {
            match fs::remove_file(self.root.join(name)) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => {
                    failed.get_or_insert(e);
                }
            }
        }
        failed.map_or(Ok(()), Err)
    }

    pub fn finish(self) -> Result<(), String> {
        self.cleanup().map_err(|e| e.to_string())
    }
}

impl Drop for TemporaryAudio {
    fn drop(&mut self) {
        let _ = self.cleanup();
        // A forked child can retain the descriptor; release ownership after cleanup explicitly.
        // SAFETY: the lock descriptor remains live until this destructor returns.
        unsafe { libc::flock(self._lock.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_owner_unlocks_even_while_an_inherited_descriptor_survives() {
        let dir = crate::meeting::tests::Directory::new();
        let owner = TemporaryAudio::open(&dir.0).unwrap();
        let inherited = owner._lock.try_clone().unwrap();
        drop(owner);
        let next = TemporaryAudio::open(&dir.0).expect("the old descriptor no longer owns the lock");
        assert!(TemporaryAudio::open(&dir.0).is_err());
        drop(inherited);
        assert!(TemporaryAudio::open(&dir.0).is_err());
        drop(next);
    }
}
