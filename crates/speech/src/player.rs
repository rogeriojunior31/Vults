//! The speakers. cpal's stream is not `Send` on every backend, so it lives on a thread of its
//! own; it opens when there is something to say and closes after a quiet while, so an idle app
//! holds no audio device.

use std::collections::VecDeque;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::kokoro::SAMPLE_RATE;

/// Where the audio goes. The real one is [`Player`]; tests use their own.
pub trait Sink: Send + Sync {
    /// Queues 24 kHz mono audio after what is already queued.
    fn play(&self, pcm: Vec<f32>);
    /// Drops everything queued, at once.
    fn clear(&self);
    /// Audio is queued or playing.
    fn playing(&self) -> bool;
}

/// After this long with nothing to play, the device is let go.
const LINGER: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct Player {
    queue: Arc<Mutex<VecDeque<f32>>>,
    wake: Mutex<mpsc::Sender<()>>,
}

impl Player {
    pub fn new() -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let (wake, woken) = mpsc::channel();
        let shared = queue.clone();
        // Ends when the player is dropped (the channel closes).
        let _ = std::thread::Builder::new()
            .name("speech-player".into())
            .spawn(move || run(&shared, &woken));
        Self {
            queue,
            wake: Mutex::new(wake),
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Sink for Player {
    fn play(&self, pcm: Vec<f32>) {
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).extend(pcm);
        let _ = self.wake.lock().unwrap_or_else(|e| e.into_inner()).send(());
    }

    fn clear(&self) {
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    fn playing(&self) -> bool {
        !self.queue.lock().unwrap_or_else(|e| e.into_inner()).is_empty()
    }
}

fn run(queue: &Arc<Mutex<VecDeque<f32>>>, woken: &mpsc::Receiver<()>) {
    let mut stream: Option<cpal::Stream> = None;
    let mut quiet_since = Instant::now();
    loop {
        match woken.recv_timeout(Duration::from_millis(250)) {
            Err(RecvTimeoutError::Disconnected) => return,
            Ok(()) if stream.is_none() => match open(queue.clone()) {
                Ok(s) => stream = Some(s),
                Err(e) => {
                    // Nowhere to play it: drop it, so Zeca does not look like he is speaking.
                    tracing::warn!("speech: {e}");
                    queue.lock().unwrap_or_else(|e| e.into_inner()).clear();
                }
            },
            _ => {}
        }
        if !queue.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
            quiet_since = Instant::now();
        } else if stream.is_some() && quiet_since.elapsed() > LINGER {
            stream = None;
        }
    }
}

fn open(queue: Arc<Mutex<VecDeque<f32>>>) -> Result<cpal::Stream, String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no speakers found")?;
    let config = device
        .default_output_config()
        .map_err(|e| format!("can't read the speakers' format: {e}"))?;
    let channels = usize::from(config.channels().max(1));
    let mut resample = Resampler::new(config.sample_rate());
    let mut fill = move |out: &mut dyn ExactSizeIterator<Item = &mut f32>| {
        let mut q = queue.lock().unwrap_or_else(|e| e.into_inner());
        let mut sample = 0.0;
        for (i, s) in out.enumerate() {
            // The same sample on every channel of a frame.
            if i % channels == 0 {
                sample = resample.next(&mut q);
            }
            *s = sample;
        }
    };
    let err = |e: cpal::Error| tracing::warn!("speakers: {e}");
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream::<f32, _, _>(
            config.into(),
            move |d, _| fill(&mut d.iter_mut()),
            err,
            None,
        ),
        cpal::SampleFormat::I16 => {
            let mut buf = Vec::new();
            device.build_output_stream::<i16, _, _>(
                config.into(),
                move |d: &mut [i16], _| {
                    buf.resize(d.len(), 0.0f32);
                    fill(&mut buf.iter_mut());
                    for (o, s) in d.iter_mut().zip(&buf) {
                        *o = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
                    }
                },
                err,
                None,
            )
        }
        other => return Err(format!("the speakers' format ({other}) is not supported")),
    }
    .map_err(|e| format!("can't open the speakers: {e}"))?;
    stream
        .play()
        .map_err(|e| format!("can't start the speakers: {e}"))?;
    Ok(stream)
}

/// Kokoro's 24 kHz into the device's rate, linearly: speech needs nothing finer.
#[derive(Debug)]
struct Resampler {
    step: f64,
    at: f64,
    a: f32,
    b: f32,
}

impl Resampler {
    fn new(rate: u32) -> Self {
        Self {
            step: f64::from(SAMPLE_RATE) / f64::from(rate.max(1)),
            at: 0.0,
            a: 0.0,
            b: 0.0,
        }
    }

    /// The next output sample; silence once the queue runs dry.
    fn next(&mut self, q: &mut VecDeque<f32>) -> f32 {
        while self.at >= 1.0 {
            self.at -= 1.0;
            self.a = self.b;
            self.b = q.pop_front().unwrap_or(0.0);
        }
        let s = self.a + (self.b - self.a) * self.at as f32;
        self.at += self.step;
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_is_resampled_to_the_device() {
        // Twice the rate: every sample comes out twice (half-way points in between).
        let mut q: VecDeque<f32> = (1..=4).map(|i| i as f32).collect();
        let mut r = Resampler::new(48_000);
        let out: Vec<f32> = (0..10).map(|_| r.next(&mut q)).collect();
        assert_eq!(out, [0.0, 0.0, 0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5]);
        // Same rate: one in, one out.
        let mut q: VecDeque<f32> = (1..=100).map(|i| i as f32).collect();
        let mut r = Resampler::new(SAMPLE_RATE);
        for _ in 0..50 {
            r.next(&mut q);
        }
        assert_eq!(q.len(), 51);
    }
}
