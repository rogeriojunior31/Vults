//! Nest-owned turns and explicitly confirmed routed capture; only status/review text reaches the UI.
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
    capture: Option<String>,
    routed: bool,
    semantic_fallback: bool,
}
struct Session {
    view: Snapshot,
    #[cfg(target_os = "linux")]
    control: Option<CaptureControl>,
    #[cfg(target_os = "linux")]
    routed: Option<mpsc::Sender<Command>>,
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

pub(super) fn page<R: tauri::Runtime>(window: &tauri::Webview<R>) -> Result<(), String> {
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
pub(super) fn visible(app: &AppHandle) -> bool {
    crate::question_visible(app, crate::NEST)
        && crate::NEST_TAB
            .lock()
            .is_ok_and(|tab| tab.as_deref() == Some("dictation"))
}

#[derive(Serialize)]
pub struct Options {
    semantic: bool,
}
#[tauri::command]
pub fn voice_conversation_options(window: tauri::Webview) -> Result<Options, String> {
    page(&window)?;
    Ok(Options {
        semantic: cfg!(all(
            feature = "voice-semantic",
            target_os = "linux",
            target_arch = "x86_64",
            target_env = "gnu"
        )),
    })
}

#[tauri::command]
pub fn voice_conversation_start(
    app: AppHandle,
    window: tauri::Webview,
    auto_finish: Option<bool>,
    semantic: Option<bool>,
    route: Option<super::routes::Selection>,
) -> Result<Snapshot, String> {
    page(&window)?;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, auto_finish, route, semantic);
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
        let route = super::routes::resolve(&app, route)?;
        if route.is_some() && auto_finish.unwrap_or(false) {
            return Err("Use explicit Finish with a continuous microphone route.".into());
        }
        let semantic = semantic.unwrap_or(false);
        if semantic && (!auto_finish.unwrap_or(false) || route.is_some() || !capture::semantic::supported()) {
            return Err(
                "Semantic finish requires the preview build and manual automatic-finish mode.".into(),
            );
        }
        let routed = route.is_some();
        let (commands, input) = mpsc::channel(4);
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
        request.2 = semantic;
        let (next, turns) = mpsc::channel(1);
        let view = Snapshot {
            id: id.clone(),
            phase: if semantic { "preparing" } else { "starting" },
            reason: None,
            draft: None,
            reply: None,
            response: None,
            capture: None,
            routed,
            semantic_fallback: false,
        };
        slot.1 = Some(Session {
            view: view.clone(),
            control: Some(control),
            routed: routed.then_some(commands),
            touched: Instant::now(),
            next: Some(next),
            generation: None,
        });
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            drive(handle, id, request, turns, route, input).await;
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
    capture: Option<String>,
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
        } else {
            finish(session, capture.as_deref())?;
        }
    } else {
        slot.1 = None;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (session, response, capture);
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
        CapturePhase::Recording => {
            if session.view.phase != "recording" {
                session.view.capture = Some(crate::traces::new_id());
            }
            "recording"
        }
        CapturePhase::Transcribing => {
            if session.routed.is_none()
                && let Some(control) = &session.control
            {
                control.finish();
            }
            session.view.capture = None;
            "transcribing"
        }
        CapturePhase::SemanticFallback => {
            session.view.semantic_fallback = true;
            return;
        }
        CapturePhase::Manual => {
            if session.view.phase != "recording" {
                return;
            }
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
    if let Some(send) = &session.routed {
        send.try_send(Command::Signal {
            generation,
            signal: Signal::StopResponse,
        })
        .map_err(|_| "Voice conversation is busy.")?;
    } else {
        session
            .next
            .as_ref()
            .ok_or("Voice conversation has ended.")?
            .try_send(capture::Request::Interrupt(generation))
            .map_err(|_| "Voice conversation is busy.")?;
    }
    session.view.phase = "interrupting";
    Ok(())
}

#[cfg(target_os = "linux")]
fn finish(session: &mut Session, capture: Option<&str>) -> Result<(), String> {
    if session.view.phase != "recording" || capture.is_none() || capture != session.view.capture.as_deref() {
        return Ok(());
    }
    if let Some(send) = &session.routed {
        send.try_send(Command::Capture {
            generation: session.generation.ok_or("Voice recording is not ready.")?,
            operation: RoutedCapture::Finish,
        })
        .map_err(|_| "Voice conversation is busy.")?;
    } else if let Some(control) = &session.control {
        control.finish();
    }
    session.view.capture = None;
    session.view.phase = "transcribing";
    Ok(())
}
#[cfg(target_os = "linux")]
fn listen(session: &mut Session, action: vults_voice::turn::Action, first: bool) {
    session.generation = Some(action.generation);
    session.view.capture = None;
    if let Some(send) = &session.routed {
        if first || matches!(action.effect, Effect::Listen { replay } if !replay.is_zero()) {
            if send
                .try_send(Command::Capture {
                    generation: action.generation,
                    operation: RoutedCapture::Begin,
                })
                .is_err()
            {
                if let Some(control) = &session.control {
                    control.cancel();
                }
            } else {
                session.view.phase = "starting";
            }
        } else if session.view.phase != "interrupting" {
            session.view.phase = "waiting";
        }
    } else if first {
        if session.next.as_ref().is_none_or(|next| {
            next.try_send(capture::Request::Capture(action.generation))
                .is_err()
        }) && let Some(control) = &session.control
        {
            control.cancel();
        }
    } else if session.view.phase != "interrupting" {
        session.view.phase = "waiting";
    }
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
    if let Some(send) = &session.routed {
        send.try_send(Command::Capture {
            generation,
            operation: RoutedCapture::Begin,
        })
        .map_err(|_| "Voice conversation is busy.")?;
        session.generation = Some(generation);
        session.view.capture = None;
        session.view.phase = "starting";
        return Ok(());
    }
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
async fn drive(
    app: AppHandle,
    id: String,
    request: CaptureRequest,
    next: mpsc::Receiver<capture::Request>,
    route: Option<vults_audio::echo::duplex::Route>,
    input: mpsc::Receiver<Command>,
) {
    let (output, mut events) = mpsc::channel(32);
    let (stop, cancel) = watch::channel(false);
    let run = async {
        let channels = Channels {
            input,
            output,
            cancel,
        };
        if let Some(route) = route {
            run_routed_conversation(app.clone(), channels, route, request).await
        } else {
            run_conversation(app.clone(), channels, request, next).await
        }
    };
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
                            listen(session, action, first_listen);
                            first_listen = false;
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
        session.routed = None;
        session.view.capture = None;
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
                capture: None,
                routed: false,
                semantic_fallback: false,
            },
            control: Some(control),
            routed: None,
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
        session.view.phase = "recording";
        capture_phase(&mut session, CapturePhase::Manual);
        assert_eq!(session.view.reason, Some("auto-unavailable"));
        capture_phase(&mut session, CapturePhase::Starting);
        assert_eq!(session.view.reason, None);
        capture_phase(&mut session, CapturePhase::SemanticFallback);
        assert!(session.view.semantic_fallback);
        capture_phase(&mut session, CapturePhase::Starting);
        assert!(session.view.semantic_fallback);
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
    fn routed_controls_use_one_stream_exact_recordings_and_only_confirmed_voice_rearms() {
        let (control, request) = capture_control();
        let active = request.active();
        let (send, mut receive) = mpsc::channel(4);
        let mut machine = vults_voice::turn::Turn::new(vults_voice::turn::BargeIn::Headphones);
        let first = machine.start(Duration::ZERO).unwrap();
        let mut session = Session {
            view: Snapshot {
                id: "routed".into(),
                phase: "starting",
                reason: None,
                draft: None,
                reply: None,
                response: None,
                capture: None,
                routed: true,
                semantic_fallback: false,
            },
            control: Some(control),
            routed: Some(send),
            touched: Instant::now(),
            next: None,
            generation: None,
        };
        listen(&mut session, first, true);
        assert!(
            matches!(receive.try_recv().unwrap(), Command::Capture { generation, operation: RoutedCapture::Begin } if generation == first.generation)
        );
        capture_phase(&mut session, CapturePhase::Recording);
        let recording = session.view.capture.clone().unwrap();
        capture_phase(&mut session, CapturePhase::Recording);
        assert_eq!(session.view.capture.as_deref(), Some(recording.as_str()));
        finish(&mut session, None).unwrap();
        finish(&mut session, Some("old")).unwrap();
        assert!(receive.try_recv().is_err());
        finish(&mut session, Some(&recording)).unwrap();
        capture_phase(&mut session, CapturePhase::Manual);
        assert_eq!(
            session.view.phase, "transcribing",
            "Late fallback cannot rearm a finalized recording"
        );
        assert!(session.view.capture.is_none());
        finish(&mut session, Some(&recording)).unwrap();
        assert!(
            matches!(receive.try_recv().unwrap(), Command::Capture { generation, operation: RoutedCapture::Finish } if generation == first.generation)
        );
        assert!(receive.try_recv().is_err());
        machine.handle(
            first.generation,
            vults_voice::turn::Event::InputCommitted,
            Duration::ZERO,
        );
        machine.handle(
            first.generation,
            vults_voice::turn::Event::FirstAudioPlayed { at: Duration::ZERO },
            Duration::ZERO,
        );
        let actions = machine.handle(
            first.generation,
            vults_voice::turn::Event::SpeechStarted,
            Duration::from_secs(1),
        );
        let probe = actions
            .into_iter()
            .find_map(|a| {
                if let Effect::ProbeInterruption { probe, .. } = a.effect {
                    Some(probe)
                } else {
                    None
                }
            })
            .unwrap();
        let actions = machine.handle(
            first.generation,
            vults_voice::turn::Event::InterruptConfirmed(probe),
            Duration::from_millis(1100),
        );
        let next = actions
            .into_iter()
            .find(|a| matches!(a.effect, Effect::Listen { .. }))
            .unwrap();
        listen(&mut session, next, false);
        assert!(
            matches!(receive.try_recv().unwrap(), Command::Capture { generation, operation: RoutedCapture::Begin } if generation == next.generation)
        );
        capture_phase(&mut session, CapturePhase::Recording);
        assert_ne!(session.view.capture.as_deref(), Some(recording.as_str()));
        finish(&mut session, Some(&recording)).unwrap();
        assert!(
            receive.try_recv().is_err(),
            "Late Finish cannot close this recording"
        );
        session.view.phase = "speaking";
        session.view.response = Some("reply-2".into());
        interrupt(&mut session, Some("old-reply")).unwrap();
        assert!(receive.try_recv().is_err());
        interrupt(&mut session, Some("reply-2")).unwrap();
        assert!(
            matches!(receive.try_recv().unwrap(), Command::Signal { generation, signal: Signal::StopResponse } if generation == next.generation)
        );
        listen(
            &mut session,
            vults_voice::turn::Action {
                generation: next.generation,
                effect: Effect::Listen {
                    replay: Duration::ZERO,
                },
            },
            false,
        );
        assert!(receive.try_recv().is_err(), "Manual Stop never admits capture");
        session.view.phase = "waiting";
        advance(&mut session).unwrap();
        assert!(
            matches!(receive.try_recv().unwrap(), Command::Capture { generation, operation: RoutedCapture::Begin } if generation == next.generation)
        );
        assert!(active());
        drop(session);
        assert!(!active());
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
                capture: None,
                routed: false,
                semantic_fallback: false,
            },
            control: Some(control),
            routed: None,
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
                capture: None,
                routed: false,
                semantic_fallback: false,
            },
            control: Some(control),
            routed: None,
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
