//! One explicitly authorized output, with a device-clock estimate distinct from queued audio.
use crate::resample::Converter;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub const RATE: u32 = 24_000;
const MAX_FRAMES: u64 = RATE as u64 * 120;
const START_TIMEOUT: Duration = Duration::from_secs(3);
const LIFETIME: Duration = Duration::from_secs(150);
const MAX_LATENCY: Duration = Duration::from_secs(2);
const MAX_MARKS: usize = 4096;
static BUSY: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Cancelled,
    Busy,
    Input,
    State,
    Device,
    Clock,
    Timeout,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Playing,
    Draining,
    Finished,
    Stopped,
    Failed,
}
#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub phase: Phase,
    /// Accepted source frames, including audio still waiting in the resampler/queue.
    pub total_frames: u64,
    pub submitted_frames: u64,
    /// Estimated device consumption; never wall time since the producer started.
    pub played_frames: u64,
    pub first_played: Option<Instant>,
    pub failure: Option<Failure>,
}

struct Mark {
    at: Instant,
    before: u64,
    frames: u64,
}
struct Queue {
    phase: Phase,
    failure: Option<Failure>,
    rate: u32,
    pcm: VecDeque<f32>,
    marks: VecDeque<Mark>,
    total: u64,
    submitted: u64,
    played: u64,
    first: Option<Instant>,
    eof: bool,
}
impl Queue {
    fn new(rate: u32) -> Self {
        Self {
            phase: Phase::Starting,
            failure: None,
            rate,
            pcm: VecDeque::new(),
            marks: VecDeque::new(),
            total: 0,
            submitted: 0,
            played: 0,
            first: None,
            eof: false,
        }
    }
    fn active(&self) -> bool {
        matches!(self.phase, Phase::Starting | Phase::Playing | Phase::Draining)
    }
    fn advance(&mut self, now: Instant) {
        if !self.active() {
            return;
        }
        while let Some(mark) = self.marks.front() {
            let Some(elapsed) = now.checked_duration_since(mark.at) else {
                break;
            };
            let frames = (elapsed.as_nanos() * u128::from(self.rate) / 1_000_000_000)
                .min(u128::from(mark.frames)) as u64;
            if frames > 0 {
                self.first.get_or_insert(mark.at);
            }
            self.played = self.played.max(mark.before + frames);
            if frames < mark.frames {
                break;
            }
            self.marks.pop_front();
        }
        if self.eof && self.pcm.is_empty() && self.marks.is_empty() {
            self.phase = Phase::Finished;
        }
    }
    fn halt(&mut self, phase: Phase, failure: Option<Failure>, now: Instant) {
        if !self.active() {
            return;
        }
        if phase != Phase::Failed {
            self.advance(now);
        }
        if self.phase != Phase::Finished {
            self.phase = phase;
            self.failure = failure;
        }
        self.pcm.clear();
        self.marks.clear();
    }
    fn cursor(&self) -> Cursor {
        let source = |frames: u64| frames.saturating_mul(u64::from(RATE)) / u64::from(self.rate);
        Cursor {
            phase: self.phase,
            total_frames: self.total,
            submitted_frames: source(self.submitted).min(self.total),
            played_frames: if self.phase == Phase::Finished && self.first.is_some() {
                self.total
            } else {
                source(self.played).min(self.total)
            },
            first_played: self.first,
            failure: self.failure,
        }
    }
}
struct Producer {
    converter: Option<Converter>,
}
struct Shared {
    queue: Mutex<Queue>,
    producer: Mutex<Producer>,
    cancel: Arc<AtomicBool>,
    stopped: AtomicBool,
    created: Instant,
}
impl Shared {
    fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire) || self.cancel.load(Ordering::Acquire)
    }
    fn fail(&self, failure: Failure) {
        self.stopped.store(true, Ordering::Release);
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).halt(
            Phase::Failed,
            Some(failure),
            Instant::now(),
        );
    }
    fn status(&self, now: Instant) -> Cursor {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        if self.stopped() {
            queue.halt(Phase::Stopped, None, now);
        } else if (queue.phase == Phase::Starting && now.duration_since(self.created) >= START_TIMEOUT)
            || now.duration_since(self.created) >= LIFETIME
        {
            self.stopped.store(true, Ordering::Release);
            queue.halt(Phase::Failed, Some(Failure::Timeout), now);
        } else {
            queue.advance(now);
        }
        queue.cursor()
    }
}

/// Starts no microphone and writes no audio file. The caller owns explicit consent/revocation.
pub struct Playback {
    shared: Arc<Shared>,
}
impl std::fmt::Debug for Playback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playback")
            .field("cursor", &self.cursor())
            .finish()
    }
}
impl Playback {
    /// Device setup runs on its own worker. Poll cursor until Playing before pushing samples.
    pub fn start(cancel: Arc<AtomicBool>) -> Result<Self, Failure> {
        Self::start_with_owner(cancel, Arc::new(()))
    }
    /// Retain the caller's native lease through blocked setup and device destruction.
    pub fn start_with_owner<T: Send + Sync + 'static>(
        cancel: Arc<AtomicBool>,
        lease: Arc<T>,
    ) -> Result<Self, Failure> {
        if cancel.load(Ordering::Acquire) {
            return Err(Failure::Cancelled);
        }
        if BUSY.swap(true, Ordering::AcqRel) {
            return Err(Failure::Busy);
        }
        let owner = Owner;
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::new(RATE)),
            producer: Mutex::new(Producer { converter: None }),
            cancel,
            stopped: AtomicBool::new(false),
            created: Instant::now(),
        });
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("voice-playback".into())
            .spawn(move || {
                let _owner = owner;
                let _lease = lease;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&worker)))
                    .unwrap_or(Err(Failure::Device));
                if let Err(failure) = result {
                    worker.fail(failure);
                }
            })
            .map_err(|_| Failure::Device)?;
        Ok(Self { shared })
    }
    /// Blocking conversion, off the audio callback; one chunk is at most one second of PCM16.
    pub fn push(&self, pcm: &[i16]) -> Result<(), Failure> {
        if pcm.is_empty() || pcm.len() > RATE as usize {
            return Err(Failure::Input);
        }
        let mut producer = self.shared.producer.lock().map_err(|_| Failure::State)?;
        {
            let queue = self.shared.queue.lock().map_err(|_| Failure::State)?;
            if self.shared.stopped() {
                return Err(Failure::Cancelled);
            }
            if queue.phase != Phase::Playing {
                return Err(Failure::State);
            }
            if queue.total.saturating_add(pcm.len() as u64) > MAX_FRAMES {
                return Err(Failure::Input);
            }
        }
        let input: Vec<f32> = pcm.iter().map(|s| f32::from(*s) / 32768.0).collect();
        let out = producer
            .converter
            .as_mut()
            .ok_or(Failure::State)?
            .push(&input)
            .map_err(|_| {
                self.shared.fail(Failure::Device);
                Failure::Device
            })?;
        let mut queue = self.shared.queue.lock().map_err(|_| Failure::State)?;
        if self.shared.stopped() || queue.phase != Phase::Playing {
            return Err(Failure::Cancelled);
        }
        queue.total += pcm.len() as u64;
        queue.pcm.extend(out);
        Ok(())
    }
    /// Producer completion is separate from cursor Phase::Finished (device drain).
    pub fn finish(&self) -> Result<(), Failure> {
        let mut producer = self.shared.producer.lock().map_err(|_| Failure::State)?;
        {
            let queue = self.shared.queue.lock().map_err(|_| Failure::State)?;
            if self.shared.stopped() {
                return Err(Failure::Cancelled);
            }
            if queue.phase != Phase::Playing || queue.total == 0 {
                return Err(Failure::State);
            }
        }
        let tail = producer
            .converter
            .take()
            .ok_or(Failure::State)?
            .finish()
            .map_err(|_| {
                self.shared.fail(Failure::Device);
                Failure::Device
            })?;
        let mut queue = self.shared.queue.lock().map_err(|_| Failure::State)?;
        if self.shared.stopped() || queue.phase != Phase::Playing {
            return Err(Failure::Cancelled);
        }
        queue.pcm.extend(tail);
        queue.eof = true;
        queue.phase = Phase::Draining;
        Ok(())
    }
    pub fn cursor(&self) -> Cursor {
        self.shared.status(Instant::now())
    }
    /// Freezes consumption before clearing queued output. Never waits for device destruction.
    pub fn stop(&self) -> Cursor {
        self.shared.stopped.store(true, Ordering::Release);
        let mut queue = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.halt(Phase::Stopped, None, Instant::now());
        queue.cursor()
    }
}
impl Drop for Playback {
    fn drop(&mut self) {
        self.stop();
    }
}
struct Owner;
impl Drop for Owner {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}

fn output_config(
    configs: impl Iterator<Item = cpal::SupportedStreamConfigRange>,
) -> Option<cpal::SupportedStreamConfig> {
    let configs: Vec<_> = configs
        .take(256)
        .filter(|c| (1..=8).contains(&c.channels()))
        .collect();
    for rate in [48_000, 24_000, 44_100, 96_000, 16_000, 8_000] {
        for format in [
            cpal::SampleFormat::F32,
            cpal::SampleFormat::I16,
            cpal::SampleFormat::U16,
        ] {
            for channels in [2, 1, 3, 4, 5, 6, 7, 8] {
                if let Some(c) = configs.iter().find(|c| {
                    c.channels() == channels
                        && c.sample_format() == format
                        && c.min_sample_rate() <= rate
                        && c.max_sample_rate() >= rate
                }) {
                    return Some(c.with_sample_rate(rate));
                }
            }
        }
    }
    None
}
fn run(shared: &Arc<Shared>) -> Result<(), Failure> {
    if shared.stopped() {
        return Err(Failure::Cancelled);
    }
    let device = cpal::default_host()
        .default_output_device()
        .ok_or(Failure::Device)?;
    let config = output_config(device.supported_output_configs().map_err(|_| Failure::Device)?)
        .ok_or(Failure::Device)?;
    let converter = Converter::at_rate(RATE, config.sample_rate()).map_err(|_| Failure::Device)?;
    if shared.stopped() {
        return Err(Failure::Cancelled);
    }
    if shared.created.elapsed() >= START_TIMEOUT {
        return Err(Failure::Timeout);
    }
    shared.queue.lock().map_err(|_| Failure::State)?.rate = config.sample_rate();
    shared.producer.lock().map_err(|_| Failure::State)?.converter = Some(converter);
    let mut native = config.config();
    if let cpal::SupportedBufferSize::Range { min, max } = *config.buffer_size() {
        // Some devices default to single-frame callbacks; prefer a supported 10 ms period.
        native.buffer_size = cpal::BufferSize::Fixed((native.sample_rate / 100).clamp(min, max));
    }
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => stream::<f32>(&device, native, shared.clone()),
        cpal::SampleFormat::I16 => stream::<i16>(&device, native, shared.clone()),
        cpal::SampleFormat::U16 => stream::<u16>(&device, native, shared.clone()),
        _ => Err(Failure::Device),
    }?;
    if shared.stopped() {
        return Err(Failure::Cancelled);
    }
    if shared.created.elapsed() >= START_TIMEOUT {
        return Err(Failure::Timeout);
    }
    stream.play().map_err(|_| Failure::Device)?;
    {
        let mut queue = shared.queue.lock().map_err(|_| Failure::State)?;
        if shared.stopped() {
            return Err(Failure::Cancelled);
        }
        if shared.created.elapsed() >= START_TIMEOUT {
            return Err(Failure::Timeout);
        }
        queue.phase = Phase::Playing;
    }
    loop {
        let status = shared.status(Instant::now());
        if !matches!(status.phase, Phase::Playing | Phase::Draining) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(stream);
    Ok(())
}
fn stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, Failure>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let (channels, rate) = (usize::from(config.channels), config.sample_rate);
    let errors = shared.clone();
    device
        .build_output_stream::<T, _, _>(
            config,
            move |data, info| {
                data.fill(T::EQUILIBRIUM);
                let stamp = info.timestamp();
                let Some(delay) = stamp.playback.checked_duration_since(stamp.callback) else {
                    shared.fail(Failure::Clock);
                    return;
                };
                render(data, channels, rate, &shared, Instant::now(), delay);
            },
            move |_| errors.fail(Failure::Device),
            Some(START_TIMEOUT),
        )
        .map_err(|_| Failure::Device)
}
fn render<T: cpal::Sample + cpal::FromSample<f32>>(
    data: &mut [T],
    channels: usize,
    rate: u32,
    shared: &Shared,
    now: Instant,
    delay: Duration,
) {
    data.fill(T::EQUILIBRIUM);
    let Ok(mut queue) = shared.queue.try_lock() else {
        return;
    };
    if shared.stopped() {
        queue.halt(Phase::Stopped, None, now);
        return;
    }
    if !matches!(queue.phase, Phase::Playing | Phase::Draining) {
        return;
    }
    if channels == 0
        || !data.len().is_multiple_of(channels)
        || data.len() / channels > rate as usize
        || delay > MAX_LATENCY
    {
        queue.halt(Phase::Failed, Some(Failure::Clock), now);
        return;
    }
    queue.advance(now);
    let frames = (data.len() / channels).min(queue.pcm.len());
    if frames == 0 {
        return;
    }
    if queue.marks.len() >= MAX_MARKS {
        queue.halt(Phase::Failed, Some(Failure::Clock), now);
        return;
    }
    // Callback scheduling jitter cannot move a later block ahead of already submitted frames.
    let at = queue.marks.back().map_or(now + delay, |mark| {
        (now + delay).max(mark.at + frame_time(mark.frames, rate))
    });
    if at.duration_since(now) > MAX_LATENCY {
        queue.halt(Phase::Failed, Some(Failure::Clock), now);
        return;
    }
    for frame in data.chunks_exact_mut(channels).take(frames) {
        let sample = queue.pcm.pop_front().unwrap_or(0.0);
        frame.fill(T::from_sample(sample));
    }
    let before = queue.submitted;
    queue.marks.push_back(Mark {
        at,
        before,
        frames: frames as u64,
    });
    queue.submitted += frames as u64;
}

fn frame_time(frames: u64, rate: u32) -> Duration {
    Duration::from_nanos((frames * 1_000_000_000).div_ceil(u64::from(rate)))
}

#[cfg(test)]
mod tests;
