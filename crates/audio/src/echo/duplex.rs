//! Explicit native duplex routes. No module loading, defaults, file output or automatic admission.
use super::{Endpoint, Pair};
use crate::{
    meeting::pulse::Connection,
    playback::{Cursor, Phase, RATE},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
mod graph;
mod native;
mod raw;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Busy,
    Cancelled,
    Route,
    Device,
    Clock,
    Overflow,
    Timeout,
    State,
    Input,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interruption {
    Disabled,
    Headphones,
    EchoCancelled,
}
#[derive(Clone)]
pub struct Route {
    server: u32,
    source: Endpoint,
    sink: Endpoint,
    echo: Option<Pair>,
    mode: Interruption,
}
impl std::fmt::Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DuplexRoute")
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}
impl Route {
    /// Immutable user choice, never proof that a live route is still verified.
    pub fn chosen_policy(&self) -> Interruption {
        self.mode
    }
    pub fn echo(pair: Pair) -> Self {
        Self {
            server: pair.server,
            source: pair.source.clone(),
            sink: pair.sink.clone(),
            echo: Some(pair),
            mode: Interruption::EchoCancelled,
        }
    }
    /// Only an explicit headphones choice grants this policy; device labels are never consulted.
    pub fn headphones(source: &str, sink: &str) -> Result<Self, Failure> {
        Self::devices(source, sink, Interruption::Headphones)
    }
    pub fn speakers(source: &str, sink: &str) -> Result<Self, Failure> {
        Self::devices(source, sink, Interruption::Disabled)
    }
    fn devices(source: &str, sink: &str, mode: Interruption) -> Result<Self, Failure> {
        let connection = Connection::inspect().map_err(|_| Failure::Device)?;
        let before = connection
            .wait(connection.client.server_info())
            .map_err(|_| Failure::Device)?;
        let name = |s: &str| std::ffi::CString::new(s).map_err(|_| Failure::Input);
        let source = connection
            .wait(connection.client.source_info_by_name(name(source)?))
            .map_err(|_| Failure::Route)?;
        let sink = connection
            .wait(connection.client.sink_info_by_name(name(sink)?))
            .map_err(|_| Failure::Route)?;
        let after = connection
            .wait(connection.client.server_info())
            .map_err(|_| Failure::Device)?;
        if before.cookie != after.cookie || source.monitor_of_sink_index.is_some() {
            return Err(Failure::Route);
        }
        Ok(Self {
            server: before.cookie,
            source: super::endpoint(source.index, &source.name, &source.props).ok_or(Failure::Route)?,
            sink: super::endpoint(sink.index, &sink.name, &sink.props).ok_or(Failure::Route)?,
            echo: None,
            mode,
        })
    }
}
const DEADLINE: Duration = Duration::from_secs(3);
const FRESH: Duration = Duration::from_millis(500);
const MAX_FRAMES: u64 = RATE as u64 * 120;
struct Span {
    wire: u64,
    before: u64,
    frames: u64,
}
struct OutputState {
    phase: Phase,
    pcm: VecDeque<i16>,
    spans: VecDeque<Span>,
    total: u64,
    submitted: u64,
    played: u64,
    first: Option<Instant>,
    eof: bool,
    drained: bool,
    failure: Option<Failure>,
    started: Instant,
}
impl OutputState {
    fn new() -> Self {
        Self {
            phase: Phase::Starting,
            pcm: VecDeque::new(),
            spans: VecDeque::new(),
            total: 0,
            submitted: 0,
            played: 0,
            first: None,
            eof: false,
            drained: false,
            failure: None,
            started: Instant::now(),
        }
    }
    fn active(&self) -> bool {
        matches!(self.phase, Phase::Starting | Phase::Playing | Phase::Draining)
    }
    fn halt(&mut self, failure: Option<Failure>) {
        if self.active() {
            self.phase = if failure.is_some() {
                Phase::Failed
            } else {
                Phase::Stopped
            };
            self.failure = failure;
        }
        self.pcm.clear();
        self.spans.clear();
    }
    fn cursor(&self) -> Cursor {
        Cursor {
            phase: self.phase,
            total_frames: self.total,
            submitted_frames: self.submitted,
            played_frames: self.played,
            first_played: self.first,
            failure: self.failure.map(|_| crate::playback::Failure::Device),
        }
    }
    fn advance(&mut self, wire: u64) {
        if !self.active() {
            return;
        }
        while let Some(span) = self.spans.front() {
            let frames = wire.saturating_sub(span.wire).min(span.frames);
            if frames > 0 {
                self.first.get_or_insert_with(Instant::now);
            }
            self.played = self.played.max(span.before + frames);
            if frames < span.frames {
                break;
            }
            self.spans.pop_front();
        }
        if self.eof && self.drained && self.played == self.total && self.total > 0 {
            self.phase = Phase::Finished;
        }
    }
}
struct State {
    failure: Option<Failure>,
    ready: bool,
    verified: bool,
    output_live: bool,
    checked: Instant,
    capture: VecDeque<Vec<i16>>,
    output: Option<Arc<Mutex<OutputState>>>,
}
struct Shared {
    state: Mutex<State>,
    cancel: Arc<AtomicBool>,
    started: Instant,
    active: Arc<dyn Fn() -> bool + Send + Sync>,
}
impl Shared {
    fn fail(&self, failure: Failure) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.failure.get_or_insert(failure);
        state.verified = false;
        state.capture.clear();
        if let Some(output) = &state.output {
            output
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .halt(Some(failure));
        }
    }
    fn check(&self) -> Result<(), Failure> {
        if self.cancel.load(Ordering::Acquire) || !(self.active)() {
            return Err(Failure::Cancelled);
        }
        let state = self.state.lock().map_err(|_| Failure::State)?;
        if let Some(f) = state.failure {
            return Err(f);
        }
        if (!state.ready && self.started.elapsed() > DEADLINE)
            || (state.ready && state.checked.elapsed() > FRESH)
            || self.started.elapsed() > Duration::from_secs(600)
        {
            return Err(Failure::Timeout);
        }
        Ok(())
    }
}
/// The caller must explicitly authorize capture and continuously consume the bounded input queue.
pub struct Session {
    shared: Arc<Shared>,
    mode: Interruption,
}
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DuplexSession")
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}
impl Session {
    pub fn start<T: Send + Sync + 'static>(
        route: Route,
        cancel: Arc<AtomicBool>,
        lease: Arc<T>,
    ) -> Result<Self, Failure> {
        Self::start_guarded(route, cancel, lease, Arc::new(|| true))
    }
    pub fn start_guarded<T: Send + Sync + 'static>(
        route: Route,
        cancel: Arc<AtomicBool>,
        lease: Arc<T>,
        active: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Result<Self, Failure> {
        if cancel.load(Ordering::Acquire) || !active() {
            return Err(Failure::Cancelled);
        }
        let owner = crate::playback::Owner::claim().map_err(|_| Failure::Busy)?;
        let mode = route.mode;
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                failure: None,
                ready: false,
                verified: false,
                output_live: false,
                checked: Instant::now(),
                capture: VecDeque::new(),
                output: None,
            }),
            cancel,
            active,
            started: Instant::now(),
        });
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("voice-duplex".into())
            .spawn(move || {
                let _lease = lease;
                let _owner = owner;
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| native::run(&route, &worker)))
                        .unwrap_or(Err(Failure::Device));
                worker.fail(result.err().unwrap_or(Failure::Cancelled));
            })
            .map_err(|_| Failure::Device)?;
        Ok(Self { shared, mode })
    }
    pub fn ready(&self) -> Result<bool, Failure> {
        self.shared.check()?;
        Ok(self.shared.state.lock().map_err(|_| Failure::State)?.ready)
    }
    /// Immutable user choice, not proof that a live route is currently verified.
    pub fn chosen_policy(&self) -> Interruption {
        self.mode
    }
    pub fn interruption(&self) -> Interruption {
        if self.shared.check().is_ok()
            && self
                .shared
                .state
                .lock()
                .is_ok_and(|s| s.ready && s.verified && s.output_live)
        {
            self.mode
        } else {
            Interruption::Disabled
        }
    }
    pub fn take_input(&self) -> Result<Option<Vec<i16>>, Failure> {
        self.shared.check()?;
        let mut state = self.shared.state.lock().map_err(|_| Failure::State)?;
        Ok(if state.ready && state.verified {
            state.capture.pop_front()
        } else {
            None
        })
    }
    pub fn output(&self) -> Result<Output, Failure> {
        self.shared.check()?;
        let mut state = self.shared.state.lock().map_err(|_| Failure::State)?;
        if !state.ready || state.output.is_some() {
            return Err(Failure::Busy);
        }
        let output = Arc::new(Mutex::new(OutputState::new()));
        state.output = Some(output.clone());
        Ok(Output {
            state: output,
            shared: self.shared.clone(),
        })
    }
    pub fn stop(&self) {
        self.shared.cancel.store(true, Ordering::Release);
        self.shared.fail(Failure::Cancelled);
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}
/// One response's content cursor; native stream offsets include silence and never become heard text.
pub struct Output {
    state: Arc<Mutex<OutputState>>,
    shared: Arc<Shared>,
}
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DuplexOutput")
            .field("cursor", &self.cursor())
            .finish()
    }
}
impl Output {
    pub fn push(&self, pcm: &[i16]) -> Result<(), Failure> {
        self.shared.check()?;
        if pcm.is_empty() || pcm.len() > RATE as usize {
            return Err(Failure::Input);
        }
        let mut s = self.state.lock().map_err(|_| Failure::State)?;
        if s.phase != Phase::Playing || s.eof {
            return Err(Failure::State);
        }
        if s.total + pcm.len() as u64 > MAX_FRAMES {
            return Err(Failure::Input);
        }
        s.total += pcm.len() as u64;
        s.pcm.extend(pcm);
        Ok(())
    }
    pub fn finish(&self) -> Result<(), Failure> {
        self.shared.check()?;
        let mut s = self.state.lock().map_err(|_| Failure::State)?;
        if s.phase != Phase::Playing || s.total == 0 {
            return Err(Failure::State);
        }
        s.eof = true;
        s.phase = Phase::Draining;
        Ok(())
    }
    pub fn cursor(&self) -> Cursor {
        if let Err(f) = self.shared.check() {
            self.shared.fail(f);
        }
        self.state.lock().unwrap_or_else(|e| e.into_inner()).cursor()
    }
    pub fn stop(&self) -> Cursor {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.halt(None);
        s.cursor()
    }
    pub fn route_verified(&self) -> bool {
        self.shared.check().is_ok()
            && self
                .shared
                .state
                .lock()
                .is_ok_and(|s| s.ready && s.verified && s.output_live)
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        self.stop();
    }
}
#[cfg(test)]
mod tests;
