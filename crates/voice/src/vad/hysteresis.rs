use std::collections::VecDeque;

use crate::SAMPLE_RATE;

const ON: f32 = 0.50;
const OFF: f32 = 0.35;
const ALPHA: f32 = 0.35;
const SPEECH: usize = SAMPLE_RATE as usize / 4;
const SILENCE: usize = SAMPLE_RATE as usize * 2 / 5;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Transition {
    Start(usize),
    End(usize),
}

#[derive(Debug, Default)]
pub(super) struct Hysteresis {
    filtered: f32,
    position: usize,
    candidate: Option<usize>,
    quiet: Option<usize>,
    active: bool,
    transitions: VecDeque<Transition>,
}

impl Hysteresis {
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(super) fn push(&mut self, probability: f32, samples: usize) {
        let at = self.position;
        self.position += samples;
        self.filtered += ALPHA * (probability - self.filtered);
        if self.active {
            // Remember quiet before the filter catches up, without emitting an unconfirmed edge.
            // Otherwise smoothing adds its decay time to Chat's end-of-turn pause.
            if probability < OFF {
                self.quiet.get_or_insert(at);
            }
            if self.filtered < OFF {
                let quiet = *self.quiet.get_or_insert(at);
                if self.position - quiet >= SILENCE {
                    // Report the edge, not the later instant when enough quiet confirms it.
                    self.transitions.push_back(Transition::End(quiet));
                    self.active = false;
                    self.quiet = None;
                }
            } else if probability >= OFF {
                self.quiet = None;
            }
        } else if self.filtered >= ON {
            let start = *self.candidate.get_or_insert(at);
            if self.position - start >= SPEECH {
                self.transitions.push_back(Transition::Start(start));
                self.active = true;
                self.candidate = None;
            }
        } else {
            self.candidate = None;
        }
    }

    pub(super) fn finish(&mut self) {
        if self.active {
            self.transitions.push_back(Transition::End(self.position));
            self.active = false;
            self.quiet = None;
        }
        self.candidate = None;
    }

    pub(super) fn pop(&mut self) -> Option<Transition> {
        self.transitions.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(detector: &mut Hysteresis, probability: f32, count: usize) {
        for _ in 0..count {
            detector.push(probability, 512);
        }
    }

    #[test]
    fn thresholds_smoothing_and_dwell_times_reject_flapping() {
        let mut detector = Hysteresis::default();
        frames(&mut detector, 1.0, 1);
        assert!(detector.pop().is_none(), "one spike is not speech");
        frames(&mut detector, 0.0, 20);
        frames(&mut detector, 1.0, 8);
        assert!(detector.pop().is_none(), "less than 250 ms above threshold");
        frames(&mut detector, 1.0, 1);
        assert_eq!(detector.pop(), Some(Transition::Start(22 * 512)));
        // The band between the thresholds sustains speech without starting new speech.
        frames(&mut detector, 0.4, 20);
        frames(&mut detector, 0.0, 12);
        assert!(detector.pop().is_none(), "less than 400 ms of quiet");
        frames(&mut detector, 0.0, 2);
        assert_eq!(detector.pop(), Some(Transition::End(50 * 512)));
        frames(&mut detector, 0.4, 30);
        detector.finish();
        assert!(detector.pop().is_none(), "the dead band cannot start speech");
    }

    #[test]
    fn transitions_are_queued_and_reset_clears_the_entire_turn() {
        let mut detector = Hysteresis::default();
        frames(&mut detector, 1.0, 12);
        frames(&mut detector, 0.0, 20);
        frames(&mut detector, 1.0, 12);
        detector.finish();
        assert_eq!(detector.pop(), Some(Transition::Start(512)));
        assert_eq!(detector.pop(), Some(Transition::End(12 * 512)));
        assert_eq!(detector.pop(), Some(Transition::Start(33 * 512)));
        assert_eq!(detector.pop(), Some(Transition::End(44 * 512)));
        detector.finish();
        assert!(detector.pop().is_none(), "finish cannot duplicate an edge");
        frames(&mut detector, 1.0, 12);
        detector.reset();
        assert!(
            detector.pop().is_none(),
            "no queued edge leaks into the next turn"
        );
        frames(&mut detector, 0.4, 20);
        detector.finish();
        assert!(detector.pop().is_none(), "no active state or filter leaks");
        detector.reset();
        frames(&mut detector, 1.0, 9);
        detector.finish();
        assert_eq!(detector.pop(), Some(Transition::Start(512)));
        assert_eq!(detector.pop(), Some(Transition::End(9 * 512)));
    }
}
