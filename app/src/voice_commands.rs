//! V8: the runtime owner's local command dispatcher. No microphone, model, paste or card answer.
#[cfg(target_os = "linux")]
mod desktop;
pub(crate) mod flock;
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

#[derive(Clone, Debug, PartialEq)]
pub enum DesktopAction {
    Media(command::MediaAction),
    Output(command::OutputAction),
    Application(vults_platform::applications::Application),
    Flock(Box<flock::Step>),
}

#[derive(Debug)]
pub struct Report {
    pub feedback: Feedback,
    pub text: String,
    pub choices: Vec<Name>,
    /// Executed by the existing runtime effect loop only after the entire batch succeeds.
    pub effects: Vec<Effect>,
    pub desktop: Vec<DesktopAction>,
}

impl Report {
    pub fn chat_fallback(&self, enabled: bool) -> bool {
        enabled
            && self.feedback == Feedback::NotCommand
            && self.choices.is_empty()
            && self.effects.is_empty()
            && self.desktop.is_empty()
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
    started: Duration,
    commands: Vec<Command>,
    selected: BTreeMap<usize, Name>,
    index: usize,
    project: bool,
    question: Clarification,
    applications: Vec<vults_platform::applications::Application>,
    sessions: Vec<targets::SessionBinding>,
}

#[derive(Debug)]
pub struct Commands {
    epoch: Instant,
    last: Option<Context>,
    pending: Option<Pending>,
}

enum Scope<'a> {
    Flock,
    Desktop(Option<&'a [vults_platform::applications::Application]>),
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
        self.execute_scoped(state, transcript, replacements, usage, now, Scope::Flock)
    }

    pub fn execute_with_applications(
        &mut self,
        state: &mut State,
        transcript: &str,
        replacements: &[(String, String)],
        usage: Value,
        now: Instant,
        apps: Option<&[vults_platform::applications::Application]>,
    ) -> Report {
        self.execute_scoped(state, transcript, replacements, usage, now, Scope::Desktop(apps))
    }

    fn execute_scoped(
        &mut self,
        state: &mut State,
        transcript: &str,
        replacements: &[(String, String)],
        usage: Value,
        now: Instant,
        scope: Scope<'_>,
    ) -> Report {
        let (apps, native_external) = match scope {
            Scope::Flock => (Some(&[][..]), false),
            Scope::Desktop(apps) => (apps, true),
        };
        let lang = state.lang;
        if state.locked {
            self.cancel();
            return report(lang, Feedback::Locked);
        }
        if self.last.as_ref().is_some_and(|last| !recent(last.at, now)) {
            self.last = None;
        }
        let elapsed = now.checked_duration_since(self.epoch).unwrap_or(Duration::MAX);
        let (commands, mut selected, question_started) = match command::parse(transcript) {
            Parsed::Quoted => {
                self.pending = None;
                return report(lang, Feedback::Quoted);
            }
            Parsed::InvalidVolume => {
                self.pending = None;
                return report(lang, Feedback::InvalidVolume);
            }
            Parsed::Refused => {
                self.pending = None;
                return report(lang, Feedback::NotByVoice);
            }
            Parsed::Commands(commands) => {
                self.pending = None;
                (commands, BTreeMap::new(), None)
            }
            Parsed::NotCommand => {
                let Some(mut pending) = self.pending.take() else {
                    return report(lang, Feedback::NotCommand);
                };
                if pending.sessions.iter().any(|binding| !binding.current(state)) {
                    return report(lang, Feedback::Stale);
                }
                let candidates = pending.question.candidates(elapsed);
                if candidates.iter().chain(pending.selected.values()).any(|name| {
                    name.kind == command::names::NameKind::Application
                        && !pending
                            .applications
                            .iter()
                            .any(|original| original.id == name.id && apps.unwrap_or(&[]).contains(original))
                }) {
                    return report(lang, Feedback::Stale);
                }
                let include_apps = matches!(
                    pending.commands[pending.index],
                    Command::Open(_) | Command::Application(_)
                );
                let catalog = Catalog::with_applications(
                    state,
                    pending.project,
                    replacements,
                    if include_apps { apps.unwrap_or(&[]) } else { &[] },
                );
                match pending.question.answer(transcript, elapsed, &catalog.resolver) {
                    Reply::Resolution(Resolution::Found(name)) => {
                        pending.selected.insert(pending.index, name);
                        (pending.commands, pending.selected, Some(pending.started))
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
        let mut desktop = Vec::new();
        let mut has_native = false;
        let mut mixed_supported = true;
        for (index, original) in commands.iter().enumerate() {
            match original {
                Command::Media(action) => {
                    has_native = true;
                    desktop.push(DesktopAction::Media(*action));
                    continue;
                }
                Command::Output(action) => {
                    has_native = true;
                    desktop.push(DesktopAction::Output(*action));
                    continue;
                }
                _ => {}
            }
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
            has_native |= native_external && matches!(command, Command::Terminal(_) | Command::Folder(_));
            let target = target(&command);
            let session = if let Some(text) = target {
                let project = matches!(command, Command::SetPreference { .. });
                let include_apps = matches!(command, Command::Open(_) | Command::Application(_));
                let catalog = if include_apps {
                    Catalog::with_applications(&shadow, project, replacements, apps.unwrap_or(&[]))
                } else {
                    Catalog::new(&shadow, project, replacements)
                };
                if matches!(command, Command::Application(_)) && apps.is_none() {
                    return application_report(lang, i18n::ApplicationFeedback::Disabled);
                }
                let resolution = if let Some(name) = selected.get(&index) {
                    if catalog.session(name).is_none() && catalog.application(name).is_none() {
                        return report(lang, Feedback::Stale);
                    }
                    Resolution::Found(name.clone())
                } else {
                    catalog.resolver.resolve(
                        text,
                        matches!(command, Command::Application(_))
                            .then_some(command::names::NameKind::Application),
                    )
                };
                match resolution {
                    Resolution::Found(name) => {
                        if let Some(app) = catalog.application(&name) {
                            selected.insert(index, name);
                            has_native = true;
                            desktop.push(DesktopAction::Application(app));
                            continue;
                        }
                        let session = catalog.session(&name);
                        // A later question must not rebind an earlier target after the flock changes.
                        selected.insert(index, name);
                        session
                    }
                    Resolution::Unknown => {
                        if matches!(command, Command::Application(_))
                            || matches!(command, Command::Open(_)) && apps.is_none()
                        {
                            return application_report(
                                lang,
                                if apps.is_none() {
                                    i18n::ApplicationFeedback::Disabled
                                } else {
                                    i18n::ApplicationFeedback::Missing
                                },
                            );
                        }
                        let mut out = report(lang, Feedback::UnknownName);
                        if matches!(command, Command::Open(_)) {
                            out.text =
                                i18n::voice_application(lang, i18n::ApplicationFeedback::NoTarget).into();
                        }
                        return out;
                    }
                    Resolution::Ambiguous(choices) => {
                        let Some(question) =
                            Clarification::new(choices.clone(), question_started.unwrap_or(elapsed))
                        else {
                            return report(lang, Feedback::Stale);
                        };
                        let other_catalog = Catalog::new(state, !project, &[]);
                        let Some(sessions) = choices
                            .iter()
                            .map(|name| (index, name))
                            .chain(selected.iter().map(|(index, name)| (*index, name)))
                            .filter(|(_, name)| name.kind != command::names::NameKind::Application)
                            .map(|(index, name)| {
                                let binding_catalog =
                                    if (name.kind == command::names::NameKind::Project) == project {
                                        &catalog
                                    } else {
                                        &other_catalog
                                    };
                                targets::SessionBinding::capture(
                                    binding_catalog,
                                    state,
                                    name,
                                    matches!(commands[index], Command::Terminal(_) | Command::Repeat(_)),
                                )
                            })
                            .collect::<Option<Vec<_>>>()
                        else {
                            return report(lang, Feedback::Stale);
                        };
                        self.pending = Some(Pending {
                            #[cfg(target_os = "linux")]
                            deadline: self.epoch + question_started.unwrap_or(elapsed) + CONTEXT_WINDOW,
                            started: question_started.unwrap_or(elapsed),
                            commands,
                            selected,
                            index,
                            project,
                            question,
                            applications: apps.unwrap_or(&[]).to_vec(),
                            sessions,
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
                Command::Media(_) | Command::Application(_) | Command::Output(_) => {
                    return report(lang, Feedback::Unavailable);
                }
                Command::Next if !shadow.ordered().is_empty() => Some(Intent::FocusNext),
                Command::Previous if !shadow.ordered().is_empty() => Some(Intent::FocusPrevious),
                Command::Focus(_) | Command::Open(_) => Some(Intent::Focus {
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
            let step = (!matches!(original, Command::Repeat(_) | Command::Undo))
                .then(|| flock::Step::plan(&shadow, &command, session.clone(), intent.as_ref()))
                .flatten();
            if let Some(step) = step {
                desktop.push(DesktopAction::Flock(Box::new(step)));
            } else {
                mixed_supported = false;
            }
            if let Some(intent) = intent {
                if !intent.voice_command_allowed() {
                    return report(lang, Feedback::NotByVoice);
                }
                let planned = core::reduce(&mut shadow, Input::User(intent), now);
                if matches!(command, Command::Folder(_) | Command::Terminal(_)) && planned.is_empty() {
                    return report(lang, Feedback::Unavailable);
                }
                effects.extend(planned);
                if matches!(
                    command,
                    Command::Focus(_) | Command::Open(_) | Command::Next | Command::Previous
                ) {
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
        if has_native {
            self.last = None;
            if !mixed_supported {
                return report(lang, Feedback::Unavailable);
            }
            let mut out = report(lang, Feedback::Requested);
            out.desktop = desktop;
            return out;
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
        desktop: Vec::new(),
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
        | Command::Open(t)
        | Command::Application(t)
        | Command::Terminal(t)
        | Command::Folder(t)
        | Command::Session(t)
        | Command::SetPreference { target: t, .. } => Some(t),
        _ => None,
    }
}
fn retarget(command: &Command, target: String) -> Option<Command> {
    Some(match command {
        Command::Focus(_) | Command::Open(_) | Command::Next | Command::Previous => Command::Focus(target),
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

#[cfg(target_os = "linux")]
pub(crate) use desktop::start as start_desktop;

fn application_report(lang: i18n::Lang, feedback: i18n::ApplicationFeedback) -> Report {
    let mut out = report(lang, Feedback::Unavailable);
    out.text = i18n::voice_application(lang, feedback).into();
    out
}
