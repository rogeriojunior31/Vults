//! V8: the runtime owner's local command dispatcher. No microphone, model, paste or card answer.
mod targets;
#[cfg(test)]
mod tests;

use serde_json::Value;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use targets::Catalog;
use vults_core::{self as core, Effect, Input, Intent, ProjectPref, SessionKey, State, i18n};
use vults_dictation::command::{
    self, Command, Parsed, Preference,
    names::{Clarification, Name, Reply, Resolution},
};

pub use i18n::VoiceFeedback as Feedback;
const CONTEXT_WINDOW: Duration = Duration::from_secs(90);

#[derive(Debug)]
pub struct Report {
    pub feedback: Feedback,
    pub text: String,
    pub choices: Vec<Name>,
    /// Executed by the existing runtime effect loop only after the entire batch succeeds.
    pub effects: Vec<Effect>,
}

impl Report {
    pub fn chat_fallback(&self, enabled: bool) -> bool {
        enabled && self.feedback == Feedback::NotCommand && self.choices.is_empty() && self.effects.is_empty()
    }
}

#[derive(Clone, Debug)]
struct Context {
    at: Instant,
    command: Command,
    undo: Option<Undo>,
}

#[derive(Clone, Debug)]
enum Undo {
    Focus {
        previous: Option<SessionKey>,
        expected: Option<SessionKey>,
    },
    Preference {
        session: SessionKey,
        cwd: String,
        pref: ProjectPref,
        previous: bool,
        expected: bool,
    },
}

#[derive(Debug)]
struct Pending {
    #[cfg(target_os = "linux")]
    deadline: Instant,
    commands: Vec<Command>,
    selected: BTreeMap<usize, Name>,
    index: usize,
    project: bool,
    question: Clarification,
}

#[derive(Debug)]
pub struct Commands {
    epoch: Instant,
    last: Option<Context>,
    pending: Option<Pending>,
}

impl Commands {
    #[cfg(target_os = "linux")]
    pub(crate) fn question_deadline(&self) -> Option<Instant> {
        self.pending.as_ref().map(|pending| pending.deadline)
    }
    pub fn new(now: Instant) -> Self {
        Self {
            epoch: now,
            last: None,
            pending: None,
        }
    }

    pub fn cancel(&mut self) {
        self.last = None;
        self.pending = None;
    }

    /// Called only by the one state owner; returned effects must run through its existing executor.
    /// A shadow state keeps unresolved, refused or unavailable batches from applying a prefix.
    pub fn execute(
        &mut self,
        state: &mut State,
        transcript: &str,
        replacements: &[(String, String)],
        usage: Value,
        now: Instant,
    ) -> Report {
        let lang = state.lang;
        if state.locked {
            self.cancel();
            return report(lang, Feedback::Locked);
        }
        if self.last.as_ref().is_some_and(|last| !recent(last.at, now)) {
            self.last = None;
        }
        let elapsed = now.checked_duration_since(self.epoch).unwrap_or(Duration::MAX);
        let (commands, mut selected) = match command::parse(transcript) {
            Parsed::Refused => {
                self.pending = None;
                return report(lang, Feedback::NotByVoice);
            }
            Parsed::Commands(commands) => {
                self.pending = None;
                (commands, BTreeMap::new())
            }
            Parsed::NotCommand => {
                let Some(mut pending) = self.pending.take() else {
                    return report(lang, Feedback::NotCommand);
                };
                let catalog = Catalog::new(state, pending.project, replacements);
                match pending.question.answer(transcript, elapsed, &catalog.resolver) {
                    Reply::Resolution(Resolution::Found(name)) => {
                        pending.selected.insert(pending.index, name);
                        (pending.commands, pending.selected)
                    }
                    Reply::Resolution(result) => {
                        let mut out = report(
                            lang,
                            if matches!(result, Resolution::Ambiguous(_)) {
                                Feedback::WhichOne
                            } else {
                                Feedback::UnknownName
                            },
                        );
                        out.choices = pending.question.candidates(elapsed).to_vec();
                        self.pending = Some(pending);
                        return out;
                    }
                    Reply::Expired => return report(lang, Feedback::Expired),
                    Reply::Stale | Reply::Inactive => return report(lang, Feedback::Stale),
                }
            }
        };
        let mut shadow = state.clone();
        let mut context = self.last.clone();
        let mut effects = Vec::new();
        let mut answers = Vec::new();
        for (index, original) in commands.iter().enumerate() {
            let command = if let Command::Repeat(target) = original {
                let Some(last) = context.as_ref().filter(|last| recent(last.at, now)) else {
                    return report(lang, Feedback::NoContext);
                };
                let Some(repeated) = retarget(&last.command, target.clone()) else {
                    return report(lang, Feedback::NoContext);
                };
                repeated
            } else {
                original.clone()
            };
            let target = target(&command);
            let session = if let Some(text) = target {
                let project = matches!(command, Command::SetPreference { .. });
                let catalog = Catalog::new(&shadow, project, replacements);
                let resolution = if let Some(name) = selected.get(&index) {
                    if catalog.session(name).is_none() {
                        return report(lang, Feedback::Stale);
                    }
                    Resolution::Found(name.clone())
                } else {
                    catalog.resolver.resolve(text, None)
                };
                match resolution {
                    Resolution::Found(name) => {
                        let session = catalog.session(&name);
                        // A later question must not rebind an earlier target after the flock changes.
                        selected.insert(index, name);
                        session
                    }
                    Resolution::Unknown => return report(lang, Feedback::UnknownName),
                    Resolution::Ambiguous(choices) => {
                        let Some(question) = Clarification::new(choices.clone(), elapsed) else {
                            return report(lang, Feedback::Stale);
                        };
                        self.pending = Some(Pending {
                            #[cfg(target_os = "linux")]
                            deadline: now + CONTEXT_WINDOW,
                            commands,
                            selected,
                            index,
                            project,
                            question,
                        });
                        let mut out = report(lang, Feedback::WhichOne);
                        out.choices = choices;
                        return out;
                    }
                }
            } else {
                None
            };
            let previous = shadow.front().cloned();
            let mut undo = None;
            let intent = match &command {
                Command::Next if !shadow.ordered().is_empty() => Some(Intent::FocusNext),
                Command::Previous if !shadow.ordered().is_empty() => Some(Intent::FocusPrevious),
                Command::Focus(_) => Some(Intent::Focus {
                    session: session.clone(),
                }),
                Command::Terminal(_) => session.clone().map(|session| Intent::Jump { session }),
                Command::Folder(_) => session.clone().map(|session| Intent::OpenFolder { session }),
                Command::SetPreference {
                    preference: pref, on, ..
                } => {
                    let Some(session) = session.clone() else {
                        return report(lang, Feedback::UnknownName);
                    };
                    let Some(cwd) = shadow.sessions[&session].cwd.clone() else {
                        return report(lang, Feedback::Unavailable);
                    };
                    let pref = project_pref(*pref);
                    undo = Some(Undo::Preference {
                        cwd,
                        previous: preference(&shadow, &session, pref),
                        session: session.clone(),
                        pref,
                        expected: *on,
                    });
                    Some(Intent::SetProjectPref {
                        session,
                        pref,
                        on: *on,
                    })
                }
                Command::Undo => {
                    let Some(last) = context.take().filter(|last| recent(last.at, now)) else {
                        return report(lang, Feedback::NoContext);
                    };
                    let Some(intent) = last.undo.as_ref().and_then(|undo| undo.intent(&shadow)) else {
                        return report(lang, Feedback::CannotUndo);
                    };
                    Some(intent)
                }
                Command::Waiting | Command::Session(_) | Command::Usage | Command::PullRequests => {
                    match read(&shadow, &command, session.as_ref(), usage.clone()) {
                        Some(text) => answers.push(text),
                        None => return report(lang, Feedback::Unavailable),
                    }
                    None
                }
                Command::Repeat(_) => unreachable!("Repeat is expanded before dispatch"),
                Command::Next | Command::Previous => return report(lang, Feedback::Unavailable),
            };
            if let Some(intent) = intent {
                if !intent.voice_command_allowed() {
                    return report(lang, Feedback::NotByVoice);
                }
                let planned = core::reduce(&mut shadow, Input::User(intent), now);
                if matches!(command, Command::Folder(_) | Command::Terminal(_)) && planned.is_empty() {
                    return report(lang, Feedback::Unavailable);
                }
                effects.extend(planned);
                if matches!(command, Command::Focus(_) | Command::Next | Command::Previous) {
                    undo = Some(Undo::Focus {
                        previous,
                        expected: shadow.focus.clone(),
                    });
                }
            }
            if !matches!(command, Command::Undo) {
                context = Some(Context {
                    at: now,
                    command,
                    undo,
                });
            }
        }
        *state = shadow;
        self.last = context;
        let mut out = report(
            lang,
            if answers.is_empty() {
                Feedback::Done
            } else {
                Feedback::Answer
            },
        );
        if !answers.is_empty() {
            out.text = answers.join("\n");
        }
        out.effects = effects;
        out
    }
}

impl Undo {
    fn intent(&self, state: &State) -> Option<Intent> {
        match self {
            Self::Focus { previous, expected }
                if state.focus == *expected
                    && previous
                        .as_ref()
                        .is_none_or(|key| state.sessions.contains_key(key)) =>
            {
                Some(Intent::Focus {
                    session: previous.clone(),
                })
            }
            Self::Preference {
                session,
                cwd,
                pref,
                previous,
                expected,
            } if state
                .sessions
                .get(session)
                .is_some_and(|s| s.cwd.as_ref() == Some(cwd))
                && preference(state, session, *pref) == *expected =>
            {
                Some(Intent::SetProjectPref {
                    session: session.clone(),
                    pref: *pref,
                    on: *previous,
                })
            }
            _ => None,
        }
    }
}

fn recent(at: Instant, now: Instant) -> bool {
    now.checked_duration_since(at)
        .is_some_and(|elapsed| elapsed <= CONTEXT_WINDOW)
}
fn report(lang: i18n::Lang, feedback: Feedback) -> Report {
    Report {
        feedback,
        text: i18n::voice_feedback(lang, feedback).into(),
        choices: Vec::new(),
        effects: Vec::new(),
    }
}
fn project_pref(pref: Preference) -> ProjectPref {
    match pref {
        Preference::Mute => ProjectPref::Mute,
        Preference::Pin => ProjectPref::Pin,
        Preference::Hide => ProjectPref::Hide,
    }
}
fn preference(state: &State, key: &SessionKey, pref: ProjectPref) -> bool {
    let prefs = state
        .sessions
        .get(key)
        .map(|s| state.prefs(s))
        .unwrap_or_default();
    match pref {
        ProjectPref::Mute => prefs.mute,
        ProjectPref::Pin => prefs.pin,
        ProjectPref::Hide => prefs.hide,
    }
}
fn target(command: &Command) -> Option<&str> {
    match command {
        Command::Focus(t)
        | Command::Terminal(t)
        | Command::Folder(t)
        | Command::Session(t)
        | Command::SetPreference { target: t, .. } => Some(t),
        _ => None,
    }
}
fn retarget(command: &Command, target: String) -> Option<Command> {
    Some(match command {
        Command::Focus(_) | Command::Next | Command::Previous => Command::Focus(target),
        Command::Terminal(_) => Command::Terminal(target),
        Command::Folder(_) => Command::Folder(target),
        Command::Session(_) => Command::Session(target),
        Command::SetPreference { preference, on, .. } => Command::SetPreference {
            target,
            preference: *preference,
            on: *on,
        },
        _ => return None,
    })
}

fn read(state: &State, command: &Command, session: Option<&SessionKey>, usage: Value) -> Option<String> {
    let lang = state.lang;
    let view = serde_json::to_value(state.view()).ok()?;
    let name = match command {
        Command::Waiting => "pending_asks",
        Command::Session(_) => "session_detail",
        Command::Usage => "usage",
        Command::PullRequests => "github_snapshot",
        _ => return None,
    };
    let value = if name == "session_detail" {
        crate::flock_tools::session_detail_exact(&view, session?)?
    } else {
        crate::flock_tools::cached_read(&view, usage, name)?
    };
    let field = |key: &str| value[key].as_str().unwrap_or_default();
    match command {
        Command::Session(_) => {
            let status = serde_json::from_value(value["status"].clone()).ok()?;
            let mut text = i18n::voice_session(lang, field("agent"), field("project"), status);
            if let Some(doing) = value["doing"].as_str() {
                text.push('\n');
                text.push_str(doing);
            }
            Some(text)
        }
        Command::Waiting => {
            let rows = value["waiting"].as_array()?;
            Some(if rows.is_empty() {
                i18n::voice_feedback(lang, Feedback::NoWaiting).into()
            } else {
                rows.iter()
                    .map(|row| {
                        i18n::voice_waiting(
                            lang,
                            row["agent"].as_str().unwrap_or_default(),
                            row["project"].as_str().unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        Command::Usage => {
            let rows = value["windows"].as_array()?;
            let text: Vec<_> = rows
                .iter()
                .filter_map(|row| {
                    Some(i18n::voice_usage(
                        lang,
                        row["agent"].as_str()?,
                        row["used_percent"].as_u64()?,
                        row["minutes"].as_u64()?,
                    ))
                })
                .collect();
            Some(if text.is_empty() {
                i18n::voice_feedback(lang, Feedback::NoUsage).into()
            } else {
                text.join("\n")
            })
        }
        Command::PullRequests => {
            let rows: Vec<_> = value["rows"]
                .as_array()?
                .iter()
                .filter(|row| matches!(row["group"].as_str(), Some("yours" | "to-review")))
                .collect();
            Some(if rows.is_empty() {
                i18n::voice_feedback(lang, Feedback::NoPullRequests).into()
            } else {
                rows.iter()
                    .map(|row| {
                        i18n::voice_pull_request(
                            lang,
                            &core::redact::secrets(row["name"].as_str().unwrap_or_default()),
                            &core::redact::secrets(row["title"].as_str().unwrap_or_default()),
                            match row["checks"].as_str() {
                                Some("passing") => Some(core::board::Checks::Passing),
                                Some("failing") => Some(core::board::Checks::Failing),
                                Some("running") => Some(core::board::Checks::Running),
                                _ => None,
                            },
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        _ => None,
    }
}
