//! A physical key grants one bounded, memory-only recording for one question.
use super::{Active, Command};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug)]
pub(crate) struct Press {
    pub token: String,
    pub cancelled: Arc<AtomicBool>,
    released: Arc<AtomicBool>,
}
impl Press {
    pub(super) fn new(token: String) -> Self {
        Self {
            token,
            cancelled: Arc::default(),
            released: Arc::default(),
        }
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    fn live(&self, expires: Instant) -> bool {
        !self.cancelled.load(Ordering::Acquire) && Instant::now() < expires
    }
}
#[derive(Debug, Default)]
pub(super) struct Routing {
    pub token: Option<String>,
    held: bool,
    held_routed: bool,
    pub blocking: bool,
    pub ordinary_stop: Arc<AtomicBool>,
    press: Option<Press>,
    pub current: Option<Press>,
}
impl Routing {
    fn key(&mut self, down: bool) -> (bool, Option<Press>) {
        let routed = self.blocking || self.token.is_some() || self.held_routed;
        if !down {
            self.held = false;
            self.held_routed = false;
            if let Some(press) = &self.press {
                press.released.store(true, Ordering::Release);
            }
            return (routed, None);
        }
        if self.held {
            return (routed, None);
        }
        self.held = true;
        self.held_routed = self.blocking || self.token.is_some();
        self.press = self.token.clone().map(Press::new);
        (self.held_routed, self.press.clone())
    }
}
fn ordinary<T>(route: &std::sync::Mutex<Routing>, admit: impl FnOnce(Arc<AtomicBool>) -> T) -> Option<T> {
    let route = route.lock().ok()?;
    if route.blocking {
        return None;
    }
    Some(admit(route.ordinary_stop.clone()))
}
/// Orders the actual ordinary admission with the question fence, without holding it over IO.
pub(crate) fn admit_ordinary<T>(app: &AppHandle, admit: impl FnOnce(Arc<AtomicBool>) -> T) -> Option<T> {
    if let Some(state) = app.try_state::<Active>() {
        ordinary(&state.route, admit)
    } else {
        Some(admit(Arc::default()))
    }
}

/// Returns true when this gesture belongs to a question, including its eventual key-up.
pub(crate) fn key(app: &AppHandle, id: &str, down: bool) -> bool {
    let Some(state) = app.try_state::<Active>() else {
        return false;
    };
    let Ok(mut route) = state.route.lock() else {
        return true;
    };
    if id == "talk" {
        let (routed, press) = route.key(down);
        drop(route);
        if let Some(press) = press {
            crate::doctor::dictation::stop_mic(app);
            crate::runtime::question_try(app, Command::VoiceStart(press));
        }
        return routed;
    }
    let active = route.blocking || route.token.is_some();
    drop(route);
    if id == "talk-cancel" && down {
        cancel(app);
    }
    active && matches!(id, "talk-again" | "talk-command")
}
pub(crate) fn invalidate<R: tauri::Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<Active>()
        && let Ok(route) = state.route.lock()
    {
        if let Some(press) = &route.press {
            press.cancel();
        }
        if let Some(press) = &route.current {
            press.cancel();
        }
    }
}

pub(crate) fn cancel(app: &AppHandle) {
    let Some(state) = app.try_state::<Active>() else {
        return;
    };
    let Ok(route) = state.route.lock() else { return };
    if let Some(press) = &route.press {
        press.cancel();
    }
    if let Some(press) = &route.current {
        press.cancel();
    }
    if let Some(token) = &route.token {
        crate::runtime::question_try(app, Command::VoiceStop(token.clone()));
    }
}

// One native load can outlive cancellation; do not queue another microphone behind it.
static BUSY: AtomicBool = AtomicBool::new(false);
struct Busy;
impl Drop for Busy {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}

pub(super) fn start(app: AppHandle, press: Press, id: u64, expires: Instant) {
    if BUSY.swap(true, Ordering::AcqRel) {
        report(
            &app,
            &press,
            id,
            Some(Err("The previous recording is still stopping.".into())),
        );
        return;
    }
    let busy = Busy;
    let model = crate::settings::voice_model(&app);
    let language = crate::voice::language(&app);
    let options = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .ok()
        .map(|s| {
            let mut options = s.dictation_capture_options();
            options.max_seconds = options.max_seconds.min(60);
            options
        });
    tauri::async_runtime::spawn_blocking(move || {
        let _busy = busy;
        let result = (|| {
            if !press.live(expires) || press.released.load(Ordering::Acquire) {
                return Ok(String::new());
            }
            let model = model.ok_or("Choose and install a local voice model first.")?;
            if !vults_voice::installed(&crate::voice::models_dir()).contains(&model.as_str()) {
                return Err("Choose and install a local voice model first.".into());
            }
            let guard = press.clone();
            let admission = press.clone();
            let options = options.ok_or("Settings are busy.")?;
            let stop_at = Instant::now() + Duration::from_secs(u64::from(options.max_seconds));
            let mic = vults_audio::Capture::start_guarded(
                options,
                |_| {},
                move || guard.live(expires),
                move || !admission.released.load(Ordering::Acquire),
            )?;
            while press.live(expires) && !press.released.load(Ordering::Acquire) && Instant::now() < stop_at {
                std::thread::sleep(Duration::from_millis(20));
            }
            let pcm = mic.finish()?;
            if !press.live(expires) {
                return Ok(String::new());
            }
            report(&app, &press, id, None);
            let transcriber = crate::voice::transcriber(&app, &model)?;
            let vad = vults_voice::vad_path(&crate::voice::models_dir());
            Ok(transcriber
                .transcribe_cancellable(&pcm, language.as_deref(), vad.as_deref(), &press.cancelled, true)?
                .unwrap_or_default())
        })();
        report(&app, &press, id, Some(result));
    });
}
fn report(app: &AppHandle, press: &Press, id: u64, result: Option<Result<String, String>>) {
    let app = app.clone();
    let token = press.token.clone();
    tauri::async_runtime::spawn(async move {
        let _ = crate::runtime::question(&app, Command::VoiceResult { token, id, result }).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_existing_hold_never_grants_a_question_and_release_keeps_its_owner() {
        let mut route = Routing::default();
        assert!(!route.key(true).0);
        route.token = Some("first".into());
        assert!(route.key(true).1.is_none());
        route.key(false);
        let first = route.key(true).1.unwrap();
        assert_eq!(first.token, "first");
        assert!(route.key(true).1.is_none());
        route.token = Some("replacement".into());
        route.key(false);
        assert!(first.released.load(Ordering::Acquire));
        assert!(!first.cancelled.load(Ordering::Acquire));
        let second = route.key(true).1.unwrap();
        first.cancel();
        assert!(second.live(Instant::now() + Duration::from_secs(1)));
        route.key(false);
        route.token = None;
        route.blocking = true;
        assert!(route.key(true).0);
        assert!(route.key(true).1.is_none());
        route.blocking = false;
        assert!(route.key(false).0, "A Sending-owned key-up cannot fall through");
        assert!(
            !route.key(true).0,
            "A fresh ordinary press resumes after the question"
        );
    }
    #[test]
    fn the_final_ordinary_admission_observes_a_question_after_an_earlier_preflight() {
        let route = Arc::new(std::sync::Mutex::new(Routing::default()));
        let old = ordinary(&route, |stop| stop).unwrap();
        assert!(!route.lock().unwrap().key(true).0);
        {
            let mut route = route.lock().unwrap();
            route.blocking = true;
            route.ordinary_stop.store(true, Ordering::Release);
        }
        assert!(ordinary(&route, |_| panic!("ordinary admission after question fence")).is_none());
        assert!(old.load(Ordering::Acquire));
        {
            let mut route = route.lock().unwrap();
            route.blocking = false;
            route.ordinary_stop = Arc::default();
        }
        assert!(
            old.load(Ordering::Acquire),
            "A closed question never revives old microphone grants"
        );
        assert!(!ordinary(&route, |stop| stop).unwrap().load(Ordering::Acquire));
    }
}
