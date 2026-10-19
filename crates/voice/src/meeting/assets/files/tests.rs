use super::*;
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "vults-asset-files-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .expect("private temp");
        }
        #[cfg(not(unix))]
        fs::create_dir(&path).expect("temp");
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const ABC: FileSpec = FileSpec {
    name: "checked",
    size: 3,
    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
};

struct CancellingRead<'a> {
    input: Cursor<Vec<u8>>,
    cancel: &'a Cancel,
    after: u64,
    eof: bool,
}
impl Read for CancellingRead<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let count = self.input.read(bytes)?;
        if self.input.position() >= self.after && (count == 0) == self.eof {
            self.cancel.cancel();
        }
        Ok(count)
    }
}

#[test]
fn cancelled_cache_checks_do_not_report_an_absent_bundle() {
    let temp = Temp::new();
    let cancel = Cancel::default();
    cancel.cancel();
    assert_eq!(
        installed(&temp.0.join("absent"), &|| cancel.check()).err(),
        Some("meeting asset installation cancelled".into())
    );
    assert_eq!(
        verify(&temp.0.join("absent"), ABC.size, ABC.sha256, &|| cancel.check()),
        Err("meeting asset installation cancelled".into())
    );
}

#[test]
fn cached_file_verification_respects_cancellation_at_eof() {
    for eof in [false, true] {
        let cancel = Cancel::default();
        let result = verify_reader(
            CancellingRead {
                input: Cursor::new(b"abc".to_vec()),
                cancel: &cancel,
                after: 3,
                eof,
            },
            ABC.size,
            ABC.sha256,
            &|| cancel.check(),
        );
        assert!(cancel.is_cancelled());
        assert_eq!(result, Err("meeting asset installation cancelled".into()));
    }
    let cancel = Cancel::default();
    assert!(verify_reader(Cursor::new(b"abc"), ABC.size, ABC.sha256, &|| cancel.check()).is_ok());
    for bytes in [b"ab".as_slice(), b"abcd".as_slice(), b"abd".as_slice()] {
        assert!(verify_reader(Cursor::new(bytes), ABC.size, ABC.sha256, &|| cancel.check()).is_err());
    }
}

#[test]
fn cancelled_member_reads_never_create_or_accept_output() {
    let temp = Temp::new();
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(write_checked(Cursor::new(b"abc"), &temp.0, ABC, &cancel).is_err());
    assert!(!temp.0.join(ABC.name).exists());

    for eof in [false, true] {
        let temp = Temp::new();
        let cancel = Cancel::default();
        assert!(
            write_checked(
                CancellingRead {
                    input: Cursor::new(b"abc".to_vec()),
                    cancel: &cancel,
                    after: 3,
                    eof,
                },
                &temp.0,
                ABC,
                &cancel,
            )
            .is_err()
        );
        assert!(cancel.is_cancelled());
        assert_eq!(
            fs::read(temp.0.join(ABC.name)).expect("staging output"),
            if eof { b"abc".as_slice() } else { b"".as_slice() }
        );
    }
}

#[test]
fn cancellation_in_the_archive_end_marker_cannot_report_success() {
    let temp = Temp::new();
    let cancel = Cancel::default();
    let bytes = archive(&[("package/file", b"abc", tar::EntryType::Regular)]);
    let after = bytes.len() as u64 - 1024 + 1;
    let result = unpack_tar(
        CancellingRead {
            input: Cursor::new(bytes),
            cancel: &cancel,
            after,
            eof: false,
        },
        &temp.0,
        &[("package/file", ABC)],
        &cancel,
    );
    assert!(cancel.is_cancelled(), "the archive end marker was read");
    assert!(result.is_err(), "cancelled extraction must not succeed");
}

fn archive(entries: &[(&str, &[u8], tar::EntryType)]) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    for &(name, bytes, kind) in entries {
        let mut h = tar::Header::new_gnu();
        h.set_mode(0o600);
        h.set_size(bytes.len() as u64);
        h.set_entry_type(kind);
        h.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        h.set_cksum();
        archive.append(&h, Cursor::new(bytes)).expect("raw test header");
    }
    archive.into_inner().expect("tar")
}

#[test]
fn archive_copies_only_exact_pinned_regular_members() {
    let temp = Temp::new();
    unpack_tar(
        Cursor::new(archive(&[
            ("ignored", b"ignored", tar::EntryType::Regular),
            ("package/file", b"abc", tar::EntryType::Regular),
        ])),
        &temp.0,
        &[("package/file", ABC)],
        &Cancel::default(),
    )
    .expect("unpack");
    assert_eq!(fs::read(temp.0.join("checked")).expect("checked bytes"), b"abc");
    assert!(!temp.0.join("ignored").exists());
}
#[test]
fn archive_rejects_traversal_duplicates_links_missing_or_corrupt_members() {
    let entries: [Vec<(&str, &[u8], tar::EntryType)>; 7] = [
        vec![("../escape", b"abc", tar::EntryType::Regular)],
        vec![("/escape", b"abc", tar::EntryType::Regular)],
        vec![
            ("package/file", b"abc", tar::EntryType::Regular),
            ("package/file", b"abc", tar::EntryType::Regular),
        ],
        vec![("package/file", b"", tar::EntryType::Symlink)],
        vec![("package/file", b"abd", tar::EntryType::Regular)],
        vec![("package/file", b"abcd", tar::EntryType::Regular)],
        vec![("other", b"abc", tar::EntryType::Regular)],
    ];
    for entries in entries {
        let temp = Temp::new();
        assert!(
            unpack_tar(
                Cursor::new(archive(&entries)),
                &temp.0,
                &[("package/file", ABC)],
                &Cancel::default()
            )
            .is_err()
        );
    }
}
#[test]
fn archive_entry_and_expansion_budgets_and_cancellation_hold() {
    let temp = Temp::new();
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(
        unpack_tar(
            Cursor::new(archive(&[("file", b"abc", tar::EntryType::Regular)])),
            &temp.0,
            &[("file", ABC)],
            &cancel
        )
        .is_err()
    );
    let mut header = tar::Header::new_gnu();
    header.set_size(64 * 1024 * 1024 + 1);
    header.set_entry_type(tar::EntryType::Regular);
    header.as_mut_bytes()[..4].copy_from_slice(b"file");
    header.set_cksum();
    assert!(
        unpack_tar(
            Cursor::new(header.as_bytes().to_vec()),
            &temp.0,
            &[("file", ABC)],
            &Cancel::default()
        )
        .is_err()
    );
    let data = archive(&vec![("ignored", b"".as_slice(), tar::EntryType::Regular); 1025]);
    assert!(unpack_tar(Cursor::new(data), &temp.0, &[], &Cancel::default()).is_err());
}
#[cfg(unix)]
#[test]
fn unsafe_install_locks_are_rejected_before_stale_data_is_removed() {
    use std::os::unix::fs::PermissionsExt;

    for (mode, linked) in [(0o644, false), (0o666, false), (0o600, true)] {
        let temp = Temp::new();
        let lock = temp.0.join(".install.lock");
        fs::write(&lock, b"preserve").expect("lock fixture");
        fs::set_permissions(&lock, fs::Permissions::from_mode(mode)).expect("lock mode");
        if linked {
            fs::hard_link(&lock, temp.0.join("shared-lock")).expect("shared inode");
        }
        let stale = temp.0.join(".install-abandoned");
        fs::create_dir(&stale).expect("stale stage");
        fs::write(stale.join("partial"), b"preserve").expect("partial fixture");
        assert!(
            Stage::begin(temp.0.clone(), &Cancel::default()).is_err(),
            "mode={mode:o}, linked={linked}"
        );
        assert_eq!(
            fs::read(stale.join("partial")).expect("untouched stale data"),
            b"preserve"
        );
        assert_eq!(fs::read(&lock).expect("untouched lock"), b"preserve");
    }

    let temp = Temp::new();
    let lock = temp.0.join(".install.lock");
    fs::write(&lock, b"preserve").expect("private lock");
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).expect("private mode");
    let Prepared::Stage(stage) = Stage::begin(temp.0.clone(), &Cancel::default()).expect("valid lock") else {
        panic!("unexpected cached bundle")
    };
    assert!(Stage::begin(temp.0.clone(), &Cancel::default()).is_err());
    drop(stage);
    assert_eq!(fs::read(lock).expect("lock not truncated"), b"preserve");
}

#[cfg(unix)]
#[test]
fn interrupted_installer_recovers_after_process_termination() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    const ROOT: &str = "VULTS_ASSETS_CRASH_TEST_ROOT";
    if let Some(root) = std::env::var_os(ROOT) {
        let root = PathBuf::from(root);
        let Prepared::Stage(stage) = Stage::begin(root.clone(), &Cancel::default()).expect("child stage")
        else {
            panic!("unexpected cached bundle")
        };
        fs::write(stage.path.join("partial.download"), b"partial").expect("interrupted download");
        fs::write(
            root.join("ready.tmp"),
            stage.path.file_name().expect("stage name").as_encoded_bytes(),
        )
        .expect("child ready");
        fs::rename(root.join("ready.tmp"), root.join("ready")).expect("publish readiness");
        // Keep the real OS lock alive until the parent terminates this process.
        loop {
            std::thread::park();
        }
    }

    struct KillOnDrop(Child);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let temp = Temp::new();
    let previous = temp.0.join("previous-version");
    fs::create_dir(&previous).expect("previous version");
    fs::write(previous.join("keep"), b"preserve").expect("existing file");
    let mut child = KillOnDrop(
        Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "meeting::assets::files::tests::interrupted_installer_recovers_after_process_termination",
            ])
            .env(ROOT, &temp.0)
            .stdout(Stdio::null())
            .spawn()
            .expect("child process"),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !temp.0.join("ready").exists() {
        assert!(
            child.0.try_wait().expect("child status").is_none(),
            "child exited before readiness"
        );
        assert!(Instant::now() < deadline, "child readiness timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    let name = fs::read_to_string(temp.0.join("ready")).expect("stage name");
    let stale = temp.0.join(name);
    assert!(stale.join("partial.download").exists());
    assert!(Stage::begin(temp.0.clone(), &Cancel::default()).is_err());
    assert!(
        stale.join("partial.download").exists(),
        "a live stage is preserved"
    );
    child.0.kill().expect("terminate child");
    assert_eq!(
        child.0.wait().expect("child termination").signal(),
        Some(libc::SIGKILL)
    );
    assert!(stale.exists(), "termination bypasses Stage::drop");

    let Prepared::Stage(recovered) = Stage::begin(temp.0.clone(), &Cancel::default()).expect("recover")
    else {
        panic!("unexpected cached bundle")
    };
    assert!(!stale.exists());
    assert_eq!(
        fs::read(previous.join("keep")).expect("previous version preserved"),
        b"preserve"
    );
    let active = recovered.path.clone();
    drop(recovered);
    assert!(!active.exists());
}

#[cfg(unix)]
#[test]
fn abandoned_cleanup_stops_before_removing_the_next_entry_on_cancellation() {
    use std::cell::Cell;

    let temp = Temp::new();
    let first = temp.0.join(".install-first");
    let second = temp.0.join(".install-second");
    let previous = temp.0.join("previous-version");
    for directory in [&first, &second, &previous] {
        fs::create_dir(directory).expect("fixture directory");
        fs::write(directory.join("keep"), b"preserve").expect("fixture bytes");
    }
    let mut entries = fs::read_dir(&temp.0)
        .expect("entries")
        .collect::<Result<Vec<_>, _>>()
        .expect("entry metadata");
    entries.sort_by_key(|entry| entry.file_name());
    let seen = Cell::new(0);
    let cancel = Cancel::default();
    let entries = entries.into_iter().map(Ok).inspect(|_| {
        seen.set(seen.get() + 1);
        if seen.get() == 2 {
            cancel.cancel();
        }
    });
    assert_eq!(
        remove_abandoned(entries, &cancel),
        Err("meeting asset installation cancelled".into())
    );
    assert!(!first.exists());
    assert_eq!(
        fs::read(second.join("keep")).expect("remaining stage"),
        b"preserve"
    );
    assert_eq!(
        fs::read(previous.join("keep")).expect("previous version"),
        b"preserve"
    );
    assert_eq!(
        remove_abandoned(
            std::iter::once_with(|| panic!("cancelled cleanup must not poll entries")),
            &cancel,
        ),
        Err("meeting asset installation cancelled".into())
    );
    remove_abandoned(fs::read_dir(&temp.0).expect("retry entries"), &Cancel::default())
        .expect("explicit retry");
    assert!(!second.exists());
    assert_eq!(
        fs::read(previous.join("keep")).expect("previous version after retry"),
        b"preserve"
    );
}

#[cfg(unix)]
#[test]
fn concurrent_staging_is_rejected_and_abandoned_attempts_are_cleaned() {
    let temp = Temp::new();
    let root = temp.0.join("assets");
    let cancel = Cancel::default();
    let Prepared::Stage(stage) = Stage::begin(root.clone(), &cancel).expect("stage") else {
        panic!("ready")
    };
    let staged = stage.path.clone();
    assert!(staged.exists());
    assert!(Stage::begin(root.clone(), &cancel).is_err());
    drop(stage);
    assert!(!staged.exists());
    let stale = root.join(".install-abandoned");
    fs::create_dir(&stale).expect("stale");
    fs::write(stale.join("partial"), b"partial").expect("partial");
    let outside = temp.0.join("outside");
    fs::create_dir(&outside).expect("outside");
    fs::write(outside.join("keep"), b"keep").expect("keep");
    std::os::unix::fs::symlink(&outside, root.join(".install-link")).expect("link");
    let second = Stage::begin(root, &cancel).expect("recovered");
    assert!(!stale.exists());
    assert!(outside.join("keep").exists());
    drop(second);
}
#[cfg(unix)]
#[test]
fn incomplete_bundles_and_cancelled_stages_never_publish_or_replace_data() {
    let temp = Temp::new();
    let root = temp.0.join("assets");
    let cancel = Cancel::default();
    let Prepared::Stage(stage) = Stage::begin(root.clone(), &cancel).expect("stage") else {
        panic!("ready")
    };
    assert!(stage.commit(&cancel).is_err());
    assert!(!root.join(directory_name()).exists());
    let Prepared::Stage(stage) = Stage::begin(root.clone(), &cancel).expect("stage") else {
        panic!("ready")
    };
    cancel.cancel();
    assert!(stage.commit(&cancel).is_err());
    assert!(!root.join(directory_name()).exists());
    let target = root.join(directory_name());
    fs::create_dir(&target).expect("incomplete bundle");
    fs::write(target.join("user-file"), b"preserve").expect("marker");
    assert!(Stage::begin(root, &Cancel::default()).is_err());
    assert_eq!(
        fs::read(target.join("user-file")).expect("unchanged"),
        b"preserve"
    );
}
#[cfg(unix)]
#[test]
fn symlink_roots_and_model_files_are_not_followed() {
    let temp = Temp::new();
    let link = temp.0.join("alias");
    std::os::unix::fs::symlink(&temp.0, &link).expect("link");
    assert!(Stage::begin(link, &Cancel::default()).is_err());
    fs::write(temp.0.join("original"), b"abc").expect("source");
    std::os::unix::fs::symlink(temp.0.join("original"), temp.0.join("checked")).expect("file link");
    assert!(
        verify(&temp.0.join("checked"), 3, ABC.sha256, &|| Cancel::default()
            .check())
        .is_err()
    );
}

#[test]
#[cfg(feature = "diarization")]
fn recording_cancellation_reaches_bundle_verification() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let stop = AtomicBool::new(false);
    let root = Temp::new();
    assert!(
        super::super::installed_for_analysis(&root.0, &stop)
            .unwrap()
            .is_none()
    );
    stop.store(true, Ordering::SeqCst);
    assert_eq!(
        super::super::installed_for_analysis(&root.0, &stop).err(),
        Some("meeting analysis cancelled".into())
    );
    let stop = AtomicBool::new(false);
    let reads = std::cell::Cell::new(0);
    let check = || {
        reads.set(reads.get() + 1);
        if reads.get() == 2 {
            stop.store(true, Ordering::SeqCst);
        }
        if stop.load(Ordering::SeqCst) {
            Err("meeting analysis cancelled".into())
        } else {
            Ok(())
        }
    };
    assert_eq!(
        verify_reader(Cursor::new(b"abc"), ABC.size, ABC.sha256, &check),
        Err("meeting analysis cancelled".into())
    );
    assert_eq!(reads.get(), 2);
}
