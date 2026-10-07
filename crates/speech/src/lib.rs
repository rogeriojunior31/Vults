//! Zeca speaks his chat replies. The reply is cut into sentences as it streams; each one is
//! synthesized on this computer (Kokoro-82M through ONNX Runtime) while the one before it plays,
//! so he starts talking before the reply ends. Code, commands and links are never read aloud,
//! and a stop is heard at once: the queue is dropped and the sentence being made is abandoned.

mod chunker;
mod espeak;
mod g2p;
mod kokoro;
mod lang;
mod models;
mod player;
mod prose;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

pub use espeak::{MISSING as ESPEAK_MISSING, find as espeak};
pub use g2p::G2p;
pub use kokoro::Kokoro;
pub use lang::{Lang, detect};
pub use models::{VOICES, Voice, download, installed, model_path, size, voice_for, voice_path};
pub use player::{Player, Sink};

/// Makes a sentence's audio. The real one is [`Engine`]; tests use their own.
pub trait Synth: Send {
    /// 24 kHz mono audio for `text`, in `lang`, with the voice `voice`. `stale` turns true once
    /// the sentence is no longer wanted: checking it right before the heavy work saves the work.
    fn speak(
        &mut self,
        text: &str,
        lang: Lang,
        voice: &str,
        stale: &dyn Fn() -> bool,
    ) -> Result<Vec<f32>, String>;
}

/// Kokoro, misaki and the voices on disk.
pub struct Engine {
    kokoro: Kokoro,
    g2p: G2p,
    voices: HashMap<&'static str, Vec<f32>>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Engine")
    }
}

impl Engine {
    /// Seconds: the model, misaki's lexicon and the voices. `espeak` None finds it on the PATH.
    pub fn load(dir: &Path, espeak: Option<std::path::PathBuf>) -> Result<Self, String> {
        let kokoro = Kokoro::load(&model_path(dir), threads())?;
        let mut voices = HashMap::new();
        for v in VOICES {
            voices.insert(v.id, kokoro::voice(&voice_path(dir, v))?);
        }
        Ok(Self {
            kokoro,
            g2p: G2p::new(espeak),
            voices,
        })
    }

    /// Ends the synthesis under way, from any thread.
    pub fn canceller(&self) -> impl Fn() + Send + Sync + 'static {
        self.kokoro.canceller()
    }
}

impl Synth for Engine {
    fn speak(
        &mut self,
        text: &str,
        lang: Lang,
        voice: &str,
        stale: &dyn Fn() -> bool,
    ) -> Result<Vec<f32>, String> {
        let phonemes = self.g2p.phonemes(text, lang)?;
        let style = self
            .voices
            .get(voice_for(lang, Some(voice)).id)
            .ok_or("the voice is missing")?;
        self.kokoro.speak(&phonemes, style, stale)
    }
}

fn threads() -> usize {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    // Half the cores, as for whisper: the agents and the desktop keep running meanwhile.
    (n / 2).clamp(1, 8)
}

/// One sentence to say.
struct Job {
    generation: u64,
    text: String,
    lang: Lang,
    voice: String,
}

/// The reply being spoken.
#[derive(Default)]
struct Reply {
    prose: prose::Prose,
    chunker: chunker::Chunker,
    /// The reply's language once a sentence told it.
    lang: Option<Lang>,
    /// When no sentence tells: what the user speaks.
    fallback: Option<Lang>,
    /// Stopped (or done): the rest of this reply is never said, until the next one begins. The
    /// filter keeps its place too, so a stop inside a code block never turns code into prose.
    muted: bool,
}

struct Shared {
    /// Bumped by every stop: a job of an older generation is dropped unspoken.
    generation: AtomicU64,
    /// Sentences queued or being made.
    pending: AtomicUsize,
    speaking: AtomicBool,
    sink: Arc<dyn Sink>,
    /// Held while a stop bumps the generation and empties the sink, and while audio is queued:
    /// a sentence checked as current is never played after a stop.
    gate: Mutex<()>,
    cancel: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
    on_change: Box<dyn Fn(bool) + Send + Sync>,
}

impl Shared {
    fn set_speaking(&self, on: bool) {
        if self.speaking.swap(on, Ordering::SeqCst) != on {
            (self.on_change)(on);
        }
    }
}

/// Speaks replies as they stream. Dropping it stops its threads.
pub struct Speaker {
    shared: Arc<Shared>,
    reply: Mutex<Reply>,
    /// Language → the voice id chosen for it.
    voices: Mutex<HashMap<Lang, String>>,
    jobs: Mutex<mpsc::Sender<Job>>,
}

impl std::fmt::Debug for Speaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Speaker")
    }
}

impl Speaker {
    /// `load` runs on the speaker's own thread (it takes seconds); sentences that come meanwhile
    /// wait for it, and if it fails they are dropped. `on_change` hears when Zeca starts and stops
    /// talking.
    pub fn start(
        load: impl FnOnce() -> Result<(Box<dyn Synth>, Box<dyn Fn() + Send + Sync>), String> + Send + 'static,
        sink: Arc<dyn Sink>,
        on_change: impl Fn(bool) + Send + Sync + 'static,
    ) -> Self {
        let shared = Arc::new(Shared {
            generation: AtomicU64::new(0),
            pending: AtomicUsize::new(0),
            speaking: AtomicBool::new(false),
            sink,
            gate: Mutex::new(()),
            cancel: Mutex::new(None),
            on_change: Box::new(on_change),
        });
        let (jobs, queue) = mpsc::channel::<Job>();
        let worker = shared.clone();
        let _ = std::thread::Builder::new()
            .name("speech".into())
            .spawn(move || work(load, &worker, &queue));
        let watcher = Arc::downgrade(&shared);
        let _ = std::thread::Builder::new()
            .name("speech-watch".into())
            .spawn(move || watch(&watcher));
        Self {
            shared,
            reply: Mutex::default(),
            voices: Mutex::default(),
            jobs: Mutex::new(jobs),
        }
    }

    /// The voice for a language (an id from [`VOICES`]).
    pub fn set_voice(&self, lang: Lang, id: &str) {
        self.voices
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(lang, id.to_string());
    }

    /// A new reply is coming: whatever was being said stops. `fallback` is the language for
    /// sentences that do not tell theirs.
    pub fn begin(&self, fallback: Lang) {
        let mut reply = self.reply.lock().unwrap_or_else(|e| e.into_inner());
        self.silence();
        *reply = Reply {
            fallback: Some(fallback),
            ..Reply::default()
        };
    }

    /// More of the reply.
    pub fn hear(&self, text: &str) {
        let mut reply = self.reply.lock().unwrap_or_else(|e| e.into_inner());
        if reply.muted {
            return;
        }
        let prose = reply.prose.push(text);
        let sentences = reply.chunker.push(&prose);
        self.queue(&mut reply, sentences);
    }

    /// The reply is complete: the rest of it is said.
    pub fn finish(&self) {
        let mut reply = self.reply.lock().unwrap_or_else(|e| e.into_inner());
        if reply.muted {
            return;
        }
        let prose = reply.prose.finish();
        let mut sentences = reply.chunker.push(&prose);
        sentences.extend(reply.chunker.flush());
        self.queue(&mut reply, sentences);
        reply.muted = true;
    }

    /// Silence, now: the queue is dropped, the sentence being made is abandoned, and the reply's
    /// rest is never said, however much of it still streams in.
    pub fn stop(&self) {
        // Under the reply's lock: a piece being queued now goes with the old generation.
        let mut reply = self.reply.lock().unwrap_or_else(|e| e.into_inner());
        reply.muted = true;
        self.silence();
    }

    fn silence(&self) {
        let gate = self.shared.gate.lock().unwrap_or_else(|e| e.into_inner());
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
        self.shared.sink.clear();
        drop(gate);
        if let Some(cancel) = self
            .shared
            .cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            cancel();
        }
        self.shared.set_speaking(false);
    }

    pub fn speaking(&self) -> bool {
        self.shared.speaking.load(Ordering::SeqCst)
    }

    fn queue(&self, reply: &mut Reply, sentences: Vec<String>) {
        let generation = self.shared.generation.load(Ordering::SeqCst);
        let voices = self.voices.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        for sentence in sentences {
            let sentence = prose::without_urls(&sentence);
            if !sentence.chars().any(char::is_alphanumeric) {
                continue;
            }
            // Each sentence in the language it is written in; one that does not tell keeps the
            // reply's, else the user's.
            let lang = match detect(&sentence) {
                Some(l) => *reply.lang.insert(l),
                None => reply.lang.or(reply.fallback).unwrap_or(Lang::En),
            };
            let voice = voices.get(&lang).cloned().unwrap_or_default();
            for text in chunker::pieces(&sentence) {
                self.shared.pending.fetch_add(1, Ordering::SeqCst);
                let job = Job {
                    generation,
                    text,
                    lang,
                    voice: voice.clone(),
                };
                if jobs.send(job).is_err() {
                    self.shared.pending.fetch_sub(1, Ordering::SeqCst);
                }
            }
        }
    }
}

impl Drop for Speaker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn work(
    load: impl FnOnce() -> Result<(Box<dyn Synth>, Box<dyn Fn() + Send + Sync>), String>,
    shared: &Shared,
    queue: &mpsc::Receiver<Job>,
) {
    let mut synth = match load() {
        Ok((synth, cancel)) => {
            *shared.cancel.lock().unwrap_or_else(|e| e.into_inner()) = Some(cancel);
            Some(synth)
        }
        Err(e) => {
            tracing::warn!("speech: {e}");
            None
        }
    };
    // Ends when the speaker is dropped (the channel closes).
    while let Ok(job) = queue.recv() {
        let stale = || shared.generation.load(Ordering::SeqCst) != job.generation;
        if !stale()
            && let Some(synth) = synth.as_mut()
        {
            match synth.speak(&job.text, job.lang, &job.voice, &stale) {
                Ok(pcm) => {
                    let _gate = shared.gate.lock().unwrap_or_else(|e| e.into_inner());
                    if !stale() {
                        shared.sink.play(pcm);
                        shared.set_speaking(true);
                    }
                }
                Err(e) if !stale() => tracing::info!("speech: a sentence was not spoken: {e}"),
                Err(_) => {}
            }
        }
        shared.pending.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Zeca stops talking once the audio has played and nothing more is coming.
fn watch(shared: &std::sync::Weak<Shared>) {
    while let Some(shared) = shared.upgrade() {
        let gate = shared.gate.lock().unwrap_or_else(|e| e.into_inner());
        if shared.speaking.load(Ordering::SeqCst)
            && !shared.sink.playing()
            && shared.pending.load(Ordering::SeqCst) == 0
        {
            shared.set_speaking(false);
        }
        drop(gate);
        drop(shared);
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests;
