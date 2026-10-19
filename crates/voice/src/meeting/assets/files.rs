use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    Bundle, Cancel,
    catalog::{self, Download, FileSpec, Format},
    directory_name,
};
use sha2::{Digest, Sha256};

// The secure installer cannot construct either state on unsupported platforms.
#[cfg_attr(not(unix), allow(dead_code))]
pub(super) enum Prepared {
    Ready(Bundle),
    Stage(Stage),
}
#[derive(Debug)]
pub(super) struct Stage {
    pub path: PathBuf,
    root: PathBuf,
    _lock: File,
    committed: bool,
}

pub(super) fn installed(
    root: &Path,
    check: &impl Fn() -> Result<(), String>,
) -> Result<Option<Bundle>, String> {
    check()?;
    let path = root.join(directory_name());
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
        Ok(m) if !m.is_dir() => return Err("meeting asset bundle must be a regular directory".into()),
        _ => {}
    }
    validate(&path, check)?;
    Ok(Some(Bundle { path }))
}

impl Stage {
    pub fn begin(root: PathBuf, cancel: &Cancel) -> Result<Prepared, String> {
        cancel.check()?;
        if !root.is_absolute() {
            return Err("meeting asset installation requires an absolute directory".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .map_err(|e| e.to_string())?;
            let md = fs::symlink_metadata(&root).map_err(|e| e.to_string())?;
            // SAFETY: geteuid has no preconditions.
            let uid = unsafe { libc::geteuid() };
            if !md.is_dir() || md.uid() != uid || md.permissions().mode() & 0o077 != 0 {
                return Err("meeting asset directory must be private and owned by this user".into());
            }
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(root.join(".install.lock"))
                .map_err(|e| e.to_string())?;
            let md = lock.metadata().map_err(|e| e.to_string())?;
            if !md.is_file() || md.uid() != uid || md.nlink() != 1 || md.permissions().mode() & 0o777 != 0o600
            {
                return Err("invalid meeting asset lock".into());
            }
            use std::os::fd::AsRawFd;
            // SAFETY: flock borrows this live descriptor; Stage releases it after cleanup.
            if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err("another meeting asset installation is running".into());
            }
            // A previous crashed attempt cannot be active while this lock is held.
            remove_abandoned(fs::read_dir(&root).map_err(|e| e.to_string())?, cancel)?;
            if let Some(bundle) = installed(&root, &|| cancel.check())? {
                return Ok(Prepared::Ready(bundle));
            }
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = root.join(format!(
                ".install-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .map_err(|e| e.to_string())?;
            Ok(Prepared::Stage(Self {
                path,
                root,
                _lock: lock,
                committed: false,
            }))
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            Err("meeting assets are supported on Linux x86-64 only".into())
        }
    }

    pub fn unpack(&self, artifact: Download, cancel: &Cancel) -> Result<(), String> {
        cancel.check()?;
        let input = regular(&self.path.join(format!("{}.download", artifact.id)))?;
        if matches!(artifact.format, Format::Raw) {
            write_checked(input, &self.path, artifact.members[0].1, cancel)?;
        } else {
            let decoded: Box<dyn Read> = match artifact.format {
                Format::Gzip => Box::new(flate2::read::GzDecoder::new(input)),
                Format::Bzip2 => Box::new(bzip2::read::BzDecoder::new(input)),
                Format::Raw => unreachable!(),
            };
            unpack_tar(decoded, &self.path, artifact.members, cancel)?;
        }
        fs::remove_file(self.path.join(format!("{}.download", artifact.id))).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn commit(mut self, cancel: &Cancel) -> Result<Bundle, String> {
        cancel.check()?;
        for &(name, bytes) in catalog::NOTICES {
            cancel.check()?;
            let mut file = create(&self.path.join(name))?;
            file.write_all(bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        validate(&self.path, &|| cancel.check())?;
        // File syncs alone do not guarantee persistence of the staging directory's member names.
        File::open(&self.path)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| format!("meeting asset staging directory sync failed: {e}"))?;
        cancel.check()?;
        let target = self.root.join(directory_name());
        // Different catalog versions use different directories; never replace a loaded runtime.
        fs::rename(&self.path, &target).map_err(|e| e.to_string())?;
        self.committed = true;
        File::open(&self.root)
            .and_then(|f| f.sync_all())
            .map_err(|e| format!("asset bundle published but directory sync failed: {e}"))?;
        Ok(Bundle { path: target })
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // A concurrent fork can retain the open description after this owner's File closes.
            // SAFETY: the lock descriptor stays live until after cleanup and this unlock.
            let _ = unsafe { libc::flock(self._lock.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

#[cfg(unix)]
fn remove_abandoned(
    entries: impl Iterator<Item = std::io::Result<fs::DirEntry>>,
    cancel: &Cancel,
) -> Result<(), String> {
    cancel.check()?;
    // The caller retains the installation lock so only abandoned attempts can be removed.
    for entry in entries {
        cancel.check()?;
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name().to_string_lossy().starts_with(".install-") {
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            cancel.check()?;
            if kind.is_dir() {
                fs::remove_dir_all(entry.path()).map_err(|e| e.to_string())?
            } else if kind.is_symlink() {
                fs::remove_file(entry.path()).map_err(|e| e.to_string())?
            }
        }
    }
    cancel.check()
}

pub(super) fn regular(path: &Path) -> Result<File, String> {
    if !fs::symlink_metadata(path).map_err(|e| e.to_string())?.is_file() {
        return Err("meeting assets must be regular files".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("meeting assets must be regular files".into());
    }
    Ok(file)
}
fn create(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path).map_err(|e| e.to_string())
}
fn validate(path: &Path, check: &impl Fn() -> Result<(), String>) -> Result<(), String> {
    let names: Vec<_> = catalog::FILES
        .iter()
        .map(|f| f.name)
        .chain(catalog::NOTICES.iter().map(|n| n.0))
        .collect();
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !names.iter().any(|name| entry.file_name() == *name) {
            return Err("unexpected file in meeting asset bundle".into());
        }
    }
    for spec in catalog::FILES {
        verify(&path.join(spec.name), spec.size, spec.sha256, check)?;
    }
    for &(name, bytes) in catalog::NOTICES {
        verify(
            &path.join(name),
            bytes.len() as u64,
            &format!("{:x}", Sha256::digest(bytes)),
            check,
        )?;
    }
    Ok(())
}
fn verify(
    path: &Path,
    size: u64,
    expected: &str,
    check: &impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    check()?;
    let input = regular(path)?;
    if input.metadata().map_err(|e| e.to_string())?.len() != size {
        return Err("meeting asset size mismatch".into());
    }
    verify_reader(input, size, expected, check)
}

fn verify_reader(
    mut input: impl Read,
    size: u64,
    expected: &str,
    check: &impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    let mut hash = Sha256::new();
    let mut read = 0;
    let mut bytes = [0; 64 * 1024];
    loop {
        check()?;
        let len = input.read(&mut bytes).map_err(|e| e.to_string())?;
        check()?;
        if len == 0 {
            break;
        }
        read += len as u64;
        if read > size {
            return Err("meeting asset grew while checking".into());
        }
        hash.update(&bytes[..len]);
    }
    if read != size || format!("{:x}", hash.finalize()) != expected {
        return Err("meeting asset fingerprint mismatch".into());
    }
    check()
}

fn write_checked(mut input: impl Read, path: &Path, spec: FileSpec, cancel: &Cancel) -> Result<(), String> {
    cancel.check()?;
    let mut file = create(&path.join(spec.name))?;
    let mut hash = Sha256::new();
    let mut done = 0;
    let mut chunk = [0; 64 * 1024];
    loop {
        cancel.check()?;
        let len = input.read(&mut chunk).map_err(|e| e.to_string())?;
        cancel.check()?;
        if len == 0 {
            break;
        }
        done += len as u64;
        if done > spec.size {
            return Err("expanded meeting asset exceeds its pinned size".into());
        }
        hash.update(&chunk[..len]);
        file.write_all(&chunk[..len]).map_err(|e| e.to_string())?;
    }
    if done != spec.size || format!("{:x}", hash.finalize()) != spec.sha256 {
        return Err("expanded meeting asset fingerprint mismatch".into());
    }
    file.sync_all().map_err(|e| e.to_string())?;
    cancel.check()
}

fn unpack_tar(
    input: impl Read,
    output: &Path,
    members: &[(&str, FileSpec)],
    cancel: &Cancel,
) -> Result<(), String> {
    struct Limited<'a, R> {
        input: R,
        left: usize,
        cancel: &'a Cancel,
    }
    impl<R: Read> Read for Limited<'_, R> {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            if self.cancel.is_cancelled() || self.left == 0 {
                return Err(std::io::Error::other(
                    "archive cancelled or exceeds expansion budget",
                ));
            }
            let len = bytes.len().min(self.left).min(64 * 1024);
            let count = self.input.read(&mut bytes[..len])?;
            self.left -= count;
            Ok(count)
        }
    }
    let mut archive = tar::Archive::new(Limited {
        input,
        left: 128 * 1024 * 1024,
        cancel,
    });
    let mut found = std::collections::BTreeSet::new();
    for (count, entry) in archive.entries().map_err(|e| e.to_string())?.enumerate() {
        cancel.check()?;
        if count >= 1024 {
            return Err("too many meeting archive entries".into());
        }
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?;
        if path.components().any(|c| !matches!(c, Component::Normal(_)))
            || path.as_os_str().as_encoded_bytes().contains(&b'\\')
            || entry.size() > 64 * 1024 * 1024
        {
            return Err("unsafe meeting archive path or entry size".into());
        }
        if let Some((name, spec)) = members.iter().find(|(name, _)| path == Path::new(name)) {
            if !entry.header().entry_type().is_file() || !found.insert(*name) {
                return Err("duplicate or non-regular meeting archive member".into());
            }
            if entry.size() != spec.size {
                return Err("meeting archive member size mismatch".into());
            }
            write_checked(&mut entry, output, *spec, cancel)?;
        }
    }
    // The final zero header can end iteration without another cancellation-aware read.
    cancel.check()?;
    if found.len() != members.len() {
        return Err("meeting archive is missing a required file".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
