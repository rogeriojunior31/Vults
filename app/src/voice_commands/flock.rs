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
    terminal: Option<vults_protocol::Terminal>,
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
                Command::Terminal(_)
                    | Command::Folder(_)
                    | Command::SetPreference { .. }
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
        let terminal = if matches!(command, Command::Terminal(_)) {
            Some(state.sessions.get(session.as_ref()?)?.terminal.clone())
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
            terminal,
        })
    }

    #[cfg(target_os = "linux")]
    pub(super) fn folder_path(&self) -> Option<&str> {
        matches!(self.command, Command::Folder(_))
            .then(|| self.targets.first()?.2.as_deref())
            .flatten()
    }

    #[cfg(target_os = "linux")]
    pub(super) fn terminal(&self) -> Option<&vults_protocol::Terminal> {
        self.terminal.as_ref()
    }

    #[cfg(any(target_os = "linux", test))]
    fn check(&self, state: &State) -> Result<(), ()> {
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
        if self.terminal.as_ref().is_some_and(|expected| {
            self.session
                .as_ref()
                .and_then(|key| state.sessions.get(key))
                .is_none_or(|s| &s.terminal != expected)
        }) {
            return Err(());
        }
        Ok(())
    }

    #[cfg(any(target_os = "linux", test))]
    pub(crate) fn apply(
        &self,
        state: &mut State,
        usage: Value,
        now: Instant,
    ) -> Result<(Vec<Effect>, String), ()> {
        self.check(state)?;
        if let Some(intent) = &self.intent {
            if !matches!(
                intent,
                Intent::Focus { .. }
                    | Intent::FocusNext
                    | Intent::FocusPrevious
                    | Intent::SetProjectPref { .. }
                    | Intent::Jump { .. }
                    | Intent::OpenFolder { .. }
            ) || !intent.voice_command_allowed()
            {
                return Err(());
            }
            let effects = core::reduce(state, Input::User(intent.clone()), now);
            if matches!(self.command, Command::Terminal(_) | Command::Folder(_)) && effects.is_empty() {
                return Err(());
            }
            Ok((effects, String::new()))
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
    Check(Box<Step>),
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
            Self::Check(step) => step.check(state).map(|_| (vec![], String::new())),
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

#[cfg(target_os = "linux")]
pub(crate) struct Completion {
    request: Request,
    receipts: Vec<tokio::sync::oneshot::Receiver<bool>>,
}

#[cfg(target_os = "linux")]
impl Completion {
    pub(crate) fn new(request: Request) -> Self {
        Self {
            request,
            receipts: vec![],
        }
    }

    pub(crate) fn work(&mut self) -> Work {
        let (reply, receive) = tokio::sync::oneshot::channel();
        self.receipts.push(receive);
        let Kind::Apply(step) = &self.request.kind else {
            unreachable!("only applied steps execute effects")
        };
        Work {
            step: step.clone(),
            ticket: self.request.ticket.clone(),
            deadline: self.request.deadline,
            scopes: self.request.scopes,
            reply,
        }
    }

    pub(crate) fn finish(self, text: String, saved: bool, lang: i18n::Lang) {
        tauri::async_runtime::spawn(async move {
            let Self { request, receipts } = self;
            let external = match &request.kind {
                Kind::Apply(step) if matches!(step.command, Command::Terminal(_) | Command::Folder(_)) => {
                    Some(matches!(step.command, Command::Folder(_)))
                }
                _ => None,
            };
            let Request {
                mut reply, deadline, ..
            } = request;
            let result = tokio::select! {
                biased;
                _ = reply.closed() => return,
                result = tokio::time::timeout_at(deadline.into(), receipts_ok(receipts)) => saved && matches!(result, Ok(true)),
            };
            let text = external.map_or(text, |folder| i18n::voice_external_requested(lang, folder).into());
            let _ = reply.send(if result { Ok(text) } else { Err(()) });
        });
    }
}

#[cfg(target_os = "linux")]
async fn receipts_ok(receipts: Vec<tokio::sync::oneshot::Receiver<bool>>) -> bool {
    for receipt in receipts {
        if !matches!(receipt.await, Ok(true)) {
            return false;
        }
    }
    true
}

#[cfg(target_os = "linux")]
pub(crate) struct Work {
    step: Box<Step>,
    ticket: crate::dictation::worker::Ticket,
    deadline: Instant,
    scopes: (bool, bool, bool),
    reply: tokio::sync::oneshot::Sender<bool>,
}

#[cfg(target_os = "linux")]
impl Work {
    fn ready(&self, now: Instant) -> bool {
        !self.reply.is_closed() && self.ticket.valid() && now < self.deadline
    }

    pub(crate) fn run(self, app: &tauri::AppHandle, action: impl FnOnce(&dyn Fn() -> bool) -> bool) {
        let allowed = || {
            self.ready(Instant::now())
                && super::desktop::enabled(
                    app,
                    &self.ticket,
                    self.deadline,
                    self.scopes.0,
                    self.scopes.1,
                    self.scopes.2,
                )
        };
        let checked = allowed()
            && tauri::async_runtime::block_on(async {
                matches!(
                    tokio::time::timeout_at(
                        self.deadline.into(),
                        crate::runtime::flock_command(
                            app,
                            Kind::Check(self.step.clone()),
                            self.ticket.clone(),
                            self.deadline,
                            self.scopes
                        )
                    )
                    .await,
                    Ok(Ok(_))
                )
            });
        let result = checked && allowed() && action(&allowed);
        let _ = self.reply.send(result);
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
    #[cfg(target_os = "linux")]
    #[test]
    fn terminal_metadata_is_pinned_and_worker_checks_do_not_emit_effects() {
        let now = Instant::now();
        let (mut state, steps) = planned(
            now,
            "pause media and then open terminal api and then open folder web",
        );
        let before = format!("{state:?}");
        Kind::Check(Box::new(steps[0].clone()))
            .run(&mut state, json!([]), now)
            .unwrap();
        assert_eq!(format!("{state:?}"), before);
        let key = steps[0].session.clone().unwrap();
        state
            .sessions
            .get_mut(&key)
            .unwrap()
            .terminal
            .env
            .insert("TMUX_PANE".into(), "%123".into());
        assert!(steps[0].check(&state).is_err());
        let before = format!("{state:?}");
        assert!(Kind::Preflight(steps).run(&mut state, json!([]), now).is_err());
        assert_eq!(format!("{state:?}"), before);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn external_receipts_confirm_submission_and_close_late_workers_without_native_calls() {
        for mode in 0..6 {
            let now = Instant::now();
            let (_, steps) = planned(now, "pause media and then open folder api");
            let (reply, mut receive) = tokio::sync::oneshot::channel();
            let request = Request {
                kind: Kind::Apply(Box::new(steps[0].clone())),
                ticket: crate::dictation::worker::Ticket::test_command(),
                deadline: now + Duration::from_millis(if mode == 3 { 20 } else { 1000 }),
                scopes: (true, false, false),
                reply,
            };
            let mut completion = Completion::new(request);
            let work = completion.work();
            assert!(work.ready(now));
            completion.finish(String::new(), true, i18n::Lang::En);
            match mode {
                0 => {
                    work.reply.send(true).unwrap();
                    let result = receive.await.unwrap().unwrap();
                    assert_eq!(result, i18n::voice_external_requested(i18n::Lang::En, true));
                }
                1 => {
                    work.reply.send(false).unwrap();
                    assert!(receive.await.unwrap().is_err());
                }
                2 => {
                    drop(work);
                    assert!(receive.await.unwrap().is_err());
                }
                3 => {
                    assert!(receive.await.unwrap().is_err());
                    assert!(!work.ready(Instant::now()));
                }
                4 => {
                    drop(receive);
                    let mut work = work;
                    tokio::time::timeout(Duration::from_secs(1), work.reply.closed())
                        .await
                        .unwrap();
                    assert!(!work.ready(Instant::now()));
                }
                _ => {
                    assert!(
                        tokio::time::timeout(Duration::from_millis(10), &mut receive)
                            .await
                            .is_err(),
                        "submission cannot be confirmed before its receipt"
                    );
                    work.reply.send(true).unwrap();
                    assert!(receive.await.unwrap().is_ok());
                }
            }
        }
        let now = Instant::now();
        let (_, steps) = planned(now, "pause media and then open terminal api");
        let (reply, _receive) = tokio::sync::oneshot::channel();
        let ticket = crate::dictation::worker::Ticket::test_command();
        let mut completion = Completion::new(Request {
            kind: Kind::Apply(Box::new(steps[0].clone())),
            ticket: ticket.clone(),
            deadline: now + Duration::from_secs(1),
            scopes: (true, false, false),
            reply,
        });
        let work = completion.work();
        ticket.test_cancel();
        assert!(!work.ready(now));
    }
}
