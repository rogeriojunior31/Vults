//! Text-question lifecycle only. The app verifies the process owner before every input;
//! nothing here can create a permission decision, microphone capture or network request.
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

const MAX_ID: usize = 128;
const MAX_QUESTION: usize = 4096;
const MAX_REQUESTS: usize = 256;
pub const LIFE: Duration = crate::ledger::MAX_LIFE;

/// OS-verified process incarnations, supplied by the app, never deserialized from client headers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Owner {
    pub relay_pid: u32,
    pub relay_started: u64,
    pub client_pid: u32,
    pub client_started: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub generation: u64,
    pub binding: crate::ledger::Binding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub ticket: Ticket,
    pub question: String,
    pub draft: String,
    pub expires: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Revocation {
    Cancelled,
    Replaced,
    Expired,
    Disconnected,
    Hidden,
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Answered,
    Revoked(Revocation),
}

/// Content-free terminal record; prompt, draft and client-supplied RPC id never enter the audit.
#[derive(Debug, PartialEq, Eq)]
pub struct Receipt {
    pub owner: Owner,
    pub ticket: Ticket,
    pub outcome: Outcome,
}

impl Receipt {
    pub fn audit(&self) -> crate::audit::Audit {
        use crate::audit::{Act, Actor, Audit};
        let (actor, act) = match self.outcome {
            Outcome::Answered => (Actor::Human, Act::Answer),
            Outcome::Revoked(Revocation::Expired) => (Actor::System, Act::Expire),
            Outcome::Revoked(_) => (Actor::System, Act::Release),
        };
        let binding: String = self.ticket.binding.0.iter().map(|b| format!("{b:02x}")).collect();
        let target = format!(
            "{binding}; relay={}:{}; client={}:{}; generation={}; {:?}",
            self.owner.relay_pid,
            self.owner.relay_started,
            self.owner.client_pid,
            self.owner.client_started,
            self.ticket.generation,
            self.outcome
        );
        Audit::new(actor, act, "mcp", "", "ask_user", &target)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Reply {
    pub request: String,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Reused,
    Full,
    Stale,
    Empty,
}

struct Pending {
    request: String,
    offer: Offer,
}

/// One authenticated MCP connection. No timer/IO; caller-owned monotonic time expires every input.
pub struct Slot {
    owner: Owner,
    nonce: [u8; 16],
    generation: u64,
    pending: Option<Pending>,
    used: BTreeSet<String>,
    records: Vec<Receipt>,
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slot")
            .field("owner", &self.owner)
            .field("generation", &self.generation)
            .field("pending", &self.pending.is_some())
            .field("requests", &self.used.len())
            .field("records", &self.records.len())
            .finish_non_exhaustive()
    }
}

fn text(value: &str, max: usize) -> bool {
    value.len() <= max
        && !value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}

impl Slot {
    /// `nonce` is fresh app-owned entropy per connection, never a client argument.
    pub fn new(owner: Owner, nonce: [u8; 16]) -> Result<Self, Error> {
        if owner.relay_pid <= 1
            || owner.client_pid <= 1
            || owner.relay_started == 0
            || owner.client_started == 0
            || nonce == [0; 16]
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            owner,
            nonce,
            generation: 0,
            pending: None,
            used: BTreeSet::new(),
            records: Vec::new(),
        })
    }

    pub fn open(&mut self, request: &str, question: &str, now: Instant) -> Result<Offer, Error> {
        self.expire(now);
        if request.is_empty()
            || request.len() > MAX_ID
            || request.chars().any(char::is_control)
            || question.trim().is_empty()
            || !text(question, MAX_QUESTION)
        {
            return Err(Error::Invalid);
        }
        if self.used.contains(request) {
            return Err(Error::Reused);
        }
        // ponytail: keep 256 ids per connection rather than evicting replay protection; reconnect at the ceiling.
        if self.used.len() >= MAX_REQUESTS {
            return Err(Error::Full);
        }
        let generation = self.generation.checked_add(1).ok_or(Error::Full)?;
        let expires = now.checked_add(LIFE).ok_or(Error::Invalid)?;
        self.revoke(Revocation::Replaced);
        self.generation = generation;
        let mut hash = Sha256::new();
        hash.update(b"vults/mcp-question/v1\0");
        hash.update(self.nonce);
        for n in [
            u64::from(self.owner.relay_pid),
            self.owner.relay_started,
            u64::from(self.owner.client_pid),
            self.owner.client_started,
            generation,
        ] {
            hash.update(n.to_le_bytes());
        }
        for field in [request, question] {
            hash.update((field.len() as u64).to_le_bytes());
            hash.update(field.as_bytes());
        }
        let ticket = Ticket {
            generation,
            binding: crate::ledger::Binding(hash.finalize().into()),
        };
        let offer = Offer {
            ticket,
            question: question.into(),
            draft: String::new(),
            expires,
        };
        self.used.insert(request.into());
        self.pending = Some(Pending {
            request: request.into(),
            offer: offer.clone(),
        });
        Ok(offer)
    }

    pub fn view(&mut self, now: Instant) -> Option<Offer> {
        self.expire(now);
        self.pending.as_ref().map(|p| p.offer.clone())
    }

    pub fn edit(&mut self, ticket: Ticket, draft: &str, now: Instant) -> Result<(), Error> {
        self.expire(now);
        let pending = self
            .pending
            .as_mut()
            .filter(|p| p.offer.ticket == ticket)
            .ok_or(Error::Stale)?;
        if !text(draft, crate::MAX_ANSWER_LEN) {
            return Err(Error::Invalid);
        }
        pending.offer.draft = draft.into();
        Ok(())
    }

    /// Only the app's separate human Send action may call this after capture has finished.
    pub fn confirm(&mut self, ticket: Ticket, now: Instant) -> Result<Reply, Error> {
        self.expire(now);
        let pending = self
            .pending
            .as_ref()
            .filter(|p| p.offer.ticket == ticket)
            .ok_or(Error::Stale)?;
        if pending.offer.draft.trim().is_empty() {
            return Err(Error::Empty);
        }
        let pending = self.pending.take().ok_or(Error::Stale)?;
        self.records.push(Receipt {
            owner: self.owner,
            ticket,
            outcome: Outcome::Answered,
        });
        Ok(Reply {
            request: pending.request,
            text: pending.offer.draft,
        })
    }

    pub fn revoke(&mut self, reason: Revocation) {
        if let Some(pending) = self.pending.take() {
            self.records.push(Receipt {
                owner: self.owner,
                ticket: pending.offer.ticket,
                outcome: Outcome::Revoked(reason),
            });
        }
    }

    fn expire(&mut self, now: Instant) {
        if self.pending.as_ref().is_some_and(|p| now >= p.offer.expires) {
            self.revoke(Revocation::Expired);
        }
    }

    pub fn take_records(&mut self) -> Vec<Receipt> {
        std::mem::take(&mut self.records)
    }
}

#[cfg(test)]
mod tests;
