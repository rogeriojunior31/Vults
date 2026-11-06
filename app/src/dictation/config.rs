use super::{
    cleanup,
    worker::{Clip, Ticket},
};
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

#[derive(Clone)]
pub(super) struct Config {
    pub(super) cleanup: Option<cleanup::Choice>,
    pub(super) model: String,
    pub(super) language: Option<String>,
    pub(super) vad: Option<PathBuf>,
    pub(super) cut_repeats: bool,
    pub(super) retry_base: bool,
    pub(super) vocabulary: bool,
    pub(super) project_vocabulary: bool,
    pub(super) project_words: Arc<OnceLock<Vec<String>>>,
    pub(super) spoken_code: bool,
    pub(super) cancel_by_voice: bool,
    pub(super) words: vults_dictation::text::Rules,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cleanup: None,
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

impl Config {
    pub(super) fn cut_repeats_for(&self, ticket: &Ticket) -> bool {
        // Repeated words can identify a different app/project; command names must not be shortened.
        self.cut_repeats && !ticket.is_command()
    }
}

pub(super) fn vocabulary_words(config: &Config, model: &str) -> Vec<String> {
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

pub(super) fn prompt_words(dictionary: &[String], project: Option<&OnceLock<Vec<String>>>) -> Vec<String> {
    let mut words = dictionary.to_vec();
    if let Some(project) = project {
        // The first decode freezes missing context too: late focus never changes a live prompt.
        words.extend(project.get_or_init(Vec::new).iter().cloned());
    }
    words
}

pub(super) fn decode(
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

pub(super) fn transcribe(
    app: &tauri::AppHandle,
    clip: Clip,
    ticket: &Ticket,
    retrying: impl FnOnce(),
) -> Result<Option<String>, String> {
    let started = std::time::Instant::now();
    let config = clip.config;
    ticket.history_measured(clip.pcm.len() as f64 / 16_000.0, config.language.clone());
    let text = decode(
        &config.model,
        vults_voice::installed(&crate::voice::models_dir()).contains(&"base"),
        config.retry_base,
        ticket,
        |id| {
            let model = crate::voice::transcriber(app, id)?;
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
                    config.cut_repeats_for(ticket),
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
            retrying();
        },
    )?;
    ticket.history_transcribed(started.elapsed());
    #[cfg(windows)]
    if vults_platform::windows_session::locked() != Some(false) {
        return Ok(None);
    }
    if ticket.is_command() {
        return ticket.prepare_text(&text, &config);
    }
    let Some(output) = ticket.prepare_output(&text, &config)? else {
        return Ok(None);
    };
    let raw = output.text();
    let text = config
        .cleanup
        .as_ref()
        .and_then(|choice| cleanup::apply(app, ticket, choice, &output, &config.words.words))
        .unwrap_or(raw);
    Ok(ticket.valid().then_some(text))
}
