//! Independent consent and durable admission; callers must own one actual transport attempt.
use crate::{gate, settings::SettingsState};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    #[default]
    ClaudeCli,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    #[default]
    Haiku,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Consent {
    pub on: bool,
    pub provider: Provider,
    pub model: Model,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger: Option<String>,
}

impl Consent {
    pub fn enabled(&self) -> bool {
        self.on && self.ledger.is_some()
    }

    /// Call off the state owner, just before a single provider request; dropping records cancellation.
    /// This does not authorize dispatch, which still needs the recording ticket and fresh catalog.
    pub fn admit(&self, app: &AppHandle, request: &super::Request) -> Result<Admission, Error> {
        let started = Instant::now();
        let deadline = tokio::time::Instant::from_std(started + super::REQUEST_TIMEOUT);
        request.prompt().map_err(|_| Error::Unavailable)?;
        if !self.current(app) {
            return Err(Error::Off);
        }
        if !vults_chat::summary::installed() {
            return Err(Error::Unavailable);
        }
        let directory = crate::history::dir();
        let mut store = vults_store::Store::open_current(&directory).map_err(|_| Error::Budget)?;
        let span = gate::shared(app)
            .reserve_voice_command_intent(&mut store, self.ledger.as_deref().ok_or(Error::Off)?)
            .ok_or(Error::Budget)?;
        let admission = Admission {
            reservation: gate::Reservation::new(span, directory),
            started,
            deadline,
        };
        if !self.current(app) {
            return Err(Error::Off);
        }
        Ok(admission)
    }

    fn current(&self, app: &AppHandle) -> bool {
        self.enabled()
            && app
                .state::<SettingsState>()
                .0
                .lock()
                .is_ok_and(|s| s.dictation_commands && s.voice_command_intent == *self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Off,
    Unavailable,
    Budget,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Returned,
    Invalid,
    Failed,
    Cancelled,
}

pub struct Admission {
    reservation: gate::Reservation,
    started: Instant,
    deadline: tokio::time::Instant,
}
impl std::fmt::Debug for Admission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntentAdmission").finish_non_exhaustive()
    }
}
impl Admission {
    pub fn deadline(&self) -> tokio::time::Instant {
        self.deadline
    }

    /// Outcomes are closed metadata, never raw heard text, model output or credentials.
    pub fn finish(self, outcome: Outcome) {
        if let Some(mut span) = self.reservation.take() {
            span.end_ms = span
                .start_ms
                .saturating_add(i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX));
            span.status = if outcome == Outcome::Returned {
                "ok"
            } else {
                "error"
            }
            .into();
            span.attrs["vults.outcome"] = serde_json::json!(match outcome {
                Outcome::Returned => "returned",
                Outcome::Invalid => "invalid",
                Outcome::Failed => "failed",
                Outcome::Cancelled => "cancelled",
            });
            crate::writer::send(crate::writer::Job::FinishRequest(span));
        }
    }
}
