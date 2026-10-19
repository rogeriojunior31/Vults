//! Pure dictation gestures, text rules and voice-command grammar; no IO or permission authority.
//! Key gestures use caller-supplied monotonic time; command names remain unresolved descriptions.

#![forbid(unsafe_code)]

use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

pub mod cleanup;
pub mod command;
pub mod styles;
pub mod text;

/// Unicode word boundaries, including unspaced scripts; punctuation and emoji are not words.
pub fn word_count(text: &str) -> usize {
    text.unicode_words().count()
}

pub const HOLD_THRESHOLD: Duration = Duration::from_millis(300);
pub const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(400);
pub const MIC_TAIL: Duration = Duration::from_millis(600);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GestureOptions {
    pub double_tap: bool,
    pub space_lock: bool,
    pub escape_cancel: bool,
}

impl Default for GestureOptions {
    fn default() -> Self {
        Self {
            double_tap: true,
            space_lock: true,
            escape_cancel: true,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    Holding {
        since: Duration,
    },
    TapWait {
        until: Duration,
    },
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Press,
    Release,
    Space,
    OtherKey,
    /// Esc or the separately bound cancel shortcut.
    Cancel,
    Tick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Lock,
    /// Keep capturing through this deadline, then transcribe; this never authorizes insertion.
    Finish {
        capture_until: Duration,
    },
    Cancel,
    /// Discard the expired tap's recording and begin a fresh one.
    Restart,
}

#[derive(Debug, Default)]
pub struct KeyLogic {
    phase: Phase,
    down: bool,
    options: GestureOptions,
}

impl KeyLogic {
    /// A held key or an active turn keeps its original gestures through completion.
    pub fn configure(&mut self, options: GestureOptions) {
        if self.phase == Phase::Idle && !self.down {
            self.options = options;
        }
    }

    pub fn options(&self) -> GestureOptions {
        self.options
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// `now` is monotonic elapsed time from the caller's fixed origin, including after reset.
    pub fn update(&mut self, event: KeyEvent, now: Duration) -> Option<Action> {
        match event {
            KeyEvent::Press => {
                // Auto-repeat must not finish a just-locked recording or restart a cancelled one.
                if self.down {
                    return None;
                }
                self.down = true;
                match self.phase {
                    Phase::Idle => {
                        self.phase = Phase::Holding { since: now };
                        Some(Action::Start)
                    }
                    Phase::TapWait { until } if now <= until => {
                        self.phase = Phase::Locked;
                        Some(Action::Lock)
                    }
                    Phase::TapWait { .. } => {
                        self.phase = Phase::Holding { since: now };
                        Some(Action::Restart)
                    }
                    Phase::Locked => self.finish(now),
                    Phase::Holding { .. } => None,
                }
            }
            KeyEvent::Release => {
                if !std::mem::replace(&mut self.down, false) {
                    return None;
                }
                let Phase::Holding { since } = self.phase else {
                    return None;
                };
                if now.saturating_sub(since) >= HOLD_THRESHOLD {
                    self.finish(now)
                } else if !self.options.double_tap {
                    self.cancel()
                } else {
                    self.phase = Phase::TapWait {
                        until: now.saturating_add(DOUBLE_TAP_WINDOW),
                    };
                    None
                }
            }
            KeyEvent::Space if self.options.space_lock && matches!(self.phase, Phase::Holding { .. }) => {
                self.phase = Phase::Locked;
                Some(Action::Lock)
            }
            KeyEvent::Space | KeyEvent::OtherKey => match self.phase {
                Phase::Holding { .. } | Phase::TapWait { .. } => self.cancel(),
                Phase::Idle | Phase::Locked => None,
            },
            KeyEvent::Cancel => self.cancel(),
            KeyEvent::Tick => match self.phase {
                // At the boundary the second press still belongs to the double tap.
                Phase::TapWait { until } if now > until => self.cancel(),
                _ => None,
            },
        }
    }

    /// Capture failed or ended externally. Consume an already-held key until its release.
    pub fn reset(&mut self) {
        self.phase = Phase::Idle;
    }

    fn finish(&mut self, now: Duration) -> Option<Action> {
        self.reset();
        Some(Action::Finish {
            capture_until: now.saturating_add(MIC_TAIL),
        })
    }

    fn cancel(&mut self) -> Option<Action> {
        if self.phase == Phase::Idle {
            return None;
        }
        self.reset();
        Some(Action::Cancel)
    }
}
