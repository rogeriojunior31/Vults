use super::*;
use std::cell::{Cell, RefCell};

struct Fake {
    state: RefCell<Snapshot>,
    valid: Cell<bool>,
    writes: Cell<usize>,
    stop_after: usize,
}
impl Fake {
    fn new(levels: Vec<f32>) -> Self {
        Self {
            state: RefCell::new(Snapshot {
                muted: vec![true; levels.len()],
                levels,
            }),
            valid: Cell::new(true),
            writes: Cell::new(0),
            stop_after: usize::MAX,
        }
    }
}
impl Io for Fake {
    fn valid(&self) -> bool {
        self.valid.get()
    }
    fn snapshot(&self) -> Result<Snapshot, Error> {
        Ok(self.state.borrow().clone())
    }
    fn set(&self, index: usize, value: f32) -> Result<(), Error> {
        self.state.borrow_mut().levels[index] = value;
        self.writes.set(self.writes.get() + 1);
        if self.writes.get() >= self.stop_after {
            self.valid.set(false);
        }
        Ok(())
    }
}
#[test]
fn preserves_relative_stream_levels_and_mutes() {
    let io = Fake::new(vec![0.8, 0.4, 0.0]);
    assert_eq!(apply(&io, Action::Volume(50), &|| true), Ok(50));
    assert_eq!(io.state.borrow().levels, [0.5, 0.25, 0.0]);
    assert_eq!(io.state.borrow().muted, [true, true, true]);
    assert_eq!(apply(&io, Action::Louder, &|| true), Ok(60));
    assert_eq!(apply(&io, Action::Quieter, &|| true), Ok(50));
}
#[test]
fn admission_and_bounds_precede_any_write() {
    let io = Fake::new(vec![0.5]);
    assert_eq!(apply(&io, Action::Volume(101), &|| true), Err(Error::Invalid));
    assert_eq!(apply(&io, Action::Volume(80), &|| false), Err(Error::Cancelled));
    io.valid.set(false);
    assert_eq!(apply(&io, Action::Volume(80), &|| true), Err(Error::Changed));
    assert_eq!(io.writes.get(), 0);
}

#[test]
fn closed_reply_and_revocation_prevent_late_writes() {
    let io = Fake::new(vec![0.8, 0.4]);
    let (reply, result) = oneshot::channel::<()>();
    drop(result);
    assert_eq!(
        apply(&io, Action::Volume(10), &|| !reply.is_closed()),
        Err(Error::Cancelled)
    );
    assert_eq!(io.writes.get(), 0);
    let calls = Cell::new(0);
    assert_eq!(
        apply(&io, Action::Volume(10), &|| {
            calls.set(calls.get() + 1);
            calls.get() < 5
        }),
        Err(Error::Cancelled)
    );
    assert_eq!(io.writes.get(), 1);
    assert_eq!(io.state.borrow().levels, [0.1, 0.4]);
}
#[test]
fn changed_group_stops_remaining_writes_without_rollback() {
    let mut io = Fake::new(vec![0.8, 0.4]);
    io.stop_after = 1;
    assert_eq!(apply(&io, Action::Volume(50), &|| true), Err(Error::Changed));
    assert_eq!(io.writes.get(), 1);
    assert_eq!(io.state.borrow().levels, [0.5, 0.4]);
}
#[test]
fn rejects_corrupt_levels_and_handles_silent_groups() {
    for levels in [vec![], vec![f32::NAN], vec![-0.1], vec![1.1], vec![0.5; 17]] {
        let io = Fake::new(levels);
        assert_eq!(apply(&io, Action::Louder, &|| true), Err(Error::Invalid));
        assert_eq!(io.writes.get(), 0);
    }
    let io = Fake::new(vec![0.0, 0.0]);
    assert_eq!(apply(&io, Action::Quieter, &|| true), Ok(0));
    assert_eq!(apply(&io, Action::Louder, &|| true), Ok(10));
}

#[test]
fn own_changes_do_not_invalidate_but_foreign_changes_do() {
    let invalid = Arc::new(AtomicBool::new(false));
    let context = GUID::from_u128(9);
    let watch = Watch {
        invalid: invalid.clone(),
        context,
    };
    watch.foreign(&context);
    assert!(!invalid.load(Ordering::Acquire));
    watch.foreign(std::ptr::null());
    assert!(invalid.load(Ordering::Acquire));
}

#[test]
#[ignore = "silent child fixture, invoked only by the owned native audio test"]
fn native_silent_child() {
    if std::env::var("VULTS_PLAYER_AUDIO_FIXTURE").as_deref() != Ok("1") {
        return;
    }
    use windows::Win32::Media::Audio::{
        AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, IAudioClient, IAudioRenderClient,
    };
    let _apartment = initialize().unwrap();
    let device = unsafe {
        enumerator()
            .unwrap()
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
    }
    .unwrap();
    let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }.unwrap();
    let format = unsafe { client.GetMixFormat() }.unwrap();
    let id = unsafe { CoCreateGuid() }.unwrap();
    let initialized =
        unsafe { client.Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 10_000_000, 0, format, Some(&id)) };
    unsafe { CoTaskMemFree(Some(format.cast())) };
    initialized.unwrap();
    let render: IAudioRenderClient = unsafe { client.GetService() }.unwrap();
    let count = unsafe { client.GetBufferSize() }.unwrap();
    unsafe { render.GetBuffer(count) }.unwrap();
    unsafe { render.ReleaseBuffer(count, AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) }.unwrap();
    unsafe { client.Start() }.unwrap();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    unsafe { client.Stop() }.unwrap();
}

#[tokio::test]
#[ignore = "requires an interactive Windows audio output; changes only its silent child session"]
async fn native_owned_audio_group() {
    use std::process::{Command, Stdio};
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "player_audio::tests::native_silent_child",
            "--ignored",
            "--nocapture",
        ])
        .env("VULTS_PLAYER_AUDIO_FIXTURE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000);
    let child = Child(command.spawn().unwrap());
    let mut candidate = None;
    for _ in 0..40 {
        candidate = list(|| true)
            .await
            .unwrap()
            .into_iter()
            .find(|item| item.pid == child.0.id());
        if candidate.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let candidate = candidate.expect("silent child must publish an audio session");
    let target = pin(candidate.selection.clone(), || true).await.unwrap();
    target.validate(|| true).await.unwrap();
    assert_eq!(target.apply(Action::Volume(37), || true).await, Ok(37));
    assert_eq!(target.apply(Action::Louder, || true).await, Ok(47));
    assert_eq!(
        target.apply(Action::Volume(60), || false).await,
        Err(Error::Cancelled)
    );
    let current = list(|| true)
        .await
        .unwrap()
        .into_iter()
        .find(|item| item.pid == child.0.id())
        .unwrap();
    assert_eq!(current.percent, 47);
    let external = pin(candidate.selection, || true).await.unwrap();
    assert_eq!(external.apply(Action::Volume(33), || true).await, Ok(33));
    for _ in 0..20 {
        if target.validate(|| true).await.is_err() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(target.apply(Action::Louder, || true).await, Err(Error::Changed));
    drop(child);
    assert!(target.validate(|| true).await.is_err());
    assert!(target.apply(Action::Volume(80), || true).await.is_err());
}
