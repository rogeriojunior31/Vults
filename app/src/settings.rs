//! The app's own settings, `~/.config/vults/settings.json`. Versioned so a later release
//! can migrate an older file instead of guessing.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// Bump when a key is added or changes meaning: from 0.1.1 on, an older release then keeps a
/// copy of the file before it writes back only the keys it knows. 0.1.0 does not read it.
const VERSION: u32 = 40;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    #[serde(other)]
    Black,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Settings {
    pub version: u32,
    /// Connector id → enabled. Off until the user turns one on.
    #[serde(default)]
    pub connectors: BTreeMap<String, bool>,
    #[serde(default = "yes")]
    pub sounds: bool,
    /// How loud the sounds are, in percent; 50 is how loud 0.1.0 played them.
    #[serde(default = "volume")]
    pub volume: u8,
    /// Seconds the open island waits, once the pointer leaves, before folding.
    #[serde(default = "fold_after")]
    pub fold_after: u32,
    /// Hovering opens the island all the way, not only the pill. Off until the user turns it on
    /// (from version 11).
    #[serde(default)]
    pub open_on_hover: bool,
    /// Permissions the user chose to always allow (exact tool and target, per project).
    #[serde(default)]
    pub rules: Vec<vults_core::Rule>,
    /// The monitor the island sits on, by maker and model; `None` lets the compositor choose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<String>,
    /// The API chat's provider (an id from `vults_chat::providers`).
    #[serde(default = "api_provider")]
    pub api_provider: String,
    /// Provider id → the model chosen for it. Keys never live here: only in the keyring.
    #[serde(default)]
    pub api_models: BTreeMap<String, String>,
    /// Show what is playing on the island. Off until the user turns it on.
    #[serde(default)]
    pub now_playing: bool,
    /// The voice model the chat's mic uses; none keeps voice off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_model: Option<String>,
    /// What the user speaks: a code (`pt`), `auto` to detect it, absent to follow the system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_language: Option<String>,
    /// External clients may request text questions; capture still requires a physical key.
    #[serde(default)]
    pub question_agents: bool,
    /// Explicit permission to paste into validated native X11 editors. Off by default.
    #[serde(default)]
    pub dictation_insert: bool,
    #[serde(default = "yes")]
    pub dictation_caption: bool,
    #[serde(default = "yes")]
    pub dictation_sound: bool,
    #[serde(default = "yes")]
    pub dictation_commands: bool,
    #[serde(default)]
    pub dictation_cleanup: bool,
    /// A lost ledger cannot silently renew an already authorized request budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dictation_cleanup_budget: Option<String>,
    /// Independent consent; toggling it never resets the durable request ledger.
    #[serde(default)]
    pub meeting_summary: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meeting_summary_budget: Option<String>,
    #[serde(default = "natural_style")]
    pub dictation_style: vults_dictation::cleanup::Style,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dictation_write_in: Option<vults_dictation::cleanup::Language>,
    #[serde(default)]
    pub dictation_sessions_english: bool,
    #[serde(default)]
    pub dictation_command_chat: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dictation_microphone: Option<String>,
    #[serde(default = "yes")]
    pub dictation_cut_repeats: bool,
    #[serde(default = "yes")]
    pub dictation_retry_base: bool,
    #[serde(default)]
    pub dictation_vocabulary: bool,
    #[serde(default)]
    pub dictation_project_vocabulary: bool,
    #[serde(default = "yes")]
    pub dictation_spoken_code: bool,
    #[serde(default = "yes")]
    pub dictation_cancel_by_voice: bool,
    #[serde(default)]
    pub dictation_music: vults_platform::music::Mode,
    #[serde(default = "yes")]
    pub dictation_double_tap: bool,
    #[serde(default = "yes")]
    pub dictation_space_lock: bool,
    #[serde(default = "yes")]
    pub dictation_escape_cancel: bool,
    #[serde(default = "dictation_keep")]
    pub dictation_keep: String,
    /// Zeca's species, a renderer id (`ui/src/character/flock/species.ts`); an unknown one draws
    /// the black vulture.
    #[serde(default = "zeca_species")]
    pub zeca_species: String,
    /// What Zeca wears: the calendar's look (`auto`), none, or one look for good.
    #[serde(default)]
    pub zeca_look: vults_core::looks::Outfit,
    /// Where the other sessions' birds are drawn from.
    #[serde(default)]
    pub flock: vults_core::flock::Flock,
    /// Now and then a vulture from outside the flock crosses the sky.
    #[serde(default = "yes")]
    pub visitors: bool,
    /// The presence preset: *Island*, *Panel*, *Quiet* or *Paused*. Version 4 knew only the first
    /// two, under the same key and words, so its file reads as is.
    #[serde(default)]
    pub presence: crate::panel::Presence,
    /// Desktop notifications: a session finished or failed, a card waiting.
    #[serde(default = "yes")]
    pub notifications: bool,
    /// Zeca, the companion who chats and listens. Off, the flock still works (ADR 0018).
    #[serde(default = "yes")]
    pub zeca: bool,
    /// The corner widget's corner; none (the default) for no widget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget: Option<crate::widget::Corner>,
    /// Mute, pin or hide, per project folder (from version 8), and its bird (from version 12).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub projects: BTreeMap<String, vults_core::ProjectPrefs>,
    /// The daily budget, in cents of a US dollar (W9, from version 20): spent, Vults starts no new
    /// session that day. Absent, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_daily_cents: Option<u32>,
    /// The budget's step already told today (`2026-10-11:1`: 80 %; `:2`: spent), so a restart
    /// does not tell it again (version 20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_told: Option<String>,
    /// Do not disturb until then, in seconds since the Unix epoch; absent when off (from
    /// version 9). Past, it is off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dnd_until: Option<u64>,
    /// Keep the local history of agent turns, for Settings → Activity (from version 13).
    #[serde(default = "yes")]
    pub history: bool,
    /// The Monday of the last week whose recap card was read, so it never comes back (from
    /// version 14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recap_shown_week: Option<String>,
    /// The language the app speaks (`en`, `pt-BR`, `es`, `zh`); absent follows the system (from
    /// version 15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default)]
    pub theme: Theme,
    /// Read Codex's plan usage every few minutes (a request to OpenAI, through the network gate).
    /// Off on a new install; a file from before version 16 has none, and gets it on when a
    /// `codex` is installed, so an upgrade keeps the meters it showed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_usage: Option<bool>,
    /// The chat's Claude Code and Codex read the user's own setup (their `CLAUDE.md`, skills,
    /// MCPs, hooks and allow rules; a project's only where Claude Code already trusts it).
    #[serde(default = "yes")]
    pub native_chat: bool,
    /// Who answers the chat's permissions (ADR 0004, from version 22): a card every time until
    /// the user chooses their policies or full access.
    #[serde(default, skip_serializing_if = "ChatAccess::is_ask")]
    pub chat_access: ChatAccess,
    /// The chat's model and effort per folder (`""` for its own empty one), then per engine
    /// (`claude`, `codex`): what a new conversation there starts with.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub chat_picks: BTreeMap<String, BTreeMap<String, Pick>>,
    /// The Nest's window as the user last left it, in logical pixels (from version 21); absent,
    /// it opens at its default size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nest_size: Option<(u32, u32)>,
    /// Zeca's web tools for the API chat (Z7): a search and a page read, requests on the user's
    /// behalf (ADR 0019). Off until the user turns them on.
    #[serde(default)]
    pub web: bool,
    /// Zeca's memory (M4): a journal of what the flock and the chat did, kept on this computer.
    /// Off until the user turns it on.
    #[serde(default)]
    pub memory: bool,
    /// Each past day of the journal summed up by a small model (M4): a request on the user's
    /// behalf (ADR 0019), off until the user turns it on, and only with the memory on.
    #[serde(default)]
    pub journal_summary: bool,
    /// The day the daily summary was turned on: only days from then go to the model, never the
    /// journal kept before the user said yes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_summary_since: Option<String>,
    /// Recall also by meaning, through a local Ollama on `127.0.0.1` (M5). Off until the user
    /// turns it on, and only with the memory on.
    #[serde(default)]
    pub local_embeddings: bool,
    /// After a conversation, a small model proposes what Zeca should remember (M6): a request on
    /// the user's behalf (ADR 0019), off until turned on, and only with the memory on.
    #[serde(default)]
    pub memory_suggest: bool,
    /// At night, a small model reads Zeca's facts and the last days' summaries to propose merges
    /// and write a debrief (M8): a request on the user's behalf (ADR 0019), off until turned on.
    #[serde(default)]
    pub sleep: bool,
    /// The search engine: `brave`, `tavily` or `searxng`.
    #[serde(default = "web_engine")]
    pub web_engine: String,
    /// The user's own SearXNG, when that is the engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub searxng_url: Option<String>,
    /// Answering cards from a paired phone (ADR 0004, L2): its relay's requests are on the user's
    /// behalf (ADR 0019). Off until the user turns it on (from version 17).
    #[serde(default)]
    pub phone: bool,
    /// How the phone is reached: `relay` (a relay's URL) or `direct` (this computer listens on an
    /// address of a LAN or a tailnet).
    #[serde(default = "phone_transport")]
    pub phone_transport: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_relay: Option<String>,
    /// Where this computer listens for the phone in `direct`: an IP address and a port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_listen: Option<String>,
}

fn phone_transport() -> String {
    "relay".into()
}

fn web_engine() -> String {
    "brave".into()
}

/// Who answers the chat's permission cards (ADR 0004). Whatever it is, a deny in a policy denies,
/// and every answer is on the audit log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChatAccess {
    /// A card for every call no Always rule covers.
    #[default]
    Ask,
    /// The user's accepted policies answer what they cover; the rest asks.
    Policies,
    /// Everything runs with no card, but a destructive call, what a policy denies or asks first,
    /// and nothing once the conversation read content from outside.
    Full,
}

impl ChatAccess {
    fn is_ask(&self) -> bool {
        *self == Self::Ask
    }

    /// How much it lets run without a card.
    fn rank(self) -> u8 {
        match self {
            Self::Ask => 0,
            Self::Policies => 1,
            Self::Full => 2,
        }
    }
}

/// A model and an effort for the chat's CLI; none is the CLI's own default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pick {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

fn zeca_species() -> String {
    "atratus".into()
}

fn api_provider() -> String {
    "anthropic".into()
}

fn yes() -> bool {
    true
}

/// Percent; a larger number in the file plays at full volume.
const VOLUME_MAX: u8 = 100;

fn volume() -> u8 {
    50
}

/// The choices the settings offer; anything else in the file is brought into this range.
const FOLD_AFTER: std::ops::RangeInclusive<u32> = 5..=120;

fn natural_style() -> vults_dictation::cleanup::Style {
    vults_dictation::cleanup::Style::Natural
}

fn dictation_keep() -> String {
    "30".into()
}

fn fold_after() -> u32 {
    15
}

impl Settings {
    #[cfg(any(target_os = "linux", test))]
    pub fn command_chat(&self) -> bool {
        self.dictation_commands && self.dictation_command_chat && self.zeca
    }

    pub fn dictation_capture_options(&self) -> vults_audio::CaptureOptions {
        vults_audio::CaptureOptions {
            device: self.dictation_microphone.clone(),
            max_seconds: 600,
        }
    }

    #[cfg(any(target_os = "linux", test))]
    pub fn dictation_options(&self) -> vults_dictation::GestureOptions {
        vults_dictation::GestureOptions {
            double_tap: self.dictation_double_tap,
            space_lock: self.dictation_space_lock,
            escape_cancel: self.dictation_escape_cancel,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            connectors: BTreeMap::new(),
            sounds: true,
            volume: volume(),
            fold_after: fold_after(),
            open_on_hover: false,
            rules: Vec::new(),
            budget_daily_cents: None,
            budget_told: None,
            monitor: None,
            api_provider: api_provider(),
            api_models: BTreeMap::new(),
            now_playing: false,
            voice_model: None,
            voice_language: None,
            question_agents: false,
            dictation_insert: false,
            dictation_caption: true,
            dictation_sound: true,
            dictation_commands: true,
            dictation_cleanup: false,
            dictation_cleanup_budget: None,
            meeting_summary: false,
            meeting_summary_budget: None,
            dictation_style: natural_style(),
            dictation_write_in: None,
            dictation_sessions_english: false,
            dictation_command_chat: false,
            dictation_microphone: None,
            dictation_cut_repeats: true,
            dictation_retry_base: true,
            dictation_vocabulary: false,
            dictation_project_vocabulary: false,
            dictation_spoken_code: true,
            dictation_cancel_by_voice: true,
            dictation_music: Default::default(),
            dictation_double_tap: true,
            dictation_space_lock: true,
            dictation_escape_cancel: true,
            dictation_keep: dictation_keep(),
            zeca_species: zeca_species(),
            zeca_look: Default::default(),
            flock: Default::default(),
            visitors: true,
            presence: Default::default(),
            notifications: true,
            zeca: true,
            widget: None,
            projects: BTreeMap::new(),
            dnd_until: None,
            history: true,
            recap_shown_week: None,
            language: None,
            theme: Theme::Black,
            codex_usage: None,
            native_chat: true,
            chat_access: ChatAccess::Ask,
            chat_picks: BTreeMap::new(),
            nest_size: None,
            web: false,
            memory: false,
            journal_summary: false,
            journal_summary_since: None,
            local_embeddings: false,
            memory_suggest: false,
            sleep: false,
            web_engine: web_engine(),
            searxng_url: None,
            phone: false,
            phone_transport: phone_transport(),
            phone_relay: None,
            phone_listen: None,
        }
    }
}

/// The settings in memory; every change is written through to disk.
#[derive(Debug)]
pub struct SettingsState(pub Mutex<Settings>);

/// What the windows may read.
#[derive(Serialize, Clone)]
pub struct Public {
    #[serde(rename = "questionAgents")]
    pub question_agents: bool,
    #[serde(rename = "dictationKeep")]
    pub dictation_keep: String,
    #[serde(rename = "dictationSound")]
    pub dictation_sound: bool,
    #[serde(rename = "dictationCommands")]
    pub dictation_commands: bool,
    #[serde(rename = "dictationCleanup")]
    pub dictation_cleanup: bool,
    #[serde(rename = "meetingSummary")]
    pub meeting_summary: bool,
    #[serde(rename = "dictationStyle")]
    pub dictation_style: vults_dictation::cleanup::Style,
    #[serde(rename = "dictationWriteIn")]
    pub dictation_write_in: Option<vults_dictation::cleanup::Language>,
    #[serde(rename = "dictationSessionsEnglish")]
    pub dictation_sessions_english: bool,
    #[serde(rename = "dictationCommandChat")]
    pub dictation_command_chat: bool,
    #[serde(rename = "dictationCutRepeats")]
    pub dictation_cut_repeats: bool,
    #[serde(rename = "dictationVocabulary")]
    pub dictation_vocabulary: bool,
    #[serde(rename = "dictationProjectVocabulary")]
    pub dictation_project_vocabulary: bool,
    #[serde(rename = "dictationRetryBase")]
    pub dictation_retry_base: bool,
    #[serde(rename = "dictationMusic")]
    pub dictation_music: vults_platform::music::Mode,
    #[serde(rename = "dictationCancelByVoice")]
    pub dictation_cancel_by_voice: bool,
    #[serde(rename = "dictationSpokenCode")]
    pub dictation_spoken_code: bool,
    #[serde(rename = "dictationMicrophone")]
    pub dictation_microphone: Option<String>,
    #[serde(rename = "dictationDoubleTap")]
    pub dictation_double_tap: bool,
    #[serde(rename = "dictationSpaceLock")]
    pub dictation_space_lock: bool,
    #[serde(rename = "dictationEscapeCancel")]
    pub dictation_escape_cancel: bool,
    #[serde(rename = "dictationCaption")]
    pub dictation_caption: bool,
    #[serde(rename = "dictationInsert")]
    pub dictation_insert: bool,
    #[serde(rename = "dictationInsertAvailable")]
    pub dictation_insert_available: bool,
    #[serde(rename = "dictationWayland")]
    pub dictation_wayland: bool,
    pub sounds: bool,
    pub volume: u8,
    /// Starts with the desktop session (an XDG autostart entry on Linux).
    pub autostart: bool,
    #[serde(rename = "foldAfter")]
    pub fold_after: u32,
    #[serde(rename = "openOnHover")]
    pub open_on_hover: bool,
    pub monitor: Option<String>,
    #[serde(rename = "nowPlaying")]
    pub now_playing: bool,
    #[serde(rename = "codexUsage")]
    pub codex_usage: bool,
    #[serde(rename = "nativeChat")]
    pub native_chat: bool,
    #[serde(rename = "chatAccess")]
    pub chat_access: ChatAccess,
    pub memory: bool,
    #[serde(rename = "journalSummary")]
    pub journal_summary: bool,
    #[serde(rename = "localEmbeddings")]
    pub local_embeddings: bool,
    #[serde(rename = "memorySuggest")]
    pub memory_suggest: bool,
    pub sleep: bool,
    #[serde(rename = "zecaSpecies")]
    pub zeca_species: String,
    #[serde(rename = "zecaLook")]
    pub zeca_look: vults_core::looks::Outfit,
    pub flock: vults_core::flock::Flock,
    pub visitors: bool,
    pub presence: crate::panel::Presence,
    pub notifications: bool,
    pub zeca: bool,
    /// Activity's local history (the island's gear reads it here, without the history itself).
    pub history: bool,
    pub widget: Option<crate::widget::Corner>,
    /// Do not disturb until then (epoch seconds), while it lasts.
    #[serde(rename = "dndUntil")]
    pub dnd_until: Option<u64>,
    /// The language chosen, or none to follow the system.
    pub language: Option<String>,
    pub theme: Theme,
    /// The language in use (`pt-BR`): the choice, or the system's.
    pub lang: String,
    /// Where the settings file and the app's data really are (XDG aware), `~` for $HOME.
    #[serde(rename = "settingsPath")]
    pub settings_path: String,
    #[serde(rename = "dataPath")]
    pub data_path: String,
}

#[tauri::command]
pub fn app_settings(app: AppHandle, state: tauri::State<'_, SettingsState>) -> Public {
    use tauri_plugin_autostart::ManagerExt;
    let s = state.0.lock().map(|s| s.clone()).unwrap_or_default();
    Public {
        question_agents: s.question_agents,
        dictation_caption: s.dictation_caption,
        dictation_sound: s.dictation_sound,
        dictation_commands: s.dictation_commands,
        dictation_cleanup: s.dictation_cleanup,
        meeting_summary: s.meeting_summary,
        dictation_style: s.dictation_style,
        dictation_write_in: s.dictation_write_in,
        dictation_sessions_english: s.dictation_sessions_english,
        dictation_command_chat: s.dictation_command_chat,
        dictation_microphone: s.dictation_microphone.clone(),
        dictation_cut_repeats: s.dictation_cut_repeats,
        dictation_retry_base: s.dictation_retry_base,
        dictation_vocabulary: s.dictation_vocabulary,
        dictation_project_vocabulary: s.dictation_project_vocabulary,
        dictation_spoken_code: s.dictation_spoken_code,
        dictation_cancel_by_voice: s.dictation_cancel_by_voice,
        dictation_music: s.dictation_music,
        dictation_double_tap: s.dictation_double_tap,
        dictation_space_lock: s.dictation_space_lock,
        dictation_escape_cancel: s.dictation_escape_cancel,
        dictation_keep: s.dictation_keep.clone(),
        dictation_wayland: dictation_wayland(),
        dictation_insert: s.dictation_insert,
        dictation_insert_available: cfg!(target_os = "linux") && dictation_insert_available(),
        sounds: s.sounds,
        volume: s.volume.min(VOLUME_MAX),
        monitor: s.monitor,
        now_playing: s.now_playing,
        codex_usage: s.codex_usage.unwrap_or(false),
        native_chat: s.native_chat,
        chat_access: s.chat_access,
        memory: s.memory,
        journal_summary: s.journal_summary,
        local_embeddings: s.local_embeddings,
        memory_suggest: s.memory_suggest,
        sleep: s.sleep,
        fold_after: s.fold_after.clamp(*FOLD_AFTER.start(), *FOLD_AFTER.end()),
        open_on_hover: s.open_on_hover,
        zeca_species: s.zeca_species,
        zeca_look: s.zeca_look,
        flock: s.flock,
        visitors: s.visitors,
        presence: s.presence,
        notifications: s.notifications,
        zeca: s.zeca,
        history: s.history,
        widget: s.widget,
        dnd_until: s.dnd_until.filter(|t| *t > epoch_now()),
        lang: resolve(s.language.as_deref()).code().to_string(),
        language: s.language,
        theme: s.theme,
        settings_path: crate::paths::shown(&path()),
        // The trailing separator marks a folder, in the platform's own separator.
        data_path: crate::paths::shown(&crate::paths::data_dir().join("")),
        // The OS is the source of truth: the user may remove the entry by hand.
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

/// Before the rename the login entry was `Vultures AI.desktop`, running the old binary: swap it
/// for ours, so an upgrade keeps starting at login and the old app never starts again.
#[cfg(target_os = "linux")]
pub fn move_legacy_autostart(app: &AppHandle) {
    use tauri_plugin_autostart::ManagerExt;
    // The same folder the autostart plugin writes to (it ignores XDG_CONFIG_HOME).
    let old = crate::paths::home()
        .join(".config/autostart")
        .join(format!("{}.desktop", vults_brand::LEGACY_NAME));
    if old.is_file()
        && std::fs::remove_file(&old).is_ok()
        && let Err(e) = app.autolaunch().enable()
    {
        tracing::warn!("can't move the old autostart entry: {e}");
    }
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    if on { launcher.enable() } else { launcher.disable() }
        .map_err(|e| format!("can't change autostart: {e}"))
}

#[tauri::command]
pub fn set_sounds(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.sounds = on)?;
    let _ = app.emit("settings", serde_json::json!({ "sounds": on }));
    Ok(())
}

/// The island plays its next cue at this volume.
#[tauri::command]
pub fn set_volume(app: AppHandle, percent: u8) -> Result<(), String> {
    let percent = percent.min(VOLUME_MAX);
    edit(&app, |s| s.volume = percent)?;
    let _ = app.emit("settings", serde_json::json!({ "volume": percent }));
    Ok(())
}

fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The saved do-not-disturb as the core's clock reads it; none once it has passed.
pub fn dnd_instant(until: Option<u64>) -> Option<std::time::Instant> {
    // Wall-clock seconds left, read again each minute (`runtime`): a suspend stops the monotonic
    // clock, never the end the user saw.
    let left = until?.checked_sub(epoch_now()).filter(|s| *s > 0)?;
    Some(std::time::Instant::now() + std::time::Duration::from_secs(left))
}

/// Do not disturb for `minutes`, or off with none: no sounds and no notifications meanwhile;
/// cards still open the island (ADR 0021). The core and the file change together.
#[tauri::command]
pub fn set_dnd(app: AppHandle, minutes: Option<u32>) -> Result<(), String> {
    let until = minutes
        .filter(|m| *m > 0)
        .map(|m| epoch_now() + u64::from(m.min(24 * 60)) * 60);
    // As `set_presence`: the core and the file change together, or neither does.
    let mut sent = Ok(());
    edit(&app, |s| {
        sent = crate::runtime::set_dnd(&app, until);
        if sent.is_ok() {
            s.dnd_until = until;
        }
    })?;
    sent?;
    tracing::info!(minutes, "do not disturb");
    let _ = app.emit("settings", serde_json::json!({ "dndUntil": until }));
    Ok(())
}

/// Off withdraws what is shown; on weighs what is going on now.
#[tauri::command]
pub fn set_notifications(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.notifications = on)?;
    crate::runtime::recheck(&app);
    Ok(())
}

/// Zeca on or off (ADR 0018). Off: the chat ends and the mic stops, and neither starts again;
/// the tray loses *Chat…*. The flock, cards, notifications and connectors are untouched.
#[tauri::command]
pub fn set_zeca(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.zeca = on)?;
    tracing::info!(on, "Zeca");
    if !on {
        crate::chat::stop(&app);
        crate::voice::cancel(&app);
    }
    crate::tray::refresh_menu(&app);
    crate::runtime::recheck(&app);
    let _ = app.emit("settings", serde_json::json!({ "zeca": on }));
    Ok(())
}

pub fn zeca(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.zeca).unwrap_or(true)
}

/// The saved end of do not disturb, epoch seconds.
pub fn dnd_until(app: &AppHandle) -> Option<u64> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.dnd_until)
}

/// The daily budget, in cents (W9).
pub fn budget_cents(app: &AppHandle) -> Option<u32> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.budget_daily_cents)
}

pub fn budget_told(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.budget_told.clone())
}

pub fn notifications(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.notifications).unwrap_or(true)
}

#[tauri::command]
pub fn set_visitors(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.visitors = on)?;
    let _ = app.emit("settings", serde_json::json!({ "visitors": on }));
    Ok(())
}

/// Zeca's species: the island draws him as it, at once.
#[tauri::command]
pub fn set_zeca_species(app: AppHandle, id: String) -> Result<(), String> {
    edit(&app, |s| s.zeca_species = id.clone())?;
    let _ = app.emit("settings", serde_json::json!({ "zecaSpecies": id }));
    Ok(())
}

#[tauri::command]
pub fn set_now_playing(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.now_playing = on)?;
    crate::media::apply(&app, on);
    Ok(())
}

/// The language as chosen: a code, `auto`, or none to follow the system.
pub fn voice_language(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.voice_language.clone())
}

pub fn voice_model(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.voice_model.clone())
}

/// The language in use: the one chosen, else the system's, else English.
pub fn resolve(chosen: Option<&str>) -> vults_core::i18n::Lang {
    chosen
        .map(vults_core::i18n::Lang::from_code)
        .or_else(|| vults_platform::system_language().map(|l| vults_core::i18n::Lang::from_code(&l)))
        .unwrap_or_default()
}

pub fn lang(app: &AppHandle) -> vults_core::i18n::Lang {
    let state = app.state::<SettingsState>();
    resolve(state.0.lock().ok().and_then(|s| s.language.clone()).as_deref())
}

#[tauri::command]
pub fn set_theme(app: AppHandle, window: tauri::Webview, theme: Theme) -> Result<(), String> {
    crate::settings_page(&window)?;
    edit(&app, |s| s.theme = theme)?;
    let _ = app.emit("settings", serde_json::json!({ "theme": theme }));
    Ok(())
}

/// The language chosen (`en`, `pt-BR`, `es`, `zh`), or none to follow the system. Every surface
/// and the core take it at once.
#[tauri::command]
pub fn set_language(app: AppHandle, language: Option<String>) -> Result<(), String> {
    use vults_core::i18n::Lang;
    let language = language.filter(|l| {
        [Lang::En, Lang::PtBr, Lang::Es, Lang::Zh]
            .iter()
            .any(|k| k.code() == l)
    });
    edit(&app, |s| s.language = language.clone())?;
    let lang = resolve(language.as_deref());
    crate::runtime::set_lang(&app, lang);
    crate::tray::refresh_menu(&app);
    let _ = app.emit(
        "settings",
        serde_json::json!({ "language": language, "lang": lang.code() }),
    );
    Ok(())
}

/// The Monday of the last recap card read.
pub fn recap_shown_week(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.recap_shown_week.clone())
}

/// Whether finished turns go to the local history.
pub fn history(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.history).unwrap_or(true)
}

/// The local history on or off. Off keeps what is there; Clear history removes it.
#[tauri::command]
pub fn set_history(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.history = on)?;
    let _ = app.emit("settings", serde_json::json!({ "history": on }));
    Ok(())
}

/// Is the Codex usage read switched on? The network gate asks before each read.
pub fn codex_usage(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.codex_usage).unwrap_or(false)
}

/// Off also takes Codex's meters off the island: a read that may not happen can't keep them fresh.
#[tauri::command]
pub fn set_codex_usage(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.codex_usage = Some(on))?;
    if !on {
        crate::usage::set(&app, vults_protocol::AgentKind::Codex, Vec::new());
    }
    crate::usage::wake();
    Ok(())
}

/// Are Zeca's web tools on?
pub fn web(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.web).unwrap_or(false)
}

/// The web tools as Settings shows them: whether a key is saved, never the key.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebView {
    on: bool,
    engine: String,
    searxng_url: Option<String>,
    has_key: bool,
}

const WEB_ENGINES: [&str; 3] = ["brave", "tavily", "searxng"];

#[tauri::command]
pub async fn web_settings(app: AppHandle) -> WebView {
    let s = app
        .state::<SettingsState>()
        .0
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    let engine = WEB_ENGINES
        .into_iter()
        .find(|e| *e == s.web_engine)
        .unwrap_or("brave");
    let has_key = engine != "searxng"
        && tauri::async_runtime::spawn_blocking(move || {
            vults_secrets::has(vults_secrets::Secret::WebKey(engine))
        })
        .await
        .unwrap_or(false);
    WebView {
        on: s.web,
        engine: engine.to_string(),
        searxng_url: s.searxng_url,
        has_key,
    }
}

/// Zeca's web on or off; a request on the user's behalf, so only their switch turns it on.
#[tauri::command]
pub fn set_web(window: tauri::Webview, app: AppHandle, on: bool) -> Result<(), String> {
    crate::settings_page(&window)?;
    edit(&app, |s| s.web = on)
}

#[tauri::command]
pub fn set_web_engine(window: tauri::Webview, app: AppHandle, engine: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    let engine = WEB_ENGINES
        .into_iter()
        .find(|e| *e == engine)
        .ok_or("Unknown search engine.")?;
    edit(&app, |s| s.web_engine = engine.to_string())
}

/// The user's own SearXNG: an http or https address, nothing else.
#[tauri::command]
pub fn set_searxng_url(window: tauri::Webview, app: AppHandle, url: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    let url = url.trim().trim_end_matches('/').to_string();
    let ok = (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 300
        && !url.chars().any(char::is_whitespace);
    if !ok {
        return Err("Give an address that starts with https://.".into());
    }
    edit(&app, |s| s.searxng_url = Some(url))
}

/// Saves a search engine's key in the keyring, the only place it is ever written.
#[tauri::command]
pub async fn set_web_key(window: tauri::Webview, engine: String, key: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    let engine = ["brave", "tavily"]
        .into_iter()
        .find(|e| *e == engine)
        .ok_or("Unknown search engine.")?;
    let key = key.trim().to_string();
    if key.is_empty() || key.len() > 200 {
        return Err("Paste the key as the engine gave it.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        vults_secrets::set(vults_secrets::Secret::WebKey(engine), &key)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn clear_web_key(window: tauri::Webview, engine: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    let engine = ["brave", "tavily"]
        .into_iter()
        .find(|e| *e == engine)
        .ok_or("Unknown search engine.")?;
    tauri::async_runtime::spawn_blocking(move || vults_secrets::delete(vults_secrets::Secret::WebKey(engine)))
        .await
        .map_err(|e| e.to_string())?
}

/// Does the chat read the user's own Claude Code and Codex setup?
pub fn native_chat(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.native_chat).unwrap_or(true)
}

/// Takes effect with the next conversation: one that started keeps what it read.
#[tauri::command]
pub fn set_native_chat(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.native_chat = on)?;
    let _ = app.emit("settings", serde_json::json!({ "nativeChat": on }));
    Ok(())
}

fn dictation_wayland() -> bool {
    #[cfg(target_os = "linux")]
    return vults_platform::insert::wayland_available();
    #[cfg(not(target_os = "linux"))]
    false
}

fn dictation_insert_available() -> bool {
    #[cfg(target_os = "linux")]
    return vults_platform::insert::available();
    #[cfg(not(target_os = "linux"))]
    false
}

#[cfg(target_os = "linux")]
pub fn dictation_insert(app: &AppHandle) -> bool {
    app.state::<SettingsState>()
        .0
        .lock()
        .map(|s| s.dictation_insert)
        .unwrap_or(false)
}

fn validate_microphone(device: Option<&str>) -> Result<(), String> {
    if device
        .is_some_and(|name| name.trim().is_empty() || name.len() > 512 || name.chars().any(char::is_control))
    {
        return Err("Invalid microphone name.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn dictation_microphones<R: tauri::Runtime>(
    window: tauri::Webview<R>,
) -> Result<Vec<String>, String> {
    crate::settings_page(&window)?;
    tauri::async_runtime::spawn_blocking(vults_audio::microphones)
        .await
        .map_err(|_| "Could not list microphones.")?
}

#[tauri::command]
pub fn set_dictation_microphone<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    device: Option<String>,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    validate_microphone(device.as_deref())?;
    crate::doctor::dictation::stop_mic(&app);
    let state = app.state::<SettingsState>();
    let mut saved = state.0.lock().map_err(|_| "settings are busy")?;
    let mut next = saved.clone();
    next.dictation_microphone = device.clone();
    save(&next).map_err(|_| "Could not save the dictation microphone.")?;
    *saved = next;
    drop(saved);
    #[cfg(target_os = "linux")]
    crate::questions::invalidate_capture(&app);
    let _ = app.emit("settings", serde_json::json!({ "dictationMicrophone": device }));
    Ok(())
}

#[tauri::command]
pub fn set_dictation_music<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    mode: vults_platform::music::Mode,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let state = app.state::<SettingsState>();
    let mut saved = state.0.lock().map_err(|_| "settings are busy")?;
    let mut next = saved.clone();
    next.dictation_music = mode;
    save(&next).map_err(|_| "Could not save the dictation music choice.")?;
    *saved = next;
    drop(saved);
    let _ = app.emit("settings", serde_json::json!({ "dictationMusic": mode }));
    Ok(())
}

fn change_dictation_processing(settings: &mut Settings, key: &str, on: bool) -> Result<&'static str, String> {
    match key {
        "projectVocabulary" => {
            settings.dictation_project_vocabulary = on;
            Ok("dictationProjectVocabulary")
        }
        "vocabulary" => {
            settings.dictation_vocabulary = on;
            Ok("dictationVocabulary")
        }
        "cutRepeats" => {
            settings.dictation_cut_repeats = on;
            Ok("dictationCutRepeats")
        }
        "retryBase" => {
            settings.dictation_retry_base = on;
            Ok("dictationRetryBase")
        }
        "spokenCode" => {
            settings.dictation_spoken_code = on;
            Ok("dictationSpokenCode")
        }
        "cancelByVoice" => {
            settings.dictation_cancel_by_voice = on;
            Ok("dictationCancelByVoice")
        }
        _ => Err("Unknown dictation processing choice.".into()),
    }
}

#[tauri::command]
pub fn set_dictation_processing<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    key: String,
    on: bool,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let state = app.state::<SettingsState>();
    let mut saved = state.0.lock().map_err(|_| "settings are busy")?;
    let mut next = saved.clone();
    let field = change_dictation_processing(&mut next, &key, on)?;
    save(&next).map_err(|_| "Could not save dictation processing choices.")?;
    *saved = next;
    drop(saved);
    let _ = app.emit("settings", serde_json::json!({ field: on }));
    Ok(())
}

fn change_dictation_gesture(settings: &mut Settings, key: &str, on: bool) -> Result<&'static str, String> {
    match key {
        "doubleTap" => {
            settings.dictation_double_tap = on;
            Ok("dictationDoubleTap")
        }
        "spaceLock" => {
            settings.dictation_space_lock = on;
            Ok("dictationSpaceLock")
        }
        "escapeCancel" => {
            settings.dictation_escape_cancel = on;
            Ok("dictationEscapeCancel")
        }
        _ => Err("Unknown dictation gesture.".into()),
    }
}

#[tauri::command]
pub fn set_dictation_gesture<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    key: String,
    on: bool,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let settings = app.state::<SettingsState>();
    let mut saved = settings.0.lock().map_err(|_| "settings are busy")?;
    let mut next = saved.clone();
    let field = change_dictation_gesture(&mut next, &key, on)?;
    save(&next).map_err(|_| "Could not save dictation gestures.")?;
    #[cfg(target_os = "linux")]
    if let Some(worker) = app.try_state::<crate::dictation::State>() {
        worker.configure(next.dictation_options());
    }
    *saved = next;
    drop(saved);
    let _ = app.emit("settings", serde_json::json!({ field: on }));
    Ok(())
}

fn bind_request_budget(
    expected: &mut Option<String>,
    store: &vults_store::Store,
) -> Result<(), &'static str> {
    let identity = store
        .request_budget_id()
        .map_err(|_| "Could not read model request budget.")?;
    if expected.as_ref().is_some_and(|saved| *saved != identity) {
        return Err("The model request budget is unavailable.");
    }
    *expected = Some(identity);
    Ok(())
}

#[tauri::command]
pub fn set_meeting_summary<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    on: bool,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let state = app.state::<SettingsState>();
    let mut saved = state.0.lock().map_err(|_| "settings are busy")?;
    let mut next = saved.clone();
    if on {
        let store = if saved.meeting_summary_budget.is_some() {
            vults_store::Store::open_current(&crate::history::dir())
        } else {
            vults_store::Store::open(&crate::history::dir())
        }
        .map_err(|_| "Could not read meeting summary budget.")?;
        bind_request_budget(&mut next.meeting_summary_budget, &store)?;
    }
    next.meeting_summary = on;
    save(&next).map_err(|_| "Could not save meeting summary choice.")?;
    *saved = next;
    drop(saved);
    crate::meetings::summary::invalidate(&app);
    let _ = app.emit("settings", serde_json::json!({"meetingSummary":on}));
    Ok(())
}

#[tauri::command]
pub fn set_dictation_cleanup<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    on: bool,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let state = app.state::<SettingsState>();
    let mut saved = state.0.lock().map_err(|_| "settings are busy")?;
    let mut next = saved.clone();
    if on {
        let store = if saved.dictation_cleanup_budget.is_some() {
            vults_store::Store::open_current(&crate::history::dir())
        } else {
            vults_store::Store::open(&crate::history::dir())
        }
        .map_err(|_| "Could not read dictation cleanup budget.")?;
        bind_request_budget(&mut next.dictation_cleanup_budget, &store)?;
        #[cfg(target_os = "linux")]
        if let Some(state) = app.try_state::<crate::dictation::cleanup::State>() {
            state.reload_styles(&store);
        }
    }
    next.dictation_cleanup = on;
    save(&next).map_err(|_| "Could not save dictation cleanup choice.")?;
    #[cfg(target_os = "linux")]
    crate::dictation::cleanup::invalidate(&app);
    *saved = next;
    drop(saved);
    #[cfg(target_os = "linux")]
    crate::dictation::cleanup::ensure_focus(&app);
    let _ = app.emit("settings", serde_json::json!({"dictationCleanup":on}));
    Ok(())
}

#[tauri::command]
pub fn set_dictation_style_options<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    style: vults_dictation::cleanup::Style,
    write_in: Option<vults_dictation::cleanup::Language>,
    sessions_english: bool,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let state = app.state::<SettingsState>();
    let mut saved = state.0.lock().map_err(|_| "settings are busy")?;
    let mut next = saved.clone();
    next.dictation_style = style;
    next.dictation_write_in = write_in;
    next.dictation_sessions_english = sessions_english;
    save(&next).map_err(|_| "Could not save dictation style choices.")?;
    *saved = next;
    drop(saved);
    let _=app.emit("settings",serde_json::json!({"dictationStyle":style,"dictationWriteIn":write_in,"dictationSessionsEnglish":sessions_english}));
    Ok(())
}

#[tauri::command]
pub fn dictation_styles<R: tauri::Runtime>(
    window: tauri::Webview<R>,
) -> Result<BTreeMap<String, String>, String> {
    crate::settings_page(&window)?;
    vults_store::Store::open(&crate::history::dir())
        .and_then(|s| s.dictation_styles())
        .map_err(|_| "Could not read dictation styles.".into())
}

#[tauri::command]
pub fn set_dictation_app_style<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    identity: String,
    style: Option<vults_dictation::cleanup::Style>,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let identity =
        vults_dictation::styles::app_id(&identity).ok_or("Choose a valid application identity.")?;
    let settings = app.state::<SettingsState>();
    let _saved = settings.0.lock().map_err(|_| "settings are busy")?;
    let store =
        vults_store::Store::open(&crate::history::dir()).map_err(|_| "Could not save dictation style.")?;
    store
        .dictation_style_set(&identity, style.map(|s| s.id()))
        .map_err(|_| "Could not save dictation style.")?;
    #[cfg(target_os = "linux")]
    if let Some(state) = app.try_state::<crate::dictation::cleanup::State>() {
        state.reload_styles(&store);
    }
    drop(_saved);
    let _ = app.emit("dictation-styles", ());
    Ok(())
}

/// Both choices are privileged: the fallback authorizes a Chat model request.
#[tauri::command]
pub fn set_dictation_commands<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    key: String,
    on: bool,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let state = app.state::<SettingsState>();
    let mut saved = state.0.lock().map_err(|_| "settings are busy")?;
    let mut next = saved.clone();
    let field = match key.as_str() {
        "enabled" => {
            next.dictation_commands = on;
            "dictationCommands"
        }
        "chat" => {
            next.dictation_command_chat = on;
            "dictationCommandChat"
        }
        _ => return Err("Unknown voice command setting.".into()),
    };
    save(&next).map_err(|_| "Could not save voice command choices.")?;
    #[cfg(target_os = "linux")]
    {
        // Cancel even when re-enabled: no ticket may inherit a later grant.
        if let Some(worker) = app.try_state::<crate::dictation::State>() {
            worker.cancel_commands();
        }
        crate::runtime::cancel_voice_commands(&app);
    }
    *saved = next;
    drop(saved);
    let _ = app.emit("settings", serde_json::json!({ field: on }));
    Ok(())
}

#[tauri::command]
pub fn set_dictation_sound(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.dictation_sound = on)?;
    let _ = app.emit("settings", serde_json::json!({ "dictationSound": on }));
    Ok(())
}

#[tauri::command]
pub fn set_dictation_caption(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.dictation_caption = on)?;
    let _ = app.emit("settings", serde_json::json!({ "dictationCaption": on }));
    Ok(())
}

#[tauri::command]
pub async fn set_dictation_insert(
    app: AppHandle,
    window: tauri::Webview,
    on: bool,
) -> Result<Option<String>, String> {
    if window.label() != crate::SETTINGS {
        return Err("Dictation insertion is changed in Settings.".into());
    }
    if on && !dictation_insert_available() {
        return Err("Dictation insertion requires native X11 or Plasma Wayland.".into());
    }
    #[cfg(target_os = "linux")]
    {
        if !on {
            crate::dictation::permission::revoke(&app);
        }
        crate::dictation::cancel(&app, false);
    }
    edit(&app, |s| s.dictation_insert = on)?;
    let _ = app.emit("settings", serde_json::json!({ "dictationInsert": on }));
    #[cfg(target_os = "linux")]
    return Ok(crate::dictation::permission::configure(app, on, true).await.err());
    #[cfg(not(target_os = "linux"))]
    Ok(None)
}

/// Who answers the chat's permissions now.
pub fn chat_access(app: &AppHandle) -> ChatAccess {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.chat_access).unwrap_or_default()
}

/// The user's choice; it holds at once, for the card the chat asks next. More access is given
/// only in Settings, like a policy (the island and the Nest show agents' text); the chat's own
/// picker only gives some back.
#[tauri::command]
pub fn set_chat_access(app: AppHandle, window: tauri::Webview, access: ChatAccess) -> Result<(), String> {
    let now = chat_access(&app);
    let less = access.rank() <= now.rank() && [crate::ISLAND, crate::NEST].contains(&window.label());
    if window.label() != crate::SETTINGS && !less {
        return Err("More access is given in Settings → Chat & voice.".into());
    }
    edit(&app, |s| s.chat_access = access)?;
    tracing::info!(?access, "chat access changed");
    let _ = app.emit("settings", serde_json::json!({ "chatAccess": access }));
    Ok(())
}

/// Is Zeca's memory on (the journal written)?
pub fn memory(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.memory).unwrap_or(false)
}

/// The day the daily summary was turned on, while it and the memory are both on.
pub fn journal_summary_since(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| {
        (s.memory && s.journal_summary)
            .then(|| s.journal_summary_since.clone())
            .flatten()
    })
}

/// Recall by meaning through a local Ollama: its switch, and only with the memory on.
pub fn local_embeddings(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().is_ok_and(|s| s.memory && s.local_embeddings)
}

/// Suggestions after a conversation: their switch, and only with the memory on.
pub fn memory_suggest(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().is_ok_and(|s| s.memory && s.memory_suggest)
}

/// Suggestions on or off; a request on the user's behalf, so only their switch turns it on.
#[tauri::command]
pub fn set_memory_suggest(window: tauri::Webview, app: AppHandle, on: bool) -> Result<(), String> {
    crate::settings_page(&window)?;
    edit(&app, |s| s.memory_suggest = on)?;
    let _ = app.emit("settings", serde_json::json!({ "memorySuggest": on }));
    Ok(())
}

/// The sleep's model pass: its switch, and only with the memory on.
pub fn sleep(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().is_ok_and(|s| s.memory && s.sleep)
}

/// The sleep's model pass on or off; a request on the user's behalf, so only their switch.
#[tauri::command]
pub fn set_sleep(window: tauri::Webview, app: AppHandle, on: bool) -> Result<(), String> {
    crate::settings_page(&window)?;
    edit(&app, |s| s.sleep = on)?;
    let _ = app.emit("settings", serde_json::json!({ "sleep": on }));
    Ok(())
}

/// Recall by meaning on or off.
#[tauri::command]
pub fn set_local_embeddings(window: tauri::Webview, app: AppHandle, on: bool) -> Result<(), String> {
    crate::settings_page(&window)?;
    edit(&app, |s| s.local_embeddings = on)?;
    let _ = app.emit("settings", serde_json::json!({ "localEmbeddings": on }));
    Ok(())
}

/// Zeca's memory on or off. Off, nothing more is written; what was is kept until Clear history.
#[tauri::command]
pub fn set_memory(window: tauri::Webview, app: AppHandle, on: bool) -> Result<(), String> {
    crate::settings_page(&window)?;
    edit(&app, |s| s.memory = on)?;
    crate::memory::refresh_attention(&app);
    let _ = app.emit("settings", serde_json::json!({ "memory": on }));
    Ok(())
}

/// The daily summary on or off; a request on the user's behalf, so only their switch turns it on.
#[tauri::command]
pub fn set_journal_summary(window: tauri::Webview, app: AppHandle, on: bool) -> Result<(), String> {
    crate::settings_page(&window)?;
    let today = crate::history::today().iso();
    edit(&app, |s| {
        s.journal_summary = on;
        s.journal_summary_since = on.then_some(today);
    })?;
    let _ = app.emit("settings", serde_json::json!({ "journalSummary": on }));
    Ok(())
}

pub fn now_playing(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.now_playing).unwrap_or(false)
}

#[tauri::command]
pub fn set_fold_after(app: AppHandle, seconds: u32) -> Result<(), String> {
    let seconds = seconds.clamp(*FOLD_AFTER.start(), *FOLD_AFTER.end());
    edit(&app, |s| s.fold_after = seconds)?;
    let _ = app.emit("settings", serde_json::json!({ "foldAfter": seconds }));
    Ok(())
}

/// Hovering opens the island: the island takes it at once.
#[tauri::command]
pub fn set_open_on_hover(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.open_on_hover = on)?;
    let _ = app.emit("settings", serde_json::json!({ "openOnHover": on }));
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct Monitor {
    /// What `set_monitor` takes.
    pub name: String,
    pub label: String,
}

/// The connected monitors. GTK is read on its own thread.
#[tauri::command]
pub async fn monitors(app: AppHandle) -> Vec<Monitor> {
    #[cfg(target_os = "linux")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = app.run_on_main_thread(move || {
            let list = vults_platform::linux::monitor_names();
            let _ = tx.send(
                list.into_iter()
                    .map(|(name, label)| Monitor { name, label })
                    .collect(),
            );
        });
        rx.await.unwrap_or_default()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Vec::new()
    }
}

#[tauri::command]
pub fn set_monitor(app: AppHandle, name: Option<String>) -> Result<(), String> {
    edit(&app, |s| s.monitor = name)?;
    crate::place_surfaces(&app);
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn monitor(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.monitor.clone())
}

/// Changes the settings and saves them.
pub fn edit(app: &AppHandle, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
    let state = app.state::<SettingsState>();
    let mut s = state.0.lock().map_err(|_| "settings are busy")?;
    #[cfg(target_os = "linux")]
    let before = (
        s.voice_model.clone(),
        s.voice_language.clone(),
        s.dictation_microphone.clone(),
    );
    change(&mut s);
    #[cfg(target_os = "linux")]
    let changed = before
        != (
            s.voice_model.clone(),
            s.voice_language.clone(),
            s.dictation_microphone.clone(),
        );
    let result = save(&s).map_err(|e| format!("can't save the settings: {e}"));
    drop(s);
    #[cfg(target_os = "linux")]
    if changed {
        crate::questions::invalidate_capture(app);
    }
    result
}

fn path() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(crate::paths::home)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::paths::home().join(".config"))
    };
    base.join(if cfg!(windows) {
        vults_brand::NAME
    } else {
        vults_brand::SLUG
    })
    .join("settings.json")
}

/// The settings as saved, read only: for the MCP server, a second process that must never write
/// or copy the file aside.
pub fn peek() -> Settings {
    std::fs::read_to_string(path())
        .map(|t| read(&t).0)
        .unwrap_or_default()
}

/// A missing file means defaults. A file we can't fully read, or one a newer release wrote, is
/// copied aside first: the next save writes only what this version understood.
pub fn load() -> Settings {
    let path = path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        // A new install: a request on the user's behalf waits for their switch (ADR 0019).
        return Settings {
            codex_usage: Some(false),
            ..Settings::default()
        };
    };
    let (mut settings, aside) = read(&text);
    // Written at once only over a file read in full: any other is copied aside first, below.
    let clean = aside.is_none();
    if upgrade_usage(
        &mut settings,
        clean && before_usage(&text),
        crate::usage::codex_installed,
    ) && clean
    {
        let _ = save(&settings);
    }
    if let Some(label) = aside {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let aside = path.with_extension(format!("json.{label}-{stamp}"));
        match std::fs::copy(&path, &aside) {
            Ok(_) => {
                tracing::warn!(
                    "the settings file ({label}) was not fully understood; it was kept as {}",
                    aside.display()
                );
                settings.version = VERSION;
            }
            // A newer file keeps its version, so `save` refuses to write over it.
            Err(e) => tracing::warn!(
                "the settings file ({label}) was not fully understood, and keeping a copy failed: {e}"
            ),
        }
    }
    settings
}

/// A file from before the usage read's switch (`older`, read in full) decides it once: on when a
/// `codex` is installed, so the island keeps the meters it showed. Any other file without it (a
/// newer one, one we could not read) gets it off: only the user turns a request on (ADR 0019).
/// True when it changed something to save.
fn upgrade_usage(settings: &mut Settings, older: bool, codex_installed: impl FnOnce() -> bool) -> bool {
    if settings.codex_usage.is_some() {
        return false;
    }
    settings.codex_usage = Some(older && codex_installed());
    true
}

/// Was `text` written before version 16, which brought the usage read's switch?
fn before_usage(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v.get("version")?.as_u64())
        .is_some_and(|v| v < 16)
}

/// The settings in `text`, and the label of the copy to keep, if one is needed: `v<N>` for a
/// file from a newer version (a save would drop its new keys), `bad` for one we can't fully read.
/// An older file becomes ours, so the next save tells an older release it has keys to keep.
/// A newer one's version stays as read: only a kept copy lets it become ours.
fn read(text: &str) -> (Settings, Option<String>) {
    let (mut settings, clean) = parse(text);
    let aside = if settings.version > VERSION {
        Some(format!("v{}", settings.version))
    } else {
        settings.version = VERSION;
        (!clean).then(|| "bad".to_string())
    };
    (settings, aside)
}

/// The settings in `text`, and whether all of it was understood. A field that doesn't fit
/// (a wrong type, a value from another version) falls back to its default; the rest is kept.
fn parse(text: &str) -> (Settings, bool) {
    if let Ok(s) = serde_json::from_str(text) {
        return (s, true);
    }
    let Ok(serde_json::Value::Object(fields)) = serde_json::from_str(text) else {
        return (Settings::default(), false);
    };
    let mut kept = serde_json::to_value(Settings::default()).unwrap_or_default();
    for (key, value) in fields {
        let before = kept.get(&key).cloned();
        kept[&key] = value;
        if serde_json::from_value::<Settings>(kept.clone()).is_err() {
            match before {
                Some(v) => kept[&key] = v,
                None => {
                    kept.as_object_mut().map(|m| m.shift_remove(&key));
                }
            }
        }
    }
    (serde_json::from_value(kept).unwrap_or_default(), false)
}

/// Still a newer release's file: no copy of it could be kept, so writing would lose its keys.
fn too_new(settings: &Settings) -> bool {
    settings.version > VERSION
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    if too_new(settings) {
        return Err(std::io::Error::other(
            "a newer release wrote them and no copy could be kept",
        ));
    }
    let path = path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(settings)?;
    let temp = path.with_extension("json.tmp");
    let mut file = std::fs::OpenOptions::new();
    file.write(true).create(true).truncate(true);
    // It names the user's projects and their Always rules: theirs alone.
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut file, 0o600);
    let mut file = file.open(&temp)?;
    std::io::Write::write_all(&mut file, text.as_bytes())?;
    // On disk before the rename, or a power cut can leave an empty file and lose every rule.
    file.sync_all()?;
    std::fs::rename(temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: &str =
        r#"{ "agent": "claude", "cwd": "/home/me/p", "tool": "Bash", "target": "Bash · cargo test" }"#;

    #[test]
    fn external_question_access_defaults_off_and_survives_only_an_explicit_saved_boolean() {
        let (old, clean) = parse(r#"{ "version": 39 }"#);
        assert!(clean && !old.question_agents);
        let (on, clean) = parse(r#"{ "version": 40, "question_agents": true }"#);
        assert!(clean && on.question_agents);
        assert!(parse(&serde_json::to_string(&on).unwrap()).0.question_agents);
        let (bad, clean) = parse(r#"{ "version": 40, "question_agents": "yes" }"#);
        assert!(!clean && !bad.question_agents);
    }
    #[test]
    fn one_bad_field_keeps_the_rest() {
        let text =
            format!(r#"{{ "version": 1, "rules": [{RULE}], "now_playing": "yes", "fold_after": 30 }}"#);
        let (s, clean) = parse(&text);
        assert!(!clean);
        assert_eq!(s.rules.len(), 1, "the always-allow rules survive a bad field");
        assert_eq!(s.fold_after, 30);
        assert!(!s.now_playing, "the bad field falls back to its default");
    }

    #[test]
    fn project_vocabulary_migrates_off_and_has_an_independent_choice() {
        let (mut old, clean) = parse(r#"{ "version": 34, "dictation_vocabulary": true }"#);
        assert!(clean && !old.dictation_project_vocabulary && old.dictation_vocabulary);
        assert_eq!(
            change_dictation_processing(&mut old, "projectVocabulary", true).unwrap(),
            "dictationProjectVocabulary"
        );
        let (loaded, clean) = parse(&serde_json::to_string(&old).unwrap());
        assert!(clean && loaded.dictation_project_vocabulary && loaded.dictation_vocabulary);
        let (invalid, clean) = parse(
            r#"{ "version": 35, "dictation_project_vocabulary": "bad", "dictation_vocabulary": true }"#,
        );
        assert!(!clean && !invalid.dictation_project_vocabulary && invalid.dictation_vocabulary);
    }

    #[test]
    fn vocabulary_is_opt_in_and_round_trips_without_changing_other_processing() {
        let (mut old, clean) = parse(r#"{ "version": 33 }"#);
        assert!(clean && !old.dictation_vocabulary);
        assert_eq!(
            change_dictation_processing(&mut old, "vocabulary", true).unwrap(),
            "dictationVocabulary"
        );
        let (loaded, clean) = parse(&serde_json::to_string(&old).unwrap());
        assert!(clean && loaded.dictation_vocabulary && loaded.dictation_retry_base);
        let (invalid, clean) =
            parse(r#"{ "version": 34, "dictation_vocabulary": "bad", "dictation_retry_base": false }"#);
        assert!(!clean && !invalid.dictation_vocabulary && !invalid.dictation_retry_base);
    }

    #[test]
    fn music_migrates_and_validates_the_saved_choice() {
        use vults_platform::music::Mode;
        let (old, clean) = parse(r#"{ "version": 32 }"#);
        assert!(clean);
        assert_eq!(old.dictation_music, Mode::Lower);
        for mode in [Mode::Lower, Mode::Pause, Mode::Leave] {
            let mut saved = old.clone();
            saved.dictation_music = mode;
            let (loaded, clean) = parse(&serde_json::to_string(&saved).unwrap());
            assert!(clean);
            assert_eq!(loaded.dictation_music, mode);
            assert!(loaded.dictation_cancel_by_voice);
        }
        let (invalid, clean) =
            parse(r#"{ "version": 33, "dictation_music": "play", "dictation_cancel_by_voice": false }"#);
        assert!(!clean && !invalid.dictation_cancel_by_voice);
        assert_eq!(invalid.dictation_music, Mode::Lower);
        assert!(serde_json::from_str::<Mode>(r#""play""#).is_err());
    }

    #[test]
    fn voice_cancel_migrates_and_keeps_the_saved_choice() {
        let (mut old, clean) = parse(r#"{ "version": 31 }"#);
        assert!(clean && old.dictation_cancel_by_voice);
        assert_eq!(
            change_dictation_processing(&mut old, "cancelByVoice", false).unwrap(),
            "dictationCancelByVoice"
        );
        let (loaded, clean) = parse(&serde_json::to_string(&old).unwrap());
        assert!(clean && !loaded.dictation_cancel_by_voice);
        assert!(loaded.dictation_spoken_code && loaded.dictation_escape_cancel);
        let (invalid, clean) = parse(
            r#"{ "version": 32, "dictation_cancel_by_voice": "bad", "dictation_escape_cancel": false }"#,
        );
        assert!(!clean && invalid.dictation_cancel_by_voice && !invalid.dictation_escape_cancel);
    }

    #[test]
    fn spoken_code_defaults_on_and_keeps_an_explicit_choice() {
        let (mut old, clean) = parse(r#"{ "version": 30 }"#);
        assert!(clean && old.dictation_spoken_code);
        assert_eq!(
            change_dictation_processing(&mut old, "spokenCode", false).unwrap(),
            "dictationSpokenCode"
        );
        let (loaded, clean) = parse(&serde_json::to_string(&old).unwrap());
        assert!(clean && !loaded.dictation_spoken_code);
        assert!(loaded.dictation_cut_repeats && loaded.dictation_retry_base);
        let (invalid, clean) =
            parse(r#"{ "version": 31, "dictation_spoken_code": "bad", "dictation_retry_base": false }"#);
        assert!(!clean && invalid.dictation_spoken_code && !invalid.dictation_retry_base);
    }

    #[test]
    fn dictation_processing_migrates_and_keeps_independent_choices() {
        let (mut settings, clean) = parse(r#"{ "version": 29 }"#);
        assert!(clean && settings.dictation_cut_repeats && settings.dictation_retry_base);
        assert_eq!(
            change_dictation_processing(&mut settings, "cutRepeats", false).unwrap(),
            "dictationCutRepeats"
        );
        assert!(settings.dictation_retry_base);
        assert_eq!(
            change_dictation_processing(&mut settings, "retryBase", false).unwrap(),
            "dictationRetryBase"
        );
        let (loaded, clean) = parse(&serde_json::to_string(&settings).unwrap());
        assert!(clean && !loaded.dictation_cut_repeats && !loaded.dictation_retry_base);
        assert!(change_dictation_processing(&mut settings, "unknown", true).is_err());
        assert_eq!(loaded, settings);
        let (invalid, clean) =
            parse(r#"{ "version": 30, "dictation_cut_repeats": "bad", "dictation_retry_base": false }"#);
        assert!(!clean && invalid.dictation_cut_repeats && !invalid.dictation_retry_base);
    }

    #[test]
    fn dictation_microphone_migrates_and_capture_keeps_its_snapshot() {
        let (mut settings, clean) = parse(r#"{ "version": 28 }"#);
        assert!(clean && settings.dictation_microphone.is_none());
        assert_eq!(settings.dictation_capture_options().max_seconds, 600);
        settings.dictation_microphone = Some("USB mic".into());
        let options = settings.dictation_capture_options();
        let (loaded, clean) = parse(&serde_json::to_string(&settings).unwrap());
        assert!(clean);
        assert_eq!(loaded.dictation_microphone.as_deref(), Some("USB mic"));
        settings.dictation_microphone = None;
        assert_eq!(options.device.as_deref(), Some("USB mic"));
        assert!(settings.dictation_capture_options().device.is_none());
        assert!(validate_microphone(None).is_ok());
        assert!(validate_microphone(Some("Disconnected microphone")).is_ok());
        for name in ["", " ", "bad\nname", "bad\0name", &"x".repeat(513)] {
            assert!(validate_microphone(Some(name)).is_err());
        }
        let (invalid, clean) =
            parse(r#"{ "version": 29, "dictation_microphone": false, "dictation_double_tap": false }"#);
        assert!(!clean && invalid.dictation_microphone.is_none());
        assert!(!invalid.dictation_double_tap);
    }

    #[test]
    fn dictation_gestures_migrate_round_trip_and_reject_unknown_keys() {
        let (old, clean) = parse(r#"{ "version": 27 }"#);
        assert!(clean);
        assert_eq!(
            old.dictation_options(),
            vults_dictation::GestureOptions::default()
        );
        let mut changed = old.clone();
        for (key, field) in [
            ("doubleTap", "dictationDoubleTap"),
            ("spaceLock", "dictationSpaceLock"),
            ("escapeCancel", "dictationEscapeCancel"),
        ] {
            assert_eq!(change_dictation_gesture(&mut changed, key, false).unwrap(), field);
        }
        let (loaded, clean) = parse(&serde_json::to_string(&changed).unwrap());
        assert!(clean);
        assert_eq!(
            loaded.dictation_options(),
            vults_dictation::GestureOptions {
                double_tap: false,
                space_lock: false,
                escape_cancel: false
            }
        );
        assert!(change_dictation_gesture(&mut changed, "unknown", true).is_err());
        assert_eq!(changed, loaded);
        let (invalid, clean) =
            parse(r#"{ "version": 28, "dictation_double_tap": "bad", "dictation_space_lock": false }"#);
        assert!(!clean);
        assert!(invalid.dictation_double_tap);
        assert!(!invalid.dictation_space_lock);
    }

    #[test]
    fn dictation_insertion_is_an_explicit_opt_in() {
        let (old, _) = parse(r#"{ "version": 23 }"#);
        assert!(!old.dictation_insert);
        let (enabled, clean) = parse(r#"{ "version": 24, "dictation_insert": true }"#);
        assert!(clean && enabled.dictation_insert);
    }

    #[test]
    fn dictation_retention_defaults_to_thirty_days_and_keeps_not_kept() {
        let (old, _) = parse(r#"{ "version": 26 }"#);
        assert_eq!(old.dictation_keep, "30");
        let (disabled, clean) = parse(r#"{ "version": 27, "dictation_keep": "0" }"#);
        assert!(clean);
        assert_eq!(disabled.dictation_keep, "0");
    }

    #[test]
    fn cleanup_consent_stays_bound_to_the_original_budget_across_disable_and_database_loss() {
        let dir = std::env::temp_dir().join(format!("vults-cleanup-consent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = vults_store::Store::open(&dir).unwrap();
        let mut settings = Settings::default();
        bind_request_budget(&mut settings.dictation_cleanup_budget, &store).unwrap();
        settings.dictation_cleanup = true;
        let (mut loaded, clean) = parse(&serde_json::to_string(&settings).unwrap());
        assert!(clean);
        assert_eq!(loaded.dictation_cleanup_budget, settings.dictation_cleanup_budget);
        loaded.dictation_cleanup = false;
        bind_request_budget(&mut loaded.dictation_cleanup_budget, &store).unwrap();
        drop(store);
        std::fs::remove_dir_all(&dir).unwrap();
        let recreated = vults_store::Store::open(&dir).unwrap();
        assert!(bind_request_budget(&mut loaded.dictation_cleanup_budget, &recreated).is_err());
        assert_eq!(loaded.dictation_cleanup_budget, settings.dictation_cleanup_budget);
        drop(recreated);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn style_defaults_and_bad_values_preserve_cleanup_consent_and_other_preferences() {
        use vults_dictation::cleanup::{Language, Style};
        let (old, clean) = parse(r#"{"version":37}"#);
        assert!(clean);
        assert_eq!(old.dictation_style, Style::Natural);
        assert_eq!(old.dictation_write_in, None);
        assert!(!old.dictation_sessions_english && !old.dictation_cleanup);
        for language in [
            Language::English,
            Language::BrazilianPortuguese,
            Language::Spanish,
            Language::SimplifiedChinese,
        ] {
            for style in [Style::Verbatim, Style::Natural, Style::Polished, Style::Prompt] {
                let saved = Settings {
                    dictation_style: style,
                    dictation_write_in: Some(language),
                    dictation_sessions_english: true,
                    ..Default::default()
                };
                let (loaded, clean) = parse(&serde_json::to_string(&saved).unwrap());
                assert!(clean);
                assert_eq!(loaded, saved);
                assert!(!loaded.dictation_cleanup);
            }
        }
        let (invalid, clean) = parse(
            r#"{"version":38,"dictation_style":"assistant","dictation_write_in":"guess","dictation_sessions_english":"yes","dictation_insert":true}"#,
        );
        assert!(!clean);
        assert_eq!(invalid.dictation_style, Style::Natural);
        assert_eq!(invalid.dictation_write_in, None);
        assert!(!invalid.dictation_sessions_english && !invalid.dictation_cleanup);
        assert!(invalid.dictation_insert);
    }

    #[test]
    fn meeting_summary_consent_defaults_off_and_cannot_rebind_a_replaced_ledger() {
        let (mut settings, clean) = parse(r#"{"version":38,"dictation_cleanup":true}"#);
        assert!(clean && !settings.meeting_summary && settings.meeting_summary_budget.is_none());
        let dir = std::env::temp_dir().join(format!("vults-summary-consent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = vults_store::Store::open(&dir).unwrap();
        bind_request_budget(&mut settings.meeting_summary_budget, &store).unwrap();
        settings.meeting_summary = true;
        let (mut loaded, clean) = parse(&serde_json::to_string(&settings).unwrap());
        assert!(clean && loaded.meeting_summary);
        loaded.meeting_summary = false;
        bind_request_budget(&mut loaded.meeting_summary_budget, &store).unwrap();
        assert_eq!(loaded.meeting_summary_budget, settings.meeting_summary_budget);
        drop(store);
        std::fs::remove_dir_all(&dir).unwrap();
        let recreated = vults_store::Store::open(&dir).unwrap();
        assert!(bind_request_budget(&mut loaded.meeting_summary_budget, &recreated).is_err());
        assert_eq!(loaded.meeting_summary_budget, settings.meeting_summary_budget);
        let (invalid, clean) = parse(r#"{"version":39,"meeting_summary":"yes"}"#);
        assert!(!clean && !invalid.meeting_summary);
        drop(recreated);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn dictation_cleanup_is_a_separate_explicit_opt_in() {
        let (old, clean) = parse(r#"{"version":36,"zeca":true,"dictation_commands":true}"#);
        assert!(clean && !old.dictation_cleanup);
        let (on, clean) = parse(r#"{"version":37,"dictation_cleanup":true}"#);
        assert!(clean && on.dictation_cleanup);
        let (bad, clean) = parse(r#"{"version":37,"dictation_cleanup":"yes"}"#);
        assert!(!clean && !bad.dictation_cleanup);
    }

    #[test]
    fn command_fallback_requires_an_explicit_valid_choice_and_zeca() {
        let (mut old, clean) = parse(r#"{"version":35}"#);
        assert!(clean && old.dictation_commands && !old.dictation_command_chat);
        assert!(!old.command_chat());
        old.dictation_command_chat = true;
        assert!(old.command_chat());
        old.zeca = false;
        assert!(!old.command_chat());
        old.zeca = true;
        old.dictation_commands = false;
        assert!(!old.command_chat());
        let (saved, clean) =
            parse(r#"{"version":36,"dictation_commands":false,"dictation_command_chat":true}"#);
        assert!(clean && !saved.dictation_commands && saved.dictation_command_chat && !saved.command_chat());
        let (invalid, clean) = parse(r#"{"version":36,"dictation_command_chat":"true"}"#);
        assert!(!clean && !invalid.dictation_command_chat && !invalid.command_chat());
    }

    #[test]
    fn dictation_sound_defaults_on_and_keeps_an_explicit_choice() {
        let (old, _) = parse(r#"{ "version": 25 }"#);
        assert!(old.dictation_sound);
        let (disabled, clean) = parse(r#"{ "version": 26, "dictation_sound": false }"#);
        assert!(clean && !disabled.dictation_sound);
    }

    #[test]
    fn dictation_caption_defaults_on_and_keeps_an_explicit_choice() {
        let (old, _) = parse(r#"{ "version": 24 }"#);
        assert!(old.dictation_caption);
        let (disabled, clean) = parse(r#"{ "version": 25, "dictation_caption": false }"#);
        assert!(clean && !disabled.dictation_caption);
    }

    #[test]
    fn open_on_hover_is_off_until_turned_on() {
        let (s, _) = parse(r#"{ "version": 10, "fold_after": 30 }"#);
        assert!(!s.open_on_hover, "a version 10 file has no key: off");
        let (s, clean) = parse(r#"{ "version": 11, "open_on_hover": true }"#);
        assert!(clean && s.open_on_hover);
    }

    #[test]
    fn the_file_0_1_0_writes_loads_with_every_field() {
        use vults_core::{flock::Flock, looks::Outfit};
        let (s, aside) = read(include_str!("../tests/fixtures/settings-0.1.0.json"));
        assert_eq!(aside, None);
        // A struct literal: a new field must be added here, and the old file gives its default.
        // The file becomes this version's, so a save marks the keys an older release would drop.
        let expected = Settings {
            version: VERSION,
            connectors: BTreeMap::from([("github".into(), true)]),
            sounds: false,
            volume: 50,
            fold_after: 30,
            open_on_hover: false,
            rules: vec![serde_json::from_str(RULE).unwrap()],
            budget_daily_cents: None,
            budget_told: None,
            monitor: Some("Samsung Electric Company LS27AG32x".into()),
            api_provider: "openrouter".into(),
            api_models: BTreeMap::from([("openrouter".into(), "anthropic/claude-opus-5.5".into())]),
            now_playing: true,
            voice_model: Some("small".into()),
            voice_language: Some("pt".into()),
            question_agents: false,
            dictation_insert: false,
            dictation_caption: true,
            dictation_sound: true,
            dictation_commands: true,
            dictation_cleanup: false,
            dictation_cleanup_budget: None,
            meeting_summary: false,
            meeting_summary_budget: None,
            dictation_style: natural_style(),
            dictation_write_in: None,
            dictation_sessions_english: false,
            dictation_command_chat: false,
            dictation_microphone: None,
            dictation_cut_repeats: true,
            dictation_retry_base: true,
            dictation_vocabulary: false,
            dictation_project_vocabulary: false,
            dictation_spoken_code: true,
            dictation_cancel_by_voice: true,
            dictation_music: Default::default(),
            dictation_double_tap: true,
            dictation_space_lock: true,
            dictation_escape_cancel: true,
            dictation_keep: dictation_keep(),
            zeca_species: "papa".into(),
            zeca_look: Outfit::Sunglasses,
            flock: Flock::World,
            visitors: false,
            presence: crate::panel::Presence::Island,
            notifications: true,
            zeca: true,
            widget: None,
            projects: BTreeMap::new(),
            dnd_until: None,
            history: true,
            recap_shown_week: None,
            language: None,
            theme: Theme::Black,
            codex_usage: None,
            native_chat: true,
            chat_access: ChatAccess::Ask,
            chat_picks: BTreeMap::new(),
            nest_size: None,
            web: false,
            memory: false,
            journal_summary: false,
            journal_summary_since: None,
            local_embeddings: false,
            memory_suggest: false,
            sleep: false,
            web_engine: web_engine(),
            searxng_url: None,
            phone: false,
            phone_transport: phone_transport(),
            phone_relay: None,
            phone_listen: None,
        };
        assert_eq!(s, expected);
    }

    #[test]
    fn an_upgrade_keeps_codex_usage_only_where_codex_is_installed() {
        let old = include_str!("../tests/fixtures/settings-0.1.0.json");
        assert!(before_usage(old));
        let (mut s, _) = read(old);
        assert!(upgrade_usage(&mut s, true, || true));
        assert_eq!(s.codex_usage, Some(true));
        let (mut s, _) = read(old);
        assert!(upgrade_usage(&mut s, true, || false));
        assert_eq!(s.codex_usage, Some(false));
        // Decided once: the user's choice is never decided again, even with Codex installed later.
        assert!(!upgrade_usage(&mut s, true, || panic!("not asked again")));
        assert_eq!(s.codex_usage, Some(false));
        let (s, _) = read(&serde_json::to_string(&s).unwrap());
        assert_eq!(s.codex_usage, Some(false), "the choice is written to the file");
    }

    /// A file we could not read, or a version 16 one without the key, never turns it on.
    #[test]
    fn only_an_older_file_read_in_full_gets_codex_usage_on() {
        for text in [
            "not json",
            "[1]",
            r#"{ "version": 16 }"#,
            r#"{ "version": 3, "codex_usage": "yes" }"#,
        ] {
            let (mut s, aside) = read(text);
            let older = aside.is_none() && before_usage(text);
            upgrade_usage(&mut s, older, || true);
            assert_eq!(s.codex_usage, Some(false), "{text}");
        }
    }

    #[test]
    fn a_version_4_file_in_panel_mode_keeps_it_as_the_panel_preset() {
        // What version 4 wrote, beside 0.1.0's keys: the place (now a preset) and notifications.
        let mut file: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/settings-0.1.0.json")).unwrap();
        file["version"] = 4.into();
        file["presence"] = "panel".into();
        file["notifications"] = false.into();
        let (s, aside) = read(&file.to_string());
        assert_eq!(aside, None, "nothing to keep aside: every key is understood");
        assert_eq!(s.version, VERSION);
        assert_eq!(s.presence, crate::panel::Presence::Panel);
        assert!(!s.notifications);
        assert_eq!(
            (s.zeca_look, s.fold_after),
            (vults_core::looks::Outfit::Sunglasses, 30)
        );
    }

    #[test]
    fn a_newer_file_is_read_as_far_as_understood_and_kept_aside() {
        let (s, aside) = read(r#"{ "version": 90, "sounds": false, "from_the_future": [1] }"#);
        assert_eq!(
            aside.as_deref(),
            Some("v90"),
            "a save would drop its new keys: keep a copy"
        );
        assert!(!s.sounds);
        // Until `load` keeps the copy, the version stays newer and `save` refuses to write.
        assert_eq!(s.version, 90);
        assert!(too_new(&s));
        // A newer file with a field we can't read is still labelled by its version.
        let (s, aside) = read(r#"{ "version": 100, "sounds": false, "fold_after": "soon" }"#);
        assert_eq!(
            (aside.as_deref(), s.sounds, s.fold_after),
            (Some("v100"), false, 15)
        );
        let (_, aside) = read(r#"{ "version": 1, "fold_after": "soon" }"#);
        assert_eq!(aside.as_deref(), Some("bad"));
    }

    #[test]
    fn a_good_file_is_read_whole_and_garbage_is_defaults() {
        let (s, clean) = parse(&format!(
            r#"{{ "version": 1, "rules": [{RULE}], "now_playing": true }}"#
        ));
        assert!(clean);
        assert!(s.now_playing && s.rules.len() == 1);
        let (s, clean) = parse("{ not json");
        assert!(!clean);
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn the_volume_is_read_and_defaults_to_how_loud_0_1_0_played() {
        let (s, _) = parse(r#"{ "version": 2, "volume": 80 }"#);
        assert_eq!(s.volume, 80);
        let (s, clean) = parse(r#"{ "version": 2, "volume": 300 }"#);
        assert_eq!((clean, s.volume), (false, 50), "out of a byte: the default");
        // Over 100 is still read; the app and `set_volume` play it at 100.
        let (s, clean) = parse(r#"{ "version": 2, "volume": 200 }"#);
        assert_eq!((clean, s.volume.min(VOLUME_MAX)), (true, 100));
        assert_eq!(Settings::default().volume, 50);
    }

    #[test]
    fn the_flock_and_zecas_species_are_read_and_default() {
        use vults_core::looks::Outfit;
        let (s, clean) = parse(r#"{ "version": 1, "zeca_species": "papa", "flock": "world" }"#);
        assert!(clean);
        assert_eq!(
            (s.zeca_species.as_str(), s.flock),
            ("papa", vults_core::flock::Flock::World)
        );
        let (s, _) = parse(r#"{ "version": 1, "flock": "mars" }"#);
        assert_eq!(
            (s.zeca_species.as_str(), s.flock),
            ("atratus", vults_core::flock::Flock::Brazil)
        );
        assert!(s.visitors, "rare visitors are on until the user turns them off");
        let (s, _) = parse(r#"{ "version": 1, "visitors": false }"#);
        assert!(!s.visitors);
        assert_eq!(s.zeca_look, Outfit::Auto, "the calendar's look by default");
        let (s, _) = parse(r#"{ "version": 1, "zeca_look": "witch-hat" }"#);
        assert_eq!(s.zeca_look, Outfit::WitchHat);
        // A look this version does not draw falls back to the calendar, and only that field does.
        let (s, _) = parse(r#"{ "version": 1, "zeca_look": "top-hat", "visitors": false }"#);
        assert_eq!((s.zeca_look, s.visitors), (Outfit::Auto, false));
    }

    #[test]
    fn the_preset_is_read_and_defaults_to_the_island() {
        use crate::panel::Presence;
        assert_eq!(Settings::default().presence, Presence::Island);
        // Version 4's two places are the first two presets, under the same key.
        for (word, presence) in [("island", Presence::Island), ("panel", Presence::Panel)] {
            let (s, aside) = read(&format!(r#"{{ "version": 4, "presence": "{word}" }}"#));
            assert_eq!((s.presence, s.version, aside), (presence, VERSION, None));
        }
        for (word, presence) in [("quiet", Presence::Quiet), ("paused", Presence::Paused)] {
            let (s, clean) = parse(&format!(r#"{{ "version": 5, "presence": "{word}" }}"#));
            assert!(clean);
            assert_eq!(s.presence, presence);
        }
        // A preset this version does not know falls back to the island, and only that field does.
        let (s, _) = parse(r#"{ "version": 5, "presence": "nest", "visitors": false }"#);
        assert_eq!((s.presence, s.visitors), (Presence::Island, false));
    }

    #[test]
    fn chat_picks_round_trip_and_stay_out_when_empty() {
        let text = r#"{ "version": 16, "chat_picks": { "/w": { "claude": { "model": "sonnet", "effort": "high" } } } }"#;
        let (s, clean) = parse(text);
        assert!(clean);
        let pick = &s.chat_picks["/w"]["claude"];
        assert_eq!(
            (pick.model.as_deref(), pick.effort.as_deref()),
            (Some("sonnet"), Some("high"))
        );
        let back = serde_json::to_value(&s).unwrap();
        assert_eq!(back["chat_picks"]["/w"]["claude"]["effort"], "high");
        assert!(
            serde_json::to_value(Settings::default())
                .unwrap()
                .get("chat_picks")
                .is_none()
        );
    }

    #[test]
    fn the_native_chat_is_on_until_turned_off() {
        assert!(Settings::default().native_chat);
        let (s, aside) = read(r#"{ "version": 16 }"#);
        assert!(
            s.native_chat && aside.is_none(),
            "a file from before the switch has it"
        );
        let (s, clean) = parse(r#"{ "version": 16, "native_chat": false }"#);
        assert!(clean && !s.native_chat);
    }

    #[test]
    fn theme_defaults_to_black_and_keeps_a_light_choice() {
        assert_eq!(Settings::default().theme, Theme::Black);
        let (light, clean) = parse(r#"{ "version": 23, "theme": "light" }"#);
        assert!(clean && light.theme == Theme::Light);
        let encoded = serde_json::to_string(&light).unwrap();
        assert_eq!(parse(&encoded).0.theme, Theme::Light);
        assert_eq!(
            parse(r#"{ "version": 23, "theme": "unknown" }"#).0.theme,
            Theme::Black
        );
    }

    #[test]
    fn the_chat_asks_until_the_user_chooses_otherwise() {
        assert_eq!(Settings::default().chat_access, ChatAccess::Ask);
        let (s, aside) = read(r#"{ "version": 21 }"#);
        assert!(s.chat_access == ChatAccess::Ask && aside.is_none());
        let (s, clean) = parse(r#"{ "version": 22, "chat_access": "full" }"#);
        assert!(clean && s.chat_access == ChatAccess::Full);
        // A value from another version reads as asking, never as more access.
        let (s, clean) = parse(r#"{ "version": 22, "chat_access": "everything" }"#);
        assert!(!clean && s.chat_access == ChatAccess::Ask);
    }

    #[test]
    fn zeca_is_on_until_turned_off() {
        assert!(Settings::default().zeca);
        let (s, aside) = read(r#"{ "version": 5, "presence": "quiet" }"#);
        assert!(s.zeca && aside.is_none(), "a file from before the switch has him");
        let (s, clean) = parse(r#"{ "version": 6, "zeca": false }"#);
        assert!(clean && !s.zeca);
    }

    #[test]
    fn the_widget_is_off_until_a_corner_is_picked() {
        use crate::widget::Corner;
        assert_eq!(Settings::default().widget, None);
        let (s, aside) = read(r#"{ "version": 6, "zeca": false }"#);
        assert!(
            s.widget.is_none() && aside.is_none(),
            "a file from before it has none"
        );
        let (s, clean) = parse(r#"{ "version": 7, "widget": "top-left" }"#);
        assert!(clean);
        assert_eq!(s.widget, Some(Corner::TopLeft));
        // A corner this version does not know: no widget, and only that field falls back.
        let (s, _) = parse(r#"{ "version": 7, "widget": "middle", "visitors": false }"#);
        assert_eq!((s.widget, s.visitors), (None, false));
    }

    #[test]
    fn project_prefs_are_read_by_folder_and_a_version_7_file_has_none() {
        let (s, aside) = read(r#"{ "version": 7, "widget": "top-left" }"#);
        assert!(s.projects.is_empty() && aside.is_none());
        let (s, clean) = parse(
            r#"{ "version": 8, "projects": { "/home/me/site": { "pin": true }, "/home/me/x": { "mute": true, "hide": true } } }"#,
        );
        assert!(clean);
        assert!(s.projects["/home/me/site"].pin);
        let x = &s.projects["/home/me/x"];
        assert!(x.mute && x.hide && !x.pin && x.species.is_none());
        // Only the choices that are on are written.
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""/home/me/site":{"pin":true}"#), "{text}");
        assert!(
            !serde_json::to_string(&Settings::default())
                .unwrap()
                .contains("projects")
        );
    }

    #[test]
    fn a_projects_bird_is_read_and_written_only_when_chosen() {
        let (s, clean) = parse(
            r#"{ "version": 12, "projects": { "/home/me/site": { "species": "vultur" }, "/home/me/x": { "pin": true } } }"#,
        );
        assert!(clean);
        assert_eq!(s.projects["/home/me/site"].species.as_deref(), Some("vultur"));
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""/home/me/site":{"species":"vultur"}"#), "{text}");
        assert!(text.contains(r#""/home/me/x":{"pin":true}"#), "{text}");
    }

    #[test]
    fn the_language_is_the_choice_else_the_systems() {
        use vults_core::i18n::Lang;
        assert_eq!(resolve(Some("es")), Lang::Es);
        assert_eq!(resolve(Some("pt-BR")), Lang::PtBr);
        let (s, clean) = parse(r#"{ "version": 15, "language": "zh" }"#);
        assert!(clean && s.language.as_deref() == Some("zh"));
        let (s, _) = parse(r#"{ "version": 14 }"#);
        assert!(s.language.is_none(), "a file from before follows the system");
    }

    #[test]
    fn the_history_is_on_until_turned_off() {
        let (s, _) = parse(r#"{ "version": 12 }"#);
        assert!(s.history, "a file from before it keeps a history");
        let (s, clean) = parse(r#"{ "version": 13, "history": false }"#);
        assert!(clean && !s.history);
    }

    #[test]
    fn do_not_disturb_is_read_and_a_past_one_is_off() {
        let (s, aside) = read(r#"{ "version": 8, "widget": "top-left" }"#);
        assert!(
            s.dnd_until.is_none() && aside.is_none(),
            "a file from before it has none"
        );
        let later = epoch_now() + 600;
        let (s, clean) = parse(&format!(r#"{{ "version": 9, "dnd_until": {later} }}"#));
        assert!(clean);
        assert_eq!(s.dnd_until, Some(later));
        assert!(dnd_instant(s.dnd_until).is_some());
        assert!(dnd_instant(Some(epoch_now() - 1)).is_none(), "past: off");
        assert!(
            !serde_json::to_string(&Settings::default())
                .unwrap()
                .contains("dnd")
        );
    }

    #[test]
    fn the_speech_keys_of_0_1_5_are_dropped_quietly() {
        let (s, aside) = read(
            r#"{ "version": 10, "speak": true, "speak_voices": { "pt": "pf_dora" }, "visitors": false }"#,
        );
        assert!(aside.is_none(), "no copy kept for keys we no longer have");
        assert!(!s.visitors);
    }

    #[test]
    fn notifications_are_on_until_turned_off() {
        assert!(Settings::default().notifications);
        let (s, _) = parse(r#"{ "version": 3 }"#);
        assert!(s.notifications, "a file from before them gets them");
        let (s, clean) = parse(r#"{ "version": 4, "notifications": false }"#);
        assert!(clean && !s.notifications);
    }
}
