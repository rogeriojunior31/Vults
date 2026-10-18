//! Independent local dictation, with opt-in validated editor/session delivery and clipboard fallback.

pub(crate) mod permission;
pub(crate) mod project;
use crate::dictation_history as history;
mod worker;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager};
use vults_audio::Capture;

use worker::{Backend, Clip, Ticket};

pub(crate) use worker::State;

pub fn start(app: &AppHandle) {
    app.manage(FocusBusy::default());
    app.manage(project::Cache::default());
    permission::start(app);
    history::start(app);
    match State::start(Native {
        app: app.clone(),
        clipboard: Arc::default(),
    }) {
        Ok(state) => {
            state.configure(
                app.state::<crate::settings::SettingsState>()
                    .0
                    .lock()
                    .map(|s| s.dictation_options())
                    .unwrap_or_default(),
            );
            app.manage(state);
        }
        Err(error) => tracing::warn!(%error, "dictation worker unavailable"),
    }
}

pub fn key(app: &AppHandle, id: &str, down: bool) {
    if id == "talk" && down {
        crate::doctor::dictation::stop_mic(app);
    }
    let observed = std::time::Instant::now();
    if crate::lock::locked(app) {
        cancel(app, true);
        return;
    }
    if let Some(state) = app.try_state::<State>() {
        // The async app-context reader is never a paste capability.
        let target = (id == "talk" && down && crate::settings::dictation_insert(app))
            .then(|| {
                app.try_state::<permission::Insertion>()
                    .and_then(|s| s.portal.capture())
                    .or_else(vults_platform::insert::Target::capture)
            })
            .flatten();
        if let Some(ticket) = state.key_at(id, down, observed) {
            if let Some(target) = target {
                ticket.pin(target);
            }
            // Capture can start while the same key callback verifies local pane metadata.
            if let Some(target) = ticket.insertion()
                && let Some(session) = capture_session(app, &target)
            {
                ticket.pin_session(session);
            }
            observe(app, ticket);
        }
    }
}

pub fn cancel(app: &AppHandle, forget: bool) {
    if let Some(state) = app.try_state::<State>() {
        state.cancel(forget);
    }
}

pub fn active(app: &AppHandle) -> bool {
    app.try_state::<State>().is_some_and(|s| s.active())
}

pub fn holds_space(app: &AppHandle) -> bool {
    app.try_state::<State>().is_some_and(|s| s.holds_space())
        && app.try_state::<crate::ShortcutKeys>().is_some_and(|keys| {
            keys.0.lock().is_ok_and(|keys| {
                keys.get("open")
                    .is_some_and(|key| vults_platform::raw_keys::uses_space(key))
            })
        })
}

#[derive(Default)]
struct FocusBusy(std::sync::Arc<std::sync::atomic::AtomicBool>);

struct FocusGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl Drop for FocusGuard {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

struct SessionTarget {
    key: vults_core::SessionKey,
    terminal: vults_protocol::Terminal,
    pane: vults_platform::insert::Pane,
    name: String,
}

fn capture_session(app: &AppHandle, target: &vults_platform::insert::Target) -> Option<SessionTarget> {
    if !target.window().is_terminal() {
        return None;
    }
    let sessions = crate::runtime::dictation_sessions(app)
        .recv_timeout(std::time::Duration::from_millis(50))
        .ok()?;
    let started = std::time::Instant::now();
    let mut found = None;
    for (key, terminal) in sessions {
        if started.elapsed() > std::time::Duration::from_millis(300) {
            return None;
        }
        if let Some(pane) = vults_platform::insert::Pane::capture(target.window(), &terminal) {
            if found.is_some() {
                return None;
            }
            let name = terminal
                .cwd
                .as_deref()
                .and_then(|cwd| std::path::Path::new(cwd).file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| key.session_id.clone());
            found = Some(SessionTarget {
                key,
                terminal,
                pane,
                name,
            });
        }
    }
    target.current().then_some(found).flatten()
}

fn observe(app: &AppHandle, ticket: Ticket) {
    let Some(busy) = app.try_state::<FocusBusy>() else {
        return;
    };
    // A stalled display server must not create one blocked reader per new recording.
    if busy.0.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let guard = FocusGuard(busy.0.clone());
    let sessions = crate::runtime::dictation_sessions(app);
    let app = app.clone();
    let started = std::time::Instant::now();
    let _ = std::thread::Builder::new()
        .name("dictation-focus".into())
        .spawn(move || {
            let _guard = guard;
            if !ticket.valid() {
                return;
            }
            let Some(window) = vults_platform::focus::read() else {
                return;
            };
            if started.elapsed() > std::time::Duration::from_millis(300) || !ticket.valid() {
                return;
            }
            let sessions = sessions
                .recv_timeout(std::time::Duration::from_millis(100))
                .unwrap_or_default();
            let session = vults_platform::focus::session(&window, &sessions).cloned();
            let pinned = ticket.session();
            let insertion = ticket.insertion();
            let context = insertion.as_ref().map_or(&window, |target| target.window());
            let project_session = vults_platform::focus::session(context, &sessions);
            let cwd = pinned
                .as_ref()
                .and_then(|s| s.terminal.cwd.as_deref())
                .or_else(|| {
                    sessions
                        .iter()
                        .find(|(key, _)| Some(key) == project_session)
                        .and_then(|(_, terminal)| terminal.cwd.as_deref())
                });
            if let Some(cache) = app.try_state::<project::Cache>() {
                ticket.set_project_words(cache.words(context, cwd));
            }
            if let Some(status) = ticket.observe(window, session) {
                let _ = app.emit("dictation", status);
            }
        });
}

#[derive(Clone)]
struct Native {
    app: AppHandle,
    clipboard: Arc<Mutex<Option<Owner>>>,
}

struct Owner {
    clipboard: vults_platform::clipboard::Clipboard,
    lease: Option<vults_platform::insert::Lease>,
}
impl Owner {
    fn copy(&mut self, text: &str) -> Result<(), String> {
        self.clipboard.copy(text)?;
        self.lease = None;
        Ok(())
    }
}

struct Recording {
    capture: Capture,
    config: Config,
    preview: vults_voice::Preview,
    music: vults_platform::music::Listening,
}

#[derive(Clone)]
struct Config {
    model: String,
    language: Option<String>,
    vad: Option<PathBuf>,
    cut_repeats: bool,
    retry_base: bool,
    vocabulary: bool,
    project_vocabulary: bool,
    project_words: Arc<OnceLock<Vec<String>>>,
    spoken_code: bool,
    cancel_by_voice: bool,
    words: vults_dictation::text::Rules,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: String::new(),
            language: None,
            vad: None,
            cut_repeats: true,
            retry_base: true,
            vocabulary: false,
            project_vocabulary: false,
            project_words: Arc::default(),
            spoken_code: true,
            cancel_by_voice: true,
            words: Default::default(),
        }
    }
}

impl Backend for Native {
    type Recording = Recording;
    type Keys = vults_platform::raw_keys::Listener;

    fn watch(
        &self,
        ticket: &Ticket,
        input: impl Fn(vults_platform::raw_keys::Key) + Send + 'static,
    ) -> Option<Self::Keys> {
        let bindings = self.app.try_state::<crate::ShortcutKeys>()?;
        let bindings = bindings.0.lock().ok()?;
        let mut actual = vec![bindings.get("talk")?.clone()];
        if let Some(again) = bindings.get("talk-again").filter(|key| !key.trim().is_empty()) {
            actual.push(again.clone());
        }
        drop(bindings);
        let live = ticket.clone();
        match vults_platform::raw_keys::listen(&actual, move || live.valid(), input) {
            Ok(Some(keys)) if ticket.attach_keys(keys.active()) => Some(keys),
            Ok(_) => None,
            Err(_) => {
                tracing::warn!("X11 dictation chords unavailable; portal shortcuts still apply");
                None
            }
        }
    }

    fn start(&self, ticket: &Ticket) -> Result<Recording, String> {
        let model = crate::settings::voice_model(&self.app).ok_or("Choose a voice model in Chat & voice.")?;
        if !vults_voice::installed(&crate::voice::models_dir()).contains(&model.as_str()) {
            return Err("The selected voice model is not installed.".into());
        }
        crate::voice::cancel(&self.app);
        for target in [crate::ISLAND, crate::NEST] {
            let _ = self.app.emit_to(target, "shortcut", "talk-cancel");
        }
        ticket.history_started(history::epoch());
        let (
            options,
            cut_repeats,
            retry_base,
            spoken_code,
            cancel_by_voice,
            music,
            vocabulary,
            project_vocabulary,
        ) = self
            .app
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .map(|s| {
                (
                    s.dictation_capture_options(),
                    s.dictation_cut_repeats,
                    s.dictation_retry_base,
                    s.dictation_spoken_code,
                    s.dictation_cancel_by_voice,
                    s.dictation_music,
                    s.dictation_vocabulary,
                    s.dictation_project_vocabulary,
                )
            })
            .map_err(|_| "settings are busy")?;
        let config = Config {
            model,
            language: crate::voice::language(&self.app),
            vad: vults_voice::vad_path(&crate::voice::models_dir()),
            cut_repeats,
            retry_base,
            vocabulary,
            project_vocabulary,
            project_words: ticket.project_words(),
            spoken_code,
            cancel_by_voice,
            words: crate::dictation_words::rules(&crate::history::dir())
                .map_err(|_| "Could not read dictation words.")?,
        };
        let (levels, meter) = (self.app.clone(), ticket.clone());
        let capture = Capture::start(options, move |level| {
            if let Some(status) = meter.feedback(Some(level.bands), None) {
                let _ = levels.emit("dictation", status);
            }
        })?;
        let (loader, events, partial) = (self.app.clone(), self.app.clone(), ticket.clone());
        let model = config.model.clone();
        let qualified = vults_voice::vocabulary::enabled(&model, config.language.as_deref());
        let words = if config.vocabulary {
            config.words.words.clone()
        } else {
            Vec::new()
        };
        let project = config.project_vocabulary.then(|| config.project_words.clone());
        let preview = vults_voice::Preview::start(
            capture.audio(),
            move || crate::voice::transcriber(&loader, &model),
            config.language.clone(),
            config.cut_repeats,
            move || {
                if qualified {
                    prompt_words(&words, project.as_deref())
                } else {
                    Vec::new()
                }
            },
            move |text| {
                if let Some(status) = partial.feedback(None, Some(text)) {
                    let _ = events.emit("dictation", status);
                }
            },
        );
        Ok(Recording {
            capture,
            config,
            preview,
            music: vults_platform::music::Listening::start(music),
        })
    }

    fn finish(&self, recording: Recording) -> Result<Clip, String> {
        let Recording {
            capture,
            config,
            preview,
            music,
        } = recording;
        drop(preview);
        let pcm = capture.finish()?;
        drop(music);
        Ok(Clip { pcm, config })
    }

    fn transcribe(&self, clip: Clip, ticket: &Ticket) -> Result<Option<String>, String> {
        let started = std::time::Instant::now();
        let config = clip.config;
        ticket.history_measured(clip.pcm.len() as f64 / 16_000.0, config.language.clone());
        let text = decode(
            &config.model,
            vults_voice::installed(&crate::voice::models_dir()).contains(&"base"),
            config.retry_base,
            ticket,
            |id| {
                let model = crate::voice::transcriber(&self.app, id)?;
                if !ticket.valid() {
                    return Ok(String::new());
                }
                if id != config.model {
                    crate::doctor::dictation::retried();
                }
                model
                    .transcribe_measured(
                        &clip.pcm,
                        config.language.as_deref(),
                        config.vad.as_deref(),
                        ticket.stop(),
                        config.cut_repeats,
                        &vocabulary_words(&config, id),
                    )
                    .map(|text| {
                        let text = text.unwrap_or_default();
                        if ticket.valid() {
                            crate::doctor::dictation::loops_cut(text.loops_cut);
                        }
                        text.text
                    })
            },
            || {
                config
                    .vad
                    .as_ref()
                    .map_or(Ok(false), |vad| vults_voice::has_speech(&clip.pcm, vad))
            },
            || {
                self.show(ticket, "retrying", None);
            },
        )?;
        ticket.history_transcribed(started.elapsed());
        ticket.prepare_text(&text, &config)
    }

    fn copy(&self, text: String, ticket: Ticket) -> Result<Option<vults_platform::insert::Outcome>, String> {
        use vults_platform::insert::{Lease, Outcome};
        let target = ticket
            .insertion()
            .filter(|_| crate::settings::dictation_insert(&self.app));
        let mut held = false;
        if let (Some(target), Some(session)) = (&target, ticket.session())
            && session.pane.current(target.window())
            && target.current()
        {
            // Read core after native pane IO, immediately before foreground validation/dispatch.
            match crate::runtime::dictation_safety(&self.app, session.key.clone(), session.terminal.clone()) {
                vults_core::DictationSafety::Waiting => held = true,
                vults_core::DictationSafety::Ready => {
                    match ticket.send(&text, || target.unchanged() && session.pane.send(&text)) {
                        Some(true) => return Ok(Some(Outcome::Session)),
                        Some(false) => {}
                        None => return Ok(None),
                    }
                }
                _ => {}
            }
        }
        let mut owner = self
            .clipboard
            .lock()
            .map_err(|_| "The clipboard owner is unavailable.")?;
        if owner.is_none() {
            *owner = Some(Owner {
                clipboard: vults_platform::clipboard::Clipboard::new()?,
                lease: None,
            });
        }
        let owner = owner.as_mut().expect("clipboard initialized");
        // Unknown terminal identities never fall through to an editor paste.
        let target = target.filter(|target| !target.window().is_terminal());
        let pending = if target.is_some() && ticket.valid() {
            Lease::prepare(&text, || {
                owner
                    .lease
                    .as_ref()
                    .and_then(Lease::previous)
                    .unwrap_or_else(|| owner.clipboard.text())
            })
        } else {
            None
        };
        let mut leased = false;
        if !ticket.commit(&text, || {
            if let Some(mut lease) = pending
                && lease.claim()
            {
                owner.lease = Some(lease);
                if let Err(error) = owner.clipboard.confirm(&text) {
                    if let Some(lease) = &owner.lease {
                        lease.copied();
                    }
                    return Err(error);
                }
                leased = true;
                return Ok(());
            }
            owner.copy(&text)
        })? {
            return Ok(None);
        }
        if !leased {
            return Ok(Some(if held { Outcome::Held } else { Outcome::Copied }));
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
        let mut outcome = Outcome::Copied;
        // Turning insertion off invalidates the ticket before the setting is saved.
        if !ticket.commit(&text, || {
            if let Some(target) = target
                && let Some(lease) = &owner.lease
                && target.paste(lease)
            {
                lease.pasted();
                outcome = Outcome::Pasted;
            }
            Ok(())
        })? {
            if let Some(lease) = &owner.lease {
                lease.copied();
            }
            return Ok(None);
        }
        if outcome == Outcome::Copied
            && let Some(lease) = &owner.lease
        {
            lease.copied();
        }
        Ok(Some(outcome))
    }

    fn show(&self, ticket: &Ticket, phase: &str, error: Option<&str>) {
        if let Some(status) = ticket.notice(phase, error) {
            if phase == "copied" {
                crate::doctor::dictation::copied();
            }
            tracing::info!(phase, "dictation state");
            let _ = self.app.emit("dictation", status);
            if let Some((entry, epoch)) = ticket.history_entry(phase) {
                history::record(&self.app, entry, epoch);
            }
        }
    }
}

fn vocabulary_words(config: &Config, model: &str) -> Vec<String> {
    if !vults_voice::vocabulary::enabled(model, config.language.as_deref()) {
        return Vec::new();
    }
    prompt_words(
        if config.vocabulary {
            &config.words.words
        } else {
            &[]
        },
        config.project_vocabulary.then_some(config.project_words.as_ref()),
    )
}

fn prompt_words(dictionary: &[String], project: Option<&OnceLock<Vec<String>>>) -> Vec<String> {
    let mut words = dictionary.to_vec();
    if let Some(project) = project {
        // The first decode freezes missing context too: late focus never changes a live prompt.
        words.extend(project.get_or_init(Vec::new).iter().cloned());
    }
    words
}

fn decode(
    model: &str,
    base_installed: bool,
    retry_base: bool,
    ticket: &Ticket,
    mut run: impl FnMut(&str) -> Result<String, String>,
    speech: impl FnOnce() -> Result<bool, String>,
    retrying: impl FnOnce(),
) -> Result<String, String> {
    if !ticket.valid() {
        return Ok(String::new());
    }
    let text = run(model)?;
    if !text.is_empty() || !ticket.valid() || model == "base" || !base_installed || !retry_base || !speech()?
    {
        return Ok(text);
    }
    retrying();
    if !ticket.valid() {
        return Ok(String::new());
    }
    run("base")
}
