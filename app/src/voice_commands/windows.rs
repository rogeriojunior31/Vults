//! Windows audio commands share the parser, ticket, consent and deadline checks with Vults actions.
use super::{DesktopAction, Feedback, Report};
use crate::dictation::worker::Ticket;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use vults_core::i18n::{self, Lang, OutputFeedback};
use vults_dictation::command::{MediaAction, OutputAction};
use vults_platform::{player_audio, system_audio};

pub(crate) fn needs_application_catalog(text: &str) -> bool {
    use vults_dictation::command::{Command, Parsed, parse};
    match parse(text) {
        Parsed::Commands(commands) => commands.iter().any(|command| {
            matches!(
                command,
                Command::Open(_) | Command::Application(_) | Command::Repeat(_)
            )
        }),
        Parsed::NotCommand => true, // A reply may choose an application from a pending ambiguity.
        _ => false,
    }
}

#[derive(Clone, Debug)]
enum Step {
    Media(vults_media::Control),
    PlayerVolume(player_audio::Action),
    Output(system_audio::Action),
    Application(vults_platform::applications::Application),
}
#[derive(Clone, Copy, Debug)]
enum Failure {
    Disabled,
    Media,
    Unavailable,
    Application,
    Output(system_audio::Error),
}

fn plan(actions: &[DesktopAction]) -> Result<Vec<Step>, Failure> {
    actions
        .iter()
        .map(|action| {
            Ok(match action {
                DesktopAction::Media(MediaAction::Volume(level)) if *level <= 100 => {
                    Step::PlayerVolume(player_audio::Action::Volume(*level))
                }
                DesktopAction::Media(MediaAction::Louder) => Step::PlayerVolume(player_audio::Action::Louder),
                DesktopAction::Media(MediaAction::Quieter) => {
                    Step::PlayerVolume(player_audio::Action::Quieter)
                }
                DesktopAction::Media(action) => Step::Media(match action {
                    MediaAction::Resume => vults_media::Control::Play,
                    MediaAction::Pause => vults_media::Control::Pause,
                    MediaAction::Next => vults_media::Control::Next,
                    MediaAction::Previous => vults_media::Control::Previous,
                    _ => return Err(Failure::Unavailable),
                }),
                DesktopAction::Output(action) => Step::Output(match action {
                    OutputAction::Mute(muted) => system_audio::Action::Mute(*muted),
                    OutputAction::Volume(level) if *level <= 100 => system_audio::Action::Volume(*level),
                    OutputAction::Louder => system_audio::Action::Louder,
                    OutputAction::Quieter => system_audio::Action::Quieter,
                    _ => return Err(Failure::Unavailable),
                }),
                DesktopAction::Application(application) => Step::Application(application.clone()),
                _ => return Err(Failure::Unavailable),
            })
        })
        .collect()
}

fn consent(saved: &crate::settings::Settings, media: bool, output: bool, applications: bool) -> bool {
    saved.dictation_commands
        && (!media || saved.dictation_command_media && saved.now_playing)
        && (!output || saved.dictation_command_output)
        && (!applications || saved.dictation_command_apps)
}

#[derive(Clone)]
struct Gate {
    app: AppHandle,
    ticket: Ticket,
    deadline: Instant,
    media: bool,
    output: bool,
    applications: bool,
    player: Option<String>,
}
impl Gate {
    fn allowed(&self) -> bool {
        self.ticket.valid()
            && Instant::now() < self.deadline
            && !crate::lock::locked(&self.app)
            && vults_platform::windows_session::locked() == Some(false)
            && self
                .app
                .state::<crate::settings::SettingsState>()
                .0
                .lock()
                .is_ok_and(|saved| consent(&saved, self.media, self.output, self.applications))
            && (!self.media
                || crate::media::media_now(self.app.clone())
                    .is_some_and(|now| Some(now.player.as_str()) == self.player.as_deref()))
            // Settings and native queries may have waited; admission must still be current.
            && self.ticket.valid()
            && !crate::lock::locked(&self.app)
            && vults_platform::windows_session::locked() == Some(false)
            && Instant::now() < self.deadline
    }
    fn owned(&self) -> impl Fn() -> bool + Send + Sync + 'static {
        let gate = self.clone();
        move || gate.allowed()
    }
}

pub(crate) fn start(app: AppHandle, heard: String, ticket: Ticket, mut report: Report, lang: Lang) {
    if report.desktop.is_empty() || !ticket.claim_native() {
        return;
    }
    let actions = std::mem::take(&mut report.desktop);
    let media = actions
        .iter()
        .any(|action| matches!(action, DesktopAction::Media(_)));
    let output = actions
        .iter()
        .any(|action| matches!(action, DesktopAction::Output(_)));
    let applications = actions
        .iter()
        .any(|action| matches!(action, DesktopAction::Application(_)));
    let gate = Gate {
        app: app.clone(),
        ticket: ticket.clone(),
        deadline: Instant::now() + Duration::from_secs(10),
        media,
        output,
        applications,
        player: media
            .then(|| crate::media::media_now(app.clone()).map(|now| now.player))
            .flatten(),
    };
    tauri::async_runtime::spawn(async move {
        let mut completed = 0;
        let mut status = None;
        let result = async {
            let steps = plan(&actions)?;
            if !gate.allowed() {
                return Err(Failure::Disabled);
            }
            if media && !vults_platform::music::wait_for_restore_until(|| gate.allowed()).await {
                return Err(Failure::Disabled);
            }
            for step in &steps {
                if let Step::Application(application) = step {
                    let application = application.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        vults_platform::applications::validate(&application)
                    })
                    .await
                    .map_err(|_| Failure::Application)?
                    .map_err(|_| Failure::Application)?;
                    if !gate.allowed() {
                        return Err(Failure::Disabled);
                    }
                }
            }
            let playback = if media {
                Some(
                    vults_media::pin(gate.player.as_deref().ok_or(Failure::Media)?)
                        .await
                        .map_err(|_| Failure::Media)?,
                )
            } else {
                None
            };
            let player_volume = if steps.iter().any(|step| matches!(step, Step::PlayerVolume(_))) {
                Some(
                    crate::media::windows_audio::binding(&app, gate.player.as_deref().ok_or(Failure::Media)?)
                        .ok_or(Failure::Media)?,
                )
            } else {
                None
            };
            if let Some(binding) = &player_volume {
                let check = binding.clone();
                let allowed = gate.owned();
                binding
                    .audio
                    .validate(move || allowed() && check.valid())
                    .await
                    .map_err(|_| Failure::Media)?;
            }
            let computer = if output {
                Some(system_audio::pin(gate.owned()).await.map_err(Failure::Output)?)
            } else {
                None
            };
            if let Some(target) = &playback {
                let controls: Vec<_> = steps
                    .iter()
                    .filter_map(|step| match step {
                        Step::Media(control) => Some(*control),
                        _ => None,
                    })
                    .collect();
                target
                    .validate_batch(&controls, || gate.allowed())
                    .await
                    .map_err(|_| Failure::Media)?;
            }
            for step in steps {
                if !gate.allowed() {
                    return Err(Failure::Disabled);
                }
                match step {
                    Step::PlayerVolume(action) => {
                        let binding = player_volume.as_ref().ok_or(Failure::Media)?.clone();
                        let check = binding.clone();
                        let allowed = gate.owned();
                        binding
                            .audio
                            .apply(action, move || allowed() && check.valid())
                            .await
                            .map_err(|_| Failure::Media)?;
                    }
                    Step::Media(control) => playback
                        .as_ref()
                        .ok_or(Failure::Media)?
                        .control(control, || gate.allowed())
                        .await
                        .map_err(|_| Failure::Media)?,
                    Step::Output(action) => {
                        status = Some(
                            computer
                                .as_ref()
                                .ok_or(Failure::Unavailable)?
                                .apply(action, gate.owned())
                                .await
                                .map_err(Failure::Output)?,
                        );
                    }
                    Step::Application(application) => {
                        vults_platform::applications::launch(&application, gate.owned())
                            .await
                            .map_err(|_| Failure::Application)?;
                    }
                }
                completed += 1;
            }
            Ok(())
        }
        .await;
        report.feedback = if result.is_ok() {
            Feedback::Done
        } else {
            Feedback::Unavailable
        };
        report.text = if applications && actions.len() > 1 {
            i18n::voice_desktop_batch(lang, result.is_ok(), completed, actions.len())
        } else if applications {
            i18n::voice_application(
                lang,
                match result {
                    Ok(()) => i18n::ApplicationFeedback::Requested,
                    Err(Failure::Disabled) => i18n::ApplicationFeedback::Disabled,
                    _ => i18n::ApplicationFeedback::Failed,
                },
            )
            .into()
        } else if media && output {
            i18n::voice_audio_batch(lang, result.is_ok(), completed, actions.len())
        } else if output {
            match result {
                Ok(()) => {
                    let status = status.expect("output confirmation precedes completion");
                    i18n::voice_output_status(lang, status.muted, status.percent)
                }
                Err(error) => i18n::voice_output_failure(
                    lang,
                    match error {
                        Failure::Disabled | Failure::Output(system_audio::Error::Cancelled) => {
                            OutputFeedback::Disabled
                        }
                        Failure::Unavailable | Failure::Output(system_audio::Error::Unavailable) => {
                            OutputFeedback::Unavailable
                        }
                        Failure::Output(system_audio::Error::Changed) => OutputFeedback::Changed,
                        _ => OutputFeedback::Failed,
                    },
                    completed,
                    actions.len(),
                ),
            }
        } else if result.is_ok() {
            i18n::voice_feedback(lang, Feedback::Done).into()
        } else {
            i18n::voice_media_failed(lang, completed, actions.len())
        };
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrelated_and_refused_commands_do_not_inventory_the_start_menu() {
        for phrase in [
            "computer volume 40",
            "pause media",
            "approve all",
            "do not open app Firefox",
        ] {
            assert!(!needs_application_catalog(phrase), "{phrase}");
        }
        for phrase in ["open app Firefox", "open Firefox", "the first one"] {
            assert!(needs_application_catalog(phrase), "{phrase}");
        }
    }

    #[test]
    fn output_consent_is_independent_of_media_and_now_playing() {
        for mask in 0_u8..16 {
            let saved = crate::settings::Settings {
                dictation_commands: mask & 1 != 0,
                dictation_command_media: mask & 2 != 0,
                now_playing: mask & 4 != 0,
                dictation_command_output: mask & 8 != 0,
                ..Default::default()
            };
            assert_eq!(consent(&saved, false, true, false), mask & 9 == 9);
            assert_eq!(consent(&saved, true, false, false), mask & 7 == 7);
            assert_eq!(consent(&saved, true, true, false), mask == 15);
        }
    }

    #[test]
    fn applications_have_independent_consent_in_single_and_mixed_batches() {
        for mask in 0_u8..32 {
            let saved = crate::settings::Settings {
                dictation_commands: mask & 1 != 0,
                dictation_command_media: mask & 2 != 0,
                now_playing: mask & 4 != 0,
                dictation_command_output: mask & 8 != 0,
                dictation_command_apps: mask & 16 != 0,
                ..Default::default()
            };
            assert_eq!(consent(&saved, false, false, true), mask & 17 == 17);
            assert_eq!(consent(&saved, true, true, true), mask == 31);
            assert_eq!(consent(&saved, false, true, true), mask & 25 == 25);
        }
    }

    #[test]
    fn all_actions_are_planned_before_native_work_starts() {
        assert!(plan(&[DesktopAction::Output(OutputAction::Volume(101))]).is_err());
        assert!(
            plan(&[
                DesktopAction::Output(OutputAction::Mute(true)),
                DesktopAction::Media(MediaAction::Volume(101))
            ])
            .is_err()
        );
        let steps = plan(&[
            DesktopAction::Media(MediaAction::Pause),
            DesktopAction::Media(MediaAction::Volume(40)),
            DesktopAction::Media(MediaAction::Louder),
            DesktopAction::Media(MediaAction::Quieter),
            DesktopAction::Output(OutputAction::Volume(40)),
            DesktopAction::Output(OutputAction::Mute(false)),
        ])
        .unwrap();
        assert!(matches!(
            steps.as_slice(),
            [
                Step::Media(vults_media::Control::Pause),
                Step::PlayerVolume(player_audio::Action::Volume(40)),
                Step::PlayerVolume(player_audio::Action::Louder),
                Step::PlayerVolume(player_audio::Action::Quieter),
                Step::Output(system_audio::Action::Volume(40)),
                Step::Output(system_audio::Action::Mute(false))
            ]
        ));
    }
}
