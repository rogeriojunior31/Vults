use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

type RawInput = Arc<dyn Fn(vults_platform::raw_keys::Key) + Send + Sync>;

type Hook = (&'static str, Box<dyn FnOnce() + Send>);

#[test]
fn vocabulary_uses_the_recording_choice_dictionary_and_actual_decode_model() {
    let mut config = Config {
        model: "turbo".into(),
        language: Some("en".into()),
        words: vults_dictation::text::Rules {
            words: vec!["Vults".into()],
            replacements: vec![("old".into(), "private replacement".into())],
            snippets: vec![("signature".into(), "private snippet".into())],
        },
        ..Default::default()
    };
    assert!(super::super::vocabulary_words(&config, "turbo").is_empty());
    config.vocabulary = true;
    let recording = config.clone();
    config.vocabulary = false;
    config.words.words = vec!["Next recording".into()];
    assert_eq!(super::super::vocabulary_words(&recording, "turbo"), ["Vults"]);
    for model in ["base", "small", "unknown"] {
        assert!(super::super::vocabulary_words(&recording, model).is_empty());
    }
    for language in [None, Some("pt"), Some("es"), Some("auto")] {
        let config = Config {
            language: language.map(str::to_owned),
            ..recording.clone()
        };
        assert!(super::super::vocabulary_words(&config, "turbo").is_empty());
    }
}

#[test]
fn project_context_freezes_per_recording_and_never_changes_spelling_or_chat_rules() {
    let h = Harness::new();
    let ticket = h.state.key("talk", true).unwrap();
    let mut config = Config {
        vocabulary: true,
        project_vocabulary: true,
        project_words: ticket.project_words(),
        language: Some("en".into()),
        words: vults_dictation::text::Rules {
            words: vec!["Personal".into()],
            ..Default::default()
        },
        ..Default::default()
    };
    ticket.set_project_words(vec!["Project".into()]);
    assert_eq!(
        super::super::vocabulary_words(&config, "turbo"),
        ["Personal", "Project"]
    );
    ticket.set_project_words(vec!["Late".into()]);
    assert_eq!(
        super::super::vocabulary_words(&config, "turbo"),
        ["Personal", "Project"]
    );
    assert_eq!(config.words.words, ["Personal"]);
    assert!(super::super::vocabulary_words(&config, "base").is_empty());
    config.vocabulary = false;
    assert_eq!(super::super::vocabulary_words(&config, "turbo"), ["Project"]);
    config.project_vocabulary = false;
    assert!(super::super::vocabulary_words(&config, "turbo").is_empty());
    h.state.cancel(false);
    h.state.key("talk", false);
    let next = h.state.key("talk", true).unwrap();
    let config = Config {
        project_words: next.project_words(),
        project_vocabulary: true,
        ..config
    };
    ticket.set_project_words(vec!["Stale".into()]);
    assert!(config.project_words.get().is_none());
    // Missing context at the first decode remains missing for final transcription.
    assert!(super::super::vocabulary_words(&config, "turbo").is_empty());
    next.set_project_words(vec!["TooLate".into()]);
    assert!(super::super::vocabulary_words(&config, "turbo").is_empty());
}

struct Hardware {
    pcm: Mutex<Vec<f32>>,
    transcript: Mutex<String>,
    config: Mutex<Config>,
    copies: Mutex<Vec<String>>,
    commands: Mutex<Vec<String>>,
    phases: Mutex<Vec<String>>,
    hook: Mutex<Option<Hook>>,
    failed_start: AtomicBool,
    failed_copy: AtomicBool,
    outcome: AtomicUsize,
    decoded: AtomicUsize,
    dropped: Arc<AtomicUsize>,
    watches: Arc<AtomicUsize>,
    raw: Mutex<Option<RawInput>>,
    failed_keys: AtomicBool,
}

#[derive(Clone)]
struct Fake(Arc<Hardware>);

impl Fake {
    fn hook(&self, stage: &str) {
        let callback = self
            .0
            .hook
            .lock()
            .expect("test operation")
            .take_if(|(at, _)| *at == stage);
        if let Some((_, call)) = callback {
            call();
        }
    }
}

struct Recording(Arc<AtomicUsize>, Config);
impl Drop for Recording {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct Keys {
    active: Arc<AtomicBool>,
    watches: Arc<AtomicUsize>,
}
impl Drop for Keys {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
        self.watches.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Backend for Fake {
    type Recording = Recording;
    type Keys = Keys;
    fn watch(
        &self,
        ticket: &Ticket,
        input: impl Fn(vults_platform::raw_keys::Key) + Send + 'static,
    ) -> Option<Keys> {
        if self.0.failed_keys.load(Ordering::SeqCst) {
            return None;
        }
        let input = Mutex::new(input);
        *self.0.raw.lock().expect("raw input") =
            Some(Arc::new(move |key| (input.lock().expect("input"))(key)));
        let active = Arc::new(AtomicBool::new(true));
        if !ticket.attach_keys(active.clone()) {
            return None;
        }
        self.0.watches.fetch_add(1, Ordering::SeqCst);
        Some(Keys {
            active,
            watches: self.0.watches.clone(),
        })
    }
    fn start(&self, _: &Ticket) -> Result<Recording, String> {
        self.hook("start");
        if self.0.failed_start.load(Ordering::SeqCst) {
            return Err("Microphone unavailable.".into());
        }
        Ok(Recording(
            self.0.dropped.clone(),
            self.0.config.lock().expect("config").clone(),
        ))
    }
    fn finish(&self, recording: Recording) -> Result<Clip, String> {
        self.hook("finish");
        Ok(Clip {
            pcm: self.0.pcm.lock().expect("test operation").clone(),
            config: recording.1.clone(),
        })
    }
    fn transcribe(&self, clip: Clip, ticket: &Ticket) -> Result<Option<String>, String> {
        self.0.decoded.fetch_add(1, Ordering::SeqCst);
        self.hook("decode");
        ticket.prepare_text(&self.0.transcript.lock().expect("transcript"), &clip.config)
    }
    fn copy(&self, text: String, ticket: Ticket) -> Result<Option<vults_platform::insert::Outcome>, String> {
        self.hook("copy");
        if self.0.outcome.load(Ordering::SeqCst) == 2 {
            return Ok(ticket
                .send(&text, || true)
                .map(|_| vults_platform::insert::Outcome::Session));
        }
        ticket
            .commit(&text, || {
                if self.0.failed_copy.load(Ordering::SeqCst) {
                    return Err("Clipboard unavailable.".into());
                }
                self.0.copies.lock().expect("test operation").push(text.clone());
                Ok(())
            })
            .map(|copied| {
                copied.then_some(if self.0.outcome.load(Ordering::SeqCst) == 1 {
                    vults_platform::insert::Outcome::Pasted
                } else if self.0.outcome.load(Ordering::SeqCst) == 3 {
                    vults_platform::insert::Outcome::Held
                } else {
                    vults_platform::insert::Outcome::Copied
                })
            })
    }
    fn show(&self, ticket: &Ticket, phase: &str, _: Option<&str>) {
        if ticket.valid() {
            self.0.phases.lock().expect("test operation").push(phase.into());
        }
    }
    fn command(&self, text: String, ticket: Ticket) -> Result<(), String> {
        self.hook("command");
        ticket.dispatch(|| self.0.commands.lock().expect("Commands").push(text));
        Ok(())
    }
}

struct Harness {
    worker: Worker<Fake>,
    state: State,
    receive: mpsc::Receiver<Command>,
    fake: Fake,
}

impl Harness {
    fn new() -> Self {
        let fake = Fake(Arc::new(Hardware {
            pcm: Mutex::new(vec![0.1; 16_000]),
            transcript: Mutex::new("spoken words".into()),
            config: Mutex::new(Config::default()),
            copies: Mutex::default(),
            commands: Mutex::default(),
            phases: Mutex::default(),
            hook: Mutex::default(),
            failed_start: AtomicBool::new(false),
            failed_copy: AtomicBool::new(false),
            outcome: AtomicUsize::new(0),
            decoded: AtomicUsize::new(0),
            dropped: Arc::default(),
            watches: Arc::default(),
            raw: Mutex::default(),
            failed_keys: AtomicBool::new(false),
        }));
        let control = Arc::new(Mutex::new(Control::default()));
        let (send, receive) = mpsc::channel();
        let origin = Instant::now();
        let worker = Worker::new(fake.clone(), control.clone(), send.clone(), origin);
        let state = State {
            control,
            send,
            origin,
        };
        Self {
            worker,
            state,
            receive,
            fake,
        }
    }
    fn input(&mut self, key: KeyEvent, at: u64) {
        if let Some(command) = event(&self.state.control, key, Duration::from_millis(at)) {
            self.worker.apply(command);
        }
    }
    fn next(&mut self) {
        self.worker.apply(
            self.receive
                .recv_timeout(Duration::from_secs(5))
                .expect("worker result"),
        );
    }
    fn finish(&mut self) {
        self.input(KeyEvent::Press, 0);
        self.input(KeyEvent::Release, 300);
        self.worker.advance(Duration::from_millis(899));
        assert_eq!(
            self.fake.0.decoded.load(Ordering::SeqCst),
            0,
            "capture includes all 600 ms of tail"
        );
        self.worker.advance(Duration::from_millis(900));
    }
    fn cancel_at(&self, stage: &'static str) {
        let state = self.state.clone();
        *self.fake.0.hook.lock().expect("test operation") =
            Some((stage, Box::new(move || state.cancel(false))));
    }
}

#[test]
fn copied_is_confirmed_only_after_the_tail_and_clipboard_commit() {
    let mut h = Harness::new();
    h.finish();
    h.next();
    assert_eq!(*h.fake.0.copies.lock().expect("test operation"), ["spoken words"]);
    assert_eq!(
        h.state.control.lock().expect("test operation").last.as_deref(),
        Some("spoken words")
    );
    assert_eq!(
        h.fake
            .0
            .phases
            .lock()
            .expect("test operation")
            .last()
            .expect("test operation"),
        "copied"
    );
    h.state.key("talk-again", true);
    h.next();
    assert_eq!(h.fake.0.copies.lock().expect("test operation").len(), 2);
    h.state.cancel(true);
    h.state.key("talk-again", true);
    h.next();
    assert_eq!(
        h.fake.0.copies.lock().expect("test operation").len(),
        2,
        "disabled voice forgets the in-memory transcript"
    );
}

#[test]
fn cancellation_at_every_external_boundary_prevents_copying() {
    for stage in ["start", "finish", "decode", "copy"] {
        let mut h = Harness::new();
        h.cancel_at(stage);
        h.finish();
        // Cancellation is enqueued before a decode result or a clipboard commit can return.
        h.next();
        if stage == "decode" || stage == "copy" {
            h.next();
        }
        assert!(
            h.fake.0.copies.lock().expect("test operation").is_empty(),
            "{stage}"
        );
        assert!(
            h.state.control.lock().expect("test operation").last.is_none(),
            "{stage}"
        );
        assert!(
            !h.fake
                .0
                .phases
                .lock()
                .expect("test operation")
                .iter()
                .any(|s| s == "copied"),
            "{stage}"
        );
    }
}

#[test]
fn cancellation_during_the_tail_drops_capture_without_decoding() {
    let mut h = Harness::new();
    h.input(KeyEvent::Press, 0);
    h.input(KeyEvent::Release, 300);
    h.state.cancel(false);
    h.worker.advance(Duration::from_millis(900));
    h.next();
    assert_eq!(h.fake.0.dropped.load(Ordering::SeqCst), 1);
    assert_eq!(h.fake.0.decoded.load(Ordering::SeqCst), 0);
}

#[test]
fn a_new_turn_invalidates_a_blocked_old_transcription() {
    let mut h = Harness::new();
    let (entered, waiting) = mpsc::channel();
    let (resume, released) = mpsc::channel();
    *h.fake.0.hook.lock().expect("test operation") = Some((
        "decode",
        Box::new(move || {
            entered.send(()).expect("test operation");
            released
                .recv_timeout(Duration::from_secs(5))
                .expect("test operation");
        }),
    ));
    h.finish();
    waiting
        .recv_timeout(Duration::from_secs(5))
        .expect("test operation");
    h.input(KeyEvent::Press, 1000);
    resume.send(()).expect("test operation");
    h.next();
    assert!(h.fake.0.copies.lock().expect("test operation").is_empty());
    h.input(KeyEvent::Release, 1300);
    h.worker.advance(Duration::from_millis(1900));
    h.next();
    assert_eq!(h.fake.0.copies.lock().expect("test operation").len(), 1);
}

#[test]
fn silence_short_audio_and_failed_clipboard_do_not_record_success() {
    for pcm in [vec![0.1; 3999], vec![0.0039; 16_000]] {
        let mut h = Harness::new();
        *h.fake.0.pcm.lock().expect("test operation") = pcm;
        h.finish();
        assert_eq!(h.fake.0.decoded.load(Ordering::SeqCst), 0);
        assert!(h.state.control.lock().expect("test operation").last.is_none());
        assert_eq!(
            h.fake
                .0
                .phases
                .lock()
                .expect("test operation")
                .last()
                .expect("test operation"),
            "empty"
        );
    }
    let mut h = Harness::new();
    h.fake.0.failed_copy.store(true, Ordering::SeqCst);
    h.finish();
    h.next();
    assert!(h.state.control.lock().expect("test operation").last.is_none());
    assert_eq!(
        h.fake
            .0
            .phases
            .lock()
            .expect("test operation")
            .last()
            .expect("test operation"),
        "failed"
    );
}

#[test]
fn capture_failure_and_the_ten_minute_cap_leave_no_recording_running() {
    let mut h = Harness::new();
    h.fake.0.failed_start.store(true, Ordering::SeqCst);
    h.input(KeyEvent::Press, 0);
    assert!(!h.state.active());
    assert_eq!(
        h.fake
            .0
            .phases
            .lock()
            .expect("test operation")
            .last()
            .expect("test operation"),
        "failed"
    );
    let mut h = Harness::new();
    h.input(KeyEvent::Press, 0);
    h.input(KeyEvent::Space, 10);
    h.worker.advance(Duration::from_secs(600));
    h.next();
    assert!(!h.state.active());
    assert!(h.worker.capture.is_none());
    assert_eq!(h.fake.0.dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn fallback_requires_empty_text_speech_and_an_installed_different_model() {
    use crate::dictation::decode;
    for (model, installed, speech, first, expected_calls) in [
        ("small", true, true, "", vec!["small", "base"]),
        ("small", false, true, "", vec!["small"]),
        ("small", true, false, "", vec!["small"]),
        ("base", true, true, "", vec!["base"]),
        ("small", true, true, "keep this", vec!["small"]),
    ] {
        let h = Harness::new();
        let ticket = Ticket::new(&h.state.control, &h.state.control.lock().expect("test operation"));
        let mut calls = Vec::new();
        let result = decode(
            model,
            installed,
            true,
            &ticket,
            |id| {
                calls.push(id.to_owned());
                Ok(if calls.len() == 1 {
                    first.into()
                } else {
                    "recovered".into()
                })
            },
            || Ok(speech),
            || {},
        )
        .expect("test operation");
        assert_eq!(calls, expected_calls);
        assert_eq!(result, if calls.len() == 2 { "recovered" } else { first });
    }
    let h = Harness::new();
    let ticket = Ticket::new(&h.state.control, &h.state.control.lock().expect("test operation"));
    h.state.control.lock().expect("test control").busy = true;
    let mut calls = 0;
    assert_eq!(
        decode(
            "small",
            true,
            true,
            &ticket,
            |_| {
                calls += 1;
                Ok(String::new())
            },
            || Ok(true),
            || h.state.cancel(false)
        )
        .expect("test operation"),
        ""
    );
    assert_eq!(calls, 1, "cancellation prevents the Base retry");
}

#[test]
fn live_feedback_is_ordered_capture_only_and_forgotten_with_its_generation() {
    let h = Harness::new();
    let ticket = h.state.key("talk", true).expect("recording");
    assert!(ticket.feedback(Some([1.0; 9]), None).is_none());
    let listening = ticket.notice("listening", None).expect("listening");
    let mut bands = [0.5; 9];
    bands[0] = f32::NAN;
    bands[1] = -1.0;
    bands[2] = 2.0;
    let partial = ticket
        .feedback(Some(bands), Some("spoken <text>".into()))
        .expect("preview");
    assert!(partial.sequence > listening.sequence);
    assert_eq!(&partial.bands[..4], &[0.0, 0.0, 1.0, 0.5]);
    assert_eq!(partial.partial.as_deref(), Some("spoken <text>"));
    assert_eq!(h.state.status().expect("snapshot").sequence, partial.sequence);
    serde_json::to_value(&partial).expect("finite wire values");
    let locked = ticket.notice("locked", None).expect("locked");
    assert_eq!(locked.partial, partial.partial);
    assert_eq!(locked.bands, partial.bands);
    let level = ticket.feedback(Some([0.2; 9]), None).expect("meter");
    assert_eq!(
        level.partial, partial.partial,
        "meter updates preserve the latest preview"
    );
    let text = ticket
        .feedback(None, Some("more words".into()))
        .expect("new preview");
    assert_eq!(
        text.bands, level.bands,
        "preview updates preserve real meter values"
    );
    for phase in ["finishing", "working", "copied", "failed", "cancelled"] {
        let ended = ticket.notice(phase, None).expect("state");
        assert_eq!(ended.bands, [0.0; 9]);
        assert!(ended.partial.is_none());
        assert!(ticket.feedback(Some([1.0; 9]), Some("late".into())).is_none());
    }
    ticket.notice("listening", None).expect("active capture");
    ticket
        .feedback(None, Some("private preview".into()))
        .expect("live text");
    h.state.cancel(true);
    assert!(
        h.state.status().is_none(),
        "forget clears preview snapshots immediately"
    );
    h.state.key("talk", false);
    let next = h.state.key("talk", true).expect("next recording");
    assert!(
        ticket
            .feedback(Some([1.0; 9]), Some("old words".into()))
            .is_none()
    );
    let clean = next.notice("listening", None).expect("next listening");
    assert!(clean.partial.is_none());
    assert_eq!(clean.bands, [0.0; 9]);
}

#[test]
fn caption_snapshots_only_expose_delivered_text_before_their_original_deadline() {
    let h = Harness::new();
    let ticket = h.state.key("talk", true).expect("recording");
    ticket
        .commit("delivered words", || Ok(()))
        .expect("confirmed write");
    for phase in ["copied", "pasted", "session", "held"] {
        let status = ticket.notice(phase, None).expect("delivery");
        assert_eq!(status.text.as_deref(), Some("delivered words"));
        assert_eq!(status.words, Some(2));
        assert!(
            status
                .caption_until
                .is_some_and(|until| until > crate::traces::now_ms())
        );
        h.state.control.lock().expect("control").notice =
            Some((status, Instant::now() - Duration::from_millis(2_100)));
        let snapshot = h.state.status().expect("delivery line still visible");
        assert!(
            snapshot.text.is_none(),
            "a new surface cannot resurrect the caption text"
        );
        assert!(snapshot.caption_until.is_none());
        assert_eq!(
            snapshot.words,
            Some(2),
            "caption expiry preserves measured delivery metadata"
        );
    }
    for phase in ["listening", "working", "failed", "empty", "cancelled"] {
        assert!(
            ticket.notice(phase, None).expect("state").words.is_none(),
            "{phase} cannot reuse the previous delivery's count"
        );
        assert!(
            ticket.notice(phase, None).expect("state").text.is_none(),
            "{phase} cannot expose an earlier delivery"
        );
    }
}

#[test]
fn status_snapshots_follow_live_order_and_expire_after_finishing() {
    let h = Harness::new();
    let ticket = Ticket::new(&h.state.control, &h.state.control.lock().expect("control"));
    let first = ticket.notice("listening", None).expect("first status");
    let second = ticket.notice("working", None).expect("second status");
    assert!(second.sequence > first.sequence);
    assert_eq!(h.state.status().expect("snapshot").sequence, second.sequence);
    h.state.cancel(true);
    assert!(
        ticket.notice("copied", None).is_none(),
        "cancelled work cannot publish success"
    );
    let ticket = Ticket::new(&h.state.control, &h.state.control.lock().expect("control"));
    let terminal = ticket.notice("cancelled", None).expect("terminal status");
    h.state.control.lock().expect("control").notice =
        Some((terminal, Instant::now() - Duration::from_secs(4)));
    assert!(
        h.state.status().is_none(),
        "a newly opened Nest cannot revive expired feedback"
    );
}

#[test]
fn focus_context_belongs_to_the_initial_press_and_never_survives_cancellation() {
    let h = Harness::new();
    let ticket = h.state.key("talk", true).expect("first press requests focus");
    assert!(
        h.state.key("talk", true).is_none(),
        "autorepeat cannot replace the target"
    );
    let window = vults_platform::focus::Window {
        class: "konsole".into(),
        title: "private document title".into(),
        pid: Some(42),
        identity: "kwin:one".into(),
    };
    let session = vults_core::SessionKey {
        agent: vults_protocol::AgentKind::Codex,
        session_id: "s".into(),
    };
    assert!(
        ticket.observe(window.clone(), Some(session.clone())).is_none(),
        "focus can arrive before listening status"
    );
    let status = ticket.notice("listening", None).expect("status");
    assert_eq!(status.app.as_deref(), Some("konsole"));
    assert!(status.terminal);
    assert_eq!(status.session, Some(session));
    let json = serde_json::to_string(&status).expect("wire status");
    assert!(!json.contains("private document title"));
    assert!(!json.contains("kwin:one"));
    h.state.cancel(false);
    assert!(
        ticket.observe(window.clone(), None).is_none(),
        "late cancelled observations are discarded"
    );
    h.state.key("talk", false);
    let next = h.state.key("talk", true).expect("new press");
    assert!(
        next.notice("listening", None).expect("status").app.is_none(),
        "new turns start unknown"
    );
    assert!(
        ticket.observe(window.clone(), None).is_none(),
        "old work cannot relabel a newer turn"
    );
    let status = next.observe(window.clone(), None).expect("live observation");
    assert_eq!(status.app.as_deref(), Some("konsole"));
    h.state.control.lock().expect("control").busy = false;
    assert!(
        next.observe(window, None).is_none(),
        "completed feedback cannot be revived by a late observation"
    );
}

fn raw(h: &Harness) -> RawInput {
    h.fake.0.raw.lock().expect("raw input").clone().expect("listener")
}

#[test]
fn raw_space_locks_and_releases_space_ownership_before_transcription() {
    use vults_platform::raw_keys::Key;
    let mut h = Harness::new();
    h.input(KeyEvent::Press, 0);
    assert!(h.state.holds_space(), "Open cannot steal the Space-lock chord");
    let input = raw(&h);
    input(Key::Space);
    h.next();
    assert_eq!(
        h.state.control.lock().expect("control").keys.phase(),
        Phase::Locked
    );
    h.input(KeyEvent::Release, 10);
    input(Key::Other);
    assert!(
        h.receive.try_recv().is_err(),
        "ordinary typing does not cancel hands-free recording"
    );
    h.input(KeyEvent::Press, 20);
    h.input(KeyEvent::Release, 21);
    assert!(!h.state.holds_space(), "Open resumes when capture is finishing");
    assert!(
        h.state
            .control
            .lock()
            .expect("control")
            .raw
            .load(Ordering::SeqCst),
        "Esc remains active for tail and decode"
    );
    h.worker.advance(Duration::from_millis(620));
    h.next();
    assert_eq!(*h.fake.0.copies.lock().expect("copies"), ["spoken words"]);
    assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
    input(Key::Escape);
    assert!(
        h.receive.try_recv().is_err(),
        "completed work rejects old keyboard callbacks"
    );
}

#[test]
fn raw_escape_cancels_the_tail_and_an_in_flight_decode() {
    use vults_platform::raw_keys::Key;
    let mut h = Harness::new();
    h.input(KeyEvent::Press, 0);
    h.input(KeyEvent::Release, 300);
    let flag = h.state.control.lock().expect("control").raw.clone();
    raw(&h)(Key::Escape);
    assert!(
        !flag.load(Ordering::SeqCst),
        "cancellation stops observation immediately"
    );
    h.next();
    h.worker.advance(Duration::from_millis(900));
    assert!(h.fake.0.copies.lock().expect("copies").is_empty());
    assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);

    let mut h = Harness::new();
    let (began, waiting) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    *h.fake.0.hook.lock().expect("hook") = Some((
        "decode",
        Box::new(move || {
            began.send(()).expect("decode started");
            resume.recv_timeout(Duration::from_secs(5)).expect("resume");
        }),
    ));
    h.finish();
    waiting
        .recv_timeout(Duration::from_secs(5))
        .expect("decode began");
    raw(&h)(Key::Escape);
    h.next();
    assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
    release.send(()).expect("resume");
    h.next();
    assert!(
        h.fake.0.copies.lock().expect("copies").is_empty(),
        "cancelled decoding cannot copy"
    );
}

#[test]
fn raw_other_keys_cancel_holds_and_taps_and_stale_listeners_cannot_cancel_new_turns() {
    use vults_platform::raw_keys::Key;
    for tap in [false, true] {
        let mut h = Harness::new();
        h.input(KeyEvent::Press, 0);
        if tap {
            h.input(KeyEvent::Release, 10);
        }
        let previous = raw(&h);
        previous(Key::Other);
        h.next();
        assert!(!h.state.active());
        assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
        h.input(KeyEvent::Release, 20);
        h.input(KeyEvent::Press, 30);
        assert!(h.state.holds_space());
        previous(Key::Escape);
        assert!(h.receive.try_recv().is_err());
        assert!(
            h.state.holds_space(),
            "old key events cannot cancel a new recording"
        );
        h.state.cancel(false);
        assert!(!h.state.holds_space());
        h.next();
        assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn missing_raw_keys_keep_portal_dictation_and_capture_failure_closes_the_listener() {
    let mut h = Harness::new();
    h.fake.0.failed_keys.store(true, Ordering::SeqCst);
    h.finish();
    assert!(!h.state.holds_space());
    h.next();
    assert_eq!(*h.fake.0.copies.lock().expect("copies"), ["spoken words"]);
    let mut h = Harness::new();
    h.fake.0.failed_start.store(true, Ordering::SeqCst);
    h.input(KeyEvent::Press, 0);
    assert!(!h.state.holds_space());
    assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
}

#[test]
fn a_confirmed_native_dispatch_reports_pasted_and_expires_without_relabeling_it_copied() {
    let mut h = Harness::new();
    h.fake.0.outcome.store(1, Ordering::SeqCst);
    h.finish();
    h.next();
    assert_eq!(
        h.fake.0.phases.lock().expect("phases").last().map(String::as_str),
        Some("pasted")
    );
    let ticket = Ticket::new(&h.state.control, &h.state.control.lock().expect("control"));
    ticket.notice("pasted", None);
    assert_eq!(h.state.status().expect("terminal status").phase, "pasted");
    h.state
        .control
        .lock()
        .expect("control")
        .notice
        .as_mut()
        .expect("notice")
        .1 -= Duration::from_secs(4);
    assert!(h.state.status().is_none());
}

#[test]
fn session_delivery_remembers_text_without_copying_and_held_delivery_only_copies() {
    for (outcome, phase, copies) in [(2, "session", 0), (3, "held", 1)] {
        let mut h = Harness::new();
        h.fake.0.outcome.store(outcome, Ordering::SeqCst);
        h.finish();
        h.next();
        assert_eq!(
            h.fake.0.phases.lock().expect("phases").last().map(String::as_str),
            Some(phase)
        );
        assert_eq!(h.fake.0.copies.lock().expect("copies").len(), copies);
        let ticket = Ticket::new(&h.state.control, &h.state.control.lock().expect("control"));
        ticket.notice(phase, None);
        h.state
            .control
            .lock()
            .expect("control")
            .notice
            .as_mut()
            .expect("notice")
            .1 -= Duration::from_secs(4);
        assert!(h.state.status().is_none());
        assert_eq!(
            h.state.control.lock().expect("control").last.as_deref(),
            Some("spoken words")
        );
    }
}

#[test]
fn failed_or_cancelled_session_delivery_cannot_replace_last_successful_text() {
    let h = Harness::new();
    let ticket = h.state.key("talk", true).expect("press");
    assert!(ticket.commit("previous", || Ok(())).expect("copy"));
    assert_eq!(ticket.send("failed", || false), Some(false));
    assert_eq!(
        h.state.control.lock().expect("control").last.as_deref(),
        Some("previous")
    );
    h.state.cancel(false);
    assert_eq!(ticket.send("late", || panic!("stale dispatch")), None);
    assert_eq!(
        h.state.control.lock().expect("control").last.as_deref(),
        Some("previous")
    );
}

#[test]
fn history_replay_updates_last_success_only_after_delivery() {
    for outcome in ["failed", "cancelled", "copied"] {
        let mut h = Harness::new();
        h.state.control.lock().unwrap().last = Some("previous success".into());
        match outcome {
            "failed" => h.fake.0.failed_copy.store(true, Ordering::SeqCst),
            "cancelled" => h.cancel_at("copy"),
            _ => {}
        }
        h.state.copy_again(Some("history entry".into())).unwrap();
        assert_eq!(
            h.state.control.lock().unwrap().last.as_deref(),
            Some("previous success"),
            "Queued text is not a successful delivery"
        );
        h.next();
        if outcome == "cancelled" {
            h.next();
        }
        assert_eq!(h.fake.0.phases.lock().unwrap().last().unwrap(), outcome);
        let expected = if outcome == "copied" {
            "history entry"
        } else {
            "previous success"
        };
        assert_eq!(h.state.control.lock().unwrap().last.as_deref(), Some(expected));
        h.fake.0.failed_copy.store(false, Ordering::SeqCst);
        h.state.copy_again(None).unwrap();
        h.next();
        assert_eq!(h.fake.0.copies.lock().unwrap().last().unwrap(), expected);
    }

    let h = Harness::new();
    h.state.control.lock().unwrap().last = Some("previous success".into());
    drop(h.receive);
    assert!(h.state.copy_again(Some("undeliverable".into())).is_err());
    let control = h.state.control.lock().unwrap();
    assert!(!control.busy);
    assert_eq!(control.last.as_deref(), Some("previous success"));
}

#[test]
fn rebinding_releases_the_dictation_latch_without_the_old_key_up() {
    let mut h = Harness::new();
    h.state.key("talk", true).unwrap();
    h.next();
    h.state.shortcuts_changed();
    h.next();
    assert!(!h.state.active());
    assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
    h.state.configure(GestureOptions {
        double_tap: false,
        ..Default::default()
    });
    assert!(
        h.state.key("talk", true).is_some(),
        "The first replacement press starts recording"
    );
    h.next();
    assert!(h.state.active());
    assert!(!h.state.control.lock().unwrap().keys.options().double_tap);
    h.state.key("talk", false);
    assert!(!h.state.active(), "The replacement release reaches its recording");
}

#[test]
fn stage_timings_cover_release_tail_and_each_delivery_but_exclude_cancelled_work() {
    for (cancel_at, outcome) in [
        (None, 0),
        (None, 1),
        (None, 2),
        (None, 3),
        (Some("decode"), 0),
        (Some("copy"), 0),
    ] {
        let mut h = Harness::new();
        h.worker.origin = Instant::now() - Duration::from_secs(2);
        h.state.origin = h.worker.origin;
        h.fake.0.outcome.store(outcome, Ordering::SeqCst);
        let ticket = h.state.key_at("talk", true, h.state.origin).unwrap();
        // Release can reach control before the worker has initialized capture/history.
        h.state
            .key_at("talk", false, h.state.origin + Duration::from_millis(300));
        h.next();
        ticket.history_started(42);
        ticket.history_measured(2.0, Some("en".into()));
        ticket.history_transcribed(Duration::from_millis(40));
        h.next();
        if let Some(stage) = cancel_at {
            h.cancel_at(stage);
        } else {
            *h.fake.0.hook.lock().unwrap() =
                Some(("copy", Box::new(|| std::thread::sleep(Duration::from_millis(5)))));
        }
        h.worker.advance(Duration::from_millis(900));
        h.next();
        if cancel_at.is_some() {
            h.next();
            ticket.history_transcribed(Duration::from_millis(999));
            assert!(ticket.history_entry("copied").is_none());
            assert!(h.state.control.lock().unwrap().history.is_none());
        } else {
            let phase = h.fake.0.phases.lock().unwrap().last().unwrap().clone();
            let (entry, _) = ticket.history_entry(&phase).unwrap();
            assert!(
                entry.timings.total_ms.unwrap() >= 600,
                "Includes tail and queueing after release"
            );
            assert_eq!(entry.timings.stt_ms, Some(40));
            assert!(entry.timings.pipeline_ms.is_some());
            assert_eq!(entry.timings.cleanup_ms, None, "No clean-up was requested");
            assert!(entry.timings.insert_ms.unwrap() >= 5);
            h.state.copy_again(None).unwrap();
            h.next();
            let replay = Ticket::new(&h.state.control, &h.state.control.lock().unwrap());
            assert!(
                replay.history_entry("copied").is_none(),
                "Replay never creates timing samples"
            );
        }
    }
}

#[test]
fn local_target_probes_do_not_turn_a_held_key_into_a_short_tap() {
    let h = Harness::new();
    let start = h.state.origin;
    assert!(h.state.key_at("talk", true, start).is_some());
    assert!(
        matches!(h.receive.try_recv().expect("start"), Command::Action(Action::Start, _, at) if at.is_zero())
    );
    h.state.key_at("talk", false, start + Duration::from_millis(350));
    assert!(
        matches!(h.receive.try_recv().expect("finish"), Command::Action(Action::Finish { .. }, _, at) if at == Duration::from_millis(350))
    );
}

#[test]
fn history_uses_measured_audio_once_and_ignores_replays_and_stale_generations() {
    let h = Harness::new();
    let ticket = h.state.key("talk", true).expect("start");
    ticket.history_started(42);
    ticket.history_measured(2.5, Some("pt".into()));
    ticket.commit("spoken words", || Ok(())).expect("copy");
    assert!(ticket.history_entry("failed").is_none());
    assert!(ticket.history_entry("working").is_none());
    let (entry, epoch) = ticket.history_entry("copied").expect("delivered text");
    assert_eq!(epoch, 42);
    assert_eq!(entry.secs, 2.5);
    assert_eq!(entry.words, 2);
    assert_eq!(entry.fixes, None);
    assert_eq!(entry.language.as_deref(), Some("pt"));
    assert_eq!(entry.heard, entry.text);
    assert!(
        ticket.history_entry("copied").is_none(),
        "Only one record per real capture"
    );
    h.state.cancel(false);
    ticket.history_started(50);
    ticket.history_measured(3.0, None);
    assert!(ticket.history_entry("copied").is_none());
    h.state
        .copy_again(Some("from history".into()))
        .expect("copy again");
    let replay = Ticket::new(&h.state.control, &h.state.control.lock().expect("control"));
    assert!(
        replay.history_entry("copied").is_none(),
        "Copy again cannot create new measurements"
    );
    assert!(
        h.state.copy_again(Some("replace".into())).is_err(),
        "A replay cannot replace an active job"
    );
}

#[test]
fn gesture_changes_keep_the_active_snapshot_and_cancel_shortcut_always_works() {
    use vults_platform::raw_keys::Key;
    let mut h = Harness::new();
    h.input(KeyEvent::Press, 0);
    h.state.configure(GestureOptions {
        double_tap: false,
        space_lock: false,
        escape_cancel: false,
    });
    assert!(h.state.holds_space(), "Current turn keeps its Space setting");
    raw(&h)(Key::Space);
    h.next();
    assert_eq!(h.state.control.lock().unwrap().keys.phase(), Phase::Locked);
    raw(&h)(Key::Escape);
    h.next();
    assert!(!h.state.active(), "Current turn keeps its Esc setting");
    h.input(KeyEvent::Release, 10);
    h.input(KeyEvent::Press, 20);
    assert!(
        !h.state.holds_space(),
        "Disabled Space does not suppress Open island"
    );
    let generation = h.state.control.lock().unwrap().generation;
    raw(&h)(Key::Escape);
    assert_eq!(
        h.state.control.lock().unwrap().generation,
        generation,
        "Disabled Esc cannot invalidate the turn"
    );
    assert!(h.state.active());
    h.state.key("talk-cancel", true);
    h.next();
    assert!(!h.state.active(), "Global Cancel still works with Esc off");
    h.input(KeyEvent::Release, 30);
    h.input(KeyEvent::Press, 40);
    h.input(KeyEvent::Release, 50);
    assert!(!h.state.active(), "Hold only never waits for a second tap");
    assert!(h.worker.capture.is_none());
    h.input(KeyEvent::Press, 60);
    raw(&h)(Key::Space);
    h.next();
    assert!(
        !h.state.active(),
        "Disabled Space follows ordinary typing cancellation"
    );
}

#[test]
fn disabled_escape_does_not_cancel_capture_tail_or_transcription() {
    use vults_platform::raw_keys::Key;
    let mut h = Harness::new();
    h.state.configure(GestureOptions {
        escape_cancel: false,
        ..Default::default()
    });
    h.input(KeyEvent::Press, 0);
    h.input(KeyEvent::Release, 300);
    let input = raw(&h);
    input(Key::Escape);
    assert!(h.state.control.lock().unwrap().busy);
    h.worker.advance(Duration::from_millis(900));
    input(Key::Escape);
    h.next();
    assert_eq!(*h.fake.0.copies.lock().unwrap(), ["spoken words"]);
}

#[test]
fn disabled_base_retry_skips_speech_detection_and_preserves_errors() {
    use crate::dictation::decode;
    let h = Harness::new();
    let ticket = Ticket::new(&h.state.control, &h.state.control.lock().expect("control"));
    for result in [Ok(String::new()), Err("decode failed".to_string())] {
        let expected = result.clone();
        let mut calls = 0;
        let actual = decode(
            "small",
            true,
            false,
            &ticket,
            |id| {
                assert_eq!(id, "small");
                calls += 1;
                result.clone()
            },
            || panic!("Disabled retries never run speech detection"),
            || panic!("Disabled retries never announce a retry"),
        );
        assert_eq!(actual, expected);
        assert_eq!(calls, 1);
    }
}

#[test]
fn spoken_code_uses_captured_context_and_keeps_heard_history() {
    for (class, session, enabled, expected) in [
        (Some("konsole"), false, true, "src/main.rs"),
        (Some("code"), true, true, "src/main.rs"),
        (Some("kwrite"), false, true, "src barra main ponto rs"),
        (None, false, true, "src barra main ponto rs"),
        (Some("konsole"), false, false, "src barra main ponto rs"),
    ] {
        let h = Harness::new();
        let ticket = h.state.key("talk", true).expect("start");
        if let Some(class) = class {
            ticket.observe(
                vults_platform::focus::Window {
                    class: class.into(),
                    title: "".into(),
                    pid: Some(42),
                    identity: "initial-window".into(),
                },
                session.then(|| vults_core::SessionKey {
                    agent: vults_protocol::AgentKind::Codex,
                    session_id: "one".into(),
                }),
            );
        }
        ticket.history_started(42);
        ticket.history_measured(2.0, Some("pt".into()));
        let text = ticket
            .prepare_text(
                "src barra main ponto rs",
                &Config {
                    spoken_code: enabled,
                    ..Default::default()
                },
            )
            .expect("processing")
            .expect("current text");
        assert_eq!(text, expected);
        ticket.commit(&text, || Ok(())).expect("copy");
        let (entry, _) = ticket.history_entry("copied").expect("history");
        assert_eq!(entry.heard, "src barra main ponto rs");
        assert_eq!(entry.text, expected);
        assert_eq!(
            entry.style,
            if entry.heard == entry.text {
                "verbatim"
            } else {
                "spoken-code"
            }
        );
        h.state.cancel(false);
        assert!(
            ticket
                .prepare_text(
                    "late dot result",
                    &Config {
                        spoken_code: enabled,
                        ..Default::default()
                    }
                )
                .unwrap()
                .is_none()
        );
        assert_eq!(
            h.state.control.lock().unwrap().last.as_deref(),
            Some(expected),
            "Late processing cannot replace delivered text"
        );
    }
}

#[test]
fn spoken_cancellation_keeps_last_success_and_replay_bypasses_phrase_detection() {
    for (enabled, text, cancelled) in [
        (true, "cancel", true),
        (true, "write this, never mind", true),
        (true, "src slash cancel", true),
        (false, "cancel", false),
        (true, "cancel the meeting at noon", false),
    ] {
        let mut h = Harness::new();
        *h.fake.0.transcript.lock().unwrap() = text.into();
        h.fake.0.config.lock().unwrap().cancel_by_voice = enabled;
        h.state.control.lock().unwrap().last = Some("previous success".into());
        h.input(KeyEvent::Press, 0);
        let ticket = Ticket::new(&h.state.control, &h.state.control.lock().unwrap());
        ticket.history_started(42);
        ticket.history_measured(2.0, Some("en".into()));
        // A Settings change during capture only belongs to the next recording.
        h.fake.0.config.lock().unwrap().cancel_by_voice = !enabled;
        h.input(KeyEvent::Release, 300);
        h.worker.advance(Duration::from_millis(900));
        h.next();
        let control = h.state.control.lock().unwrap();
        assert!(!control.busy);
        assert!(!control.raw.load(Ordering::SeqCst));
        assert!(control.insertion.is_none() && control.session.is_none());
        assert_eq!(control.keys.phase(), Phase::Idle);
        assert_eq!(
            control.last.as_deref(),
            Some(if cancelled { "previous success" } else { text })
        );
        if cancelled {
            assert!(control.history.is_none());
        }
        drop(control);
        assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
        assert_eq!(
            h.fake.0.phases.lock().unwrap().last().map(String::as_str),
            Some(if cancelled { "cancelled" } else { "copied" })
        );
        assert_eq!(h.fake.0.copies.lock().unwrap().len(), usize::from(!cancelled));
        if cancelled {
            assert!(
                ticket.history_entry("copied").is_none(),
                "Spoken cancellation cannot create history or totals"
            );
        }
        h.state
            .copy_again(Some("cancel".into()))
            .expect("replay literal text");
        h.next();
        assert_eq!(
            h.fake.0.copies.lock().unwrap().last().map(String::as_str),
            Some("cancel")
        );
    }
}

#[test]
fn your_words_snapshot_is_used_once_with_heard_and_written_history() {
    let mut h = Harness::new();
    *h.fake.0.transcript.lock().unwrap() = "cloud code signature".into();
    h.fake.0.config.lock().unwrap().words = vults_dictation::text::Rules {
        replacements: vec![("cloud code".into(), "Claude Code".into())],
        snippets: vec![("signature".into(), "vults\n$1 src slash main".into())],
        words: vec!["Vults".into()],
    };
    h.input(KeyEvent::Press, 0);
    let ticket = Ticket::new(&h.state.control, &h.state.control.lock().unwrap());
    ticket.history_started(42);
    ticket.history_measured(2.0, Some("en".into()));
    h.fake.0.config.lock().unwrap().words = Default::default();
    h.input(KeyEvent::Release, 300);
    h.worker.advance(Duration::from_millis(900));
    h.next();
    let expected = "Claude Code vults\n$1 src slash main";
    assert_eq!(h.fake.0.copies.lock().unwrap().as_slice(), [expected]);
    let (entry, _) = ticket.history_entry("copied").unwrap();
    assert_eq!(entry.heard, "cloud code signature");
    assert_eq!(entry.text, expected);
    assert_eq!(entry.style, "your-words");
    assert_eq!(
        entry.fixes, None,
        "Correction metrics retain their later-phase contract"
    );
    h.state.copy_again(None).unwrap();
    h.next();
    assert_eq!(h.fake.0.copies.lock().unwrap().last().unwrap(), expected);
    assert_eq!(h.fake.0.decoded.load(Ordering::SeqCst), 1);
    assert!(ticket.history_entry("copied").is_none());
}

#[test]
fn cancel_precedes_expansion_and_expanded_cancel_is_literal() {
    for (heard, expected) in [("cancel", None), ("signature", Some("cancel"))] {
        let mut h = Harness::new();
        *h.fake.0.transcript.lock().unwrap() = heard.into();
        h.fake.0.config.lock().unwrap().words.snippets = vec![
            ("cancel".into(), "must not deliver".into()),
            ("signature".into(), "cancel".into()),
        ];
        h.input(KeyEvent::Press, 0);
        h.input(KeyEvent::Release, 300);
        h.worker.advance(Duration::from_millis(900));
        h.next();
        assert_eq!(
            h.fake.0.copies.lock().unwrap().last().map(String::as_str),
            expected
        );
        assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn your_words_and_spoken_code_keep_separate_history_styles_and_reject_stale_work() {
    let h = Harness::new();
    let ticket = h.state.key("talk", true).unwrap();
    ticket.observe(
        vults_platform::focus::Window {
            class: "konsole".into(),
            title: "".into(),
            pid: Some(42),
            identity: "window".into(),
        },
        None,
    );
    ticket.history_started(42);
    ticket.history_measured(2.0, None);
    let config = Config {
        words: vults_dictation::text::Rules {
            replacements: vec![("source".into(), "src".into())],
            ..Default::default()
        },
        ..Default::default()
    };
    let text = ticket
        .prepare_text("source slash main", &config)
        .unwrap()
        .unwrap();
    assert_eq!(text, "src/main");
    ticket.commit(&text, || Ok(())).unwrap();
    let (entry, _) = ticket.history_entry("copied").unwrap();
    assert_eq!(
        (entry.style.as_str(), entry.heard.as_str()),
        ("your-words+spoken-code", "source slash main")
    );
    h.state.cancel(false);
    assert!(ticket.prepare_text("late", &config).unwrap().is_none());
}

#[test]
fn text_growth_failure_never_copies_or_overwrites_previous_success() {
    let mut h = Harness::new();
    *h.fake.0.transcript.lock().unwrap() = "trigger trigger trigger trigger trigger".into();
    h.fake.0.config.lock().unwrap().words.snippets = vec![("trigger".into(), "x".repeat(65_536))];
    h.state.control.lock().unwrap().last = Some("previous success".into());
    h.input(KeyEvent::Press, 0);
    h.input(KeyEvent::Release, 300);
    h.worker.advance(Duration::from_millis(900));
    h.next();
    assert!(h.fake.0.copies.lock().unwrap().is_empty());
    assert_eq!(
        h.state.control.lock().unwrap().last.as_deref(),
        Some("previous success")
    );
    assert_eq!(h.fake.0.phases.lock().unwrap().last().unwrap(), "failed");
    assert_eq!(h.fake.0.watches.load(Ordering::SeqCst), 0);
}

#[test]
fn command_capture_bypasses_text_processing_clipboard_and_last_dictation() {
    let mut h = Harness::new();
    h.fake.0.config.lock().expect("Config").words.replacements = vec![("spoken".into(), "approve".into())];
    let ticket = h
        .state
        .key_at("talk-command", true, h.state.origin)
        .expect("Command starts");
    assert!(ticket.is_command());
    h.next();
    h.state
        .key_at("talk-command", false, h.state.origin + Duration::from_millis(300));
    h.next();
    h.worker.advance(Duration::from_millis(900));
    h.next();
    assert_eq!(*h.fake.0.commands.lock().expect("Commands"), ["spoken words"]);
    assert!(h.fake.0.copies.lock().expect("Clipboard").is_empty());
    assert!(h.state.control.lock().expect("Control").last.is_none());
    assert!(!h.state.active());
}

#[test]
fn a_different_key_cannot_release_command_capture_and_stale_dispatch_is_rejected() {
    let mut h = Harness::new();
    let ticket = h
        .state
        .key_at("talk-command", true, h.state.origin)
        .expect("Command starts");
    h.next();
    h.state
        .key_at("talk", false, h.state.origin + Duration::from_millis(300));
    assert!(h.state.active());
    assert!(h.receive.try_recv().is_err());
    h.state.cancel(false);
    h.next();
    assert!(
        ticket
            .dispatch(|| panic!("A cancelled command must never run"))
            .is_none()
    );
    assert!(h.fake.0.copies.lock().expect("Clipboard").is_empty());
    assert!(h.fake.0.commands.lock().expect("Commands").is_empty());
}

#[test]
fn cancellation_after_decode_prevents_command_admission_and_copy_again_keeps_only_dictation() {
    let mut h = Harness::new();
    h.state.control.lock().expect("Control").last = Some("previous dictation".into());
    h.state.key_at("talk-command", true, h.state.origin);
    h.next();
    h.state
        .key_at("talk-command", false, h.state.origin + Duration::from_millis(300));
    h.next();
    h.worker.advance(Duration::from_millis(900));
    h.cancel_at("command");
    h.next();
    h.next();
    assert!(h.fake.0.commands.lock().expect("Commands").is_empty());
    h.state.copy_again(None).expect("Copy previous dictation");
    h.next();
    assert_eq!(
        *h.fake.0.copies.lock().expect("Clipboard"),
        ["previous dictation"]
    );
}

#[test]
fn command_tickets_keep_their_mode_when_another_recording_replaces_them() {
    let h = Harness::new();
    let command = h.state.key("talk-command", true).expect("Command starts");
    h.state.cancel(false);
    h.state.key("talk-command", false);
    let dictation = h.state.key("talk", true).expect("Dictation starts");
    assert!(command.is_command());
    assert!(!command.valid());
    assert!(!dictation.is_command());
}

#[test]
fn command_setting_changes_invalidate_handoffs_without_interrupting_dictation() {
    let h = Harness::new();
    h.state.control.lock().unwrap().last = Some("ordinary dictation".into());
    let ordinary = h.state.key("talk", true).expect("Dictation");
    h.state.cancel_commands();
    assert!(ordinary.valid());
    h.state.cancel(false);
    h.state.key("talk", false);
    let command = h.state.key("talk-command", true).expect("Command");
    h.state.cancel_commands();
    assert!(!command.valid());
    assert!(
        command
            .dispatch(|| panic!("A cancelled handoff cannot send"))
            .is_none()
    );
    assert_eq!(
        h.state.control.lock().unwrap().last.as_deref(),
        Some("ordinary dictation")
    );
    h.state.key("talk-command", false);
    let next = h
        .state
        .key("talk-command", true)
        .expect("A new command can start");
    next.command_result("asked", "Which one?".into(), "open apy", None);
    h.state.cancel_commands();
    assert!(
        !next.valid(),
        "A completed clarification ticket cannot survive a settings change"
    );
}

#[test]
fn cleanup_keeps_typed_local_edits_and_retained_heard_text_with_measured_outcomes() {
    use vults_dictation::cleanup::{Options, Request, Style};
    for applied in [false, true] {
        let h = Harness::new();
        let ticket = h.state.key("talk", true).unwrap();
        ticket.history_started(42);
        ticket.history_measured(2.0, Some("en".into()));
        let config = Config {
            words: vults_dictation::text::Rules {
                snippets: vec![("signature".into(), "Private literal payload $1".into())],
                ..Default::default()
            },
            ..Default::default()
        };
        let output = ticket
            .prepare_output("please add signature", &config)
            .unwrap()
            .unwrap();
        let req = Request::build(
            &output,
            &Options {
                style: Style::Natural,
                ..Options::default()
            },
        )
        .unwrap()
        .unwrap();
        assert!(
            !req.messages()
                .last()
                .unwrap()
                .text
                .contains("Private literal payload")
        );
        assert_eq!(req.original(), "please add Private literal payload $1");
        ticket.history_cleanup(
            Duration::from_millis(17),
            applied.then_some(vults_dictation::styles::Resolved {
                style: vults_dictation::cleanup::Style::Polished,
                write_in: Some(vults_dictation::cleanup::Language::Spanish),
            }),
        );
        assert!(ticket.commit(req.original(), || Ok(())).unwrap());
        let (entry, _) = ticket.history_entry("copied").unwrap();
        assert_eq!(entry.heard, "please add signature");
        assert_eq!(entry.text, req.original());
        assert_eq!(entry.timings.cleanup_ms, Some(17));
        assert_eq!(entry.style, if applied { "polished" } else { "your-words" });
        assert_eq!(entry.language.as_deref(), Some("en"));
        assert_eq!(entry.output_language.as_deref(), applied.then_some("es"));
        h.state.cancel(false);
        ticket.history_cleanup(Duration::from_millis(999), None);
        assert!(ticket.prepare_output("late", &config).unwrap().is_none());
        assert!(ticket.history_entry("copied").is_none());
    }
}

#[test]
fn a_native_ticket_is_claimed_once_and_cancelled_tickets_never_admit_work() {
    let ticket = Ticket::test_command();
    assert!(ticket.claim_native());
    assert!(!ticket.clone().claim_native());
    let cancelled = Ticket::test_command();
    cancelled.test_cancel();
    assert!(!cancelled.claim_native());
}

#[test]
fn cancel_key_invalidates_native_work_after_transcription_has_finished() {
    let h = Harness::new();
    let ticket = {
        let mut control = h.state.control.lock().unwrap();
        control.command = true;
        control.busy = false;
        Ticket::new(&h.state.control, &control)
    };
    assert!(ticket.claim_native());
    h.state.key("talk-cancel", true);
    assert!(!ticket.valid());
    assert!(!ticket.claim_native());
}

#[test]
fn command_and_dictation_routes_preserve_their_own_text_processing_contract() {
    for heard in [
        "pause media",
        "mute notifications for web",
        "open app Go Go Go Go Go Go",
        "open project O Boticario",
        "abrir aplicativo A Vida",
        "abrir proyecto La Casa",
        "open app status slash bar",
    ] {
        for key in ["talk", "talk-command"] {
            for replace in [false, true] {
                let mut h = Harness::new();
                *h.fake.0.transcript.lock().unwrap() = heard.into();
                if replace {
                    h.fake.0.config.lock().unwrap().words.replacements =
                        vec![(heard.into(), "approve".into())];
                }
                let ticket = h.state.key_at(key, true, h.state.origin).expect("Capture starts");
                let config = h.fake.0.config.lock().unwrap().clone();
                assert_eq!(config.cut_repeats_for(&ticket), key == "talk", "{heard}");
                h.next();
                h.state
                    .key_at(key, false, h.state.origin + Duration::from_millis(300));
                h.next();
                h.worker.advance(Duration::from_millis(900));
                h.next();
                if key == "talk-command" {
                    assert_eq!(*h.fake.0.commands.lock().unwrap(), [heard], "{heard}");
                    assert!(h.fake.0.copies.lock().unwrap().is_empty());
                    assert!(h.state.control.lock().unwrap().last.is_none());
                } else {
                    let expected = if replace { "approve" } else { heard };
                    assert_eq!(*h.fake.0.copies.lock().unwrap(), [expected], "{heard}");
                    assert!(
                        h.fake.0.commands.lock().unwrap().is_empty(),
                        "Dictation never dispatches a command, even when a user replacement produces one"
                    );
                    assert_eq!(h.state.control.lock().unwrap().last.as_deref(), Some(expected));
                }
                assert!(!h.state.active());
            }
        }
    }
    let h = Harness::new();
    let dictation = h.state.key("talk", true).unwrap();
    assert!(
        !Config {
            cut_repeats: false,
            ..Default::default()
        }
        .cut_repeats_for(&dictation)
    );
}
