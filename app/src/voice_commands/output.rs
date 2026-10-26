//! Output consent is independent from project notifications and playback controls.
use super::{Feedback, Report};
use crate::dictation::worker::Ticket;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use vults_core::i18n::{self, Lang, OutputFeedback};
use vults_dictation::command::OutputAction;
use vults_platform::system_audio::{self, Action, Error};

fn enabled(app: &AppHandle, ticket: &Ticket) -> bool {
    ticket.valid()
        && !crate::lock::locked(app)
        && app
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .is_ok_and(|saved| saved.dictation_commands && saved.dictation_command_output)
}

pub(crate) fn start(app: AppHandle, heard: String, ticket: Ticket, mut report: Report, lang: Lang) {
    let actions = std::mem::take(&mut report.output);
    if actions.is_empty() || !ticket.claim_native() {
        return;
    }
    if !enabled(&app, &ticket) {
        report.feedback = Feedback::Unavailable;
        report.text = i18n::voice_output_failure(lang, OutputFeedback::Disabled, 0, actions.len());
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(4);
    tauri::async_runtime::spawn(async move {
        let mut completed = 0;
        let result = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
            let allowed = || Instant::now() < deadline && enabled(&app, &ticket);
            let target = system_audio::pin(allowed).await?;
            tauri::async_runtime::spawn_blocking(vults_platform::music::wait_for_restore)
                .await
                .map_err(|_| Error::Failed)?;
            let mut last = None;
            for action in &actions {
                last = Some(target.apply(control(*action), allowed).await?);
                completed += 1;
            }
            last.ok_or(Error::Invalid)
        })
        .await;
        match result {
            Ok(Ok(status)) => {
                report.feedback = Feedback::Done;
                report.text = i18n::voice_output_status(lang, status.muted, status.percent);
            }
            result => {
                let feedback = match result {
                    Ok(Err(Error::Unavailable)) => OutputFeedback::Unavailable,
                    Ok(Err(Error::Changed)) => OutputFeedback::Changed,
                    _ => OutputFeedback::Failed,
                };
                report.feedback = Feedback::ExecutionFailed;
                report.text = i18n::voice_output_failure(lang, feedback, completed, actions.len());
            }
        }
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
    });
}

fn control(action: OutputAction) -> Action {
    match action {
        OutputAction::Mute(muted) => Action::Mute(muted),
        OutputAction::Volume(level) => Action::Volume(level),
        OutputAction::Louder => Action::Louder,
        OutputAction::Quieter => Action::Quieter,
    }
}
