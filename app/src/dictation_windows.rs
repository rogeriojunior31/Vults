//! Windows capture/delivery adapters around the shared dictation state machine and decoder.
#[path = "dictation/cleanup.rs"]
pub(crate) mod cleanup;
#[path = "dictation/config.rs"]
mod config;
#[path = "dictation/project.rs"]
pub(crate) mod project;
#[path = "dictation/worker.rs"]
pub(crate) mod worker;
use config::Config;
#[cfg(test)]
use config::{decode, vocabulary_words};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
pub(crate) use worker::State;
use worker::{Backend, Clip, Ticket};
pub(crate) mod permission {
    use tauri::AppHandle;
    pub fn status(app: AppHandle) -> String {
        if !crate::settings::dictation_insert(&app) {
            "off"
        } else if vults_platform::focus::start() {
            "ready"
        } else {
            "unavailable"
        }
        .into()
    }
    pub async fn configure(_app: AppHandle, on: bool, _interactive: bool) -> Result<(), String> {
        if !on || vults_platform::focus::start() {
            Ok(())
        } else {
            Err("Windows focus tracking is unavailable.".into())
        }
    }
    pub async fn retry(app: AppHandle, window: tauri::Webview) -> Result<(), String> {
        crate::settings_page(&window)?;
        configure(app, true, true).await
    }
    pub fn revoke(app: &AppHandle) {
        super::cancel(app, false);
    }
    pub fn shutdown(app: &AppHandle) {
        super::cancel(app, true);
    }
}

// Native terminal panes remain outside Windows' delivery authority.
#[allow(dead_code)]
struct SessionTarget {
    key: vults_core::SessionKey,
    name: String,
}

pub fn start(app: &AppHandle) {
    cleanup::start(app);
    app.manage(project::Cache::default());
    crate::dictation_history::start(app);
    let _ = vults_platform::focus::start();
    match State::start(Native {
        app: app.clone(),
        clipboard: Arc::new(Mutex::new(None)),
    }) {
        Ok(state) => {
            if let Ok(settings) = app.state::<crate::settings::SettingsState>().0.lock() {
                state.configure(settings.dictation_options());
            }
            app.manage(state);
        }
        Err(error) => tracing::warn!(%error,"dictation worker unavailable"),
    }
}
pub fn key(app: &AppHandle, id: &str, down: bool) {
    if crate::lock::locked(app) || vults_platform::windows_session::locked() != Some(false) {
        cancel(app, true);
        return;
    }
    if matches!(id, "talk" | "talk-command") && down {
        crate::doctor::dictation::stop_mic(app);
        crate::voice::cancel(app);
    }
    if id == "talk-cancel" && down {
        crate::runtime::cancel_voice_commands(app);
    }
    let settings = app.state::<crate::settings::SettingsState>();
    let Ok(saved) = settings.0.lock().map(|saved| saved.clone()) else {
        return;
    };
    if id == "talk-command" && down && !saved.dictation_commands {
        return;
    }
    let target = (id == "talk" && down && saved.dictation_insert)
        .then(vults_platform::insert::Target::capture)
        .flatten();
    let observed = (down && matches!(id, "talk" | "talk-command"))
        .then(vults_platform::focus::key_down)
        .flatten();
    let choice = (id == "talk" && down)
        .then(|| cleanup::snapshot(app, &saved, target.as_ref().map(|target| target.window()), false))
        .flatten();
    let ticket = app
        .try_state::<State>()
        .and_then(|state| state.key_at(id, down, std::time::Instant::now()));
    drop(saved);
    if let Some(ticket) = ticket {
        ticket.pin_cleanup(choice);
        if let Some(window) = observed {
            let sessions = crate::runtime::dictation_sessions(app);
            let app = app.clone();
            let context = ticket.clone();
            let _ = std::thread::Builder::new()
                .name("dictation-context".into())
                .spawn(move || {
                    let sessions = sessions
                        .recv_timeout(std::time::Duration::from_millis(100))
                        .unwrap_or_default();
                    if !context.valid() {
                        return;
                    }
                    let session = vults_platform::focus::session(&window, &sessions);
                    let cwd = sessions
                        .iter()
                        .find(|(key, _)| Some(key) == session)
                        .and_then(|(_, terminal)| terminal.cwd.as_deref());
                    if let Some(cache) = app.try_state::<project::Cache>() {
                        context.set_project_words(cache.words(&window, cwd));
                    }
                    if let Some(status) = context.observe(window, session.cloned()) {
                        let _ = app.emit("dictation", status);
                    }
                });
        }
        if let Some(target) = target {
            ticket.pin(target);
        }
    }
}
pub fn cancel(app: &AppHandle, forget: bool) {
    if forget {
        crate::runtime::cancel_voice_commands(app);
    }
    if let Some(state) = app.try_state::<State>() {
        state.cancel(forget);
    }
}
pub fn active(app: &AppHandle) -> bool {
    app.try_state::<State>().is_some_and(|state| state.active())
}
pub fn holds_space(app: &AppHandle) -> bool {
    app.try_state::<State>().is_some_and(|state| state.holds_space())
}
#[derive(Clone)]
struct Native {
    app: AppHandle,
    clipboard: Arc<Mutex<Option<vults_platform::clipboard::Clipboard>>>,
}
struct Recording {
    capture: vults_audio::Capture,
    config: Config,
    music: vults_platform::music::Listening,
}
impl Backend for Native {
    type Recording = Recording;
    type Keys = ();
    fn watch(&self, _: &Ticket, _: impl Fn(vults_platform::raw_keys::Key) + Send + 'static) -> Option<()> {
        None
    }
    fn start(&self, ticket: &Ticket) -> Result<Recording, String> {
        let model = crate::settings::voice_model(&self.app).ok_or("Choose a voice model in Chat & voice.")?;
        if !vults_voice::installed(&crate::voice::models_dir()).contains(&model.as_str()) {
            return Err("The selected voice model is not installed.".into());
        }
        ticket.history_started(crate::dictation_history::epoch());
        let saved = self.app.state::<crate::settings::SettingsState>();
        let saved = saved.0.lock().map_err(|_| "settings are busy")?.clone();
        let options = saved.dictation_capture_options();
        let music_mode = saved.dictation_music;
        let config = Config {
            cleanup: ticket.cleanup_choice(),
            model,
            language: crate::voice::language(&self.app),
            vad: vults_voice::vad_path(&crate::voice::models_dir()),
            cut_repeats: saved.dictation_cut_repeats,
            retry_base: saved.dictation_retry_base,
            vocabulary: saved.dictation_vocabulary,
            project_vocabulary: saved.dictation_project_vocabulary,
            project_words: ticket.project_words(),
            spoken_code: saved.dictation_spoken_code,
            cancel_by_voice: saved.dictation_cancel_by_voice,
            words: crate::dictation_words::rules(&crate::history::dir())
                .map_err(|_| "Could not read dictation words.")?,
        };
        drop(saved);
        let (app, meter, live) = (self.app.clone(), ticket.clone(), ticket.clone());
        let capture = vults_audio::Capture::start_guarded(
            options,
            move |level| {
                if let Some(status) = meter.feedback(Some(level.bands), None) {
                    let _ = app.emit("dictation", status);
                }
            },
            move || live.valid() && !live.stop().load(std::sync::atomic::Ordering::SeqCst),
            || true,
        )?;
        let (app, live) = (self.app.clone(), ticket.clone());
        let music = vults_platform::music::Listening::start_guarded(music_mode, move || {
            if !live.valid() || live.stop().load(std::sync::atomic::Ordering::SeqCst) {
                return false;
            }
            let enabled = app
                .state::<crate::settings::SettingsState>()
                .0
                .lock()
                .is_ok_and(|saved| saved.dictation_music == vults_platform::music::Mode::Pause);
            enabled
                && !crate::lock::locked(&app)
                && vults_platform::windows_session::locked() == Some(false)
                && live.valid()
                && !live.stop().load(std::sync::atomic::Ordering::SeqCst)
        });
        Ok(Recording {
            capture,
            config,
            music,
        })
    }
    fn finish(&self, recording: Recording) -> Result<Clip, String> {
        let Recording {
            capture,
            config,
            music,
        } = recording;
        let pcm = capture.finish();
        drop(music);
        Ok(Clip { pcm: pcm?, config })
    }
    fn transcribe(&self, clip: Clip, ticket: &Ticket) -> Result<Option<String>, String> {
        config::transcribe(&self.app, clip, ticket, || self.show(ticket, "retrying", None))
    }
    fn copy(&self, text: String, ticket: Ticket) -> Result<Option<vults_platform::insert::Outcome>, String> {
        if vults_platform::windows_session::locked() != Some(false) {
            cancel(&self.app, true);
            return Ok(None);
        }
        let target = ticket
            .insertion()
            .filter(|_| crate::settings::dictation_insert(&self.app));
        if let Some(target) = target {
            match ticket.send(&text, || target.insert(&text)) {
                Some(true) => return Ok(Some(vults_platform::insert::Outcome::Pasted)),
                None => return Ok(None),
                Some(false) => {}
            }
        }
        let mut owner = self
            .clipboard
            .lock()
            .map_err(|_| "The clipboard owner is unavailable.")?;
        if owner.is_none() {
            *owner = Some(vults_platform::clipboard::Clipboard::new()?);
        }
        if !ticket.commit(&text, || {
            owner.as_mut().expect("clipboard initialized").copy(&text)?;
            Ok(())
        })? {
            return Ok(None);
        }
        Ok(Some(vults_platform::insert::Outcome::Copied))
    }
    fn command(&self, text: String, ticket: Ticket) -> Result<(), String> {
        let replacements = crate::dictation_words::rules(&crate::history::dir())
            .map_err(|_| "Could not read dictation words.")?
            .replacements;
        crate::runtime::voice_command(&self.app, text, ticket, replacements)
    }
    fn show(&self, ticket: &Ticket, phase: &str, error: Option<&str>) {
        if phase == "cancelled" && ticket.is_command() {
            crate::runtime::cancel_voice_commands(&self.app);
        }
        if let Some(status) = ticket.notice(phase, error) {
            if phase == "copied" {
                crate::doctor::dictation::copied();
            }
            let _ = self.app.emit("dictation", status);
            if let Some((entry, epoch)) = ticket.history_entry(phase) {
                crate::dictation_history::record(&self.app, entry, epoch);
            }
        }
    }
}

pub(crate) fn command_result(
    app: &AppHandle,
    ticket: &Ticket,
    heard: &str,
    report: crate::voice_commands::Report,
    deadline: Option<std::time::Instant>,
) {
    use crate::voice_commands::Feedback;
    let phase = match report.feedback {
        Feedback::Done | Feedback::SentToChat => "commanded",
        Feedback::Answer => "answered",
        Feedback::WhichOne => "asked",
        Feedback::NotByVoice => "refused",
        Feedback::NotCommand => "unknown",
        Feedback::UnknownName if !report.choices.is_empty() => "asked",
        _ => "failed",
    };
    let text = if report.choices.is_empty() {
        report.text
    } else {
        format!(
            "{}\n{}",
            report.text,
            report
                .choices
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        )
    };
    if let Some(status) = ticket.command_result(phase, text.clone(), heard, deadline) {
        let _ = app.emit("dictation", status);
        if let Some((entry, epoch)) = ticket.command_history(heard, &text) {
            crate::dictation_history::record(app, entry, epoch);
        }
    }
}
