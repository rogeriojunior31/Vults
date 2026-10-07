//! Desktop notifications: what to tell the user when they may not be looking at the island. Pure:
//! [`Notifier::update`] compares what the state calls for with what is already shown, and the app
//! only sends the changes.
//!
//! A notification can only bring the island up ([`open`]); it never answers a card (ADR 0004).

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::{AgentKind, Intent, Pending, Presence, SessionKey, State, Status, i18n};

/// Where a card opens the island at the top (*Island*, *Quiet*) and plays its sound, a
/// notification would only repeat it: one comes when the card has waited this long, for a user
/// away from the screen.
pub const NEEDS_YOU_AFTER: Duration = Duration::from_secs(20);

/// Longest body, in characters: a notification is a glance, the island holds the rest.
const MAX_BODY: usize = 160;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A permission or a question card waits for the user.
    NeedsYou,
    Finished,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: Kind,
    /// Plain text.
    pub title: String,
    /// Plain text, maybe empty.
    pub body: String,
}

/// One notification per session at most: a newer one replaces it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Show { session: SessionKey, notice: Notice },
    Withdraw { session: SessionKey },
}

/// What the user chose. The preset is the state's: *Paused* shows none, and by the panel
/// (*Panel*) a card notifies at once, the island being out of sight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prefs {
    /// Settings → General → Notifications.
    pub on: bool,
}

/// What is on the desktop now, per session.
#[derive(Debug, Default)]
pub struct Notifier {
    shown: BTreeMap<SessionKey, Notice>,
    /// News (finished, failed) that came while notifications were off or the app was paused:
    /// old by the time they are back on, so it is not raised then.
    missed: BTreeMap<SessionKey, Notice>,
}

impl Notifier {
    /// The changes that bring the desktop in line with `state`. A card answered, a session back
    /// at work or gone, or notifications switched off withdraw what was shown.
    pub fn update(&mut self, state: &State, now: Instant, prefs: Prefs) -> Vec<Change> {
        let all = wanted(state, now, state.presence == Presence::Panel);
        let wanted = if prefs.on && state.presence != Presence::Paused {
            self.missed.retain(|k, n| all.get(k) == Some(n));
            let missed = &self.missed;
            all.into_iter()
                .filter(|(k, n)| missed.get(k) != Some(n))
                .collect()
        } else {
            self.missed = all
                .into_iter()
                .filter(|(_, n)| n.kind != Kind::NeedsYou)
                .collect();
            BTreeMap::new()
        };
        let mut changes: Vec<Change> = self
            .shown
            .keys()
            .filter(|k| !wanted.contains_key(*k))
            .map(|k| Change::Withdraw { session: k.clone() })
            .collect();
        changes.extend(
            wanted
                .iter()
                .filter(|(k, n)| self.shown.get(*k) != Some(*n))
                .map(|(k, n)| Change::Show {
                    session: k.clone(),
                    notice: n.clone(),
                }),
        );
        self.shown = wanted;
        changes
    }
}

/// A click on a notification: that session in front, and the island up (the app's part). The
/// only thing a notification does; a card's own session is in front anyway.
pub fn open(session: &SessionKey) -> Intent {
    Intent::Focus {
        session: Some(session.clone()),
    }
}

fn wanted(state: &State, now: Instant, at_once: bool) -> BTreeMap<SessionKey, Notice> {
    state
        .sessions
        .values()
        .filter_map(|s| {
            // A muted project tells nothing; a hidden one only its waiting card (ADR 0009: the
            // island still opens on a muted project's card).
            if state.prefs(s).mute || !state.visible(s) {
                return None;
            }
            let who = who(state.lang, &s.key, &s.project);
            let card = state
                .pending
                .iter()
                .find(|p| p.session == s.key && (at_once || now.duration_since(p.since) >= NEEDS_YOU_AFTER));
            let (kind, body) = match (card, s.status) {
                (Some(p), _) => (Kind::NeedsYou, ask(p)),
                (None, Status::Finished) => (Kind::Finished, s.note.clone().unwrap_or_default()),
                (None, Status::Failed) => (Kind::Failed, s.note.clone().unwrap_or_default()),
                _ => return None,
            };
            let notice = Notice {
                kind,
                title: i18n::notice_title(state.lang, kind, &who),
                body: clip(&body),
            };
            Some((s.key.clone(), notice))
        })
        .collect()
}

/// What the card asks: the question, or the agent's words for the action, or what it would run.
fn ask(p: &Pending) -> String {
    match p.questions.first() {
        Some(q) => q.question.clone(),
        None => p.ask.description.clone().unwrap_or_else(|| p.target.clone()),
    }
}

/// The project, or the agent when the session has no folder.
fn who(lang: i18n::Lang, key: &SessionKey, project: &str) -> String {
    if !project.is_empty() {
        return project.to_string();
    }
    match key.agent {
        AgentKind::Claude => "Claude Code".into(),
        AgentKind::Codex => "Codex".into(),
        AgentKind::Gemini => "Gemini CLI".into(),
        AgentKind::Other => match key.session_id.split_once('/') {
            Some((name, _)) if !name.is_empty() => name.into(),
            _ => i18n::some_agent(lang).into(),
        },
    }
}

/// One line, cut at [`MAX_BODY`] characters.
fn clip(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= MAX_BODY {
        return line;
    }
    let mut cut: String = line.chars().take(MAX_BODY - 1).collect();
    cut.push('…');
    cut
}
