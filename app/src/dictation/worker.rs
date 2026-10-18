use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

use vults_dictation::{Action, GestureOptions, KeyEvent, KeyLogic, Phase};
use vults_store::dictations::Timings;

use super::Config;

#[cfg(test)]
mod tests;

pub(super) struct Clip {
    pub pcm: Vec<f32>,
    pub config: Config,
}

pub(super) trait Backend: Clone + Send + 'static {
    type Recording: Send;
    type Keys: Send;
    fn watch(
        &self,
        ticket: &Ticket,
        input: impl Fn(vults_platform::raw_keys::Key) + Send + 'static,
    ) -> Option<Self::Keys>;
    fn start(&self, ticket: &Ticket) -> Result<Self::Recording, String>;
    fn finish(&self, recording: Self::Recording) -> Result<Clip, String>;
    /// None discards this dictation; replay uses the already delivered text directly.
    fn transcribe(&self, clip: Clip, ticket: &Ticket) -> Result<Option<String>, String>;
    fn copy(&self, text: String, ticket: Ticket) -> Result<Option<vults_platform::insert::Outcome>, String>;
    fn show(&self, ticket: &Ticket, phase: &str, error: Option<&str>);
}

#[derive(Default)]
struct Control {
    keys: KeyLogic,
    options: GestureOptions,
    generation: u64,
    busy: bool,
    last: Option<String>,
    stop: Arc<AtomicBool>,
    notice: Option<(Status, Instant)>,
    sequence: u64,
    target: Option<Target>,
    insertion: Option<Arc<vults_platform::insert::Target>>,
    session: Option<Arc<super::SessionTarget>>,
    raw: Arc<AtomicBool>,
    history: Option<History>,
    released: Option<Duration>,
    project_words: Arc<OnceLock<Vec<String>>>,
}

struct History {
    epoch: u64,
    at: i64,
    secs: Option<f64>,
    language: Option<String>,
    heard: Option<String>,
    style: &'static str,
    timings: Timings,
}

struct Target {
    window: vults_platform::focus::Window,
    session: Option<vults_core::SessionKey>,
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct Status {
    sequence: u64,
    generation: u64,
    phase: String,
    error: Option<String>,
    app: Option<String>,
    terminal: bool,
    session: Option<vults_core::SessionKey>,
    session_name: Option<String>,
    bands: [f32; 9],
    partial: Option<String>,
    text: Option<String>,
    words: Option<usize>,
    caption_until: Option<i64>,
}

impl Control {
    fn next(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.stop = Arc::default();
        self.generation += 1;
        self.notice = None;
        self.history = None;
        self.released = None;
        self.project_words = Arc::default();
        self.target = None;
        self.insertion = None;
        self.session = None;
        self.raw.store(false, Ordering::SeqCst);
        self.raw = Arc::default();
    }
}

#[derive(Clone)]
pub(super) struct Ticket {
    control: Arc<Mutex<Control>>,
    generation: u64,
    stop: Arc<AtomicBool>,
}

impl Ticket {
    pub fn project_words(&self) -> Arc<OnceLock<Vec<String>>> {
        let state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == self.generation {
            state.project_words.clone()
        } else {
            Arc::default()
        }
    }

    pub fn set_project_words(&self, words: Vec<String>) {
        let state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == self.generation && state.busy {
            let _ = state.project_words.set(words);
        }
    }

    pub fn history_started(&self, epoch: u64) {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == self.generation && state.busy {
            state.history = Some(History {
                epoch,
                at: crate::history::unix_now(),
                secs: None,
                language: None,
                heard: None,
                style: "verbatim",
                timings: Timings::default(),
            });
        }
    }

    pub fn history_measured(&self, secs: f64, language: Option<String>) {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == self.generation
            && let Some(history) = &mut state.history
        {
            history.secs = Some(secs);
            history.language = language;
        }
    }

    pub fn history_transcribed(&self, elapsed: Duration) {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == self.generation
            && state.busy
            && let Some(history) = &mut state.history
        {
            history.timings.stt_ms = Some(milliseconds(elapsed));
        }
    }

    pub fn history_entry(&self, phase: &str) -> Option<(vults_store::dictations::Entry, u64)> {
        if !matches!(phase, "session" | "held" | "pasted" | "copied") {
            return None;
        }
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation != self.generation {
            return None;
        }
        let history = state.history.take()?;
        let text = state.last.clone()?;
        let heard = history.heard.unwrap_or_else(|| text.clone());
        let style = history.style;
        let words = i64::try_from(vults_dictation::word_count(&text)).ok()?;
        if words == 0 {
            return None;
        }
        Some((
            vults_store::dictations::Entry {
                at: history.at,
                app: state.target.as_ref().map(|t| t.window.class.clone()),
                session: state
                    .target
                    .as_ref()
                    .and_then(|t| t.session.as_ref())
                    .map(|s| format!("{}:{}", s.agent.name(), s.session_id)),
                heard,
                text,
                style: style.into(),
                language: history.language,
                secs: history.secs?,
                words,
                fixes: None,
                outcome: phase.into(),
                timings: history.timings,
            },
            history.epoch,
        ))
    }

    pub fn prepare_text(&self, heard: &str, config: &Config) -> Result<Option<String>, String> {
        let started = Instant::now();
        let technical = {
            let state = self.control.lock().unwrap_or_else(|e| e.into_inner());
            if state.generation != self.generation || !state.busy {
                return Ok(None);
            }
            if config.cancel_by_voice && vults_dictation::text::cancels_dictation(heard) {
                return Ok(None);
            }
            state
                .target
                .as_ref()
                .is_some_and(|target| target.window.is_terminal() || target.session.is_some())
        };
        // Processing does not hold the control lock: cancellation can invalidate this turn immediately.
        let output = config
            .words
            .apply(heard, config.spoken_code && technical)
            .map_err(str::to_owned)?;
        let text = output.text();
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation != self.generation || !state.busy {
            return Ok(None);
        }
        if let Some(history) = &mut state.history {
            history.timings.pipeline_ms = Some(milliseconds(started.elapsed()));
            history.heard = Some(heard.to_owned());
            history.style = if heard == text {
                "verbatim"
            } else {
                match (output.words_changed, output.code_changed) {
                    (true, true) => "your-words+spoken-code",
                    (true, false) => "your-words",
                    _ => "spoken-code",
                }
            };
        }
        Ok(Some(text))
    }

    pub fn pin_session(&self, session: super::SessionTarget) {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == self.generation && state.busy {
            if let Some(target) = &mut state.target {
                target.session = Some(session.key.clone());
            }
            state.session = Some(Arc::new(session));
        }
    }

    pub fn session(&self) -> Option<Arc<super::SessionTarget>> {
        let state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        (state.generation == self.generation)
            .then(|| state.session.clone())
            .flatten()
    }
    pub fn pin(&self, target: vults_platform::insert::Target) {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == self.generation && state.busy {
            state.target = Some(Target {
                window: target.window().clone(),
                session: None,
            });
            state.insertion = Some(Arc::new(target));
        }
    }

    pub fn insertion(&self) -> Option<Arc<vults_platform::insert::Target>> {
        let state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        (state.generation == self.generation)
            .then(|| state.insertion.clone())
            .flatten()
    }

    pub fn notice(&self, phase: &str, error: Option<&str>) -> Option<Status> {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation != self.generation {
            return None;
        }
        state.sequence += 1;
        let previous = state.notice.as_ref().map(|(status, _)| status);
        let live = matches!(phase, "listening" | "locked");
        let text = matches!(phase, "session" | "held" | "pasted" | "copied")
            .then(|| state.last.clone())
            .flatten();
        let status = Status {
            sequence: state.sequence,
            generation: self.generation,
            phase: phase.to_owned(),
            error: error.map(str::to_owned),
            app: state.target.as_ref().map(|target| target.window.class.clone()),
            terminal: state
                .target
                .as_ref()
                .is_some_and(|target| target.window.is_terminal()),
            session: state.target.as_ref().and_then(|target| target.session.clone()),
            session_name: state.session.as_ref().map(|session| session.name.clone()),
            bands: if live {
                previous.map_or([0.0; 9], |status| status.bands)
            } else {
                [0.0; 9]
            },
            partial: live
                .then(|| previous.and_then(|status| status.partial.clone()))
                .flatten(),
            words: text.as_deref().map(vults_dictation::word_count),
            text,
            caption_until: matches!(
                phase,
                "session" | "held" | "pasted" | "copied" | "empty" | "cancelled" | "failed"
            )
            .then(|| crate::traces::now_ms().saturating_add(2_000)),
        };
        state.notice = Some((status.clone(), Instant::now()));
        Some(status)
    }

    pub fn feedback(&self, bands: Option<[f32; 9]>, partial: Option<String>) -> Option<Status> {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation != self.generation || !state.busy {
            return None;
        }
        let (status, _) = state.notice.as_ref()?;
        if !matches!(status.phase.as_str(), "listening" | "locked") {
            return None;
        }
        state.sequence += 1;
        let sequence = state.sequence;
        let (status, _) = state.notice.as_mut()?;
        status.sequence = sequence;
        if let Some(bands) = bands {
            status.bands = bands.map(|n| if n.is_finite() { n.clamp(0.0, 1.0) } else { 0.0 });
        }
        if let Some(text) = partial {
            status.partial = Some(text);
        }
        Some(status.clone())
    }
    pub fn observe(
        &self,
        window: vults_platform::focus::Window,
        session: Option<vults_core::SessionKey>,
    ) -> Option<Status> {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation != self.generation || !state.busy {
            return None;
        }
        // Context arriving later must never relabel a pinned pane as another session.
        if state.session.is_none() && state.insertion.is_none() {
            state.target = Some(Target { window, session });
        }
        let (mut status, since) = state.notice.clone()?;
        if status.generation != self.generation {
            return None;
        }
        let target = state.target.as_ref()?;
        status.app = Some(target.window.class.clone());
        status.terminal = target.window.is_terminal();
        status.session = target.session.clone();
        state.sequence += 1;
        status.sequence = state.sequence;
        state.notice = Some((status.clone(), since));
        Some(status)
    }
    pub fn attach_keys(&self, active: Arc<AtomicBool>) -> bool {
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation != self.generation || !state.busy {
            active.store(false, Ordering::SeqCst);
            return false;
        }
        state.raw.store(false, Ordering::SeqCst);
        state.raw = active;
        state.raw.load(Ordering::SeqCst)
    }
    fn new(control: &Arc<Mutex<Control>>, state: &Control) -> Self {
        Self {
            control: control.clone(),
            generation: state.generation,
            stop: state.stop.clone(),
        }
    }
    pub fn stop(&self) -> &AtomicBool {
        &self.stop
    }

    pub fn valid(&self) -> bool {
        self.control.lock().unwrap_or_else(|e| e.into_inner()).generation == self.generation
    }

    /// Cancellation and the native clipboard write have one ordering.
    pub fn commit(&self, text: &str, write: impl FnOnce() -> Result<(), String>) -> Result<bool, String> {
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if control.generation != self.generation {
            return Ok(false);
        }
        write()?;
        control.last = Some(text.to_owned());
        Ok(true)
    }

    pub fn send(&self, text: &str, send: impl FnOnce() -> bool) -> Option<bool> {
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if control.generation != self.generation {
            return None;
        }
        let sent = send();
        if sent {
            control.last = Some(text.to_owned());
        }
        Some(sent)
    }
}

enum Command {
    Action(Action, Ticket, Duration),
    Again(Ticket, String),
    Result(Ticket, Result<Option<String>, String>),
}

#[derive(Clone)]
pub(crate) struct State {
    control: Arc<Mutex<Control>>,
    send: mpsc::Sender<Command>,
    origin: Instant,
}

impl State {
    pub fn configure(&self, options: GestureOptions) {
        self.control.lock().unwrap_or_else(|e| e.into_inner()).options = options;
    }

    pub fn shortcuts_changed(&self) {
        self.cancel(false);
        // The old binding may never release; it must not latch the replacement key.
        let _ = self.key_at("talk", false, Instant::now());
    }

    pub fn status(&self) -> Option<Status> {
        let state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        let (status, since) = state.notice.as_ref()?;
        let terminal = matches!(
            status.phase.as_str(),
            "session" | "held" | "pasted" | "copied" | "empty" | "cancelled" | "failed"
        );
        if terminal && since.elapsed() > Duration::from_secs(if status.phase == "failed" { 8 } else { 3 }) {
            return None;
        }
        let mut snapshot = status.clone();
        if terminal && since.elapsed() >= Duration::from_secs(2) {
            snapshot.text = None;
            snapshot.caption_until = None;
        }
        Some(snapshot)
    }
    pub(super) fn start(backend: impl Backend) -> Result<Self, String> {
        let (send, receive) = mpsc::channel();
        let control = Arc::new(Mutex::new(Control::default()));
        let state = Self {
            control: control.clone(),
            send: send.clone(),
            origin: Instant::now(),
        };
        let origin = state.origin;
        std::thread::Builder::new()
            .name("dictation".into())
            .spawn(move || {
                let mut worker = Worker::new(backend, control.clone(), send, origin);
                loop {
                    match receive.recv_timeout(Duration::from_millis(20)) {
                        Ok(command) => worker.apply(command),
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    let now = origin.elapsed();
                    if let Some(command) = event(&control, KeyEvent::Tick, now) {
                        worker.apply(command);
                    }
                    worker.advance(now);
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(state)
    }

    pub fn holds_space(&self) -> bool {
        let control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        control.raw.load(Ordering::SeqCst)
            && control.keys.options().space_lock
            && control.keys.phase() != Phase::Idle
    }
    pub fn active(&self) -> bool {
        self.control
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys
            .phase()
            != Phase::Idle
    }

    #[cfg(test)]
    pub(super) fn key(&self, id: &str, down: bool) -> Option<Ticket> {
        self.key_at(id, down, Instant::now())
    }

    pub(super) fn key_at(&self, id: &str, down: bool, observed: Instant) -> Option<Ticket> {
        if id == "talk-again" && down {
            let _ = self.copy_again(None);
            return None;
        }
        let key = match (id, down) {
            ("talk", true) => KeyEvent::Press,
            ("talk", false) => KeyEvent::Release,
            ("talk-cancel", true) => {
                self.cancel(false);
                return None;
            }
            _ => return None,
        };
        if let Some(command) = event(
            &self.control,
            key,
            observed.saturating_duration_since(self.origin),
        ) {
            let probe = match &command {
                Command::Action(Action::Start | Action::Restart, ticket, _) => Some(ticket.clone()),
                _ => None,
            };
            let _ = self.send.send(command);
            return probe;
        }
        None
    }

    pub fn copy_again(&self, text: Option<String>) -> Result<(), String> {
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if control.busy {
            return Err("Dictation is busy. Finish or cancel it first.".into());
        }
        control.next();
        control.busy = true;
        let text = text.or_else(|| control.last.clone()).unwrap_or_default();
        let ticket = Ticket::new(&self.control, &control);
        if self.send.send(Command::Again(ticket, text)).is_err() {
            control.busy = false;
            return Err("The dictation worker is unavailable.".into());
        }
        Ok(())
    }

    pub fn cancel(&self, forget: bool) {
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        let active = control.busy || control.keys.phase() != Phase::Idle;
        if !active && !forget {
            return;
        }
        control.keys.reset();
        control.next();
        control.busy = false;
        if forget {
            control.last = None;
        }
        if !active {
            return;
        }
        let ticket = Ticket::new(&self.control, &control);
        let _ = self
            .send
            .send(Command::Action(Action::Cancel, ticket, self.origin.elapsed()));
    }
}

fn event(control: &Arc<Mutex<Control>>, key: KeyEvent, now: Duration) -> Option<Command> {
    let mut state = control.lock().unwrap_or_else(|e| e.into_inner());
    if key == KeyEvent::Press {
        let options = state.options;
        state.keys.configure(options);
    }
    let action = state.keys.update(key, now)?;
    if matches!(action, Action::Start | Action::Restart | Action::Cancel) {
        state.next();
        state.busy = action != Action::Cancel;
    }
    if matches!(action, Action::Finish { .. }) {
        state.released = Some(now);
    }
    let ticket = Ticket::new(control, &state);
    Some(Command::Action(action, ticket, now))
}

fn raw_event(ticket: &Ticket, key: vults_platform::raw_keys::Key, now: Duration) -> Option<Command> {
    use vults_platform::raw_keys::Key;
    let mut state = ticket.control.lock().unwrap_or_else(|e| e.into_inner());
    if state.generation != ticket.generation || !state.busy {
        return None;
    }
    let action = match key {
        Key::Escape if state.keys.options().escape_cancel => Action::Cancel,
        Key::Escape => return None,
        Key::Space => state.keys.update(KeyEvent::Space, now)?,
        Key::Other => state.keys.update(KeyEvent::OtherKey, now)?,
    };
    if action == Action::Cancel {
        state.keys.reset();
        state.next();
        state.busy = false;
    }
    Some(Command::Action(action, Ticket::new(&ticket.control, &state), now))
}

struct Worker<B: Backend> {
    backend: B,
    control: Arc<Mutex<Control>>,
    send: mpsc::Sender<Command>,
    capture: Option<(B::Recording, Ticket, Duration)>,
    tail: Option<Duration>,
    keys: Option<B::Keys>,
    origin: Instant,
}

impl<B: Backend> Worker<B> {
    fn new(backend: B, control: Arc<Mutex<Control>>, send: mpsc::Sender<Command>, origin: Instant) -> Self {
        Self {
            backend,
            control,
            send,
            capture: None,
            tail: None,
            keys: None,
            origin,
        }
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::Action(action, ticket, now) => {
                if !ticket.valid() {
                    return;
                }
                match action {
                    Action::Start | Action::Restart => {
                        self.capture = None;
                        self.tail = None;
                        self.keys = None;
                        let raw = ticket.clone();
                        let origin = self.origin;
                        let send = self.send.clone();
                        self.keys = self.backend.watch(&ticket, move |key| {
                            if let Some(command) = raw_event(&raw, key, origin.elapsed()) {
                                let _ = send.send(command);
                            }
                        });
                        if !ticket.valid() {
                            self.keys = None;
                            return;
                        }
                        match self.backend.start(&ticket) {
                            Ok(capture) if ticket.valid() => {
                                self.backend.show(&ticket, "listening", None);
                                self.capture = Some((capture, ticket, now));
                            }
                            Ok(_) => {}
                            Err(error) => self.ended(ticket, Err(error)),
                        }
                    }
                    Action::Lock => self.backend.show(&ticket, "locked", None),
                    Action::Finish { capture_until } => {
                        if self.capture.is_some() {
                            self.tail = Some(capture_until);
                            self.backend.show(&ticket, "finishing", None);
                        }
                    }
                    Action::Cancel => {
                        self.keys = None;
                        self.capture = None;
                        self.tail = None;
                        self.backend.show(&ticket, "cancelled", None);
                    }
                }
            }
            Command::Again(ticket, text) => self.ended(ticket, Ok(Some(text))),
            Command::Result(ticket, result) => self.ended(ticket, result),
        }
    }

    fn advance(&mut self, now: Duration) {
        let Some((_, ticket, started)) = &self.capture else {
            return;
        };
        if !ticket.valid() {
            self.keys = None;
            self.capture = None;
            self.tail = None;
            return;
        }
        if now.saturating_sub(*started) >= Duration::from_secs(600) {
            self.tail = Some(now);
            let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
            if state.generation == ticket.generation {
                state.released.get_or_insert(now);
            }
        }
        if !self.tail.is_some_and(|until| now >= until) {
            return;
        }
        self.tail = None;
        let (recording, ticket, _) = self.capture.take().expect("capture checked");
        {
            let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
            if state.generation == ticket.generation {
                state.keys.reset();
            }
        }
        let clip = match self.backend.finish(recording) {
            Ok(clip) => clip,
            Err(error) => {
                self.ended(ticket, Err(error));
                return;
            }
        };
        if !ticket.valid() {
            return;
        }
        if clip.pcm.len() < 4000 || !clip.pcm.iter().any(|s| s.abs() >= 0.004) {
            self.ended(ticket, Ok(Some(String::new())));
            return;
        }
        self.backend.show(&ticket, "working", None);
        let backend = self.backend.clone();
        let send = self.send.clone();
        let job = ticket.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("dictation-transcribe".into())
            .spawn(move || {
                if job.valid() {
                    let result = backend.transcribe(clip, &job);
                    let _ = send.send(Command::Result(job, result));
                }
            })
        {
            self.ended(ticket, Err(error.to_string()));
        }
    }

    fn ended(&mut self, ticket: Ticket, result: Result<Option<String>, String>) {
        if !ticket.valid() {
            return;
        }
        let delivery_started = Instant::now();
        let result = match result {
            Ok(Some(text)) if !text.is_empty() => match self.backend.copy(text, ticket.clone()) {
                Ok(Some(vults_platform::insert::Outcome::Pasted)) => Ok("pasted"),
                Ok(Some(vults_platform::insert::Outcome::Copied)) => Ok("copied"),
                Ok(Some(vults_platform::insert::Outcome::Session)) => Ok("session"),
                Ok(Some(vults_platform::insert::Outcome::Held)) => Ok("held"),
                Ok(None) => return,
                Err(error) => Err(error),
            },
            Ok(Some(_)) => Ok("empty"),
            Ok(None) => Ok("cancelled"),
            Err(error) => Err(error),
        };
        let delivered = self.origin.elapsed();
        let insert_ms = milliseconds(delivery_started.elapsed());
        {
            let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
            if state.generation != ticket.generation {
                return;
            }
            if matches!(result, Ok("session" | "held" | "pasted" | "copied")) {
                let total = state
                    .released
                    .map(|released| milliseconds(delivered.saturating_sub(released)));
                if let Some(history) = &mut state.history {
                    history.timings.total_ms = total;
                    history.timings.insert_ms = Some(insert_ms);
                }
            }
            state.busy = false;
            if result == Ok("cancelled") {
                state.history = None;
            }
            state.insertion = None;
            state.keys.reset();
            state.raw.store(false, Ordering::SeqCst);
        }
        self.keys = None;
        match result {
            Ok(phase) => self.backend.show(&ticket, phase, None),
            Err(error) => self.backend.show(&ticket, "failed", Some(&error)),
        }
        let mut state = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.generation == ticket.generation {
            state.session = None;
        }
    }
}

fn milliseconds(elapsed: Duration) -> i64 {
    i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
}
