use super::raw::{Change, Context, ContextState, Facility, Mainloop, Stream, StreamState};
use super::*;
use graph::{Graph, Pinned};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct Native {
    capture: Option<Stream>,
    playback: Option<Stream>,
    context: Context,
    // Native streams and context must be destroyed before their mainloop.
    mainloop: Mainloop,
    last_capture: Instant,
}
impl Drop for Native {
    fn drop(&mut self) {
        if let Some(s) = &mut self.capture {
            s.clear_callbacks();
        }
        if let Some(s) = &mut self.playback {
            s.clear_callbacks();
        }
        self.context.set_subscribe_callback(None);
        self.context.disconnect();
    }
}
impl Native {
    fn pump(&mut self, shared: &Shared) -> Result<(), Failure> {
        shared.check()?;
        // Drain ready protocol work before sleeping; one packet per tick starves duplex audio.
        let mut idle = false;
        for _ in 0..64 {
            if !self.mainloop.iterate()? {
                idle = true;
                break;
            }
            shared.check()?;
        }
        if !idle {
            return Err(Failure::Overflow);
        }
        if matches!(
            self.context.state(),
            ContextState::Failed | ContextState::Terminated
        ) {
            return Err(Failure::Device);
        }
        shared.check()
    }
    fn wait(&mut self, shared: &Shared, done: impl Fn(&Self) -> bool) -> Result<(), Failure> {
        let start = Instant::now();
        while !done(self) {
            self.pump(shared)?;
            if start.elapsed() > DEADLINE {
                return Err(Failure::Timeout);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        self.pump(shared)
    }
    fn graph(&mut self, shared: &Shared) -> Result<Graph, Failure> {
        let graph = graph::request(&self.context)?;
        self.wait(shared, |_| graph.graph.borrow().completed() != Ok(false))?;
        let graph = graph.graph.borrow().clone();
        graph.completed()?;
        Ok(graph)
    }
    fn stream(
        &mut self,
        name: &str,
        device: &str,
        record: bool,
        shared: &Arc<Shared>,
    ) -> Result<Stream, Failure> {
        shared.check()?;
        let mut stream = Stream::new(&mut self.context, name)?;
        let state = shared.clone();
        stream.on_moved(Box::new(move || state.fail(Failure::Route)));
        let state = shared.clone();
        stream.on_overflow(Box::new(move || state.fail(Failure::Overflow)));
        stream.attach(device, record)?;
        self.wait(shared, |_| {
            matches!(
                stream.state(),
                StreamState::Ready | StreamState::Failed | StreamState::Terminated
            )
        })?;
        if stream.state() != StreamState::Ready || !stream.spec_matches() {
            return Err(Failure::Device);
        }
        Ok(stream)
    }
    fn read(&mut self, shared: &Shared, publish: bool) -> Result<(), Failure> {
        if self.last_capture.elapsed() > FRESH && shared.state.lock().map_err(|_| Failure::State)?.ready {
            return Err(Failure::Timeout);
        }
        let stream = self.capture.as_mut().ok_or(Failure::Device)?;
        if stream.state() != StreamState::Ready || stream.suspended() {
            return Err(Failure::Device);
        }
        for _ in 0..16 {
            let Some(bytes) = stream.read()? else {
                return Ok(());
            };
            self.last_capture = Instant::now();
            if publish && !bytes.is_empty() {
                let samples = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|s| i16::from_le_bytes(*s))
                    .collect();
                let mut state = shared.state.lock().map_err(|_| Failure::State)?;
                if !state.verified || state.failure.is_some() || shared.cancel.load(Ordering::Acquire) {
                    continue;
                }
                if state.capture.len() >= 16 {
                    return Err(Failure::Overflow);
                }
                state.capture.push_back(samples);
            }
        }
        // Never rearm with unread capture from an unverified interval.
        Err(Failure::Overflow)
    }
}
fn uncork(native: &mut Native, mut stream: Stream, shared: &Shared) -> Result<Stream, Failure> {
    shared.check()?;
    let operation = stream.uncork()?;
    native.wait(shared, |_| operation.result.get().is_some())?;
    if operation.result.get() != Some(true) {
        return Err(Failure::Device);
    }
    Ok(stream)
}
fn timing(info: &raw::TimingInfo, wire_written: u64) -> Result<u64, Failure> {
    if info.read_index_corrupt != 0
        || info.write_index_corrupt != 0
        || info.sink_usec > 2_000_000
        || info.write_index < 0
        || info.write_index as u64 > wire_written * 2
        || info.write_index % 2 != 0
    {
        return Err(Failure::Clock);
    }
    // The server read cursor includes underrun silence. Content spans below exclude those gaps.
    let read = info.read_index.max(0) as u64 / 2;
    let pending = info.sink_usec.saturating_mul(u64::from(RATE)).div_ceil(1_000_000);
    Ok(read.saturating_sub(pending))
}
pub(super) fn run(route: &Route, shared: &Arc<Shared>) -> Result<(), Failure> {
    shared.check()?;
    let mainloop = Mainloop::new()?;
    let context = Context::new(&mainloop)?;
    let mut native = Native {
        mainloop,
        context,
        capture: None,
        playback: None,
        last_capture: Instant::now(),
    };
    let socket = crate::meeting::pulse::local_socket().map_err(|_| Failure::Route)?;
    let endpoint = format!("unix:{}", socket.to_str().ok_or(Failure::Route)?);
    native.context.connect(&endpoint)?;
    native.wait(shared, |n| n.context.state() == ContextState::Ready)?;
    let dirty = Rc::new(Cell::new(true));
    let watch: Rc<RefCell<Option<Pinned>>> = Rc::new(RefCell::new(None));
    let revision = Rc::new(Cell::new(0_u64));
    let events = revision.clone();
    let changes = dirty.clone();
    let watched = watch.clone();
    let state = shared.clone();
    native
        .context
        .set_subscribe_callback(Some(Box::new(move |facility, operation, index| {
            let Some(p) = watched.borrow().clone() else {
                changes.set(true);
                return;
            };
            let physical = match facility {
                Some(Facility::Source) => p.capture.as_ref().is_some_and(|(_, n)| n.index == index),
                Some(Facility::Sink) => p.playback.as_ref().is_some_and(|(_, n)| n.index == index),
                _ => false,
            };
            let node = match facility {
                Some(Facility::Source) => {
                    p.source.index == index || p.capture.as_ref().is_some_and(|(_, n)| n.index == index)
                }
                Some(Facility::Sink) => {
                    p.sink.index == index || p.playback.as_ref().is_some_and(|(_, n)| n.index == index)
                }
                _ => false,
            };
            let edge = match facility {
                Some(Facility::SourceOutput) => p.capture.as_ref().is_some_and(|(e, _)| e.index == index),
                Some(Facility::SinkInput) => p.playback.as_ref().is_some_and(|(e, _)| e.index == index),
                _ => false,
            };
            if physical
                || edge
                || (node && operation == Some(Change::Removed))
                || matches!(facility, Some(Facility::Card))
                || (facility == Some(Facility::Module) && route_module(&p) == Some(index))
            {
                state.fail(Failure::Route);
            } else if node || facility == Some(Facility::Server) {
                let mut state = state.state.lock().unwrap_or_else(|e| e.into_inner());
                state.verified = false;
                state.capture.clear();
                events.set(events.get().wrapping_add(1));
                changes.set(true);
            }
        })));
    let subscription = native.context.subscribe()?;
    native.wait(shared, |_| subscription.result.get().is_some())?;
    if subscription.result.get() != Some(true) {
        return Err(Failure::Device);
    }
    let mut capture = native.stream("Vults voice input", &route.source.name, true, shared)?;
    if capture.get_device_index() != Some(route.source.index) {
        return Err(Failure::Route);
    }
    capture = uncork(&mut native, capture, shared)?;
    native.capture = Some(capture);
    let underflow = Rc::new(Cell::new(false));
    let mut wire_written = 0;
    open_output(&mut native, route, shared, &underflow, &mut wire_written)?;
    let pinned = stable_graph(&mut native, route, None, shared, &revision, &dirty)?;
    *watch.borrow_mut() = Some(pinned.clone());
    dirty.set(false);
    let stream = native.playback.take().ok_or(Failure::Device)?;
    native.playback = Some(uncork(&mut native, stream, shared)?);
    stable_graph(&mut native, route, Some(&pinned), shared, &revision, &dirty)?;
    native.read(shared, false)?;
    shared.check()?;
    {
        let mut s = shared.state.lock().map_err(|_| Failure::State)?;
        s.ready = true;
        s.verified = true;
        s.output_live = true;
        s.checked = Instant::now();
    }
    let mut pending_graph = None;
    let mut requested_revision = revision.get();
    let mut checked = Instant::now();
    let mut timed = Instant::now();
    let mut drain: Option<raw::Ack> = None;
    let mut draining_since = None;
    loop {
        native.pump(shared)?;
        if pending_graph.is_none()
            && (dirty.replace(false) || checked.elapsed() >= Duration::from_millis(100))
        {
            requested_revision = revision.get();
            pending_graph = Some(graph::request(&native.context)?);
        }
        if let Some(graph) = &pending_graph
            && graph.graph.borrow().completed()?
        {
            if Pinned::from_graph(route, &graph.graph.borrow())? != pinned {
                return Err(Failure::Route);
            }
            if requested_revision == revision.get() && !dirty.get() {
                if !shared.state.lock().map_err(|_| Failure::State)?.verified {
                    native.read(shared, false)?;
                }
                let mut state = shared.state.lock().map_err(|_| Failure::State)?;
                state.checked = Instant::now();
                state.verified = true;
                checked = Instant::now();
            }
            pending_graph = None;
        }
        let publish = shared.state.lock().map_err(|_| Failure::State)?.verified;
        native.read(shared, publish)?;
        let output = shared.state.lock().map_err(|_| Failure::State)?.output.clone();
        if let Some(output) = &output {
            let active = output.lock().map_err(|_| Failure::State)?.active();
            if !active {
                if let Some(mut stream) = native.playback.take() {
                    stream.clear_callbacks();
                    stream.disconnect()?;
                    native.wait(shared, |_| stream.state() == StreamState::Terminated)?;
                }
                let mut state = shared.state.lock().map_err(|_| Failure::State)?;
                state.output = None;
                state.output_live = false;
                drain = None;
                draining_since = None;
            } else if native.playback.is_none() {
                open_output(&mut native, route, shared, &underflow, &mut wire_written)?;
                stable_graph(&mut native, route, Some(&pinned), shared, &revision, &dirty)?;
                let stream = native.playback.take().ok_or(Failure::Device)?;
                native.playback = Some(uncork(&mut native, stream, shared)?);
                stable_graph(&mut native, route, Some(&pinned), shared, &revision, &dirty)?;
                native.read(shared, false)?;
                shared.check()?;
                let mut s = shared.state.lock().map_err(|_| Failure::State)?;
                s.verified = true;
                s.output_live = true;
                s.checked = Instant::now();
                checked = Instant::now();
            }
        }
        let Some(stream) = native.playback.as_mut() else {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        };
        if stream.state() != StreamState::Ready || stream.get_device_index() != Some(route.sink.index) {
            return Err(Failure::Route);
        }
        if native.capture.as_ref().and_then(Stream::get_device_index) != Some(route.source.index) {
            return Err(Failure::Route);
        }
        if underflow.replace(false) && drain.is_none() {
            return Err(Failure::Clock);
        }
        if let Some(output) = &output {
            let mut s = output.lock().map_err(|_| Failure::State)?;
            if s.phase == Phase::Starting {
                s.phase = Phase::Playing;
            }
            if s.started.elapsed() > Duration::from_secs(150) {
                return Err(Failure::Timeout);
            }
            if let Some(info) = stream.timing() {
                let consumed = timing(&info, wire_written)?;
                s.advance(consumed);
            }
            if let Some(done) = &drain {
                if done.result.get() == Some(false) {
                    return Err(Failure::Device);
                }
                if done.result.get() == Some(true) {
                    s.drained = true;
                }
                let played = s.played;
                s.advance(0);
                debug_assert!(s.played >= played);
                if draining_since.is_some_and(|at: Instant| at.elapsed() > DEADLINE) {
                    return Err(Failure::Timeout);
                }
            }
            if s.eof && s.pcm.is_empty() && drain.is_none() {
                drain = Some(stream.drain()?);
                draining_since = Some(Instant::now());
            }
        }
        if drain.is_none() {
            let frames = (stream.writable_size().ok_or(Failure::Device)? / 2).min(RATE as usize / 50);
            if frames > 0 {
                let mut bytes = vec![0; frames * 2];
                if let Some(output) = &output {
                    let mut s = output.lock().map_err(|_| Failure::State)?;
                    if s.active() {
                        let count = frames.min(s.pcm.len());
                        if count > 0 {
                            if s.spans.len() >= 4096 {
                                return Err(Failure::Clock);
                            }
                            let before = s.submitted;
                            s.spans.push_back(Span {
                                wire: wire_written,
                                before,
                                frames: count as u64,
                            });
                            for target in bytes.as_chunks_mut::<2>().0.iter_mut().take(count) {
                                target
                                    .copy_from_slice(&s.pcm.pop_front().ok_or(Failure::State)?.to_le_bytes());
                            }
                            s.submitted += count as u64;
                        }
                        if shared.cancel.load(Ordering::Acquire) {
                            return Err(Failure::Cancelled);
                        }
                        stream.write_copy(&bytes).map_err(|_| Failure::Device)?;
                    } else {
                        continue;
                    }
                } else {
                    stream.write_copy(&bytes).map_err(|_| Failure::Device)?;
                }
                wire_written += frames as u64;
            }
        }
        if timed.elapsed() >= Duration::from_millis(10) {
            stream.update_timing()?;
            timed = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn route_module(p: &Pinned) -> Option<u32> {
    p.capture.as_ref().and_then(|(e, _)| e.module)
}
fn open_output(
    native: &mut Native,
    route: &Route,
    shared: &Arc<Shared>,
    underflow: &Rc<Cell<bool>>,
    wire: &mut u64,
) -> Result<(), Failure> {
    let mut stream = native.stream("Vults voice output", &route.sink.name, false, shared)?;
    if stream.get_device_index() != Some(route.sink.index) {
        return Err(Failure::Route);
    }
    let starved = underflow.clone();
    stream.on_underflow(Box::new(move || starved.set(true)));
    let bytes = vec![0; RATE as usize / 10 * 2];
    stream.write_copy(&bytes).map_err(|_| Failure::Device)?;
    *wire = RATE as u64 / 10;
    native.playback = Some(stream);
    underflow.set(false);
    Ok(())
}

fn stable_graph(
    native: &mut Native,
    route: &Route,
    expected: Option<&Pinned>,
    shared: &Shared,
    revision: &Cell<u64>,
    dirty: &Cell<bool>,
) -> Result<Pinned, Failure> {
    let start = Instant::now();
    loop {
        dirty.set(false);
        let before = revision.get();
        let graph = native.graph(shared)?;
        let pinned = Pinned::from_graph(route, &graph)?;
        if expected.is_some_and(|old| *old != pinned) {
            return Err(Failure::Route);
        }
        if before == revision.get() && !dirty.get() {
            return Ok(pinned);
        }
        native.read(shared, false)?;
        if start.elapsed() > DEADLINE {
            return Err(Failure::Timeout);
        }
    }
}

#[test]
fn corrupt_timing_and_device_latency_never_become_heard_content() {
    let mut info = raw::TimingInfo {
        read_index: 4800,
        write_index: 9600,
        sink_usec: 50_000,
        ..Default::default()
    };
    assert_eq!(timing(&info, 4800), Ok(1200));
    info.read_index_corrupt = 1;
    assert_eq!(timing(&info, 4800), Err(Failure::Clock));
    info.read_index_corrupt = 0;
    info.write_index_corrupt = 1;
    assert_eq!(timing(&info, 4800), Err(Failure::Clock));
    info.write_index_corrupt = 0;
    info.sink_usec = 2_000_001;
    assert_eq!(timing(&info, 4800), Err(Failure::Clock));
    info.sink_usec = 0;
    info.write_index = 9602;
    assert_eq!(timing(&info, 4800), Err(Failure::Clock));
}
