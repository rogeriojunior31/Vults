//! A follow-up proposal is not capture authority. The adapter owns consent, visibility and devices.
use super::Generation;
use std::time::Duration;

pub const LIMIT: Duration = Duration::from_secs(8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    SpokenResponse,
    Tool,
    Notification,
    Permission,
}

pub struct Reply<'a> {
    pub generation: Generation,
    pub source: Source,
    pub text: &'a str,
    pub played_frames: u64,
    pub total_frames: u64,
    pub interrupted: bool,
    pub synchronized: bool,
}
impl std::fmt::Debug for Reply<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FollowUpReply")
            .field("generation", &self.generation)
            .field("source", &self.source)
            .field("bytes", &self.text.len())
            .field("played_frames", &self.played_frames)
            .field("total_frames", &self.total_frames)
            .field("interrupted", &self.interrupted)
            .field("synchronized", &self.synchronized)
            .finish()
    }
}
impl Reply<'_> {
    fn question(&self) -> bool {
        self.source == Source::SpokenResponse
            && !self.interrupted
            && self.synchronized
            && self.total_frames > 0
            && self.played_frames == self.total_frames
            && self.text.trim_end().ends_with(['?', '\u{ff1f}'])
            && self.text.chars().any(char::is_alphabetic)
    }
}

/// An opaque, once-only proposal for the exact following Listen generation. No capture authority.
#[derive(Debug)]
pub struct Window {
    generation: Generation,
    deadline: Duration,
    claimed_at: Duration,
}
impl Window {
    pub fn generation(&self) -> Generation {
        self.generation
    }
    pub fn remaining(&self, now: Duration) -> Option<Duration> {
        if now < self.claimed_at {
            return None;
        }
        self.deadline.checked_sub(now).filter(|left| !left.is_zero())
    }
}

#[derive(Debug)]
pub struct FollowUp {
    enabled: bool,
    latest: Option<Generation>,
    pending: Option<Window>,
    now: Duration,
}
impl Default for FollowUp {
    fn default() -> Self {
        Self::new(false)
    }
}
impl FollowUp {
    /// Freeze an explicitly reviewed choice for one active conversation. This opens nothing.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            latest: None,
            pending: None,
            now: Duration::ZERO,
        }
    }
    fn advance(&mut self, now: Duration) -> bool {
        if now < self.now {
            return false;
        }
        self.now = now;
        if self.pending.as_ref().is_some_and(|p| p.remaining(now).is_none()) {
            self.pending = None;
        }
        true
    }
    /// `drained_at` is actual completed playback, not queued audio or model completion.
    pub fn completed(&mut self, reply: Reply<'_>, drained_at: Duration, now: Duration) -> bool {
        if !self.advance(now)
            || drained_at > now
            || reply.generation.0 == 0
            || self.latest.is_some_and(|old| reply.generation <= old)
        {
            return false;
        }
        self.latest = Some(reply.generation);
        self.pending = None;
        if !self.enabled || !reply.question() {
            return false;
        }
        let Some(deadline) = drained_at.checked_add(LIMIT).filter(|at| *at > now) else {
            return false;
        };
        let Some(next) = reply.generation.0.checked_add(1) else {
            return false;
        };
        self.pending = Some(Window {
            generation: Generation(next),
            deadline,
            claimed_at: now,
        });
        true
    }
    /// The adapter must still revalidate the conversation and use the original remaining budget.
    pub fn take(&mut self, generation: Generation, now: Duration) -> Option<Window> {
        if !self.advance(now) {
            return None;
        }
        // A late reply from a skipped Listen must never recreate its proposal.
        if let Some(previous) = generation.0.checked_sub(1) {
            let retired = Generation(previous);
            self.latest = Some(self.latest.unwrap_or(retired).max(retired));
        }
        let pending = self.pending.as_ref()?;
        if generation > pending.generation {
            self.pending = None;
            return None;
        }
        if generation != pending.generation {
            return None;
        }
        let mut window = self.pending.take()?;
        window.claimed_at = now;
        Some(window)
    }
    /// Departure/lock/consent loss permanently revokes this conversation's proposals.
    pub fn cancel(&mut self) {
        self.enabled = false;
        self.pending = None;
    }
}

#[cfg(test)]
mod tests;
