//! One admission and ordered execution for local desktop commands.
use super::{DesktopAction, Feedback, Report};
use crate::dictation::worker::Ticket;
use std::{
    future::Future,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};
use vults_core::i18n::{self, ApplicationFeedback, Lang, OutputFeedback};
use vults_dictation::command::{MediaAction, OutputAction};
use vults_platform::system_audio::{self, Error as OutputError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Error {
    Disabled,
    NoPlayer,
    Media,
    Application,
    Flock,
    FolderUnavailable,
    TerminalUnavailable,
    Output(OutputError),
}

trait Io {
    fn prepare(&mut self, actions: &[DesktopAction]) -> impl Future<Output = Result<(), Error>> + Send;
    fn apply(&mut self, action: DesktopAction) -> impl Future<Output = Result<(), Error>> + Send;
}

async fn execute(io: &mut impl Io, actions: &[DesktopAction], completed: &mut usize) -> Result<(), Error> {
    io.prepare(actions).await?;
    for action in actions {
        io.apply(action.clone()).await?;
        *completed += 1;
    }
    Ok(())
}

struct Backend<'a> {
    app: &'a AppHandle,
    ticket: &'a Ticket,
    deadline: Instant,
    media: bool,
    output: bool,
    applications: bool,
    player: Option<String>,
    playback: Option<vults_media::Target>,
    computer: Option<system_audio::Target>,
    status: Option<system_audio::Status>,
    answers: Vec<String>,
}

pub(super) fn consent(
    saved: &crate::settings::Settings,
    media: bool,
    output: bool,
    applications: bool,
) -> bool {
    saved.dictation_commands
        && (!media || saved.dictation_command_media && saved.now_playing)
        && (!output || saved.dictation_command_output)
        && (!applications || saved.dictation_command_apps)
}

pub(super) fn enabled(
    app: &AppHandle,
    ticket: &Ticket,
    deadline: Instant,
    media: bool,
    output: bool,
    applications: bool,
) -> bool {
    ticket.valid()
        && Instant::now() < deadline
        && !crate::lock::locked(app)
        && app
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .is_ok_and(|s| consent(&s, media, output, applications))
}

// The cached player is captured before scheduling; never adopt a later selection in preflight.
fn selected_player<'a>(admitted: Option<&'a str>, shown: Option<&str>) -> Result<&'a str, Error> {
    let admitted = admitted.ok_or(Error::NoPlayer)?;
    if shown != Some(admitted) {
        return Err(Error::Media);
    }
    Ok(admitted)
}

impl Backend<'_> {
    fn enabled(&self) -> bool {
        enabled(
            self.app,
            self.ticket,
            self.deadline,
            self.media,
            self.output,
            self.applications,
        )
    }

    async fn flock(&self, kind: super::flock::Kind) -> Result<String, Error> {
        crate::runtime::flock_command(
            self.app,
            kind,
            self.ticket.clone(),
            self.deadline,
            (self.media, self.output, self.applications),
        )
        .await
        .map_err(|_| Error::Flock)
    }

    fn media_allowed(&self) -> bool {
        let shown = crate::media::media_now(self.app.clone());
        self.enabled()
            && selected_player(
                self.player.as_deref(),
                shown.as_ref().map(|now| now.player.as_str()),
            )
            .is_ok()
    }
}

fn external_available(step: &super::flock::Step) -> Result<(), Error> {
    if step
        .folder_path()
        .is_some_and(|path| !crate::open::folder_available(path))
    {
        return Err(Error::FolderUnavailable);
    }
    if step
        .terminal()
        .is_some_and(|terminal| !vults_platform::jump::available(terminal))
    {
        return Err(Error::TerminalUnavailable);
    }
    Ok(())
}

impl Io for Backend<'_> {
    async fn prepare(&mut self, actions: &[DesktopAction]) -> Result<(), Error> {
        // Check every required consent before any native connection or effect.
        if !self.enabled() {
            return Err(Error::Disabled);
        }
        for action in actions {
            if let DesktopAction::Application(application) = action {
                let application = application.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    vults_platform::applications::validate(&application)
                })
                .await
                .map_err(|_| Error::Application)?
                .map_err(|_| Error::Application)?;
                if !self.enabled() {
                    return Err(Error::Disabled);
                }
            }
        }
        for action in actions {
            if let DesktopAction::Flock(step) = action {
                let step = step.as_ref().clone();
                tauri::async_runtime::spawn_blocking(move || external_available(&step))
                    .await
                    .map_err(|_| Error::Flock)??;
                if !self.enabled() {
                    return Err(Error::Disabled);
                }
            }
        }
        if self.media {
            let shown = crate::media::media_now(self.app.clone());
            let player = selected_player(
                self.player.as_deref(),
                shown.as_ref().map(|now| now.player.as_str()),
            )?;
            if !self.enabled() {
                return Err(Error::Disabled);
            }
            self.playback = Some(vults_media::pin(player).await.map_err(|_| Error::Media)?);
        }
        if self.output {
            self.computer = Some(
                system_audio::pin(|| self.enabled())
                    .await
                    .map_err(Error::Output)?,
            );
        }
        if self.media || self.output {
            tauri::async_runtime::spawn_blocking(vults_platform::music::wait_for_restore)
                .await
                .map_err(|_| Error::Media)?;
        }
        if let Some(target) = &self.playback {
            let controls: Vec<_> = actions
                .iter()
                .filter_map(|action| match action {
                    DesktopAction::Media(action) => Some(media_control(*action)),
                    _ => None,
                })
                .collect();
            target
                .validate_batch(&controls, || self.media_allowed())
                .await
                .map_err(|_| Error::Media)?;
        }
        let steps = actions
            .iter()
            .filter_map(|action| match action {
                DesktopAction::Flock(step) => Some(step.as_ref().clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !steps.is_empty() {
            self.flock(super::flock::Kind::Preflight(steps)).await?;
        }
        if !self.enabled() {
            return Err(Error::Disabled);
        }
        Ok(())
    }

    async fn apply(&mut self, action: DesktopAction) -> Result<(), Error> {
        if !self.enabled() {
            return Err(Error::Disabled);
        }
        match action {
            DesktopAction::Flock(step) => {
                let text = self.flock(super::flock::Kind::Apply(step)).await?;
                if !text.is_empty() {
                    self.answers.push(text);
                }
                Ok(())
            }
            DesktopAction::Application(application) => {
                let app = self.app.clone();
                let ticket = self.ticket.clone();
                let (deadline, media, output, applications) =
                    (self.deadline, self.media, self.output, self.applications);
                tauri::async_runtime::spawn_blocking(move || {
                    vults_platform::applications::launch(&application, || {
                        enabled(&app, &ticket, deadline, media, output, applications)
                    })
                })
                .await
                .map_err(|_| Error::Application)?
                .map_err(|_| Error::Application)
            }
            DesktopAction::Media(action) => self
                .playback
                .as_ref()
                .ok_or(Error::NoPlayer)?
                .control(media_control(action), || self.media_allowed())
                .await
                .map_err(|_| Error::Media),
            DesktopAction::Output(action) => {
                let status = self
                    .computer
                    .as_ref()
                    .ok_or(Error::Output(OutputError::Unavailable))?
                    .apply(output_control(action), || self.enabled())
                    .await
                    .map_err(Error::Output)?;
                self.status = Some(status);
                Ok(())
            }
        }
    }
}

pub(crate) fn start(app: AppHandle, heard: String, ticket: Ticket, mut report: Report, lang: Lang) {
    let actions = std::mem::take(&mut report.desktop);
    if actions.is_empty() || !ticket.claim_native() {
        return;
    }
    let media = actions
        .iter()
        .any(|action| matches!(action, DesktopAction::Media(_)));
    let output = actions
        .iter()
        .any(|action| matches!(action, DesktopAction::Output(_)));
    let applications = actions
        .iter()
        .any(|action| matches!(action, DesktopAction::Application(_)));
    let flock = actions
        .iter()
        .any(|action| matches!(action, DesktopAction::Flock(_)));
    let deadline = Instant::now() + Duration::from_secs(4);
    let player = if media {
        crate::media::media_now(app.clone()).map(|now| now.player)
    } else {
        None
    };
    tauri::async_runtime::spawn(async move {
        let mut backend = Backend {
            app: &app,
            ticket: &ticket,
            deadline,
            media,
            output,
            applications,
            player,
            playback: None,
            computer: None,
            status: None,
            answers: vec![],
        };
        let mut completed = 0;
        let result = tokio::time::timeout_at(
            tokio::time::Instant::from_std(deadline),
            execute(&mut backend, &actions, &mut completed),
        )
        .await;
        let success = matches!(result, Ok(Ok(())));
        report.feedback = if success {
            Feedback::Done
        } else {
            Feedback::ExecutionFailed
        };
        report.text = if let Ok(Err(Error::FolderUnavailable | Error::TerminalUnavailable)) = &result {
            i18n::voice_external_unavailable(lang, matches!(result, Ok(Err(Error::FolderUnavailable)))).into()
        } else if flock {
            i18n::voice_flock_batch(lang, success, completed, actions.len())
        } else if applications && actions.len() > 1 {
            i18n::voice_desktop_batch(lang, success, completed, actions.len())
        } else if applications {
            i18n::voice_application(
                lang,
                match result {
                    Ok(Ok(())) => ApplicationFeedback::Requested,
                    Ok(Err(Error::Disabled)) => ApplicationFeedback::Disabled,
                    _ => ApplicationFeedback::Failed,
                },
            )
            .into()
        } else if media && output {
            i18n::voice_audio_batch(lang, success, completed, actions.len())
        } else if output {
            match result {
                Ok(Ok(())) => {
                    let status = backend.status.expect("output confirmation precedes completion");
                    i18n::voice_output_status(lang, status.muted, status.percent)
                }
                ref result => {
                    let feedback = match result {
                        Ok(Err(Error::Disabled)) => OutputFeedback::Disabled,
                        Ok(Err(Error::Output(OutputError::Unavailable))) => OutputFeedback::Unavailable,
                        Ok(Err(Error::Output(OutputError::Changed))) => OutputFeedback::Changed,
                        _ => OutputFeedback::Failed,
                    };
                    i18n::voice_output_failure(lang, feedback, completed, actions.len())
                }
            }
        } else {
            match result {
                Ok(Ok(())) => i18n::voice_feedback(lang, Feedback::Done).into(),
                Ok(Err(Error::Disabled)) => i18n::voice_media_unavailable(lang, true).into(),
                Ok(Err(Error::NoPlayer)) => i18n::voice_media_unavailable(lang, false).into(),
                _ => i18n::voice_media_failed(lang, completed, actions.len()),
            }
        };
        if matches!(
            result,
            Ok(Err(Error::Disabled
                | Error::NoPlayer
                | Error::FolderUnavailable
                | Error::TerminalUnavailable))
        ) && completed == 0
        {
            report.feedback = Feedback::Unavailable;
        }
        if !backend.answers.is_empty() {
            report.text.push('\n');
            report.text.push_str(&backend.answers.join("\n"));
        }
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
    });
}

fn media_control(action: MediaAction) -> vults_media::Control {
    use vults_media::Control;
    match action {
        MediaAction::Pause => Control::Pause,
        MediaAction::Resume => Control::Play,
        MediaAction::Next => Control::Next,
        MediaAction::Previous => Control::Previous,
        MediaAction::Volume(value) => Control::Volume(value),
        MediaAction::Quieter => Control::VolumeDown,
        MediaAction::Louder => Control::VolumeUp,
    }
}

fn output_control(action: OutputAction) -> system_audio::Action {
    use system_audio::Action;
    match action {
        OutputAction::Mute(muted) => Action::Mute(muted),
        OutputAction::Volume(level) => Action::Volume(level),
        OutputAction::Louder => Action::Louder,
        OutputAction::Quieter => Action::Quieter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_required_audio_consent_must_be_enabled() {
        for mask in 0..32 {
            let saved = crate::settings::Settings {
                dictation_commands: mask & 1 != 0,
                dictation_command_media: mask & 2 != 0,
                now_playing: mask & 4 != 0,
                dictation_command_output: mask & 8 != 0,
                dictation_command_apps: mask & 16 != 0,
                ..crate::settings::Settings::default()
            };
            assert_eq!(consent(&saved, true, true, true), mask == 31);
            assert_eq!(consent(&saved, true, false, false), mask & 7 == 7);
            assert_eq!(consent(&saved, false, true, false), mask & 9 == 9);
            assert_eq!(consent(&saved, false, false, true), mask & 17 == 17);
        }
    }

    #[test]
    fn external_preflight_checks_real_paths_and_missing_terminal_metadata_without_opening_anything() {
        use vults_core::{Intent, State};
        use vults_dictation::command::Command;
        let now = Instant::now();
        let mut state = super::super::tests::fixture(now);
        let key = state.sessions.keys().next().unwrap().clone();
        state.sessions.get_mut(&key).unwrap().cwd = Some(std::env::temp_dir().to_string_lossy().into_owned());
        let intent = Intent::OpenFolder { session: key.clone() };
        let folder = super::super::flock::Step::plan(
            &state,
            &Command::Folder("api".into()),
            Some(key.clone()),
            Some(&intent),
        )
        .unwrap();
        assert_eq!(external_available(&folder), Ok(()));
        let missing = std::env::temp_dir().join(format!(
            "vults-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        state.sessions.get_mut(&key).unwrap().cwd = Some(missing.to_string_lossy().into_owned());
        let folder = super::super::flock::Step::plan(
            &state,
            &Command::Folder("api".into()),
            Some(key.clone()),
            Some(&intent),
        )
        .unwrap();
        assert_eq!(external_available(&folder), Err(Error::FolderUnavailable));
        let terminal = super::super::flock::Step::plan(
            &state,
            &Command::Terminal("api".into()),
            Some(key.clone()),
            Some(&Intent::Jump { session: key }),
        )
        .unwrap();
        assert_eq!(external_available(&terminal), Err(Error::TerminalUnavailable));
        let read = super::super::flock::Step::plan(&State::default(), &Command::Waiting, None, None).unwrap();
        assert_eq!(external_available(&read), Ok(()));
    }

    #[derive(Default)]
    struct Fake {
        prepared: bool,
        seen: Vec<DesktopAction>,
        fail: Option<usize>,
        reject: bool,
        hang: Option<usize>,
    }
    impl Io for Fake {
        async fn prepare(&mut self, _: &[DesktopAction]) -> Result<(), Error> {
            self.prepared = true;
            if self.reject { Err(Error::Disabled) } else { Ok(()) }
        }
        async fn apply(&mut self, action: DesktopAction) -> Result<(), Error> {
            assert!(self.prepared, "all targets must be prepared before effects");
            if self.hang == Some(self.seen.len()) {
                std::future::pending::<()>().await;
            }
            if self.fail == Some(self.seen.len()) {
                return Err(Error::Disabled);
            }
            self.seen.push(action);
            Ok(())
        }
    }

    #[test]
    fn media_admission_keeps_the_original_player_even_if_none_was_available() {
        let player = "org.mpris.MediaPlayer2.firefox";
        let admitted = Some(player);
        assert_eq!(selected_player(admitted, admitted), Ok(player));
        for (original, shown, expected) in [
            (admitted, Some("org.mpris.MediaPlayer2.spotify"), Error::Media),
            (admitted, None, Error::Media),
            (None, admitted, Error::NoPlayer),
            (None, None, Error::NoPlayer),
        ] {
            assert_eq!(selected_player(original, shown), Err(expected));
        }
    }

    #[tokio::test]
    async fn preflight_order_failure_and_timeout_preserve_the_confirmed_prefix_without_retry() {
        let actions = [
            DesktopAction::Media(MediaAction::Pause),
            DesktopAction::Output(OutputAction::Mute(true)),
            DesktopAction::Application(super::super::tests::installed_app("editor.desktop", "Editor")),
            DesktopAction::Flock(Box::new(
                super::super::flock::Step::plan(
                    &vults_core::State::default(),
                    &vults_dictation::command::Command::Waiting,
                    None,
                    None,
                )
                .unwrap(),
            )),
            DesktopAction::Media(MediaAction::Resume),
        ];
        let mut fake = Fake::default();
        let mut completed = 0;
        execute(&mut fake, &actions, &mut completed).await.unwrap();
        assert_eq!(fake.seen, actions);
        assert_eq!(completed, actions.len());
        let mut fake = Fake {
            reject: true,
            ..Fake::default()
        };
        completed = 0;
        assert!(execute(&mut fake, &actions, &mut completed).await.is_err());
        assert!(fake.seen.is_empty());
        assert_eq!(completed, 0);
        for index in 0..actions.len() {
            let mut fake = Fake {
                fail: Some(index),
                ..Fake::default()
            };
            completed = 0;
            assert!(execute(&mut fake, &actions, &mut completed).await.is_err());
            assert_eq!(fake.seen, actions[..index]);
            assert_eq!(completed, index);
            let mut fake = Fake {
                hang: Some(index),
                ..Fake::default()
            };
            completed = 0;
            assert!(
                tokio::time::timeout(
                    Duration::from_millis(10),
                    execute(&mut fake, &actions, &mut completed)
                )
                .await
                .is_err()
            );
            assert_eq!(fake.seen, actions[..index]);
            assert_eq!(completed, index);
        }
    }
}
