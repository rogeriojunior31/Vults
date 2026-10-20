//! Local silence completion never acquires a microphone or reopens a finished turn.
use super::*;
use std::sync::Mutex;
use vults_voice::{
    Preview, Vad,
    turn::{
        Generation,
        end::{Decision, EndOfTurn, Event as Edge, Language},
    },
};

struct Policy {
    end: EndOfTurn,
    generation: Generation,
    speaking: bool,
}
impl Policy {
    fn new(generation: Generation, language: Option<&str>) -> Self {
        let mut end = EndOfTurn::new(false);
        let language = match language {
            Some("en") => Language::English,
            Some("pt") => Language::Portuguese,
            _ => Language::Other,
        };
        assert!(end.begin(generation, language, Duration::ZERO));
        Self {
            end,
            generation,
            speaking: false,
        }
    }
    fn update(&mut self, speech: bool, text: Option<&str>, now: Duration) -> bool {
        if speech != self.speaking {
            self.speaking = speech;
            self.end.handle(
                self.generation,
                if speech {
                    Edge::SpeechStarted
                } else {
                    Edge::SpeechEnded
                },
                now,
            );
        }
        if let Some(text) = text
            && matches!(
                self.end.handle(self.generation, Edge::Transcript(text), now),
                Decision::End(_)
            )
        {
            return true;
        }
        matches!(
            self.end.handle(self.generation, Edge::Tick, now),
            Decision::End(_)
        )
    }
}
struct Automatic {
    stop: Arc<AtomicBool>,
    ended: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
}
impl Automatic {
    fn start(
        owner: Worker,
        work: impl FnOnce(&Worker, &AtomicBool) -> Result<bool, ()> + Send + 'static,
    ) -> Self {
        let automatic = Self {
            stop: Arc::default(),
            ended: Arc::default(),
            failed: Arc::default(),
        };
        let (stop, ended, failed) = (
            automatic.stop.clone(),
            automatic.ended.clone(),
            automatic.failed.clone(),
        );
        let spawned = std::thread::Builder::new()
            .name("voice-turn-end".into())
            .spawn(move || {
                // The actual detector worker retains the desktop lease through delayed native teardown.
                let result = work(&owner, &stop);
                if !stop.load(Ordering::SeqCst)
                    && owner.live().is_ok()
                    && *owner.end.borrow() == End::Recording
                {
                    match result {
                        Ok(value) => ended.store(value, Ordering::SeqCst),
                        Err(()) => failed.store(true, Ordering::SeqCst),
                    }
                }
            });
        if spawned.is_err() {
            automatic.failed.store(true, Ordering::SeqCst);
        }
        automatic
    }
}
impl Drop for Automatic {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}
pub(super) struct Recording {
    microphone: vults_audio::Capture,
    automatic: Option<Automatic>,
    requested: bool,
    reported: bool,
    owner: Worker,
}
impl Recording {
    pub(super) fn new(
        microphone: vults_audio::Capture,
        generation: Option<Generation>,
        language: Option<String>,
        owner: Worker,
        load: impl FnOnce() -> Result<Arc<vults_voice::Transcriber>, String> + Send + 'static,
    ) -> Self {
        let automatic = generation.map(|generation| {
            let audio = microphone.audio();
            Automatic::start(owner.clone(), move |owner, stop| {
                if stop.load(Ordering::SeqCst)
                    || owner.live().is_err()
                    || *owner.end.borrow() != End::Recording
                {
                    return Ok(false);
                }
                let path = vults_voice::vad_path(&crate::voice::models_dir()).ok_or(())?;
                if !vults_voice::gpu() {
                    return Err(());
                }
                let mut vad = Vad::load(&path).map_err(|_| ())?;
                if stop.load(Ordering::SeqCst)
                    || owner.live().is_err()
                    || *owner.end.borrow() != End::Recording
                {
                    return Ok(false);
                }
                let text = Arc::new(Mutex::new(None));
                let words = text.clone();
                let (loader, publisher) = (owner.clone(), owner.clone());
                let mut policy = Policy::new(generation, language.as_deref());
                let preview = Preview::start(
                    audio.clone(),
                    move || {
                        loader.live().map_err(|_| "Voice capture ended.".to_string())?;
                        load()
                    },
                    language,
                    true,
                    Vec::new,
                    move |text| {
                        // Both callbacks retain the lease until the actual preview worker exits.
                        if publisher.live().is_ok()
                            && let Ok(mut words) = words.lock()
                        {
                            *words = Some(text.chars().take(4096).collect::<String>());
                        }
                    },
                );
                let started = Instant::now();
                let mut read = 0;
                while !stop.load(Ordering::SeqCst)
                    && owner.live().is_ok()
                    && *owner.end.borrow() == End::Recording
                {
                    std::thread::sleep(Duration::from_millis(100));
                    if preview.failed() {
                        return Err(());
                    }
                    let frames = audio.len();
                    if frames <= read {
                        continue;
                    }
                    read = frames;
                    let pcm = audio.tail(3000);
                    let speech = vad.speech(&pcm).map_err(|_| ())?;
                    if preview.failed() {
                        return Err(());
                    }
                    // A busy decoder must not let an old quiet window finish newer speech.
                    if audio.len().saturating_sub(frames) > 3200 {
                        continue;
                    }
                    if stop.load(Ordering::SeqCst) || owner.live().is_err() {
                        return Ok(false);
                    }
                    let speaking = speech
                        .last()
                        .is_some_and(|range| range.end.saturating_add(3200) >= pcm.len());
                    let text = text.lock().ok().and_then(|mut text| text.take());
                    if policy.update(speaking, text.as_deref(), started.elapsed()) {
                        return Ok(true);
                    }
                }
                Ok(false)
            })
        });
        Self {
            microphone,
            automatic,
            requested: generation.is_some(),
            reported: false,
            owner,
        }
    }
    pub(super) fn ended(&mut self, _now: Duration) -> bool {
        if let Some(automatic) = &self.automatic {
            if !automatic.failed.load(Ordering::SeqCst) {
                return automatic.ended.load(Ordering::SeqCst);
            }
            self.automatic = None;
        }
        if self.requested && !self.reported {
            self.reported = true;
            let _ = self.owner.notify(CapturePhase::Manual);
        }
        false
    }
    pub(super) fn finish(mut self) -> Result<Vec<f32>, String> {
        // Signal the watchers without waiting on inference before closing the microphone.
        drop(self.automatic.take());
        self.microphone.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy(language: &str) -> Policy {
        let mut turn = vults_voice::turn::Turn::new(vults_voice::turn::BargeIn::Disabled);
        Policy::new(turn.start(Duration::ZERO).unwrap().generation, Some(language))
    }
    #[test]
    fn a_delayed_detector_cannot_delay_finish_or_publish_after_departure_and_keeps_its_lease() {
        for cancel in [false, true] {
            let (control, owner, _events, _cancel, state) = super::super::tests::fixture();
            let (entered, waiting) = std::sync::mpsc::channel();
            let (release, released) = std::sync::mpsc::channel();
            let (exit, exited) = std::sync::mpsc::channel();
            let automatic = Automatic::start(owner, move |_, _| {
                entered.send(()).unwrap();
                released.recv().unwrap();
                exit.send(()).unwrap();
                Ok(true)
            });
            waiting.recv_timeout(Duration::from_secs(1)).unwrap();
            let ended = automatic.ended.clone();
            if cancel {
                control.cancel();
            } else {
                control.finish();
            }
            let (returned, finished) = std::sync::mpsc::channel();
            let dropper = std::thread::spawn(move || {
                drop(automatic);
                let _ = returned.send(());
            });
            let responsive = finished.recv_timeout(Duration::from_secs(1)).is_ok();
            assert!(!ended.load(Ordering::SeqCst));
            assert_eq!(state.begin().err(), Some(Failure::Busy));
            release.send(()).unwrap();
            dropper.join().unwrap();
            assert!(responsive, "Finish must not wait for detector teardown");
            exited.recv_timeout(Duration::from_secs(1)).unwrap();
            let deadline = Instant::now() + Duration::from_secs(1);
            while state.running.load(Ordering::SeqCst) && Instant::now() < deadline {
                std::thread::yield_now();
            }
            assert!(!state.running.load(Ordering::SeqCst));
            assert!(!ended.load(Ordering::SeqCst));
        }
    }

    #[test]
    fn local_words_and_confirmed_silence_are_both_required_and_completion_is_once_only() {
        let ms = Duration::from_millis;
        let mut end = policy("en");
        assert!(!end.update(false, Some("hallucinated quiet"), ms(100)));
        assert!(!end.update(false, None, ms(4000)));
        assert!(!end.update(true, Some("actual speech"), ms(4100)));
        assert!(!end.update(false, None, ms(4200)));
        assert!(!end.update(false, None, ms(5799)));
        assert!(end.update(false, None, ms(5800)));
        assert!(!end.update(false, None, ms(5900)));
        let mut empty = policy("en");
        assert!(!empty.update(true, None, ms(100)));
        assert!(!empty.update(false, None, ms(200)));
        assert!(!empty.update(false, None, ms(8000)));
    }
    #[test]
    fn continuation_words_growth_and_new_speech_keep_the_existing_language_policy() {
        let ms = Duration::from_millis;
        for (language, text) in [("pt", "eu penso que"), ("en", "I think that and")] {
            let mut end = policy(language);
            assert!(!end.update(true, Some(text), ms(100)));
            assert!(!end.update(false, None, ms(200)));
            assert!(!end.update(false, Some(text), ms(1800)));
            assert!(!end.update(false, None, ms(2799)));
            assert!(end.update(false, None, ms(2800)));
        }
        let mut end = policy("en");
        assert!(!end.update(true, Some("hello"), ms(100)));
        assert!(!end.update(false, None, ms(200)));
        assert!(!end.update(false, Some("hello again"), ms(1500)));
        assert!(!end.update(false, None, ms(1800)));
        assert!(!end.update(true, None, ms(3000)));
        assert!(!end.update(false, None, ms(3100)));
        assert!(!end.update(false, None, ms(4699)));
        assert!(end.update(false, None, ms(4700)));
    }
}
