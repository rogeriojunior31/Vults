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
    end: Arc<Mutex<EndOfTurn>>,
    generation: Generation,
    speaking: bool,
    semantic: Option<Arc<semantic::Service>>,
    job: Option<semantic::Job>,
    waiting: Option<vults_voice::turn::end::Ticket>,
    clock: Instant,
}
impl Policy {
    fn new(generation: Generation, language: Option<&str>, semantic: Option<Arc<semantic::Service>>) -> Self {
        let now = semantic
            .as_ref()
            .map_or(Duration::ZERO, |service| service.clock.elapsed());
        Self::at(generation, language, semantic, now)
    }
    fn at(
        generation: Generation,
        language: Option<&str>,
        semantic: Option<Arc<semantic::Service>>,
        now: Duration,
    ) -> Self {
        let clock = semantic
            .as_ref()
            .map_or_else(Instant::now, |service| service.clock);
        let end = semantic.as_ref().map_or_else(
            || Arc::new(Mutex::new(EndOfTurn::new(false))),
            |service| service.policy.clone(),
        );
        let language = match language {
            Some("en") => Language::English,
            Some("pt") => Language::Portuguese,
            _ => Language::Other,
        };
        assert!(
            end.lock()
                .unwrap_or_else(|e| e.into_inner())
                .begin(generation, language, now)
        );
        Self {
            end,
            generation,
            speaking: false,
            semantic,
            job: None,
            waiting: None,
            clock,
        }
    }
    fn edge(&self, event: Edge<'_>, now: Duration) -> Decision {
        self.end
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .handle(self.generation, event, now)
    }
    fn deadline(&self, ticket: vults_voice::turn::end::Ticket) -> Option<Duration> {
        self.end
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .semantic_deadline(ticket)
    }
    #[cfg(test)]
    fn update(&mut self, speech: bool, text: Option<&str>, now: Duration) -> bool {
        self.update_audio(speech, text, now, Vec::new)
    }
    fn decision(&mut self, decision: Decision, now: Duration) -> bool {
        match decision {
            Decision::End(reason) => {
                if (self.job.as_ref().is_some_and(|job| self.clock + now >= job.until)
                    || matches!(reason, vults_voice::turn::end::Reason::HoldLimit) && self.waiting.is_some())
                    && let Some(service) = &self.semantic
                {
                    service.fail();
                }
                self.job = None;
                self.waiting = None;
                true
            }
            Decision::Score(ticket) => {
                self.waiting = Some(ticket);
                false
            }
            Decision::Wait => false,
        }
    }
    fn update_audio(
        &mut self,
        speech: bool,
        text: Option<&str>,
        now: Duration,
        snapshot: impl Fn() -> Vec<f32>,
    ) -> bool {
        if speech != self.speaking {
            self.speaking = speech;
            self.edge(
                if speech {
                    Edge::SpeechStarted
                } else {
                    Edge::SpeechEnded
                },
                now,
            );
        }
        let decision = text
            .map(|text| self.edge(Edge::Transcript(text), now))
            .unwrap_or(Decision::Wait);
        if self.decision(decision, now) {
            return true;
        }
        // Current VAD/text revoke old tickets before any asynchronous score can affect completion.
        if self
            .job
            .as_ref()
            .is_some_and(|job| job.generation != self.generation || self.deadline(job.ticket).is_none())
        {
            self.job = None;
        }
        if let Some(job) = &self.job {
            let result = if self.clock + now >= job.until {
                Some(Err(()))
            } else {
                job.poll()
            };
            if let Some(result) = result {
                let job = self.job.take().expect("Admitted semantic job");
                let decision = self.edge(Edge::Score(job.ticket, result), now);
                if !self
                    .end
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .semantic_enabled()
                    && let Some(service) = &self.semantic
                {
                    service.fail();
                }
                if self.decision(decision, now) {
                    return true;
                }
            }
        }
        let decision = self.edge(Edge::Tick, now);
        if self.decision(decision, now) {
            return true;
        }
        if let Some(ticket) = self.waiting {
            if let Some(deadline) = self.deadline(ticket) {
                if now >= deadline || self.semantic.as_ref().is_some_and(|service| service.failed()) {
                    if let Some(service) = &self.semantic {
                        service.fail();
                    }
                    self.waiting = None;
                    let decision = self.edge(Edge::Score(ticket, Err(())), now);
                    return self.decision(decision, now);
                }
                if self.job.is_none()
                    && let Some(service) = &self.semantic
                {
                    self.job = service.request(self.generation, ticket, self.clock + deadline, snapshot);
                    if self.job.is_some() {
                        self.waiting = None;
                    }
                }
            } else {
                self.waiting = None;
            }
        }
        false
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
                let mut policy = Policy::new(generation, language.as_deref(), owner.owner.semantic.clone());
                let mut reported_semantic = false;
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
                let started = policy.clock;
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
                    if stop.load(Ordering::SeqCst)
                        || owner.live().is_err()
                        || *owner.end.borrow() != End::Recording
                    {
                        return Ok(false);
                    }
                    let speaking = speech
                        .last()
                        .is_some_and(|range| range.end.saturating_add(3200) >= pcm.len());
                    let text = text.lock().ok().and_then(|mut text| text.take());
                    let ended = policy
                        .update_audio(speaking, text.as_deref(), started.elapsed(), || audio.tail(8000));
                    if !reported_semantic
                        && owner
                            .owner
                            .semantic
                            .as_ref()
                            .is_some_and(|service| service.failed())
                    {
                        owner.notify(CapturePhase::SemanticFallback).map_err(|_| ())?;
                        reported_semantic = true;
                    }
                    if ended {
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
        Policy::new(
            turn.start(Duration::ZERO).unwrap().generation,
            Some(language),
            None,
        )
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
    fn semantic_policy(
        score: impl FnMut(&[f32], &AtomicBool, Instant) -> Result<f32, ()> + Send + 'static,
    ) -> (Policy, Arc<semantic::Service>) {
        let (_control, worker, _events, _cancel, _state) = super::super::tests::fixture();
        let service = semantic::Service::scoring(
            worker.owner.lease.clone(),
            worker.owner.active.clone(),
            Box::new(score),
        )
        .unwrap();
        let mut turn = vults_voice::turn::Turn::new(vults_voice::turn::BargeIn::Disabled);
        let policy = Policy::at(
            turn.start(Duration::ZERO).unwrap().generation,
            Some("en"),
            Some(service.clone()),
            Duration::ZERO,
        );
        (policy, service)
    }
    #[test]
    fn semantic_scores_use_audio_only_after_words_and_silence_and_finish_once() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = calls.clone();
        let (mut policy, service) = semantic_policy(move |pcm, _, _| {
            assert_eq!(pcm, &[0.25; 100]);
            seen.fetch_add(1, Ordering::SeqCst);
            Ok(0.9)
        });
        let ms = Duration::from_millis;
        assert!(!policy.update_audio(false, Some("hallucinated quiet"), ms(100), || panic!("No speech")));
        assert!(!policy.update(false, None, ms(4000)));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(!policy.update(true, Some("real speech"), ms(4100)));
        assert!(!policy.update(false, None, ms(4200)));
        assert!(!policy.update_audio(false, None, ms(5800), || vec![0.25; 100]));
        let until = Instant::now() + Duration::from_secs(1);
        loop {
            if policy.update_audio(false, None, ms(5900), || vec![0.25; 100]) {
                break;
            }
            assert!(Instant::now() < until);
            std::thread::sleep(ms(1));
        }
        assert!(!policy.update(false, None, ms(6000)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!service.failed());
    }
    #[test]
    fn fresh_speech_revokes_a_delayed_score_without_poisoning_the_conversation() {
        let (entered, waiting) = std::sync::mpsc::sync_channel(1);
        let (release, released) = std::sync::mpsc::sync_channel(1);
        let (mut policy, service) = semantic_policy(move |_, _, _| {
            entered.send(()).unwrap();
            released.recv().unwrap();
            Ok(0.99)
        });
        let ms = Duration::from_millis;
        policy.update(true, Some("real words"), ms(100));
        policy.update(false, None, ms(200));
        assert!(!policy.update(false, None, ms(1800)));
        waiting.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(!policy.update(true, None, ms(1850)));
        assert!(policy.job.is_none());
        release.send(()).unwrap();
        std::thread::sleep(ms(10));
        assert!(!policy.update(true, Some("still speaking"), ms(1900)));
        assert!(!service.failed());
    }
    #[test]
    fn a_failed_score_disables_semantics_for_following_turns_but_keeps_silence_completion() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = calls.clone();
        let (mut policy, service) = semantic_policy(move |_, _, _| {
            seen.fetch_add(1, Ordering::SeqCst);
            Ok(f32::NAN)
        });
        let ms = Duration::from_millis;
        policy.update(true, Some("real words"), ms(100));
        policy.update(false, None, ms(200));
        policy.update(false, None, ms(1800));
        let until = Instant::now() + Duration::from_secs(1);
        while !policy.update(false, None, ms(1900)) {
            assert!(Instant::now() < until);
            std::thread::sleep(ms(1));
        }
        assert!(service.failed());
        let mut machine = vults_voice::turn::Turn::new(vults_voice::turn::BargeIn::Disabled);
        let first = machine.start(Duration::ZERO).unwrap();
        machine.handle(
            first.generation,
            vults_voice::turn::Event::LocalCompleted,
            Duration::ZERO,
        );
        let next = machine.start(Duration::ZERO).unwrap();
        let mut following = Policy::at(next.generation, Some("en"), Some(service), ms(2000));
        following.update(true, Some("following words"), ms(2100));
        following.update(false, None, ms(2200));
        assert!(following.update(false, None, ms(3800)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn a_hung_score_uses_the_original_hold_deadline_and_does_not_block_completion() {
        let (entered, waiting) = std::sync::mpsc::sync_channel(1);
        let (release, released) = std::sync::mpsc::sync_channel(1);
        let (mut policy, service) = semantic_policy(move |_, _, _| {
            entered.send(()).unwrap();
            released.recv().unwrap();
            Ok(0.99)
        });
        let ms = Duration::from_millis;
        policy.update(true, Some("real words"), ms(100));
        policy.update(false, None, ms(200));
        policy.update(false, None, ms(1800));
        waiting.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(policy.job.as_ref().unwrap().until, policy.clock + ms(3000));
        let started = Instant::now();
        assert!(policy.update(false, None, ms(3000)));
        assert!(started.elapsed() < ms(100));
        assert!(service.failed());
        assert!(!policy.update(false, None, ms(3100)));
        release.send(()).unwrap();
    }
    #[test]
    fn a_high_score_cannot_finish_after_new_partial_words_move_the_silence_gate() {
        let (entered, waiting) = std::sync::mpsc::sync_channel(1);
        let (release, released) = std::sync::mpsc::sync_channel(1);
        let (mut policy, service) = semantic_policy(move |_, _, _| {
            entered.send(()).unwrap();
            released.recv().unwrap();
            Ok(0.99)
        });
        let ms = Duration::from_millis;
        policy.update(true, Some("hello"), ms(100));
        policy.update(false, None, ms(200));
        policy.update(false, None, ms(1800));
        waiting.recv_timeout(Duration::from_secs(1)).unwrap();
        release.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(1);
        while policy.job.is_some() {
            assert!(!policy.update(false, Some("hello again"), ms(1850)));
            assert!(Instant::now() < until);
            std::thread::sleep(ms(1));
        }
        assert!(!policy.update(false, None, ms(3449)));
        assert!(policy.update(false, None, ms(3450)));
        assert!(
            !service.failed(),
            "A healthy score and later word growth are not inference failure"
        );
    }
}
