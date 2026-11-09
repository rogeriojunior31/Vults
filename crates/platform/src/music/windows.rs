//! Best-effort pause scopes. Windows has no media-session volume control or write context.
use super::Mode;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Playback,
    },
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
};

const TICK: Duration = Duration::from_millis(25);
const BUDGET: Duration = Duration::from_secs(1);
static SERIAL: Mutex<()> = Mutex::new(());
static PENDING: AtomicUsize = AtomicUsize::new(0);

/// Drop signals the worker; capture never waits for a player or the media service.
#[derive(Debug)]
pub struct Listening(Option<mpsc::Sender<()>>);
impl Listening {
    pub fn start_guarded(mode: Mode, allowed: impl Fn() -> bool + Send + 'static) -> Self {
        if mode != Mode::Pause {
            return Self(None);
        }
        let (end, stopped) = mpsc::channel();
        PENDING.fetch_add(1, Ordering::AcqRel);
        let pending = Pending;
        let _ = std::thread::Builder::new()
            .name("dictation-media".into())
            .spawn(move || {
                let _pending = pending;
                let active = || stopped.try_recv() == Err(mpsc::TryRecvError::Empty) && allowed();
                let _serial = loop {
                    if !active() {
                        return;
                    }
                    match SERIAL.try_lock() {
                        Ok(guard) => break guard,
                        Err(std::sync::TryLockError::Poisoned(_)) => return,
                        Err(std::sync::TryLockError::WouldBlock) => std::thread::sleep(TICK),
                    }
                };
                if unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_err() {
                    return;
                }
                let _apartment = Apartment;
                if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                {
                    runtime.block_on(hold(active));
                }
            });
        Self(Some(end))
    }
}
impl Drop for Listening {
    fn drop(&mut self) {
        self.0.take();
    }
}
struct Pending;
impl Drop for Pending {
    fn drop(&mut self) {
        PENDING.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

/// Voice commands must not race the automatic resume. Admission remains cancellable.
pub async fn wait_for_restore_until(allowed: impl Fn() -> bool) -> bool {
    while PENDING.load(Ordering::Acquire) != 0 {
        if !allowed() {
            return false;
        }
        tokio::time::sleep(TICK).await;
    }
    allowed()
}

/// Graceful exit gives accepted pauses a bounded chance to resume.
pub fn shutdown() {
    let deadline = Instant::now() + Duration::from_secs(3);
    while PENDING.load(Ordering::Acquire) != 0 && Instant::now() < deadline {
        std::thread::sleep(TICK);
    }
}

#[derive(Default)]
struct Claim {
    issued: bool,
    pause_event: bool,
    touched: bool,
    confirmed: bool,
}
impl Claim {
    fn playback(&mut self, paused: bool) {
        // SMTC supplies no event context. Only one expected pause event is ours;
        // even a later play/pause pair sampled as Paused relinquishes the claim.
        if self.issued && paused && !self.pause_event {
            self.pause_event = true;
        } else {
            self.touched = true;
        }
    }
    fn restore(&self) -> bool {
        self.confirmed && self.pause_event && !self.touched
    }
}

struct Paused {
    session: Session,
    claim: Arc<Mutex<Claim>>,
    playback: i64,
    properties: i64,
    timeline: Option<i64>,
}
impl Drop for Paused {
    fn drop(&mut self) {
        let _ = self.session.RemovePlaybackInfoChanged(self.playback);
        let _ = self.session.RemoveMediaPropertiesChanged(self.properties);
    }
}
fn playing(session: &Session, status: Playback) -> bool {
    session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .is_ok_and(|current| current == status)
}
fn position(session: &Session) -> Option<i64> {
    session
        .GetTimelineProperties()
        .and_then(|timeline| timeline.Position())
        .ok()
        .map(|time| time.Duration)
}
fn controls(session: &Session, pause: bool) -> bool {
    session
        .GetPlaybackInfo()
        .and_then(|info| info.Controls())
        .is_ok_and(|controls| {
            if pause {
                controls.IsPauseEnabled() == Ok(true)
            } else {
                controls.IsPlayEnabled() == Ok(true)
            }
        })
}
fn watch(session: Session) -> Option<Paused> {
    let claim = Arc::new(Mutex::new(Claim::default()));
    let state = claim.clone();
    let playback = session
        .PlaybackInfoChanged(&TypedEventHandler::new(
            move |sender: windows::core::Ref<Session>, _| {
                let paused = sender
                    .as_ref()
                    .is_some_and(|session| playing(session, Playback::Paused));
                if let Ok(mut state) = state.lock() {
                    state.playback(paused);
                }
                Ok(())
            },
        ))
        .ok()?;
    let state = claim.clone();
    let properties = match session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
        if let Ok(mut state) = state.lock() {
            state.touched = true;
        }
        Ok(())
    })) {
        Ok(token) => token,
        Err(_) => {
            let _ = session.RemovePlaybackInfoChanged(playback);
            return None;
        }
    };
    Some(Paused {
        session,
        claim,
        playback,
        properties,
        timeline: None,
    })
}

async fn hold(active: impl Fn() -> bool) {
    let Ok(request) = Manager::RequestAsync() else {
        return;
    };
    let Ok(Ok(manager)) = tokio::time::timeout(BUDGET, request).await else {
        return;
    };
    let Ok(sessions) = manager.GetSessions() else {
        return;
    };
    hold_sessions(active, &manager, sessions.into_iter().take(16).collect()).await;
}

async fn hold_sessions(active: impl Fn() -> bool, manager: &Manager, sessions: Vec<Session>) {
    let changed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let marker = changed.clone();
    let Ok(token) = manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
        marker.store(true, Ordering::Release);
        Ok(())
    })) else {
        return;
    };
    let mut paused = Vec::new();
    let deadline = Instant::now() + BUDGET;
    for session in sessions {
        if !active() || Instant::now() >= deadline || changed.load(Ordering::Acquire) {
            break;
        }
        if !playing(&session, Playback::Playing) || !controls(&session, true) {
            continue;
        }
        let Some(mut target) = watch(session) else {
            continue;
        };
        if !active() || !playing(&target.session, Playback::Playing) {
            break;
        }
        if let Ok(mut claim) = target.claim.lock() {
            if claim.touched {
                continue;
            }
            claim.issued = true;
        } else {
            continue;
        }
        if !active() || changed.load(Ordering::Acquire) || Instant::now() >= deadline {
            break;
        }
        let Ok(request) = target.session.TryPauseAsync() else {
            continue;
        };
        let accepted = matches!(
            tokio::time::timeout_at(deadline.into(), request).await,
            Ok(Ok(true))
        );
        while accepted
            && Instant::now() < deadline
            && (!playing(&target.session, Playback::Paused)
                || target
                    .claim
                    .lock()
                    .is_ok_and(|claim| !claim.pause_event && !claim.touched))
        {
            tokio::time::sleep(TICK).await;
        }
        if accepted && playing(&target.session, Playback::Paused) {
            target.timeline = position(&target.session);
            if let Ok(mut claim) = target.claim.lock() {
                claim.confirmed = true;
            }
            paused.push(target);
        }
    }
    while active() {
        tokio::time::sleep(TICK).await;
    }
    let deadline = Instant::now() + BUDGET;
    for target in &paused {
        if Instant::now() >= deadline || changed.load(Ordering::Acquire) {
            break;
        }
        let present = manager
            .GetSessions()
            .is_ok_and(|sessions| sessions.into_iter().any(|session| session == target.session));
        let untouched = target.claim.lock().is_ok_and(|claim| claim.restore());
        if !present
            || !untouched
            || !playing(&target.session, Playback::Paused)
            || !controls(&target.session, false)
            || target.timeline != position(&target.session)
        {
            continue;
        }
        // Never retry an ambiguous write; a late completion cannot authorize another play.
        if let Ok(request) = target.session.TryPlayAsync() {
            let _ = tokio::time::timeout_at(deadline.into(), request).await;
        }
    }
    drop(paused);
    let _ = manager.RemoveSessionsChanged(token);
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod native_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_confirmed_uninterrupted_pause_is_restored() {
        let mut claim = Claim::default();
        assert!(!claim.restore());
        claim.issued = true;
        claim.playback(true);
        assert!(!claim.restore());
        claim.confirmed = true;
        assert!(claim.restore());
        claim.playback(true);
        assert!(
            !claim.restore(),
            "a later play/pause pair must not inherit our write"
        );
    }
    #[test]
    fn changes_before_pause_or_manual_play_relinquish_ownership() {
        for before in [true, false] {
            let mut claim = Claim::default();
            if before {
                claim.playback(true);
            }
            claim.issued = true;
            claim.playback(true);
            claim.confirmed = true;
            if !before {
                claim.playback(false);
            }
            assert!(!claim.restore());
        }
    }
    #[test]
    fn unsupported_lower_and_leave_never_start_a_worker() {
        for mode in [Mode::Lower, Mode::Leave] {
            let guard = Listening::start_guarded(mode, || panic!("inactive mode"));
            assert!(guard.0.is_none());
        }
    }
}
