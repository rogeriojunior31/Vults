use super::*;
use std::io::Cursor;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn verified_bytes_survive_changes_to_the_original_buffer() {
    let mut original = b"checked model".to_vec();
    let checked = read_checked(
        Cursor::new(&original),
        original.len(),
        &hash(&original),
        &AtomicBool::new(false),
    )
    .expect("read")
    .expect("not cancelled");
    original.fill(0);
    assert_eq!(checked, b"checked model");
}

#[test]
fn rejects_truncation_extra_bytes_and_equal_size_corruption() {
    let expected = b"model";
    for bytes in [b"mode".as_slice(), b"models".as_slice(), b"other".as_slice()] {
        assert!(
            read_checked(
                Cursor::new(bytes),
                expected.len(),
                &hash(expected),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}

#[test]
fn cancellation_before_admission_never_opens_a_path() {
    assert!(
        read_models(
            Path::new("/missing/segmentation"),
            Path::new("/missing/embedding"),
            &AtomicBool::new(true)
        )
        .expect("cancelled before IO")
        .is_none()
    );
}

struct CancelDuringRead<'a> {
    input: Cursor<Vec<u8>>,
    stop: &'a AtomicBool,
}
impl Read for CancelDuringRead<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let count = self.input.read(out)?;
        self.stop.store(true, Ordering::SeqCst);
        Ok(count)
    }
}

#[test]
fn cancellation_during_a_chunk_or_at_eof_never_returns_partial_bytes() {
    for len in [10, 128 * 1024] {
        let bytes = vec![42; len];
        let stop = AtomicBool::new(false);
        let reader = CancelDuringRead {
            input: Cursor::new(bytes.clone()),
            stop: &stop,
        };
        assert!(
            read_checked(reader, len, &hash(&bytes), &stop)
                .expect("cancel")
                .is_none()
        );
    }
}

struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "vults-speaker-models-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&path).expect("private test directory");
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn regular_file_loading_keeps_the_checked_bytes_after_path_replacement() {
    let dir = Directory::new();
    let path = dir.0.join("model");
    std::fs::write(&path, b"model").expect("write");
    let bytes = read_file(&path, 5, &hash(b"model"), &AtomicBool::new(false))
        .expect("verify")
        .expect("bytes");
    std::fs::write(&path, b"other").expect("replace");
    assert_eq!(bytes, b"model");
    assert!(read_file(&path, 5, &hash(b"model"), &AtomicBool::new(false)).is_err());
    assert!(read_file(&path, 6, &hash(b"model"), &AtomicBool::new(false)).is_err());
    assert!(read_file(&dir.0, 5, &hash(b"model"), &AtomicBool::new(false)).is_err());
}

#[cfg(unix)]
#[test]
fn symlinks_sockets_and_fifos_are_rejected_without_opening_a_stream() {
    use std::os::unix::{fs::symlink, net::UnixListener};
    let dir = Directory::new();
    let file = dir.0.join("file");
    std::fs::write(&file, b"model").expect("file");
    let link = dir.0.join("link");
    symlink(&file, &link).expect("link");
    let socket = dir.0.join("socket");
    let _listener = UnixListener::bind(&socket).expect("socket");
    let fifo = dir.0.join("fifo");
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).expect("path");
    // SAFETY: the NUL-terminated path lives across this call in the test's private directory.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    for path in [link, socket, fifo] {
        assert!(read_file(&path, 5, &hash(b"model"), &AtomicBool::new(false)).is_err());
    }
}
