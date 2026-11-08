use super::*;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    task::Poll,
};

struct Fake {
    state: RefCell<Snapshot>,
    valid: Cell<bool>,
    writes: Cell<usize>,
    fail: bool,
    change_after_write: bool,
}
impl Fake {
    fn new(channels: &[f32], muted: bool) -> Self {
        Self {
            state: RefCell::new(Snapshot {
                channels: channels.into(),
                muted,
            }),
            valid: Cell::new(true),
            writes: Cell::new(0),
            fail: false,
            change_after_write: false,
        }
    }
    fn writing(&self) -> Result<(), Error> {
        self.writes.set(self.writes.get() + 1);
        if self.change_after_write {
            self.valid.set(false);
        }
        if self.fail { Err(Error::Failed) } else { Ok(()) }
    }
}
impl Io for Fake {
    fn valid(&self) -> bool {
        self.valid.get()
    }
    fn snapshot(&self) -> Result<Snapshot, Error> {
        Ok(self.state.borrow().clone())
    }
    fn mute(&self, muted: bool) -> Result<(), Error> {
        self.writing()?;
        self.state.borrow_mut().muted = muted;
        Ok(())
    }
    fn channel(&self, index: usize, volume: f32) -> Result<(), Error> {
        self.writing()?;
        self.state.borrow_mut().channels[index] = volume;
        Ok(())
    }
}

#[test]
fn volume_preserves_balance_and_mute_and_relative_steps_are_bounded() {
    let io = Fake::new(&[0.8, 0.4], true);
    assert_eq!(
        apply(&io, Action::Volume(40), &|| true).unwrap(),
        Status {
            muted: true,
            percent: 40
        }
    );
    assert_eq!(io.state.borrow().channels, [0.4, 0.2]);
    assert_eq!(apply(&io, Action::Louder, &|| true).unwrap().percent, 50);
    assert_eq!(io.state.borrow().channels, [0.5, 0.25]);
    assert_eq!(apply(&io, Action::Quieter, &|| true).unwrap().percent, 40);
    let zero = Fake::new(&[0.0, 0.0], false);
    assert_eq!(apply(&zero, Action::Quieter, &|| true).unwrap().percent, 0);
    assert_eq!(apply(&zero, Action::Volume(100), &|| true).unwrap().percent, 100);
    assert_eq!(apply(&zero, Action::Louder, &|| true).unwrap().percent, 100);
    assert_eq!(zero.state.borrow().channels, [1.0, 1.0]);
}

#[test]
fn mute_does_not_change_levels_and_invalid_or_cancelled_work_never_writes() {
    let io = Fake::new(&[0.7, 0.35], false);
    assert!(apply(&io, Action::Mute(true), &|| true).unwrap().muted);
    assert_eq!(io.state.borrow().channels, [0.7, 0.35]);
    assert_eq!(apply(&io, Action::Volume(101), &|| true), Err(Error::Invalid));
    assert_eq!(apply(&io, Action::Mute(false), &|| false), Err(Error::Cancelled));
    io.valid.set(false);
    assert_eq!(apply(&io, Action::Mute(false), &|| true), Err(Error::Changed));
    assert_eq!(io.writes.get(), 1);
    let invalid = Fake::new(&[f32::NAN], false);
    assert_eq!(apply(&invalid, Action::Volume(40), &|| true), Err(Error::Invalid));
    assert_eq!(invalid.writes.get(), 0);
}

#[test]
fn user_changes_and_failures_stop_remaining_channel_writes_without_retry() {
    let mut io = Fake::new(&[0.8, 0.4], false);
    io.change_after_write = true;
    assert_eq!(apply(&io, Action::Volume(40), &|| true), Err(Error::Changed));
    assert_eq!(io.writes.get(), 1);
    let mut failed = Fake::new(&[0.8, 0.4], false);
    failed.fail = true;
    assert_eq!(apply(&failed, Action::Volume(40), &|| true), Err(Error::Failed));
    assert_eq!(failed.writes.get(), 1);
}

#[test]
fn cancellation_during_native_validation_prevents_the_write() {
    let io = Fake::new(&[0.5], false);
    let checks = Cell::new(0);
    let allowed = || {
        checks.set(checks.get() + 1);
        checks.get() == 1
    };
    assert_eq!(apply(&io, Action::Mute(true), &allowed), Err(Error::Cancelled));
    assert_eq!(io.writes.get(), 0);
}

#[test]
fn device_switches_and_foreign_volume_events_invalidate_the_pinned_target() {
    let invalid = Arc::new(AtomicBool::new(false));
    let watcher: IMMNotificationClient = DeviceWatch {
        endpoint_id: "fixture".into(),
        invalid: invalid.clone(),
    }
    .into();
    unsafe {
        watcher
            .OnDefaultDeviceChanged(eRender, eMultimedia, PCWSTR::null())
            .unwrap()
    };
    assert!(invalid.load(Ordering::Acquire));
    // Returning to the old default does not revive an expired target.
    unsafe {
        watcher
            .OnDefaultDeviceChanged(eRender, eMultimedia, PCWSTR::null())
            .unwrap()
    };
    assert!(invalid.load(Ordering::Acquire));
    invalid.store(false, Ordering::Release);
    let context = GUID::from_u128(1);
    let volume: IAudioEndpointVolumeCallback = VolumeWatch {
        invalid: invalid.clone(),
        context,
    }
    .into();
    let mut event = AUDIO_VOLUME_NOTIFICATION_DATA {
        guidEventContext: context,
        ..Default::default()
    };
    unsafe { volume.OnNotify(&mut event).unwrap() };
    assert!(!invalid.load(Ordering::Acquire));
    event.guidEventContext = GUID::zeroed();
    unsafe { volume.OnNotify(&mut event).unwrap() };
    assert!(invalid.load(Ordering::Acquire));
}

#[tokio::test]
async fn dropping_a_waiter_cancels_queued_audio_writes() {
    let (tx, rx) = mpsc::channel();
    let target = Target(tx);
    let mut pending = Box::pin(target.apply(Action::Mute(true), || true));
    std::future::poll_fn(|context| {
        assert!(pending.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    let Request::Apply {
        action,
        allowed,
        reply,
    } = rx.try_recv().unwrap();
    drop(pending);
    let io = Fake::new(&[0.5], false);
    assert_eq!(
        apply(&io, action, &|| allowed() && !reply.is_closed()),
        Err(Error::Cancelled)
    );
    assert_eq!(io.writes.get(), 0);
    assert!(matches!(pin(|| false).await, Err(Error::Cancelled)));
}

#[test]
#[ignore = "reads and rewrites the current physical output settings without changing them"]
fn native_output_readback_and_same_value_writes() {
    let native = Native::new().unwrap();
    assert!(native.valid());
    let before = native.snapshot().unwrap();
    validate(&before).unwrap();
    guard(&native, &|| true).unwrap();
    native.mute(before.muted).unwrap();
    for (index, volume) in before.channels.iter().enumerate() {
        guard(&native, &|| true).unwrap();
        native.channel(index, *volume).unwrap();
    }
    let after = native.snapshot().unwrap();
    assert_eq!(after.muted, before.muted);
    assert_eq!(after.channels, before.channels);
    assert_eq!(
        apply(&native, Action::Mute(before.muted), &|| true)
            .unwrap()
            .percent,
        percent(&before.channels)
    );
}
