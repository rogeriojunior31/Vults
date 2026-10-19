//! Conversation-only end-of-turn policy. Callers own capture limits and semantic-model consent/IO.
//! Partial transcript churn never extends silence unless the word-count high-water mark grows.

use std::time::Duration;

use super::Generation;

pub const BASE_SILENCE: Duration = Duration::from_millis(1600);
pub const EXTENDED_SILENCE: Duration = Duration::from_millis(2600);
pub const SEMANTIC_POLL: Duration = Duration::from_millis(200);
pub const SEMANTIC_HOLD: Duration = Duration::from_millis(1200);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Portuguese,
    Other,
}

pub fn silence_delay(text: &str, language: Language) -> Duration {
    let last = text
        .split_whitespace()
        .rfind(|word| word.chars().any(char::is_alphanumeric))
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    let words = match language {
        Language::English => {
            "and or but because so if when while although unless the a an to for from with of in on at by as about until um uh hmm well"
        }
        Language::Portuguese => {
            "e ou mas porque pois se quando enquanto embora que de da do das dos em no na nos nas com sem por para pra pro ao aos a as o os um uma uns umas entao então tipo assim hum ate até hã né" // check-english:allow
        }
        Language::Other => "",
    };
    let extending = words.split_ascii_whitespace().any(|word| word == last);
    if extending { EXTENDED_SILENCE } else { BASE_SILENCE }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket(u64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event<'a> {
    SpeechStarted,
    SpeechEnded,
    Transcript(&'a str),
    /// Audio-only completion probability for the requested snapshot; never a text classifier.
    Score(Ticket, Result<f32, ()>),
    Tick,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Silence,
    Semantic,
    HoldLimit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Wait,
    /// The adapter snapshots the current authorized audio window, not the partial transcript.
    Score(Ticket),
    End(Reason),
}

/// Keep one instance per explicitly started conversation: one model failure disables its later turns.
#[derive(Debug)]
pub struct EndOfTurn {
    generation: Option<Generation>,
    language: Language,
    now: Duration,
    semantic: bool,
    failed: bool,
    serial: u64,
    pending: Option<Ticket>,
    complete: bool,
    speaking: bool,
    quiet: Option<Duration>,
    growth: Option<Duration>,
    high_water: usize,
    words: usize,
    delay: Duration,
    hold: Option<Duration>,
    next_score: Option<Duration>,
}
impl EndOfTurn {
    pub fn new(semantic: bool) -> Self {
        Self {
            generation: None,
            language: Language::Other,
            now: Duration::ZERO,
            semantic,
            failed: false,
            serial: 0,
            pending: None,
            complete: true,
            speaking: false,
            quiet: None,
            growth: None,
            high_water: 0,
            words: 0,
            delay: BASE_SILENCE,
            hold: None,
            next_score: None,
        }
    }
    /// Bind only a new Listen action from the retained turn machine, with the same monotonic clock.
    pub fn begin(&mut self, generation: Generation, language: Language, now: Duration) -> bool {
        if self.generation.is_some_and(|old| generation <= old) || now < self.now {
            return false;
        }
        let failed = self.failed;
        let serial = self.serial;
        *self = Self::new(self.semantic);
        self.failed = failed;
        self.serial = serial;
        self.generation = Some(generation);
        self.language = language;
        self.now = now;
        self.complete = false;
        true
    }
    pub fn semantic_enabled(&self) -> bool {
        self.semantic && !self.failed
    }
    fn ready_at(&self) -> Option<Duration> {
        self.quiet
            .filter(|_| !self.speaking && self.words > 0)
            .map(|quiet| quiet.max(self.growth.unwrap_or(quiet)).saturating_add(self.delay))
    }
    fn ready(&self) -> bool {
        self.ready_at().is_some_and(|at| self.now >= at)
    }
    fn end(&mut self, reason: Reason) -> Decision {
        self.complete = true;
        self.pending = None;
        Decision::End(reason)
    }
    pub fn handle(&mut self, generation: Generation, event: Event<'_>, now: Duration) -> Decision {
        if self.generation != Some(generation) || self.complete || now < self.now {
            return Decision::Wait;
        }
        if matches!(event, Event::Score(ticket, _) if self.pending != Some(ticket)) {
            return Decision::Wait;
        }
        self.now = now;
        match event {
            Event::Cancel => {
                self.complete = true;
                self.pending = None;
                return Decision::Wait;
            }
            Event::SpeechStarted => {
                self.speaking = true;
                self.quiet = None;
                self.pending = None;
                self.hold = None;
                self.next_score = None;
            }
            Event::SpeechEnded if self.speaking => {
                self.speaking = false;
                self.quiet = Some(now);
            }
            Event::Transcript(text) => {
                self.words = text
                    .split_whitespace()
                    .filter(|word| word.chars().any(char::is_alphanumeric))
                    .count();
                self.delay = silence_delay(text, self.language);
                if self.words > self.high_water {
                    self.high_water = self.words;
                    self.growth = Some(now);
                }
            }
            Event::Score(ticket, result) if self.pending == Some(ticket) => {
                self.pending = None;
                match result {
                    Ok(score) if score.is_finite() && (0.0..=1.0).contains(&score) => {
                        if score >= 0.5
                            && self.ready()
                            && self.hold.is_some_and(|at| now.saturating_sub(at) < SEMANTIC_HOLD)
                        {
                            return self.end(Reason::Semantic);
                        }
                    }
                    _ => self.failed = true,
                }
            }
            _ => {}
        }
        if !self.ready() {
            return Decision::Wait;
        }
        if !self.semantic_enabled() {
            return self.end(Reason::Silence);
        }
        // Scheduler delay cannot grant another semantic wait after the nominal deadline.
        let ready_at = self.ready_at().expect("checked readiness");
        let hold = *self.hold.get_or_insert(ready_at);
        if now.saturating_sub(hold) >= SEMANTIC_HOLD {
            return self.end(Reason::HoldLimit);
        }
        if self.pending.is_some() || self.next_score.is_some_and(|at| now < at) {
            return Decision::Wait;
        }
        let Some(serial) = self.serial.checked_add(1) else {
            self.failed = true;
            return self.end(Reason::Silence);
        };
        self.serial = serial;
        let ticket = Ticket(serial);
        self.pending = Some(ticket);
        self.next_score = Some(now.saturating_add(SEMANTIC_POLL));
        Decision::Score(ticket)
    }
}

#[cfg(test)]
mod tests;
