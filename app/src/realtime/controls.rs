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
    follow_up: bool,
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
    #[cfg(target_os = "linux")]
    origin: Instant,
    #[cfg(target_os = "linux")]
    follow_up: vults_voice::turn::follow_up::FollowUp,
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
    follow_up: Option<bool>,
    route: Option<super::routes::Selection>,
) -> Result<Snapshot, String> {
    page(&window)?;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, auto_finish, route, semantic, follow_up);
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
        let follow_up = follow_up.unwrap_or(false);
        if follow_up && (!auto_finish.unwrap_or(false) || route.is_some()) {
            return Err("Answer without the key requires manual mode with automatic finish.".into());
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
            follow_up: false,
        };
        slot.1 = Some(Session {
            view: view.clone(),
            control: Some(control),
            routed: routed.then_some(commands),
            touched: Instant::now(),
            next: Some(next),
            generation: None,
            origin: Instant::now(),
            follow_up: vults_voice::turn::follow_up::FollowUp::new(follow_up),
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
            session.view.follow_up = false;
            "transcribing"
        }
        CapturePhase::FollowUpExpired => {
            if !session.view.follow_up {
                return;
            }
            if let Some(control) = &session.control {
                control.finish();
            }
            session.view.follow_up = false;
            session.view.capture = None;
            session.view.reason = Some("follow-up-expired");
            "waiting"
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
    if session.generation == Some(action.generation) && matches!(session.view.phase, "starting" | "recording")
    {
        return;
    }
    session.generation = Some(action.generation);
    session.view.capture = None;
    session.view.follow_up = false;
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
        let now = Instant::now();
        if let Some(window) = session
            .follow_up
            .take(action.generation, now.duration_since(session.origin))
            && let Some(left) = window.remaining(now.duration_since(session.origin))
            && let Some(until) = now.checked_add(left)
        {
            let _ = advance_recording(session, Some(until));
        }
    }
}

#[cfg(target_os = "linux")]
fn advance(session: &mut Session) -> Result<(), String> {
    advance_recording(session, None)
}
#[cfg(target_os = "linux")]
fn advance_recording(session: &mut Session, until: Option<Instant>) -> Result<(), String> {
    if session.view.phase != "waiting" {
        return Err("Wait for Zeca before recording another turn.".into());
    }
    if until.is_some() && session.routed.is_some() {
        return Err("Bounded answers require manual microphone mode.".into());
    }
    if until.is_some_and(|at| Instant::now() >= at) {
        return Ok(());
    }
    session.view.follow_up = false;
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
    if session.next.as_ref().is_none_or(|send| {
        send.try_send(match until {
            Some(until) => capture::Request::FollowUp { generation, until },
            None => capture::Request::Capture(generation),
        })
        .is_err()
    }) {
        control.cancel();
        return Err("Voice conversation has ended.".into());
    }
    session.generation = Some(generation);
    session.view.phase = "starting";
    session.view.follow_up = until.is_some();
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
fn event_phase(session: &mut Session, event: Event, first_listen: &mut bool) {
    match event {
        Event::Capture(phase) => capture_phase(session, phase),
        Event::Action(action) if matches!(action.effect, Effect::Listen { .. }) => {
            listen(session, action, *first_listen);
            *first_listen = false;
        }
        Event::Action(action)
            if action.effect == Effect::Generate && session.generation == Some(action.generation) =>
        {
            session.view.reason = None;
            session.view.response = None;
            session.view.phase = "responding";
        }
        Event::Action(action)
            if action.effect == Effect::ClearPlayback
                && session.generation == Some(action.generation)
                && matches!(session.view.phase, "speaking" | "interrupting") =>
        {
            session.view.response = None;
            session.view.reason = Some("interrupted");
            session.view.phase = "interrupting";
        }
        Event::Speaking(generation)
            if session.generation == Some(generation)
                && session.view.reason != Some("interrupted")
                && session.view.phase == "responding" =>
        {
            session.view.response = Some(crate::traces::new_id());
            session.view.phase = "speaking";
        }
        Event::History(record) => {
            if session.generation == Some(record.generation) && session.control.is_some() {
                record.observe_follow_up(&mut session.follow_up, session.origin, Instant::now());
            }
            session.view.reply = Some(bounded(record.text));
            if record.interrupted && session.view.phase == "interrupting" {
                session.view.phase = "waiting";
            } else if !record.interrupted {
                session.view.phase = "responding";
            }
        }
        Event::Local { reply, .. } => session.view.reply = Some(bounded(reply.text)),
        _ => {}
    }
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
                    update(&app, &id, |session| event_phase(session, event, &mut first_listen));
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
        session.view.follow_up = false;
        session.follow_up.cancel();
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
                follow_up: false,
            },
            control: Some(control),
            routed: None,
            touched: Instant::now(),
            next: Some(next),
            generation: Some(generation),
            origin: Instant::now(),
            follow_up: Default::default(),
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
        assert_eq!(
            session.view.reason, None,
            "The click alone is not native confirmation"
        );
        let mut first_listen = false;
        event_phase(
            &mut session,
            Event::Action(vults_voice::turn::Action {
                generation,
                effect: Effect::ClearPlayback,
            }),
            &mut first_listen,
        );
        assert_eq!(session.view.reason, Some("interrupted"));
        assert!(session.view.response.is_none());
        event_phase(&mut session, Event::Speaking(generation), &mut first_listen);
        assert_eq!(
            session.view.phase, "interrupting",
            "Delayed audible feedback cannot revive stopped output"
        );
        event_phase(
            &mut session,
            Event::History(Record {
                generation,
                completed_at: Instant::now(),
                identity: vults_chat::realtime::Identity {
                    attempt: "attempt".into(),
                    response: "reply".into(),
                    item: "item".into(),
                },
                text: "Heard prefix [interrupted]".into(),
                interrupted: true,
                played_frames: 24,
                total_frames: 48,
                server_synchronized: true,
            }),
            &mut first_listen,
        );
        assert_eq!(session.view.phase, "waiting");
        assert_eq!(session.view.reason, Some("interrupted"));
        event_phase(
            &mut session,
            Event::Action(vults_voice::turn::Action {
                generation,
                effect: Effect::ClearPlayback,
            }),
            &mut first_listen,
        );
        assert_eq!(
            session.view.phase, "waiting",
            "Duplicate confirmation cannot restart the stopping state"
        );
        assert!(
            receive.try_recv().is_err(),
            "Feedback cannot reopen the microphone"
        );
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
        assert_eq!(session.view.reply.as_deref(), Some("Heard prefix [interrupted]"));
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
                follow_up: false,
            },
            control: Some(control),
            routed: Some(send),
            touched: Instant::now(),
            next: None,
            generation: None,
            origin: Instant::now(),
            follow_up: Default::default(),
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
        session.view.phase = "speaking";
        session.view.response = Some("audible".into());
        let mut first_listen = false;
        let next = *actions
            .iter()
            .find(|a| matches!(a.effect, Effect::Listen { .. }))
            .unwrap();
        for action in actions {
            event_phase(&mut session, Event::Action(action), &mut first_listen);
        }
        assert_eq!(session.view.phase, "starting");
        assert!(session.view.response.is_none());
        event_phase(&mut session, Event::Speaking(first.generation), &mut first_listen);
        event_phase(
            &mut session,
            Event::Action(vults_voice::turn::Action {
                generation: first.generation,
                effect: Effect::ClearPlayback,
            }),
            &mut first_listen,
        );
        assert_eq!(
            session.view.phase, "starting",
            "Retired output cannot overwrite the replay capture"
        );
        assert!(
            matches!(receive.try_recv().unwrap(), Command::Capture { generation, operation: RoutedCapture::Begin } if generation == next.generation)
        );
        capture_phase(&mut session, CapturePhase::Recording);
        assert_ne!(session.view.capture.as_deref(), Some(recording.as_str()));
        event_phase(&mut session, Event::Speaking(next.generation), &mut first_listen);
        assert_eq!(
            session.view.phase, "recording",
            "Audible feedback requires response generation first"
        );
        event_phase(
            &mut session,
            Event::Action(vults_voice::turn::Action {
                generation: next.generation,
                effect: Effect::Generate,
            }),
            &mut first_listen,
        );
        event_phase(&mut session, Event::Speaking(next.generation), &mut first_listen);
        assert_eq!(session.view.phase, "speaking");
        let response = session.view.response.clone();
        event_phase(&mut session, Event::Speaking(next.generation), &mut first_listen);
        assert_eq!(
            session.view.response, response,
            "Duplicate feedback retains the Stop response token"
        );
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
    fn follow_up_requires_explicit_consent_exact_heard_question_and_expires_back_to_waiting() {
        for enabled in [false, true] {
            let (control, request) = capture_control();
            control.finish();
            let (send, mut requests) = mpsc::channel(2);
            let mut machine = vults_voice::turn::Turn::default();
            let first = machine.start(Duration::ZERO).unwrap().generation;
            let origin = Instant::now() - Duration::from_secs(3);
            let mut session = Session {
                view: Snapshot {
                    id: "follow-up".into(),
                    phase: "speaking",
                    reason: None,
                    draft: None,
                    reply: None,
                    response: Some("reply".into()),
                    capture: None,
                    routed: false,
                    semantic_fallback: false,
                    follow_up: false,
                },
                control: Some(control),
                routed: None,
                touched: Instant::now(),
                next: Some(send),
                generation: Some(first),
                origin,
                follow_up: vults_voice::turn::follow_up::FollowUp::new(enabled),
            };
            let mut first_listen = false;
            event_phase(
                &mut session,
                Event::History(Record {
                    identity: vults_chat::realtime::Identity {
                        attempt: "attempt".into(),
                        response: "response".into(),
                        item: "item".into(),
                    },
                    generation: first,
                    completed_at: Instant::now() - Duration::from_secs(2),
                    text: "Continue?".into(),
                    interrupted: false,
                    played_frames: 4800,
                    total_frames: 4800,
                    server_synchronized: true,
                }),
                &mut first_listen,
            );
            machine.handle(first, vults_voice::turn::Event::LocalCompleted, Duration::ZERO);
            let following = machine.start(Duration::from_secs(1)).unwrap();
            event_phase(&mut session, Event::Action(following), &mut first_listen);
            if !enabled {
                assert!(requests.try_recv().is_err());
                assert_eq!(session.view.phase, "waiting");
                continue;
            }
            let capture::Request::FollowUp { generation, until } = requests.try_recv().unwrap() else {
                panic!("A bounded answer only")
            };
            assert_eq!(generation, following.generation);
            assert!(
                until.duration_since(Instant::now()) < Duration::from_secs(7),
                "Receipt handling cannot restart eight seconds"
            );
            assert!(session.view.follow_up);
            assert_eq!(session.generation, Some(generation));
            event_phase(&mut session, Event::Action(following), &mut first_listen);
            assert!(
                requests.try_recv().is_err(),
                "Duplicate Listen cannot rearm capture"
            );
            capture_phase(&mut session, CapturePhase::Recording);
            let token = session.view.capture.clone().unwrap();
            finish(&mut session, Some(&token)).unwrap();
            assert_eq!(session.view.phase, "transcribing");
            assert!(
                session.view.follow_up,
                "Requested Finish must not hide a racing expiry before native close acknowledges it"
            );
            capture_phase(&mut session, CapturePhase::FollowUpExpired);
            assert_eq!(session.view.phase, "waiting");
            assert_eq!(session.view.reason, Some("follow-up-expired"));
            assert!(!session.view.follow_up && session.view.capture.is_none());
            finish(&mut session, Some(&token)).unwrap();
            assert!(requests.try_recv().is_err());
            advance(&mut session).unwrap();
            assert!(
                matches!(requests.try_recv().unwrap(),capture::Request::Capture(value) if value==generation)
            );
            assert_eq!(
                session.generation,
                Some(generation),
                "Manual continuation retains response authority"
            );
            event_phase(
                &mut session,
                Event::Action(vults_voice::turn::Action {
                    generation,
                    effect: Effect::Generate,
                }),
                &mut first_listen,
            );
            event_phase(&mut session, Event::Speaking(generation), &mut first_listen);
            assert_eq!(session.view.phase, "speaking");
            drop(session);
            assert!(!request.active()());
        }
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
                follow_up: false,
            },
            control: Some(control),
            routed: None,
            touched: Instant::now(),
            next: None,
            generation: None,
            origin: Instant::now(),
            follow_up: Default::default(),
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
                follow_up: false,
            },
            control: Some(control),
            routed: None,
            touched: Instant::now(),
            next: None,
            generation: None,
            origin: Instant::now(),
            follow_up: Default::default(),
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
