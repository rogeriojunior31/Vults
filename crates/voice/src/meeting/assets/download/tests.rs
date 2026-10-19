use super::*;
use crate::meeting::assets::{catalog::Format, install, review_id};
use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "vults-asset-stream-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&p).expect("temp");
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
const TEST: Download = Download {
    id: "fixture",
    url: "https://github.com/fixture",
    size: 3,
    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    format: Format::Raw,
    members: &[],
};
struct Deny {
    attempts: AtomicUsize,
    done: Mutex<Vec<Outcome>>,
}
impl Gate for Deny {
    fn allow(&self, _: &'static str) -> bool {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        false
    }
    fn done(&self, _: &'static str, outcome: Outcome, _: u64) {
        self.done.lock().expect("trace").push(outcome);
    }
}
fn gate() -> Deny {
    Deny {
        attempts: AtomicUsize::new(0),
        done: Mutex::new(Vec::new()),
    }
}
#[test]
fn redirects_are_https_only_without_credentials_or_unapproved_hosts() {
    for url in [
        "https://github.com/a",
        "https://release-assets.githubusercontent.com/a?token=fixture",
        "https://objects.githubusercontent.com/a",
    ] {
        assert!(allowed(&Url::parse(url).expect("URL")));
    }
    for url in [
        "http://github.com/a",
        "https://github.com.evil.test/a",
        "https://127.0.0.1/a",
        "https://github.com:8443/a",
        "https://user:secret@github.com/a",
        "https://github.com/a#fragment",
    ] {
        assert!(!allowed(&Url::parse(url).expect("URL")));
    }
}
#[tokio::test]
async fn valid_stream_is_verified_and_reports_bounded_content_free_progress() {
    let temp = Temp::new();
    let mut file = tokio::fs::File::create(temp.0.join("part")).await.expect("file");
    let progress = Mutex::new(Vec::new());
    copy_chunks(
        &mut futures_util::stream::iter([Ok(b"a".as_slice()), Ok(b"bc".as_slice())]),
        &mut file,
        TEST,
        &Cancel::default(),
        tokio::time::Instant::now() + Duration::from_secs(1),
        &|p| progress.lock().expect("progress").push(p),
    )
    .await
    .expect("checked");
    assert_eq!(std::fs::read(temp.0.join("part")).expect("data"), b"abc");
    let p = progress.lock().expect("progress");
    assert_eq!((p[0].downloaded, p[1].downloaded, p[1].total), (1, 3, 3));
    assert_eq!(p[0].artifact, "fixture");
}
#[tokio::test]
async fn short_long_corrupt_and_failed_streams_are_rejected() {
    for chunks in [
        vec![Ok(b"ab".as_slice())],
        vec![Ok(b"abcd".as_slice())],
        vec![Ok(b"abd".as_slice())],
        vec![Err("fixture failure".into())],
    ] {
        let temp = Temp::new();
        let mut file = tokio::fs::File::create(temp.0.join("part")).await.expect("file");
        assert!(
            copy_chunks(
                &mut futures_util::stream::iter(chunks),
                &mut file,
                TEST,
                &Cancel::default(),
                tokio::time::Instant::now() + Duration::from_secs(1),
                &|_| {}
            )
            .await
            .is_err()
        );
    }
}
#[tokio::test]
async fn an_expired_deadline_rejects_even_an_immediately_ready_valid_stream() {
    let temp = Temp::new();
    let path = temp.0.join("part");
    let mut file = tokio::fs::File::create(&path).await.expect("file");
    let progress = AtomicUsize::new(0);
    let result = copy_chunks(
        &mut futures_util::stream::iter([Ok(b"abc".as_slice())]),
        &mut file,
        TEST,
        &Cancel::default(),
        tokio::time::Instant::now() - Duration::from_secs(1),
        &|_| {
            progress.fetch_add(1, Ordering::SeqCst);
        },
    )
    .await;
    assert_eq!(result, Err("meeting asset download timed out".into()));
    assert_eq!(progress.load(Ordering::SeqCst), 0);
    assert!(std::fs::read(path).expect("staging data").is_empty());
}

#[tokio::test]
async fn pre_cancelled_fetch_creates_no_staging_file() {
    let temp = Temp::new();
    let gate = gate();
    let cancel = Cancel::default();
    cancel.cancel();
    let progress = AtomicUsize::new(0);
    let result = fetch(TEST, &temp.0, &gate, &cancel, &|_| {
        progress.fetch_add(1, Ordering::SeqCst);
    })
    .await;
    assert_eq!(result, Err("meeting asset installation cancelled".into()));
    assert_eq!(gate.attempts.load(Ordering::SeqCst), 0);
    assert!(gate.done.lock().expect("trace").is_empty());
    assert_eq!(progress.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read_dir(&temp.0).expect("staging directory").count(), 0);
}

#[tokio::test]
async fn cancellation_from_a_ready_chunk_never_writes_or_reports_progress() {
    let temp = Temp::new();
    let path = temp.0.join("part");
    let mut file = tokio::fs::File::create(&path).await.expect("file");
    let cancel = Cancel::default();
    let mut stream = futures_util::stream::iter([Ok(b"abc".as_slice())]).inspect(|_| cancel.cancel());
    let progress = AtomicUsize::new(0);
    let result = copy_chunks(
        &mut stream,
        &mut file,
        TEST,
        &cancel,
        tokio::time::Instant::now() + Duration::from_secs(60),
        &|_| {
            progress.fetch_add(1, Ordering::SeqCst);
        },
    )
    .await;
    file.flush().await.expect("drain pending writes");
    assert_eq!(result, Err("meeting asset installation cancelled".into()));
    assert_eq!(progress.load(Ordering::SeqCst), 0);
    assert!(std::fs::read(path).expect("staging data").is_empty());
}

#[tokio::test]
async fn cancellation_and_deadline_interrupt_a_pending_stream() {
    let temp = Temp::new();
    let mut file = tokio::fs::File::create(temp.0.join("part")).await.expect("file");
    let cancel = Cancel::default();
    let stop = async {
        tokio::task::yield_now().await;
        cancel.cancel();
    };
    let mut pending = futures_util::stream::pending::<Result<&[u8], String>>();
    let copy = copy_chunks(
        &mut pending,
        &mut file,
        TEST,
        &cancel,
        tokio::time::Instant::now() + Duration::from_secs(1),
        &|_| {},
    );
    let (_, result) = tokio::join!(stop, copy);
    assert!(result.is_err());
    assert!(
        copy_chunks(
            &mut futures_util::stream::pending::<Result<&[u8], String>>(),
            &mut file,
            TEST,
            &Cancel::default(),
            tokio::time::Instant::now(),
            &|_| {}
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn denied_gate_stale_review_and_pre_cancelled_installs_send_no_request() {
    let temp = Temp::new();
    let gate = gate();
    let root = temp.0.join("assets");
    assert!(
        install(root.clone(), "stale", &gate, Cancel::default(), |_| {})
            .await
            .is_err()
    );
    assert!(!root.exists());
    assert_eq!(gate.attempts.load(Ordering::SeqCst), 0);
    let cancel = Cancel::default();
    cancel.cancel();
    let cancelled = install(root.clone(), &review_id(), &gate, cancel, |_| {}).await;
    if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        assert!(cancelled.expect("cancel").is_none());
    } else {
        assert_eq!(
            cancelled.expect_err("unsupported platform"),
            "meeting assets are supported on Linux x86-64 only"
        );
    }
    assert_eq!(gate.attempts.load(Ordering::SeqCst), 0);
    assert!(!root.exists());
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    {
        let failed = Cancel::default();
        assert!(
            install(root.clone(), &review_id(), &gate, failed.clone(), |_| {})
                .await
                .is_err()
        );
        assert!(
            !failed.is_cancelled(),
            "ordinary failure must not masquerade as cancellation"
        );
        assert_eq!(gate.attempts.load(Ordering::SeqCst), 1);
        assert!(gate.done.lock().expect("trace").is_empty());
        assert!(!root.join(crate::meeting::assets::directory_name()).exists());
        assert!(std::fs::read_dir(root).expect("root").all(|e| {
            !e.expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".install-")
        }));
    }
}
#[test]
fn request_admission_rejects_expiry_before_charging_the_gate() {
    let gate = gate();
    assert_eq!(
        Trace::begin(
            &gate,
            "fixture",
            &Cancel::default(),
            tokio::time::Instant::now() - Duration::from_secs(1),
        )
        .err(),
        Some("meeting asset download timed out".into())
    );
    assert_eq!(gate.attempts.load(Ordering::SeqCst), 0);
    assert!(gate.done.lock().expect("trace").is_empty());
}

#[test]
fn cancellation_during_request_admission_closes_its_reserved_trace() {
    struct CancellingGate {
        cancel: Cancel,
        done: Mutex<Vec<Outcome>>,
    }
    impl Gate for CancellingGate {
        fn allow(&self, _: &'static str) -> bool {
            self.cancel.cancel();
            true
        }
        fn done(&self, id: &'static str, outcome: Outcome, _: u64) {
            assert_eq!(id, "fixture");
            self.done.lock().expect("trace").push(outcome);
        }
    }
    let gate = CancellingGate {
        cancel: Cancel::default(),
        done: Mutex::new(Vec::new()),
    };
    assert_eq!(
        Trace::begin(
            &gate,
            "fixture",
            &gate.cancel,
            tokio::time::Instant::now() + Duration::from_secs(60),
        )
        .err(),
        Some("meeting asset installation cancelled".into())
    );
    assert_eq!(*gate.done.lock().expect("trace"), [Outcome::Cancelled]);
}

#[test]
fn dropped_request_keeps_only_its_artifact_id_and_cancelled_outcome() {
    let gate = gate();
    {
        let _request = Trace {
            gate: &gate,
            id: "fixture",
            started: Instant::now(),
            outcome: Outcome::Cancelled,
        };
    }
    assert_eq!(*gate.done.lock().expect("trace"), [Outcome::Cancelled]);
}
