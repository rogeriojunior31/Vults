//! What the desktop tells a paired phone, sealed in an envelope: a card waiting for an answer (with
//! the binding the phone signs back), that a card left the line, and that pairing went through.
//! The phone only shows and decides; everything here is what the island would show.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use vults_core::ledger::Binding;
use vults_core::{PENDING_TTL, Pending, State};

use crate::{Error, VERSION, hex, parse};

/// A desktop's message to the phone.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ToPhone {
    /// Pairing went through: the phone is known by this id and name.
    Welcome {
        device: String,
        name: String,
    },
    Card(Box<Card>),
    /// The card left the line (answered, sent to the terminal, its time over): drop it.
    Gone {
        request: String,
    },
}

#[derive(Serialize, Deserialize)]
struct Message {
    v: u8,
    #[serde(flatten)]
    body: ToPhone,
}

impl ToPhone {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&Message {
            v: VERSION,
            body: self.clone(),
        })
        .unwrap_or_default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(parse::<Message>(bytes)?.body)
    }
}

/// Longest target a card carries: a patch over hundreds of files would not fit in one message
/// ([`crate::MAX_MESSAGE`]). The binding still covers the whole target; the card says it is cut,
/// and core refuses the phone's *Always* past it.
const MAX_TARGET: usize = vults_core::MAX_PHONE_TARGET;

/// A card as the phone shows it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub request: String,
    /// What the phone signs back: the card's binding, in hex.
    pub binding: String,
    pub agent: String,
    pub project: String,
    /// The folder the card was offered in, the scope of an *Always*.
    pub cwd: Option<String>,
    pub tool: String,
    pub target: String,
    pub description: Option<String>,
    /// The whole command, when the target shows only its start.
    pub full: Option<String>,
    /// A cut command is never ruled: the phone offers no *Always* for it.
    pub cut: bool,
    pub questions: Vec<Question>,
    /// How long the card still waits, in milliseconds, as the desktop sent it.
    pub left_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub question: String,
    pub header: String,
    pub multi: bool,
    pub options: Vec<String>,
}

impl Card {
    /// The card for `p`, waiting in `state`, at `now`.
    pub fn of(state: &State, p: &Pending, now: Instant) -> Self {
        let project = state
            .sessions
            .get(&p.session)
            .map(|s| s.project.clone())
            .unwrap_or_default();
        let left = PENDING_TTL.saturating_sub(now.saturating_duration_since(p.since));
        let long = p.target.len() > MAX_TARGET;
        let target = if long {
            let end = (0..=MAX_TARGET)
                .rev()
                .find(|i| p.target.is_char_boundary(*i))
                .unwrap_or(0);
            format!("{}…", &p.target[..end])
        } else {
            p.target.clone()
        };
        Self {
            request: p.request.0.clone(),
            binding: hex(&Binding::of(p).0),
            agent: p.session.agent.name().to_string(),
            project,
            cwd: p.cwd.clone(),
            tool: p.tool.clone(),
            target,
            description: p.ask.description.clone(),
            full: p.ask.full.clone(),
            // What the phone does not show whole, it may not make a rule of.
            cut: p.ask.cut || long,
            questions: p
                .questions
                .iter()
                .map(|q| Question {
                    question: q.question.clone(),
                    header: q.header.clone(),
                    multi: q.multi,
                    options: q.options.iter().map(|o| o.label.clone()).collect(),
                })
                .collect(),
            left_ms: u64::try_from(left.as_millis()).unwrap_or(u64::MAX),
        }
    }

    /// The binding to sign, as bytes.
    pub fn binding(&self) -> Result<[u8; 32], Error> {
        crate::answer::unhex(&self.binding)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn a_long_target_is_cut_on_the_card_and_bound_whole() {
        use vults_core::{
            AgentEvent, AgentKind, AgentUpdate, Ask, Input, RequestId, SessionKey, Terminal, reduce,
        };
        let now = Instant::now();
        let mut state = State::default();
        let files: Vec<String> = (0..2_000).map(|i| format!("src/file{i}.rs")).collect();
        let target = format!("Edit · {}", files.join(", "));
        reduce(
            &mut state,
            Input::Agent(AgentUpdate {
                session: SessionKey {
                    agent: AgentKind::Codex,
                    session_id: "s1".into(),
                },
                cwd: Some("/home/me/vults".into()),
                terminal: Terminal::default(),
                agent_id: None,
                call_id: None,
                event: AgentEvent::PermissionRequested {
                    request: RequestId("r1".into()),
                    tool: "apply_patch".into(),
                    target: target.clone(),
                    ask: Ask::default(),
                },
                received: None,
            }),
            now,
        );
        let p = &state.pending[0];
        let card = Card::of(&state, p, now);
        assert!(card.cut && card.target.ends_with('…') && card.target.len() < target.len());
        assert_eq!(card.binding, hex(&Binding::of(p).0));
        assert!(ToPhone::Card(Box::new(card)).to_bytes().len() < crate::MAX_MESSAGE);
    }

    #[test]
    fn a_message_goes_both_ways() {
        let m = ToPhone::Gone { request: "r1".into() };
        assert_eq!(ToPhone::from_bytes(&m.to_bytes()).unwrap(), m);
        assert_eq!(
            ToPhone::from_bytes(br#"{"v":2,"kind":"gone","request":"r1"}"#),
            Err(Error::Version)
        );
    }
}
