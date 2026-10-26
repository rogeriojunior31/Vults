//! Explicit command-key playback. Admission never opens Chat or bypasses the media switches.
use super::{Feedback, Report};
use crate::dictation::worker::Ticket;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use vults_core::i18n::{self, Lang};
use vults_dictation::command::MediaAction;

fn enabled(app: &AppHandle, ticket: &Ticket, player: &str) -> bool {
    ticket.valid()
        && !crate::lock::locked(app)
        && app
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .is_ok_and(|s| s.dictation_commands && s.dictation_command_media && s.now_playing)
        && crate::media::media_now(app.clone()).is_some_and(|now| now.player == player)
}

pub(crate) fn start(app: AppHandle, heard: String, ticket: Ticket, mut report: Report, lang: Lang) {
    if !ticket.claim_native() {
        return;
    }
    let actions = std::mem::take(&mut report.media);
    let consent = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .is_ok_and(|s| s.dictation_commands && s.dictation_command_media && s.now_playing);
    let player = crate::media::media_now(app.clone()).map(|now| now.player);
    if !consent || player.is_none() {
        report.feedback = Feedback::Unavailable;
        report.text = i18n::voice_media_unavailable(lang, !consent).into();
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
        return;
    }
    let player = player.expect("selected player checked");
    let deadline = Instant::now() + Duration::from_secs(4);
    tauri::async_runtime::spawn(async move {
        let mut completed = 0;
        let result = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
            if Instant::now() >= deadline || !enabled(&app, &ticket, &player) {
                return Err(());
            }
            let target = vults_media::pin(&player).await.map_err(|_| ())?;
            // A late blocking barrier only waits; dropping its receiver cannot execute an action.
            tauri::async_runtime::spawn_blocking(vults_platform::music::wait_for_restore)
                .await
                .map_err(|_| ())?;
            for action in &actions {
                target
                    .control(control(*action), || {
                        Instant::now() < deadline && enabled(&app, &ticket, &player)
                    })
                    .await
                    .map_err(|_| ())?;
                completed += 1;
            }
            Ok::<_, ()>(())
        })
        .await;
        report.feedback = if matches!(result, Ok(Ok(()))) {
            Feedback::Done
        } else {
            Feedback::ExecutionFailed
        };
        report.text = if report.feedback == Feedback::Done {
            i18n::voice_feedback(lang, Feedback::Done).into()
        } else {
            i18n::voice_media_failed(lang, completed, actions.len())
        };
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
    });
}

fn control(action: MediaAction) -> vults_media::Control {
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
