//! Nest-owned manual turns. The renderer receives status and review text, never PCM or authority.
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
#[cfg(target_os = "linux")]
use {
    super::*,
    std::time::{Duration, Instant},
    tokio::sync::{mpsc, watch},
    vults_voice::turn::Effect,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    id: String,
    phase: &'static str,
    reason: Option<&'static str>,
    draft: Option<String>,
    reply: Option<String>,
    response: Option<String>,
}
struct Session {
    view: Snapshot,
    #[cfg(target_os = "linux")]
    control: Option<CaptureControl>,
    #[cfg(target_os = "linux")]
    touched: Instant,
    #[cfg(target_os = "linux")]
    next: Option<mpsc::Sender<capture::Request>>,
    #[cfg(target_os = "linux")]
    generation: Option<vults_voice::turn::Generation>,
}
#[derive(Default)]
pub(crate) struct Controls(Mutex<(u64, Option<Session>)>);

fn current<'a>(slot: &'a mut Option<Session>, id: &str) -> Option<&'a mut Session> {
    if !slot.as_ref().is_some_and(|session| session.view.id == id) {
        return None;
    }
    #[cfg(target_os = "linux")]
    if slot
        .as_ref()
        .is_some_and(|session| session.touched.elapsed() >= Duration::from_secs(3))
    {
        *slot = None;
        return None;
    }
    slot.as_mut()
}

fn page<R: tauri::Runtime>(window: &tauri::Webview<R>) -> Result<(), String> {
    if window.label() == crate::NEST {
        Ok(())
    } else {
        Err("Only the Nest can control voice conversation.".into())
    }
}
pub(crate) fn depart<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<Controls>()
        && let Ok(mut slot) = state.0.lock()
    {
        // Dropping the sole controller revokes capture, response and recovery together.
        slot.1 = None;
    }
}
#[cfg(target_os = "linux")]
fn visible(app: &AppHandle) -> bool {
    crate::question_visible(app, crate::NEST)
        && crate::NEST_TAB
            .lock()
            .is_ok_and(|tab| tab.as_deref() == Some("dictation"))
}

#[tauri::command]
pub fn voice_conversation_start(
    app: AppHandle,
    window: tauri::Webview,
    auto_finish: Option<bool>,
) -> Result<Snapshot, String> {
    page(&window)?;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, auto_finish);
        Err("Voice conversation is available on Linux only.".into())
    }
    #[cfg(target_os = "linux")]
    {
        if !visible(&app) || crate::lock::locked(&app) {
            return Err("Open the voice conversation page before recording.".into());
        }
        let settings = app.state::<crate::settings::SettingsState>();
        let saved = settings.0.lock().map_err(|_| "Voice settings are busy.")?;
        if !saved.zeca || !saved.voice_realtime.enabled() {
            return Err("Enable voice conversation in Dictation settings first.".into());
        }
        drop(saved);
        let controls = app.state::<Controls>();
        let mut slot = controls.0.lock().map_err(|_| "Voice conversation is busy.")?;
        if slot.1.as_ref().is_some_and(|session| session.control.is_some())
            || app.state::<Arc<State>>().running.load(Ordering::SeqCst)
        {
            return Err("A voice conversation is already running.".into());
        }
        slot.0 = slot
            .0
            .checked_add(1)
            .ok_or("Restart the app before recording again.")?;
        let id = slot.0.to_string();
        let (control, mut request) = capture_control();
        request.1 = auto_finish.unwrap_or(false);
        let (next, turns) = mpsc::channel(1);
        let view = Snapshot {
            id: id.clone(),
            phase: "starting",
            reason: None,
            draft: None,
            reply: None,
            response: None,
        };
        slot.1 = Some(Session {
            view: view.clone(),
            control: Some(control),
            touched: Instant::now(),
            next: Some(next),
            generation: None,
        });
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            drive(handle, id, request, turns).await;
        });
        Ok(view)
    }
}

#[tauri::command]
pub fn voice_conversation_status(
    app: AppHandle,
    window: tauri::Webview,
    id: String,
) -> Result<Snapshot, String> {
    page(&window)?;
    #[cfg(target_os = "linux")]
    if !visible(&app) || crate::lock::locked(&app) {
        depart(&app);
        return Err("Voice conversation is no longer visible.".into());
    }
    let controls = app.state::<Controls>();
    let mut slot = controls.0.lock().map_err(|_| "Voice conversation is busy.")?;
    let session = current(&mut slot.1, &id).ok_or("Voice conversation has ended.")?;
    #[cfg(target_os = "linux")]
    {
        session.touched = Instant::now();
    }
    Ok(session.view.clone())
}

#[tauri::command]
pub fn voice_conversation_control(
    app: AppHandle,
    window: tauri::Webview,
    id: String,
    action: String,
    response: Option<String>,
) -> Result<(), String> {
    page(&window)?;
    if !matches!(action.as_str(), "finish" | "cancel" | "next" | "interrupt") {
        return Err("Invalid voice conversation action.".into());
    }
    let controls = app.state::<Controls>();
    let mut slot = controls.0.lock().map_err(|_| "Voice conversation is busy.")?;
    let Some(session) = current(&mut slot.1, &id) else {
        return Ok(());
    };
    #[cfg(target_os = "linux")]
    if action != "cancel" {
        if !visible(&app) || crate::lock::locked(&app) {
            slot.1 = None;
            return Err("Voice conversation is no longer visible.".into());
        }
        if action == "interrupt" {
            interrupt(session, response.as_deref())?;
        } else if action == "next" {
            advance(session)?;
        } else if session.view.phase == "recording"
            && let Some(control) = &session.control
        {
            control.finish();
        }
    } else {
        slot.1 = None;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (session, response);
        slot.1 = None;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn capture_phase(session: &mut Session, phase: CapturePhase) {
    session.view.phase = match phase {
        CapturePhase::Starting => {
            session.view.reason = None;
            "starting"
        }
        CapturePhase::Recording => "recording",
        CapturePhase::Transcribing => {
            if let Some(control) = &session.control {
                control.finish();
            }
            "transcribing"
        }
        CapturePhase::Manual => {
            session.view.reason = Some("auto-unavailable");
            "recording"
        }
    };
}

#[cfg(target_os = "linux")]
fn interrupt(session: &mut Session, response: Option<&str>) -> Result<(), String> {
    if session.view.phase != "speaking" || response.is_none() || response != session.view.response.as_deref()
    {
        return Ok(());
    }
    let generation = session.generation.ok_or("Voice conversation is not ready.")?;
    session
        .next
        .as_ref()
        .ok_or("Voice conversation has ended.")?
        .try_send(capture::Request::Interrupt(generation))
        .map_err(|_| "Voice conversation is busy.")?;
    session.view.phase = "interrupting";
    Ok(())
}

#[cfg(target_os = "linux")]
fn advance(session: &mut Session) -> Result<(), String> {
    if session.view.phase != "waiting" {
        return Err("Wait for Zeca before recording another turn.".into());
    }
    let generation = session
        .generation
        .take()
        .ok_or("Voice conversation is not ready.")?;
    let control = session.control.as_ref().ok_or("Voice conversation has ended.")?;
    if !control.restart() {
        return Err("Voice recording is not ready.".into());
    }
    if session
        .next
        .as_ref()
        .is_none_or(|send| send.try_send(capture::Request::Capture(generation)).is_err())
    {
        control.cancel();
        return Err("Voice conversation has ended.".into());
    }
    session.view.phase = "starting";
    Ok(())
}

#[cfg(target_os = "linux")]
fn update<R: tauri::Runtime>(app: &tauri::AppHandle<R>, id: &str, change: impl FnOnce(&mut Session)) -> bool {
    let controls = app.state::<Controls>();
    let Ok(mut slot) = controls.0.lock() else {
        return false;
    };
    let Some(session) = current(&mut slot.1, id) else {
        return false;
    };
    change(session);
    true
}
#[cfg(target_os = "linux")]
fn reason(failure: vults_chat::realtime::transport::Failure) -> &'static str {
    use vults_chat::realtime::transport::Failure::*;
    match failure {
        Off => "off",
        Budget => "budget",
        Key => "key",
        Input => "input",
        Output => "output",
        Timeout => "timeout",
        Cancelled => "cancelled",
        Busy => "busy",
        Ledger => "ledger",
        Transport | Protocol | Response => "provider",
    }
}
#[cfg(target_os = "linux")]
fn bounded(text: String) -> String {
    text.chars().take(4096).collect()
}

#[cfg(target_os = "linux")]
async fn drive(app: AppHandle, id: String, request: CaptureRequest, next: mpsc::Receiver<capture::Request>) {
    let (send, input) = mpsc::channel(1);
    // Subsequent native turns enter through the retained, generation-bound capture adapter.
    drop(send);
    let (output, mut events) = mpsc::channel(32);
    let (stop, cancel) = watch::channel(false);
    let run = run_conversation(
        app.clone(),
        Channels {
            input,
            output,
            cancel,
        },
        request,
        next,
    );
    tokio::pin!(run);
    let mut poll = tokio::time::interval(Duration::from_millis(100));
    let mut receiving = true;
    let mut first_listen = true;
    let result = loop {
        tokio::select! { biased;
            _ = poll.tick() => {
                let shown = visible(&app);
                if !shown { depart(&app); }
                if !shown || !update(&app, &id, |_| {}) {
                    // Retain the future so native/provider teardown follows its existing path.
                    stop.send_replace(true);
                }
            },
            event = events.recv(), if receiving => {
                if let Some(event) = event {
                    update(&app, &id, |session| match event {
                        Event::Capture(phase) => capture_phase(session, phase),
                        Event::Action(action) if matches!(action.effect, Effect::Listen { .. }) => {
                            if first_listen {
                                first_listen = false;
                                if session.next.as_ref().is_none_or(|next| next.try_send(capture::Request::Capture(action.generation)).is_err())
                                    && let Some(control) = &session.control { control.cancel(); }
                            }
                            else { session.generation = Some(action.generation); if session.view.phase != "interrupting" { session.view.phase = "waiting"; } }
                        },
                        Event::Action(action) if matches!(action.effect, Effect::Generate) => session.view.phase = "responding",
                        Event::Speaking(generation) => { session.generation = Some(generation); session.view.response = Some(crate::traces::new_id()); session.view.phase = "speaking"; },
                        Event::History(record) => {
                            session.view.reply = Some(bounded(record.text));
                            if record.interrupted && session.view.phase == "interrupting" { session.view.phase = "waiting"; }
                            else if !record.interrupted { session.view.phase = "responding"; }
                        },
                        Event::Local { reply, .. } => session.view.reply = Some(bounded(reply.text)),
                        _ => {},
                    });
                } else { receiving = false; }
            },
            result = &mut run => break result,
        }
    };
    // Only bounded heard/local reply text and reviewable recovery cross this boundary.
    update(&app, &id, |session| {
        session.control = None;
        session.next = None;
        session.generation = None;
        match result {
            Ok(None) => session.view.phase = "finished",
            Ok(Some(Recovery::Text {
                reason: failure,
                text,
            })) => {
                session.view.phase = "recovery";
                session.view.reason = Some(reason(failure));
                session.view.draft = Some(text);
            }
            Ok(Some(Recovery::Audio { reason: failure, .. })) | Err(failure) => {
                session.view.phase = if failure == vults_chat::realtime::transport::Failure::Cancelled {
                    "cancelled"
                } else {
                    "failed"
                };
                session.view.reason = Some(reason(failure));
            }
        }
    });
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn next_turn_requires_waiting_generation_and_a_live_single_controller() {
        let (control, request) = capture_control();
        let active = request.active();
        let mut machine = vults_voice::turn::Turn::new(vults_voice::turn::BargeIn::Disabled);
        let generation = machine.start(Duration::ZERO).unwrap().generation;
        let (next, mut receive) = mpsc::channel(1);
        let mut session = Session {
            view: Snapshot {
                id: "1".into(),
                phase: "recording",
                reason: None,
                draft: None,
                reply: Some("Heard reply".into()),
                response: None,
            },
            control: Some(control),
            touched: Instant::now(),
            next: Some(next),
            generation: Some(generation),
        };
        assert!(advance(&mut session).is_err());
        assert!(receive.try_recv().is_err());
        interrupt(&mut session, Some("reply-1")).unwrap();
        assert!(
            receive.try_recv().is_err(),
            "An inaudible/stale click cannot enqueue interruption"
        );
        session.view.phase = "speaking";
        session.view.response = Some("reply-1".into());
        interrupt(&mut session, Some("old-reply")).unwrap();
        assert!(
            receive.try_recv().is_err(),
            "A stale reply cannot interrupt current output"
        );
        interrupt(&mut session, Some("reply-1")).unwrap();
        assert!(
            matches!(receive.try_recv().unwrap(), capture::Request::Interrupt(value) if value == generation)
        );
        assert_eq!(session.view.phase, "interrupting");
        interrupt(&mut session, Some("reply-1")).unwrap();
        assert!(
            receive.try_recv().is_err(),
            "A duplicate cannot interrupt twice or start capture"
        );
        capture_phase(&mut session, CapturePhase::Manual);
        assert_eq!(session.view.reason, Some("auto-unavailable"));
        capture_phase(&mut session, CapturePhase::Starting);
        assert_eq!(session.view.reason, None);
        capture_phase(&mut session, CapturePhase::Transcribing);
        assert_eq!(session.view.phase, "transcribing");
        session.view.phase = "waiting";
        assert!(advance(&mut session).is_ok());
        assert!(
            matches!(receive.try_recv().unwrap(), capture::Request::Capture(value) if value == generation)
        );
        assert_eq!(session.view.phase, "starting");
        assert_eq!(session.view.reply.as_deref(), Some("Heard reply"));
        assert!(advance(&mut session).is_err(), "A duplicate cannot admit capture");
        session.control.as_ref().unwrap().finish();
        session.view.phase = "waiting";
        session.generation = Some(generation);
        drop(receive);
        assert!(advance(&mut session).is_err());
        assert!(!active(), "A disconnected driver revokes the whole owner");
    }

    #[test]
    fn only_nest_can_control_voice_and_departure_discards_the_controller() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        app.manage(Controls::default());
        for label in [crate::NEST, crate::SETTINGS, crate::ISLAND, "widget"] {
            let view = tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::default())
                .build()
                .unwrap();
            assert_eq!(page(view.as_ref()).is_ok(), label == crate::NEST);
        }
        let (control, request) = capture_control();
        let active = request.active();
        let controls = app.state::<Controls>();
        controls.0.lock().unwrap().1 = Some(Session {
            view: Snapshot {
                id: "1".into(),
                phase: "recording",
                reason: None,
                draft: None,
                reply: None,
                response: None,
            },
            control: Some(control),
            touched: Instant::now(),
            next: None,
            generation: None,
        });
        assert!(active());
        depart(app.handle());
        assert!(!active());
        assert!(controls.0.lock().unwrap().1.is_none());
    }

    #[test]
    fn stale_identity_and_expired_heartbeat_cannot_publish_or_keep_recording() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        app.manage(Controls::default());
        let (control, request) = capture_control();
        let active = request.active();
        let controls = app.state::<Controls>();
        controls.0.lock().unwrap().1 = Some(Session {
            view: Snapshot {
                id: "2".into(),
                phase: "recording",
                reason: None,
                draft: None,
                reply: None,
                response: None,
            },
            control: Some(control),
            touched: Instant::now(),
            next: None,
            generation: None,
        });
        assert!(!update(app.handle(), "1", |_| panic!(
            "Stale work cannot publish"
        )));
        assert!(active());
        assert!(update(app.handle(), "2", |s| s.view.phase = "transcribing"));
        controls.0.lock().unwrap().1.as_mut().unwrap().touched = Instant::now() - Duration::from_secs(4);
        assert!(
            current(&mut controls.0.lock().unwrap().1, "2").is_none(),
            "Status/control cannot renew an already expired controller"
        );
        assert!(!update(app.handle(), "2", |_| panic!(
            "Expired work cannot publish"
        )));
        assert!(!active());
        assert!(controls.0.lock().unwrap().1.is_none());
    }
}
