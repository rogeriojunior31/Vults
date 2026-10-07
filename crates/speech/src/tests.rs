//! The speaker with a stand-in voice and speakers: what is said, in what order, and how a stop
//! silences it. Plus the real model, when its files are given (see `the_real_model_speaks`).

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use super::*;

/// Says each sentence as one sample per character, after `delay`; remembers what it said.
struct Fake {
    said: Arc<Mutex<Vec<(String, Lang, String)>>>,
    delay: Duration,
}

impl Synth for Fake {
    fn speak(
        &mut self,
        text: &str,
        lang: Lang,
        voice: &str,
        stale: &dyn Fn() -> bool,
    ) -> Result<Vec<f32>, String> {
        let until = Instant::now() + self.delay;
        while Instant::now() < until {
            if stale() {
                return Err("stopped".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.said
            .lock()
            .unwrap()
            .push((text.to_string(), lang, voice.to_string()));
        Ok(vec![0.0; text.len()])
    }
}

/// Speakers that hold what they get until `drain`.
#[derive(Default)]
struct Speakers {
    queued: Mutex<Vec<f32>>,
    clears: AtomicUsize,
}

impl Sink for Speakers {
    fn play(&self, pcm: Vec<f32>) {
        self.queued.lock().unwrap().extend(pcm);
    }
    fn clear(&self) {
        self.queued.lock().unwrap().clear();
        self.clears.fetch_add(1, Ordering::SeqCst);
    }
    fn playing(&self) -> bool {
        !self.queued.lock().unwrap().is_empty()
    }
}

struct Rig {
    speaker: Speaker,
    said: Arc<Mutex<Vec<(String, Lang, String)>>>,
    speakers: Arc<Speakers>,
    changes: Arc<Mutex<Vec<bool>>>,
    cancelled: Arc<AtomicBool>,
}

fn rig(delay: Duration) -> Rig {
    let said = Arc::new(Mutex::new(Vec::new()));
    let speakers = Arc::new(Speakers::default());
    let changes = Arc::new(Mutex::new(Vec::new()));
    let cancelled = Arc::new(AtomicBool::new(false));
    let (s, c, k) = (said.clone(), changes.clone(), cancelled.clone());
    let speaker = Speaker::start(
        move || {
            let cancel: Box<dyn Fn() + Send + Sync> = Box::new(move || k.store(true, Ordering::SeqCst));
            Ok((Box::new(Fake { said: s, delay }) as Box<dyn Synth>, cancel))
        },
        speakers.clone(),
        move |on| c.lock().unwrap().push(on),
    );
    Rig {
        speaker,
        said,
        speakers,
        changes,
        cancelled,
    }
}

fn wait_until(what: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !what() {
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_streaming_reply_is_said_sentence_by_sentence_without_its_code() {
    let r = rig(Duration::ZERO);
    r.speaker.set_voice(Lang::Pt, "pf_dora");
    r.speaker.begin(Lang::En);
    for piece in [
        "I found the bug in the parser. Run this",
        ":\n```sh\nrm -rf /\n```\n",
        "Then `cargo test` again to be sure. Pronto, ",
        "terminei o arquivo.",
    ] {
        r.speaker.hear(piece);
    }
    // The first sentences go before the reply ends.
    wait_until(|| r.said.lock().unwrap().len() == 2);
    assert_eq!(r.said.lock().unwrap()[0].0, "I found the bug in the parser.");
    assert!(r.changes.lock().unwrap().contains(&true), "Zeca speaks");
    r.speaker.finish();
    wait_until(|| r.said.lock().unwrap().len() == 3);
    let said = r.said.lock().unwrap().clone();
    assert_eq!(
        (said[1].0.as_str(), said[1].1),
        ("Run this: Then again to be sure.", Lang::En)
    );
    assert_eq!(
        (said[2].0.as_str(), said[2].1, said[2].2.as_str()),
        ("Pronto, terminei o arquivo.", Lang::Pt, "pf_dora")
    );
    assert!(
        said.iter()
            .all(|(s, ..)| !s.contains("-rf") && !s.contains("cargo"))
    );
    // Played out: he stops.
    r.speakers.clear();
    wait_until(|| r.changes.lock().unwrap().last() == Some(&false));
}

#[test]
fn a_stop_silences_him_at_once_and_drops_the_rest() {
    let r = rig(Duration::from_millis(300));
    r.speaker.begin(Lang::En);
    r.speaker.hear("This is the first sentence to say. This is the second sentence to say. And a third one comes after it. ");
    wait_until(|| r.speaker.speaking());
    r.speaker.stop();
    assert!(!r.speaker.speaking());
    assert!(!r.speakers.playing(), "the queue is dropped");
    assert!(
        r.cancelled.load(Ordering::SeqCst),
        "the sentence being made is abandoned"
    );
    assert_eq!(r.changes.lock().unwrap().last(), Some(&false));
    // Nothing of that reply comes later, even what was still in the chunker.
    r.speaker.finish();
    std::thread::sleep(Duration::from_millis(700));
    assert_eq!(r.said.lock().unwrap().len(), 1);
    assert!(!r.speakers.playing());
    // The next reply is said as usual.
    r.speaker.begin(Lang::En);
    r.speaker.hear("A new reply, said in full.");
    r.speaker.finish();
    wait_until(|| r.said.lock().unwrap().len() == 2);
}

#[test]
fn a_new_reply_stops_the_last_one() {
    let r = rig(Duration::from_millis(50));
    r.speaker.begin(Lang::En);
    r.speaker
        .hear("One long reply that goes on and on. And on and on it goes. ");
    wait_until(|| r.speaker.speaking());
    let clears = r.speakers.clears.load(Ordering::SeqCst);
    r.speaker.begin(Lang::Pt);
    assert!(r.speakers.clears.load(Ordering::SeqCst) > clears);
    // A sentence that does not tell its language takes the user's.
    r.speaker.hear("OK.");
    r.speaker.finish();
    wait_until(|| r.said.lock().unwrap().last().is_some_and(|s| s.0 == "OK."));
    assert_eq!(r.said.lock().unwrap().last().map(|s| s.1), Some(Lang::Pt));
}

#[test]
fn a_model_that_fails_to_load_says_nothing() {
    let changes = Arc::new(Mutex::new(Vec::new()));
    let c = changes.clone();
    let speakers = Arc::new(Speakers::default());
    let speaker = Speaker::start(
        || Err("no model".into()),
        speakers.clone(),
        move |on| c.lock().unwrap().push(on),
    );
    speaker.begin(Lang::En);
    speaker.hear("Nobody will hear this sentence at all.");
    speaker.finish();
    std::thread::sleep(Duration::from_millis(200));
    assert!(!speakers.playing() && changes.lock().unwrap().is_empty());
}

#[test]
fn espeak_is_never_linked() {
    // GPL-3.0 (ADR 0013): only ever run as a separate program.
    let lock = include_str!("../../../Cargo.lock");
    for name in ["espeak-rs", "espeak-ng-sys", "espeak-sys", "sherpa-onnx"] {
        assert!(
            !lock.contains(&format!("name = \"{name}")),
            "{name} is in Cargo.lock"
        );
    }
}

/// The real model, when `SPEECH_TEST_MODELS` names a folder with the downloaded files: a sentence
/// in each language comes out as audio of a plausible length. Portuguese also needs espeak-ng (on
/// the PATH, or `SPEECH_TEST_ESPEAK`). `SPEECH_TEST_OUT` names a folder to keep the audio in, to
/// listen to (`ffplay -f f32le -ar 24000 -ch_layout mono en.f32`).
#[test]
fn the_real_model_speaks() {
    let Some(dir) = std::env::var_os("SPEECH_TEST_MODELS") else {
        return;
    };
    let espeak = std::env::var_os("SPEECH_TEST_ESPEAK").map(std::path::PathBuf::from);
    let mut engine = Engine::load(Path::new(&dir), espeak.clone()).expect("load");
    let never = || false;
    let en = engine
        .speak(
            "Hi, I'm Zeca. I finished the review.",
            Lang::En,
            "am_michael",
            &never,
        )
        .expect("english");
    let seconds = en.len() as f32 / kokoro::SAMPLE_RATE as f32;
    assert!((1.0..6.0).contains(&seconds), "{seconds} s");
    assert!(en.iter().any(|s| s.abs() > 0.05), "silence");
    let keep = |name: &str, pcm: &[f32]| {
        if let Some(out) = std::env::var_os("SPEECH_TEST_OUT") {
            let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
            std::fs::write(Path::new(&out).join(name), bytes).expect("write");
        }
    };
    keep("en.f32", &en);
    if espeak.or_else(espeak::find).is_some() {
        let pt = engine
            .speak(
                "Oi, eu sou o Zeca. Terminei a revis\u{e3}o.",
                Lang::Pt,
                "pm_alex",
                &never,
            )
            .expect("portuguese");
        assert!(pt.len() > kokoro::SAMPLE_RATE as usize, "{}", pt.len());
        keep("pt.f32", &pt);
    }
    // A cancel before the run ends it there.
    assert!(
        engine
            .speak("Never said.", Lang::En, "af_heart", &|| true)
            .is_err()
    );
}
