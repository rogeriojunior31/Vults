//! Native launch admission is separate from Chat and from media consent.
use super::{Feedback, Report};
use crate::dictation::worker::Ticket;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use vults_core::i18n::{self, ApplicationFeedback, Lang};

pub(crate) fn start(app: AppHandle, heard: String, ticket: Ticket, mut report: Report, lang: Lang) {
    let Some(application) = report.application.take() else {
        return;
    };
    if !ticket.claim_native() {
        return;
    }
    let consent = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .is_ok_and(|saved| saved.dictation_commands && saved.dictation_command_apps);
    if !consent {
        report.feedback = Feedback::Unavailable;
        report.text = i18n::voice_application(lang, ApplicationFeedback::Disabled).into();
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    tauri::async_runtime::spawn(async move {
        let launch_app = app.clone();
        let launch_ticket = ticket.clone();
        let work = tauri::async_runtime::spawn_blocking(move || {
            vults_platform::applications::launch(&application, || {
                Instant::now() < deadline
                    && launch_ticket.valid()
                    && !crate::lock::locked(&launch_app)
                    && launch_app
                        .state::<crate::settings::SettingsState>()
                        .0
                        .lock()
                        .is_ok_and(|saved| saved.dictation_commands && saved.dictation_command_apps)
            })
        });
        let result = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), work).await;
        let launched = matches!(result, Ok(Ok(Ok(()))));
        report.feedback = if launched {
            Feedback::Done
        } else {
            Feedback::ExecutionFailed
        };
        report.text = i18n::voice_application(
            lang,
            if launched {
                ApplicationFeedback::Requested
            } else {
                ApplicationFeedback::Failed
            },
        )
        .into();
        crate::dictation::command_result(&app, &ticket, &heard, report, None);
    });
}
