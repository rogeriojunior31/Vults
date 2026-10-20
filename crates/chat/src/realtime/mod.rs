//! Offline Realtime protocol/cost contract. No sockets, keys, microphone or admission authority.
//! The app must reserve the quoted ceiling through its monetary gate BEFORE sending a prepared event.
mod pricing;
pub mod transport;

use pricing::{CONTEXT, Rates, Usage};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const MAX_EVENT_BYTES: usize = 256 * 1024;
const MAX_RESPONSES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Model,
    Limits,
    Rates,
    State,
    Event,
    Configuration,
}

#[derive(Debug)]
pub struct Plan {
    model: &'static str,
    rates: Rates,
    post_instructions: u64,
    output: u64,
}

impl Plan {
    pub fn new(model: &str, post_instructions: u64, output: u64) -> Result<Self, Rejection> {
        let (model, rates) = Rates::for_model(model).ok_or(Rejection::Model)?;
        if !(1..=4096).contains(&output) || post_instructions == 0 || post_instructions > CONTEXT - output {
            return Err(Rejection::Limits);
        }
        Ok(Self {
            model,
            rates,
            post_instructions,
            output,
        })
    }

    pub fn model(&self) -> &'static str {
        self.model
    }

    pub fn ceiling_microusd(&self, now_ms: i64) -> Option<u64> {
        self.rates.ceiling(self.post_instructions, self.output, now_ms)
    }

    pub fn session_update(&self) -> Value {
        json!({"type":"session.update", "session": {
            "type":"realtime", "instructions":persona!(), "tools":[], "tool_choice":"none",
            "output_modalities":["audio"], "max_output_tokens":self.output,
            "truncation":{"type":"retention_ratio", "retention_ratio":0.8,
                "token_limits":{"post_instructions":self.post_instructions}},
            "audio":{"input":{"format":{"type":"audio/pcm","rate":24000},
                "transcription":null,"turn_detection":null},
                "output":{"format":{"type":"audio/pcm","rate":24000},"voice":"marin"}}
        }})
    }

    fn confirmed(&self, session: &Value) -> bool {
        let update = self.session_update();
        let wanted = &update["session"];
        session["model"] == self.model
            && [
                "type",
                "instructions",
                "tools",
                "tool_choice",
                "output_modalities",
                "max_output_tokens",
                "truncation",
            ]
            .iter()
            .all(|key| session.get(key) == wanted.get(key))
            && [
                "/audio/input/format",
                "/audio/input/transcription",
                "/audio/input/turn_detection",
                "/audio/output/format",
                "/audio/output/voice",
            ]
            .iter()
            .all(|path| session.pointer(path) == wanted.pointer(path))
    }
}

#[derive(Debug)]
enum State {
    Configuring,
    Ready,
    Creating { attempt: String },
    Responding { attempt: String, response: String },
    Closed,
}

#[derive(Debug)]
pub struct Session {
    plan: Plan,
    state: State,
    session_id: Option<String>,
    used_attempts: BTreeSet<String>,
}

/// Prepared wire data is not permission to send. Reserve ceiling_microusd with this exact attempt.
#[derive(Debug)]
pub struct Prepared {
    pub ceiling_microusd: u64,
    pub event: Value,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Settlement {
    pub attempt: String,
    pub actual_microusd: Option<u64>,
    pub ok: bool,
    pub within_bounds: bool,
}

fn id(value: &Value) -> Option<&str> {
    value.as_str().filter(|s| valid_id(s))
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

impl Session {
    pub fn new(plan: Plan) -> Self {
        Self {
            plan,
            state: State::Configuring,
            session_id: None,
            used_attempts: BTreeSet::new(),
        }
    }

    pub fn prepare(&mut self, attempt: &str, now_ms: i64) -> Result<Prepared, Rejection> {
        if !matches!(self.state, State::Ready)
            || !valid_id(attempt)
            || self.used_attempts.contains(attempt)
            || self.used_attempts.len() >= MAX_RESPONSES
        {
            return Err(Rejection::State);
        }
        let ceiling_microusd = self.plan.ceiling_microusd(now_ms).ok_or(Rejection::Rates)?;
        self.used_attempts.insert(attempt.into());
        self.state = State::Creating {
            attempt: attempt.into(),
        };
        Ok(Prepared {
            ceiling_microusd,
            event: json!({"type":"response.create", "response": {
                "max_output_tokens":self.plan.output, "tools":[], "tool_choice":"none",
                "metadata":{"vults_attempt":attempt}
            }}),
        })
    }

    /// Any protocol failure permanently closes this owner; the caller stops transport and keeps
    /// an unresolved hold charged. Neither a late config nor an old callback can reopen it.
    pub fn receive(&mut self, bytes: &[u8]) -> Result<Option<Settlement>, Rejection> {
        if matches!(self.state, State::Closed) {
            return Err(Rejection::State);
        }
        let result = if bytes.len() > MAX_EVENT_BYTES {
            Err(Rejection::Event)
        } else {
            serde_json::from_slice(bytes)
                .map_err(|_| Rejection::Event)
                .and_then(|event| self.event(event))
        };
        if result.is_err() {
            self.state = State::Closed;
        }
        result
    }

    fn event(&mut self, event: Value) -> Result<Option<Settlement>, Rejection> {
        match event["type"].as_str().ok_or(Rejection::Event)? {
            "session.created" => {
                if !matches!(self.state, State::Configuring)
                    || self.session_id.is_some()
                    || event["session"]["model"] != self.plan.model
                {
                    return Err(Rejection::Configuration);
                }
                self.session_id = Some(id(&event["session"]["id"]).ok_or(Rejection::Event)?.into());
            }
            "session.updated" => {
                if !matches!(self.state, State::Configuring | State::Ready)
                    || !self.plan.confirmed(&event["session"])
                {
                    return Err(Rejection::Configuration);
                }
                let session_id = id(&event["session"]["id"]).ok_or(Rejection::Configuration)?;
                if self.session_id.as_ref().is_some_and(|known| known != session_id) {
                    return Err(Rejection::Configuration);
                }
                self.session_id = Some(session_id.into());
                self.state = State::Ready;
            }
            "response.created" => {
                let State::Creating { attempt } = &self.state else {
                    return Err(Rejection::State);
                };
                let response = &event["response"];
                if response["metadata"]["vults_attempt"] != *attempt || response["status"] != "in_progress" {
                    return Err(Rejection::Event);
                }
                self.state = State::Responding {
                    attempt: attempt.clone(),
                    response: id(&response["id"]).ok_or(Rejection::Event)?.into(),
                };
            }
            "response.done" => {
                let State::Responding { attempt, response } = &self.state else {
                    return Err(Rejection::State);
                };
                let done = &event["response"];
                if done["id"] != *response {
                    return Err(Rejection::Event);
                }
                let ok = match done["status"].as_str() {
                    Some("completed") => true,
                    Some("cancelled" | "failed" | "incomplete") => false,
                    _ => return Err(Rejection::Event),
                };
                let usage = serde_json::from_value::<Usage>(done["usage"].clone()).ok();
                let actual_microusd = usage.as_ref().and_then(|u| self.plan.rates.cost(u));
                let within_bounds = actual_microusd.is_some()
                    && usage.as_ref().is_some_and(|u| {
                        u.input_tokens <= CONTEXT
                            && u.input_token_details.audio_tokens <= self.plan.post_instructions
                            && u.output_tokens <= self.plan.output
                    });
                let settlement = Settlement {
                    attempt: attempt.clone(),
                    actual_microusd,
                    ok,
                    within_bounds,
                };
                self.state = if within_bounds {
                    State::Ready
                } else {
                    State::Closed
                };
                return Ok(Some(settlement));
            }
            "error" => return Err(Rejection::Event),
            kind if kind.starts_with("response.") => {
                let State::Responding { response, .. } = &self.state else {
                    return Err(Rejection::State);
                };
                if event["response_id"] != *response {
                    return Err(Rejection::Event);
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests;
