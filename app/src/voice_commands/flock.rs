//! Prepared flock steps bind only the state their command depends on.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    command: Command,
    session: Option<SessionKey>,
    intent: Option<Intent>,
    targets: Vec<(SessionKey, Instant, Option<String>)>,
    focus: Option<(Option<SessionKey>, Option<SessionKey>)>,
    order: Option<Vec<SessionKey>>,
    pref: Option<(SessionKey, ProjectPref, bool)>,
}

impl Step {
    pub(super) fn plan(
        state: &State,
        command: &Command,
        session: Option<SessionKey>,
        intent: Option<&Intent>,
    ) -> Option<Self> {
        let navigation = matches!(
            command,
            Command::Focus(_) | Command::Open(_) | Command::Next | Command::Previous
        );
        if !navigation
            && !matches!(
                command,
                Command::SetPreference { .. }
                    | Command::Waiting
                    | Command::Session(_)
                    | Command::Usage
                    | Command::PullRequests
            )
        {
            return None;
        }
        let order = matches!(command, Command::Next | Command::Previous)
            .then(|| state.ordered().iter().map(|s| s.key.clone()).collect::<Vec<_>>());
        let targets = session
            .iter()
            .chain(order.iter().flatten())
            .map(|key| {
                let target = state.sessions.get(key)?;
                Some((key.clone(), target.started, target.cwd.clone()))
            })
            .collect::<Option<Vec<_>>>()?;
        let pref = if let Some(Intent::SetProjectPref { session, pref, .. }) = intent {
            Some((session.clone(), *pref, preference(state, session, *pref)))
        } else {
            None
        };
        Some(Self {
            command: command.clone(),
            session,
            intent: intent.cloned(),
            targets,
            focus: navigation.then(|| (state.focus.clone(), state.front().cloned())),
            order,
            pref,
        })
    }

    #[cfg(any(target_os = "linux", test))]
    pub(crate) fn apply(
        &self,
        state: &mut State,
        usage: Value,
        now: Instant,
    ) -> Result<(Vec<Effect>, String), ()> {
        if state.locked
            || self.targets.iter().any(|(key, started, cwd)| {
                !state
                    .sessions
                    .get(key)
                    .is_some_and(|s| s.started == *started && s.cwd == *cwd)
            })
            || self
                .focus
                .as_ref()
                .is_some_and(|(focus, front)| state.focus != *focus || state.front() != front.as_ref())
            || self
                .order
                .as_ref()
                .is_some_and(|order| state.ordered().iter().map(|s| &s.key).ne(order.iter()))
            || self
                .pref
                .as_ref()
                .is_some_and(|(key, pref, expected)| preference(state, key, *pref) != *expected)
        {
            return Err(());
        }
        if let Some(intent) = &self.intent {
            if !matches!(
                intent,
                Intent::Focus { .. }
                    | Intent::FocusNext
                    | Intent::FocusPrevious
                    | Intent::SetProjectPref { .. }
            ) || !intent.voice_command_allowed()
            {
                return Err(());
            }
            Ok((
                core::reduce(state, Input::User(intent.clone()), now),
                String::new(),
            ))
        } else {
            read(state, &self.command, self.session.as_ref(), usage)
                .map(|text| (vec![], text))
                .ok_or(())
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(crate) enum Kind {
    Preflight(Vec<Step>),
    Apply(Box<Step>),
}

#[cfg(target_os = "linux")]
impl Kind {
    pub(crate) fn run(
        &self,
        state: &mut State,
        usage: Value,
        now: Instant,
    ) -> Result<(Vec<Effect>, String), ()> {
        match self {
            Self::Preflight(steps) => {
                let mut shadow = state.clone();
                for step in steps {
                    step.apply(&mut shadow, usage.clone(), now)?;
                }
                Ok((vec![], String::new()))
            }
            Self::Apply(step) => step.apply(state, usage, now),
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(crate) struct Request {
    pub kind: Kind,
    pub ticket: crate::dictation::worker::Ticket,
    pub deadline: Instant,
    pub scopes: (bool, bool, bool),
    pub reply: tokio::sync::oneshot::Sender<Result<String, ()>>,
}

#[cfg(target_os = "linux")]
impl Request {
    pub(crate) fn ready(&self, now: Instant) -> bool {
        !self.reply.is_closed() && now < self.deadline
    }

    pub(crate) fn dispatch(
        &self,
        state: &mut State,
        usage: Value,
        now: Instant,
        locked: impl Fn() -> bool,
    ) -> Result<(Vec<Effect>, String), ()> {
        self.ticket
            .dispatch(|| {
                if !self.ready(Instant::now()) || locked() || !self.ticket.is_command() {
                    return Err(());
                }
                self.kind.run(state, usage, now)
            })
            .unwrap_or(Err(()))
    }

    pub(crate) fn consent(&self, saved: &crate::settings::Settings) -> bool {
        super::desktop::consent(saved, self.scopes.0, self.scopes.1, self.scopes.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn planned(now: Instant, text: &str) -> (State, Vec<Step>) {
        let mut state = super::super::tests::fixture(now);
        let mut commands = Commands::new(now);
        let out = commands.execute(&mut state, text, &[], json!([]), now);
        assert_eq!(out.feedback, Feedback::Requested);
        let steps = out
            .desktop
            .into_iter()
            .filter_map(|action| match action {
                DesktopAction::Flock(step) => Some(*step),
                _ => None,
            })
            .collect();
        (state, steps)
    }

    #[test]
    fn guards_reject_changed_targets_and_user_edits_but_allow_unrelated_progress() {
        let now = Instant::now();
        let (state, steps) = planned(now, "mute api and then pause media and then open web");
        let mut live = state.clone();
        live.sessions.values_mut().next().unwrap().step_count += 1;
        assert!(steps[0].apply(&mut live, json!([]), now).is_ok());
        assert!(steps[1].apply(&mut live, json!([]), now).is_ok());
        let mut live = state.clone();
        let key = steps[0].session.clone().unwrap();
        live.sessions.get_mut(&key).unwrap().started += Duration::from_secs(1);
        let before = format!("{live:?}");
        assert!(steps[0].apply(&mut live, json!([]), now).is_err());
        assert_eq!(format!("{live:?}"), before);
        let mut live = state.clone();
        live.sessions.get_mut(&key).unwrap().cwd = Some("/moved".into());
        assert!(steps[0].apply(&mut live, json!([]), now).is_err());
        let mut live = state.clone();
        core::reduce(
            &mut live,
            Input::User(Intent::SetProjectPref {
                session: key.clone(),
                pref: ProjectPref::Mute,
                on: true,
            }),
            now,
        );
        assert!(steps[0].apply(&mut live, json!([]), now).is_err());
        let mut live = state.clone();
        live.focus = Some(key);
        assert!(steps[1].apply(&mut live, json!([]), now).is_err());
        let (mut live, steps) = planned(now, "pause media and then next");
        let key = live.ordered()[1].key.clone();
        core::reduce(
            &mut live,
            Input::User(Intent::SetProjectPref {
                session: key,
                pref: ProjectPref::Pin,
                on: true,
            }),
            now,
        );
        assert!(steps[0].apply(&mut live, json!([]), now).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn preflight_is_shadow_only_and_queued_expired_cancelled_or_closed_work_is_inert() {
        let now = Instant::now();
        let (state, steps) = planned(now, "mute api and then pause media and then open web");
        let mut live = state.clone();
        let before = format!("{live:?}");
        Kind::Preflight(steps.clone())
            .run(&mut live, json!([]), now)
            .unwrap();
        assert_eq!(format!("{live:?}"), before);
        let key = steps[1].session.clone().unwrap();
        live.sessions.remove(&key);
        let before = format!("{live:?}");
        assert!(
            Kind::Preflight(steps.clone())
                .run(&mut live, json!([]), now)
                .is_err()
        );
        assert_eq!(
            format!("{live:?}"),
            before,
            "invalid suffix cannot apply its prefix"
        );
        for mode in 0..5 {
            let (reply, receive) = tokio::sync::oneshot::channel();
            let ticket = crate::dictation::worker::Ticket::test_command();
            if mode == 1 {
                ticket.test_cancel();
            }
            let request = Request {
                kind: Kind::Apply(Box::new(steps[0].clone())),
                ticket,
                deadline: if mode == 2 {
                    now - Duration::from_secs(1)
                } else {
                    now + Duration::from_secs(2)
                },
                scopes: (true, false, false),
                reply,
            };
            let mut live = state.clone();
            let before = format!("{live:?}");
            if mode == 3 {
                drop(receive);
            }
            let result = request.dispatch(&mut live, json!([]), now, || mode == 4);
            if mode == 0 {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
                assert_eq!(format!("{live:?}"), before);
            }
        }
    }
}
