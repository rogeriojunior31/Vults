//! One admission and ordered execution for player and computer-output commands.
use super::{AudioAction, Feedback, Report};
use crate::dictation::worker::Ticket;
use std::{
    future::Future,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};
use vults_core::i18n::{self, Lang, OutputFeedback};
use vults_dictation::command::{MediaAction, OutputAction};
use vults_platform::system_audio::{self, Error as OutputError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Error {
    Disabled,
    NoPlayer,
    Media,
    Output(OutputError),
}

trait Io {
    fn prepare(&mut self, actions: &[AudioAction]) -> impl Future<Output = Result<(), Error>> + Send;
    fn apply(&mut self, action: AudioAction) -> impl Future<Output = Result<(), Error>> + Send;
}

async fn execute(io: &mut impl Io, actions: &[AudioAction], completed: &mut usize) -> Result<(), Error> {
    io.prepare(actions).await?;
    for action in actions {
        io.apply(*action).await?;
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
    player: Option<String>,
    playback: Option<vults_media::Target>,
    computer: Option<system_audio::Target>,
    status: Option<system_audio::Status>,
}

fn consent(saved: &crate::settings::Settings, media: bool, output: bool) -> bool {
    saved.dictation_commands
        && (!media || saved.dictation_command_media && saved.now_playing)
        && (!output || saved.dictation_command_output)
}

impl Backend<'_> {
    fn enabled(&self) -> bool {
        self.ticket.valid()
            && Instant::now() < self.deadline
            && !crate::lock::locked(self.app)
            && self
                .app
                .state::<crate::settings::SettingsState>()
                .0
                .lock()
                .is_ok_and(|s| consent(&s, self.media, self.output))
    }

    fn media_allowed(&self) -> bool {
        self.enabled()
            && crate::media::media_now(self.app.clone())
                .is_some_and(|now| self.player.as_deref() == Some(now.player.as_str()))
    }
}

impl Io for Backend<'_> {
    async fn prepare(&mut self, actions: &[AudioAction]) -> Result<(), Error> {
        // Check every required consent before any native connection or effect.
        if !self.enabled() {
            return Err(Error::Disabled);
        }
        if self.media {
            self.player = crate::media::media_now(self.app.clone()).map(|now| now.player);
            let player = self.player.as_deref().ok_or(Error::NoPlayer)?;
            self.playback = Some(vults_media::pin(player).await.map_err(|_| Error::Media)?);
        }
        if self.output {
            self.computer = Some(
                system_audio::pin(|| self.enabled())
                    .await
                    .map_err(Error::Output)?,
            );
        }
        tauri::async_runtime::spawn_blocking(vults_platform::music::wait_for_restore)
            .await
            .map_err(|_| Error::Media)?;
        if let Some(target) = &self.playback {
            let controls: Vec<_> = actions
                .iter()
                .filter_map(|action| match action {
                    AudioAction::Media(action) => Some(media_control(*action)),
                    _ => None,
                })
                .collect();
            target
                .validate_batch(&controls, || self.media_allowed())
                .await
                .map_err(|_| Error::Media)?;
        }
        if !self.enabled() {
            return Err(Error::Disabled);
        }
        Ok(())
    }

    async fn apply(&mut self, action: AudioAction) -> Result<(), Error> {
        if !self.enabled() {
            return Err(Error::Disabled);
        }
        match action {
            AudioAction::Media(action) => self
                .playback
                .as_ref()
                .ok_or(Error::NoPlayer)?
                .control(media_control(action), || self.media_allowed())
                .await
                .map_err(|_| Error::Media),
            AudioAction::Output(action) => {
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
    let actions = std::mem::take(&mut report.audio);
    if actions.is_empty() || !ticket.claim_native() {
        return;
    }
    let media = actions
        .iter()
        .any(|action| matches!(action, AudioAction::Media(_)));
    let output = actions
        .iter()
        .any(|action| matches!(action, AudioAction::Output(_)));
    let deadline = Instant::now() + Duration::from_secs(4);
    tauri::async_runtime::spawn(async move {
        let mut backend = Backend {
            app: &app,
            ticket: &ticket,
            deadline,
            media,
            output,
            player: None,
            playback: None,
            computer: None,
            status: None,
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
        report.text = if media && output {
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
        if matches!(result, Ok(Err(Error::Disabled | Error::NoPlayer))) && completed == 0 {
            report.feedback = Feedback::Unavailable;
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
        for mask in 0..16 {
            let saved = crate::settings::Settings {
                dictation_commands: mask & 1 != 0,
                dictation_command_media: mask & 2 != 0,
                now_playing: mask & 4 != 0,
                dictation_command_output: mask & 8 != 0,
                ..crate::settings::Settings::default()
            };
            assert_eq!(consent(&saved, true, true), mask == 15);
            assert_eq!(consent(&saved, true, false), mask & 7 == 7);
            assert_eq!(consent(&saved, false, true), mask & 9 == 9);
        }
    }

    #[derive(Default)]
    struct Fake {
        prepared: bool,
        seen: Vec<AudioAction>,
        fail: Option<usize>,
        reject: bool,
        hang: Option<usize>,
    }
    impl Io for Fake {
        async fn prepare(&mut self, _: &[AudioAction]) -> Result<(), Error> {
            self.prepared = true;
            if self.reject { Err(Error::Disabled) } else { Ok(()) }
        }
        async fn apply(&mut self, action: AudioAction) -> Result<(), Error> {
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

    #[tokio::test]
    async fn preflight_order_failure_and_timeout_preserve_the_confirmed_prefix_without_retry() {
        let actions = [
            AudioAction::Media(MediaAction::Pause),
            AudioAction::Output(OutputAction::Mute(true)),
            AudioAction::Media(MediaAction::Resume),
        ];
        let mut fake = Fake::default();
        let mut completed = 0;
        execute(&mut fake, &actions, &mut completed).await.unwrap();
        assert_eq!(fake.seen, actions);
        assert_eq!(completed, 3);
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
