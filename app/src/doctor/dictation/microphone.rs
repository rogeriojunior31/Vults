use serde::Serialize;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use vults_audio::{Capture, CaptureOptions, SAMPLE_RATE};

const SECONDS: u32 = 6;
static NEXT: AtomicU64 = AtomicU64::new(0);
static MIC: Mutex<Option<Arc<Test>>> = Mutex::new(None);

#[derive(Debug, Clone, Default, Serialize)]
pub struct Snapshot {
    id: u64,
    phase: &'static str,
    elapsed_ms: u64,
    bands: [f32; 9],
    noise_db: Option<f32>,
    verdict: Option<&'static str>,
}
struct Test {
    stop: AtomicBool,
    running: AtomicBool,
    touched: Mutex<Instant>,
    snapshot: Mutex<Snapshot>,
}
impl Test {
    fn expired(&self, now: Instant) -> bool {
        self.stop.load(Ordering::SeqCst)
            || self.touched.lock().map_or(true, |at| {
                now.saturating_duration_since(*at) > Duration::from_secs(2)
            })
    }
    fn read(&self, id: u64, now: Instant) -> Result<Snapshot, String> {
        let snapshot = self
            .snapshot
            .lock()
            .map_err(|_| "Microphone test is unavailable.")?
            .clone();
        if snapshot.id != id {
            return Err("Microphone test has ended.".into());
        }
        if snapshot.phase == "running" && self.expired(now) {
            self.cancel();
        }
        if let Ok(mut at) = self.touched.lock() {
            *at = now;
        }
        self.snapshot
            .lock()
            .map(|s| s.clone())
            .map_err(|_| "Microphone test is unavailable.".into())
    }
    fn cancel(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut s) = self.snapshot.lock() {
            s.phase = "stopped";
            s.bands = [0.0; 9];
        }
    }
}

pub fn stop_mic<R: tauri::Runtime>(_app: &AppHandle<R>) {
    if let Ok(slot) = MIC.lock()
        && let Some(test) = slot.as_ref()
    {
        test.cancel();
    }
}

#[tauri::command]
pub fn dictation_mic_stop(window: tauri::Webview, id: u64) -> Result<(), String> {
    crate::settings_page(&window)?;
    if let Ok(slot) = MIC.lock()
        && let Some(test) = slot.as_ref()
        && test.snapshot.lock().is_ok_and(|s| s.id == id)
    {
        test.cancel();
    }
    Ok(())
}

#[tauri::command]
pub fn dictation_mic_status(window: tauri::Webview, id: u64) -> Result<Snapshot, String> {
    crate::settings_page(&window)?;
    let slot = MIC.lock().map_err(|_| "Microphone test is unavailable.")?;
    let test = slot.as_ref().ok_or("Microphone test is unavailable.")?;
    test.read(id, Instant::now())
}

#[tauri::command]
pub fn dictation_mic_start(app: AppHandle, window: tauri::Webview) -> Result<u64, String> {
    crate::settings_page(&window)?;
    if crate::lock::locked(&app) {
        return Err("Unlock the screen before testing the microphone.".into());
    }
    #[cfg(target_os = "linux")]
    if crate::dictation::active(&app) {
        return Err("Finish dictation before testing the microphone.".into());
    }
    if crate::voice::recording(&app) {
        return Err("Finish Chat recording before testing the microphone.".into());
    }
    let mut options = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .map_err(|_| "Could not read microphone settings.")?
        .dictation_capture_options();
    options.max_seconds = SECONDS;
    let mut slot = MIC.lock().map_err(|_| "Microphone test is unavailable.")?;
    if slot
        .as_ref()
        .is_some_and(|test| test.running.load(Ordering::SeqCst))
    {
        return Err("A microphone test is already running.".into());
    }
    let id = NEXT.fetch_add(1, Ordering::SeqCst) + 1;
    let test = Arc::new(Test {
        stop: AtomicBool::new(false),
        running: AtomicBool::new(true),
        touched: Mutex::new(Instant::now()),
        snapshot: Mutex::new(Snapshot {
            id,
            phase: "running",
            ..Snapshot::default()
        }),
    });
    *slot = Some(test.clone());
    std::thread::Builder::new()
        .name("dictation-mic-test".into())
        .spawn(move || run(test.clone(), options))
        .map_err(|_| {
            *slot = None;
            "Microphone test is unavailable."
        })?;
    Ok(id)
}

fn run(test: Arc<Test>, options: CaptureOptions) {
    struct Finished(Arc<Test>);
    impl Drop for Finished {
        fn drop(&mut self) {
            self.0.running.store(false, Ordering::SeqCst);
        }
    }
    let _finished = Finished(test.clone());
    let levels = test.clone();
    let result = Capture::start(options, move |meter| {
        if let Ok(mut s) = levels.snapshot.lock()
            && !levels.stop.load(Ordering::SeqCst)
        {
            s.bands = meter.bands;
        }
    })
    .and_then(|capture| {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(u64::from(SECONDS)) && !test.expired(Instant::now()) {
            if let Ok(mut s) = test.snapshot.lock() {
                s.elapsed_ms = capture.audio().len() as u64 * 1000 / u64::from(SAMPLE_RATE);
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        capture.finish()
    });
    if test.expired(Instant::now()) {
        test.cancel();
        return;
    }
    if let Ok(mut s) = test.snapshot.lock() {
        if test.stop.load(Ordering::SeqCst) {
            return;
        }
        s.bands = [0.0; 9];
        match result {
            Ok(pcm) => {
                let (noise, verdict) = measure(&pcm);
                s.phase = "done";
                s.noise_db = noise;
                s.verdict = Some(verdict);
            }
            Err(_) => {
                s.phase = "error";
                s.verdict = None;
            }
        }
    }
}

// The initial quiet two seconds estimate ambient noise, in dBFS, rather than acoustic dB SPL.
fn measure(pcm: &[f32]) -> (Option<f32>, &'static str) {
    if pcm.len() < SAMPLE_RATE as usize * 3 || pcm.iter().any(|v| !v.is_finite()) {
        return (None, "no-signal");
    }
    let quiet = &pcm[..SAMPLE_RATE as usize * 2];
    let rms = (quiet.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>() / quiet.len() as f64).sqrt();
    let noise = (20.0 * rms.max(1e-6).log10()) as f32;
    let peak = pcm.iter().fold(0.0_f32, |peak, v| peak.max(v.abs()));
    (
        Some(noise),
        if peak < 0.005 {
            "no-signal"
        } else if peak >= 0.99 {
            "clipping"
        } else if noise > -30.0 {
            "noisy"
        } else {
            "ok"
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measurements_use_quiet_prefix_and_never_invent_a_missing_reading() {
        assert_eq!(measure(&[]), (None, "no-signal"));
        let mut pcm = vec![0.001; SAMPLE_RATE as usize * 6];
        pcm[SAMPLE_RATE as usize * 2..].fill(0.3);
        let (noise, verdict) = measure(&pcm);
        assert!((noise.unwrap() + 60.0).abs() < 0.01);
        assert_eq!(verdict, "ok");
        pcm[0] = 1.0;
        assert_eq!(measure(&pcm).1, "clipping");
        pcm.fill(0.1);
        assert_eq!(measure(&pcm).1, "noisy");
        pcm.fill(0.0);
        assert_eq!(measure(&pcm), (Some(-120.0), "no-signal"));
        pcm[0] = f32::NAN;
        assert_eq!(measure(&pcm), (None, "no-signal"));
    }
    #[test]
    fn missing_heartbeat_and_cancellation_end_the_test() {
        let at = Instant::now();
        let test = Test {
            stop: AtomicBool::new(false),
            running: AtomicBool::new(true),
            touched: Mutex::new(at),
            snapshot: Mutex::new(Snapshot::default()),
        };
        assert!(!test.expired(at + Duration::from_secs(1)));
        assert!(test.expired(at + Duration::from_secs(3)));
        test.cancel();
        assert!(test.expired(at));
        assert_eq!(test.snapshot.lock().unwrap().phase, "stopped");
    }
    #[test]
    fn stale_poll_cannot_renew_or_cancel_a_new_test_and_completed_readings_survive() {
        let at = Instant::now();
        let test = Test {
            stop: AtomicBool::new(false),
            running: AtomicBool::new(true),
            touched: Mutex::new(at),
            snapshot: Mutex::new(Snapshot {
                id: 7,
                phase: "running",
                ..Snapshot::default()
            }),
        };
        assert!(test.read(6, at + Duration::from_secs(5)).is_err());
        assert!(!test.stop.load(Ordering::SeqCst));
        assert_eq!(*test.touched.lock().unwrap(), at);
        assert_eq!(
            test.read(7, at + Duration::from_secs(3)).unwrap().phase,
            "stopped"
        );
        test.snapshot.lock().unwrap().phase = "done";
        assert_eq!(test.read(7, at + Duration::from_secs(8)).unwrap().phase, "done");
    }
}
