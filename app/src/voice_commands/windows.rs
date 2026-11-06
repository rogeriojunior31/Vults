//! Windows media commands share the parser, ticket, consent and deadline checks with Vults actions.
use super::{DesktopAction, Feedback, Report};
use crate::dictation::worker::Ticket;
use tauri::{AppHandle, Manager};
use vults_core::i18n::{self, Lang};

pub(crate) fn start(app: AppHandle, heard: String, ticket: Ticket, mut report: Report, lang: Lang) {
    if !ticket.claim_native() {
        return;
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let enabled = move |app: &AppHandle, ticket: &Ticket| {
        ticket.valid()
            && std::time::Instant::now() < deadline
            && !crate::lock::locked(app)
            && vults_platform::windows_session::locked() == Some(false)
            && app
                .state::<crate::settings::SettingsState>()
                .0
                .lock()
                .is_ok_and(|s| s.dictation_commands && s.dictation_command_media && s.now_playing)
    };
    let actions = std::mem::take(&mut report.desktop);
    let shown = crate::media::media_now(app.clone()).map(|now| now.player);
    tauri::async_runtime::spawn(async move {
        let result = async {
            if !enabled(&app, &ticket)
                || actions
                    .iter()
                    .any(|action| !matches!(action, DesktopAction::Media(_)))
            {
                return Err(());
            }
            let player = shown.as_deref().ok_or(())?;
            let target = vults_media::pin(player).await.map_err(|_| ())?;
            let controls: Vec<_> = actions
                .iter()
                .filter_map(|action| {
                    if let DesktopAction::Media(action) = action {
                        Some(match action {
                            vults_dictation::command::MediaAction::Resume => vults_media::Control::Play,
                            vults_dictation::command::MediaAction::Pause => vults_media::Control::Pause,
                            vults_dictation::command::MediaAction::Next => vults_media::Control::Next,
                            vults_dictation::command::MediaAction::Previous => vults_media::Control::Previous,
                            _ => return None,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            if controls.len() != actions.len() {
                return Err(());
            }
            let allowed = || {
                enabled(&app, &ticket)
                    && crate::media::media_now(app.clone()).is_some_and(|now| now.player == player)
            };
            target.validate_batch(&controls, &allowed).await.map_err(|_| ())?;
            for control in controls {
                target.control(control, &allowed).await.map_err(|_| ())?;
            }
            Ok(())
        }
        .await;
        report.feedback = if result.is_ok() {
            Feedback::Done
        } else {
            Feedback::Unavailable
        };
        report.text = i18n::voice_feedback(lang, report.feedback).into();
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
    });
}
