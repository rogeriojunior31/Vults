//! A temporary user association between an exact media session and a live mixer group.
use super::{AudioChoice, MediaState};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Manager};
use vults_platform::player_audio;

#[derive(Debug, Default)]
pub(super) struct State {
    epoch: u64,
    choices: Vec<(AudioChoice, player_audio::Selection)>,
    binding: Option<Arc<Binding>>,
}
#[derive(Debug)]
pub(crate) struct Binding {
    live: AtomicBool,
    player: String,
    playback: vults_media::Target,
    pub(crate) audio: player_audio::Target,
    choice: AudioChoice,
}
impl Binding {
    pub(crate) fn valid(&self) -> bool {
        self.live.load(Ordering::Acquire) && self.playback.present() && self.live.load(Ordering::Acquire)
    }
}
pub(crate) fn clear<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let state = app.state::<MediaState>();
    let mut audio = state.audio.lock().unwrap_or_else(|e| e.into_inner());
    audio.epoch = audio.epoch.wrapping_add(1);
    audio.choices.clear();
    if let Some(old) = audio.binding.take() {
        old.live.store(false, Ordering::Release);
    }
}
fn allowed(app: &AppHandle, player: &str) -> bool {
    !crate::lock::locked(app)
        && vults_platform::windows_session::locked() == Some(false)
        && app
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .is_ok_and(|s| s.dictation_commands && s.dictation_command_media && s.now_playing)
        && super::media_now(app.clone()).is_some_and(|now| now.player == player)
}
pub(crate) fn binding(app: &AppHandle, player: &str) -> Option<Arc<Binding>> {
    let binding = app.state::<MediaState>().audio.lock().ok()?.binding.clone()?;
    (binding.player == player && binding.valid()).then_some(binding)
}
pub(super) fn status(app: &AppHandle) -> Option<AudioChoice> {
    let player = super::media_now(app.clone())?.player;
    if !allowed(app, &player) {
        return None;
    }
    binding(app, &player).map(|binding| binding.choice.clone())
}
pub(super) async fn choices(app: AppHandle) -> Result<Vec<AudioChoice>, String> {
    clear(&app);
    let player = super::media_now(app.clone())
        .ok_or("No media player is selected.")?
        .player;
    if !allowed(&app, &player) {
        return Err("Media audio selection is disabled.".into());
    }
    let epoch = app
        .state::<MediaState>()
        .audio
        .lock()
        .map_err(|_| "Media audio is busy.")?
        .epoch;
    let gate_app = app.clone();
    let gate_player = player.clone();
    let native = player_audio::list(move || allowed(&gate_app, &gate_player))
        .await
        .map_err(|_| "Could not read application audio sessions.")?;
    if !allowed(&app, &player) {
        return Err("Media player changed.".into());
    }
    let state = app.state::<MediaState>();
    let mut audio = state.audio.lock().map_err(|_| "Media audio is busy.")?;
    if audio.epoch != epoch {
        return Err("Audio selection changed.".into());
    }
    let mut result = Vec::new();
    for (index, candidate) in native.into_iter().enumerate() {
        let token = format!("{epoch}:{index}");
        let choice = AudioChoice {
            token,
            app: candidate.app,
            output: candidate.output,
            pid: candidate.pid,
            percent: candidate.percent,
        };
        audio.choices.push((choice.clone(), candidate.selection));
        result.push(choice);
    }
    Ok(result)
}
pub(super) async fn select(app: AppHandle, token: Option<String>) -> Result<(), String> {
    let Some(token) = token else {
        clear(&app);
        return Ok(());
    };
    let player = super::media_now(app.clone())
        .ok_or("No media player is selected.")?
        .player;
    if !allowed(&app, &player) {
        return Err("Media audio selection is disabled.".into());
    }
    let (epoch, choice, selection) = {
        let state = app.state::<MediaState>();
        let mut audio = state.audio.lock().map_err(|_| "Media audio is busy.")?;
        let (choice, selection) = audio
            .choices
            .iter()
            .find(|(choice, _)| choice.token == token)
            .ok_or("Refresh application audio sessions before choosing.")?
            .clone();
        if let Some(old) = audio.binding.take() {
            old.live.store(false, Ordering::Release);
        }
        audio.epoch = audio.epoch.wrapping_add(1);
        (audio.epoch, choice, selection)
    };
    let playback = vults_media::pin(&player).await?;
    let gate_app = app.clone();
    let gate_player = player.clone();
    let target = player_audio::pin(selection, move || allowed(&gate_app, &gate_player))
        .await
        .map_err(|_| "Application audio changed. Refresh and choose again.")?;
    if !allowed(&app, &player) || !playback.present() {
        return Err("Media player changed.".into());
    }
    let state = app.state::<MediaState>();
    let mut audio = state.audio.lock().map_err(|_| "Media audio is busy.")?;
    if audio.epoch != epoch {
        return Err("Audio selection changed.".into());
    }
    audio.binding = Some(Arc::new(Binding {
        live: AtomicBool::new(true),
        player,
        playback,
        audio: target,
        choice,
    }));
    Ok(())
}
