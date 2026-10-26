//! Chat from the island, and the files dropped on it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use vults_core::audit::{Act, Actor, Audit};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, mpsc, oneshot};
use vults_chat::providers::{self, Provider as ApiProvider};
use vults_chat::{Approver, Chat, Delta, Provider, Turn};
use vults_secrets as secrets;
use vults_store::conversations::{self, ConversationId};
use vults_store::profiles::Snapshot;

use crate::settings::{self, SettingsState};
use crate::{ISLAND, paths};

/// Dropped files bigger than this are refused: the model would not read them whole anyway.
const MAX_FILE: u64 = 20 * 1024 * 1024;
const MAX_ATTACHMENTS: usize = 16;
/// Inbox copies older than this are deleted at start-up.
const INBOX_KEEP: Duration = Duration::from_secs(7 * 24 * 3600);

/// One conversation at a time; a turn holds the lock until it ends. Permissions the turn is
/// waiting on live outside that lock, so a click can answer them meanwhile.
#[derive(Debug, Clone)]
pub struct ChatState(Arc<ChatInner>);

#[derive(Debug)]
pub struct ChatInner {
    chat: Mutex<Chat>,
    waiting: Arc<Waiting>,
    /// Stops the turn running now, if one is.
    stop: std::sync::Mutex<Option<oneshot::Sender<()>>>,
    /// The saved conversation the chat is in; none until its first message.
    current: std::sync::Mutex<Option<ConversationId>>,
    generation: std::sync::atomic::AtomicU32,
    /// Pinned recipient, changed under the conversation lock alongside the runtime.
    profile: std::sync::Mutex<Option<Snapshot>>,
    /// An unsent conversation may override the profile defaults without changing its revision.
    specialist_choice: std::sync::Mutex<Option<vults_store::profiles::ExecutionDefaults>>,
    /// What a window opening now must show, read without waiting on a turn (which holds `chat`).
    live: std::sync::Mutex<Live>,
    /// Unsent composer state, shared by the island and disposable Nest, never saved to disk.
    draft: std::sync::Mutex<Draft>,
}

impl std::ops::Deref for ChatState {
    type Target = ChatInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default)]
pub struct SpecialistChatState(std::sync::Mutex<HashMap<String, ChatState>>);

impl SpecialistChatState {
    fn runtime(&self, profile: &str, snapshot: &Snapshot) -> Result<ChatState, String> {
        let mut runtimes = self.0.lock().map_err(|_| "Specialist Chat is busy.")?;
        if let Some(state) = runtimes.get(profile) {
            return Ok(state.clone());
        }
        let state = ChatState::specialist(snapshot)?;
        runtimes.insert(profile.to_owned(), state.clone());
        Ok(state)
    }

    fn existing(&self, profile: &str) -> Result<Option<ChatState>, String> {
        self.0
            .lock()
            .map(|runtimes| runtimes.get(profile).cloned())
            .map_err(|_| "Specialist Chat is busy.".into())
    }

    fn pending_count(&self) -> usize {
        self.0
            .lock()
            .map(|runtimes| runtimes.values().map(|state| state.waiting.pending_count()).sum())
            .unwrap_or_default()
    }
}

async fn specialist_snapshot(profile: &str) -> Result<Snapshot, String> {
    if let Some(preset) = vults_zeca::profiles::PRESETS.iter().find(|p| p.id == profile) {
        return Ok(Snapshot {
            source: vults_store::profiles::Source::Preset {
                id: preset.id.into(),
                revision: preset.revision,
            },
            title: preset.title.into(),
            about: preset.about.into(),
            instructions: preset.instructions.into(),
            species: preset.species.into(),
            look: "none".into(),
            skills: Vec::new(),
        });
    }
    let id = profile
        .strip_prefix("custom-")
        .and_then(|id| id.parse::<i64>().ok())
        .filter(|id| *id > 0 && format!("custom-{id}") == profile)
        .ok_or("That profile is unavailable.")?;
    store(move |s| s.agent_profiles())
        .await?
        .into_iter()
        .find(|p| p.id == id)
        .map(Snapshot::from)
        .ok_or_else(|| "That profile is unavailable.".into())
}

fn specialist_route(window: &tauri::Webview) -> Result<(), String> {
    if window.label() == crate::NEST {
        Ok(())
    } else {
        Err("Specialist Chat is available in the Nest only.".into())
    }
}

async fn specialist_runtime(app: &AppHandle, profile: &str) -> Result<ChatState, String> {
    let snapshot = specialist_snapshot(profile).await?;
    app.state::<SpecialistChatState>().runtime(profile, &snapshot)
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Draft {
    text: String,
    files: Vec<String>,
    folder: Option<String>,
    provider: Option<Provider>,
    #[serde(default)]
    mentions: Vec<crate::mentions::Mention>,
}

impl Draft {
    #[cfg(target_os = "linux")]
    fn has_content(&self) -> bool {
        !self.text.trim().is_empty() || !self.files.is_empty() || !self.mentions.is_empty()
    }

    fn validate(&self) -> Result<(), String> {
        if self.text.len() > 1024 * 1024
            || self.files.len() > 256
            || self.files.iter().any(|f| f.len() > 4096)
            || self.folder.as_ref().is_some_and(|f| f.len() > 4096)
        {
            return Err("Chat draft is too large.".into());
        }
        crate::mentions::validate(&self.mentions)
    }
}

#[tauri::command]
pub fn chat_draft(state: tauri::State<'_, ChatState>) -> Result<DraftView, String> {
    let draft = state.draft.lock().map_err(|e| e.to_string())?;
    Ok(DraftView {
        draft: draft.clone(),
        generation: state.generation(),
    })
}

#[derive(serde::Serialize)]
pub struct DraftView {
    #[serde(flatten)]
    draft: Draft,
    generation: u32,
}

#[tauri::command]
pub fn chat_draft_set(
    window: tauri::Webview,
    state: tauri::State<'_, ChatState>,
    draft: Draft,
    generation: u32,
) -> Result<(), String> {
    if !crate::runtime::card_host(&window) {
        return Err("Chat draft belongs to the island or Nest.".into());
    }
    draft.validate()?;
    {
        let mut saved = state.draft.lock().map_err(|e| e.to_string())?;
        state.check_generation(generation)?;
        *saved = draft.clone();
    }
    sync(
        window.app_handle(),
        Sync::Draft {
            generation,
            from: window.label().into(),
            draft,
        },
    );
    Ok(())
}

/// Preparing a question never resets a conversation or grants permission to send it.
#[tauri::command]
pub fn chat_prepare_zeca(
    app: AppHandle,
    window: tauri::Webview,
    state: tauri::State<'_, ChatState>,
    generation: u32,
) -> Result<(), String> {
    if !crate::runtime::card_host(&window) {
        return Err("Chat draft belongs to the island or Nest.".into());
    }
    if !crate::settings::zeca(&app) {
        return Err("Zeca is off (Settings → Flock & Zeca).".into());
    }
    state.check_companion(generation)
}

/// The chat as it is this moment.
#[derive(Debug, Default)]
struct Live {
    /// A turn runs.
    busy: bool,
    tools: bool,
    activity: vults_chat::activity::Activity,
    permissions: HashMap<String, vults_chat::Detail>,
    started: bool,
    /// The turn's message and what Zeca has said in it so far: not saved until the turn ends.
    sent: Option<(String, Vec<String>)>,
    text: String,
    /// What the turn's cards say the conversation read from outside (Z8), for cards rebuilt there.
    outside: Option<String>,
}

/// A card the chat waits on: its answer's channel, and what it asks (for the audit log).
#[derive(Debug)]
struct Asked {
    answer: oneshot::Sender<vults_chat::Answer>,
    tool: String,
    target: String,
    /// When the card came up (Unix ms), and the turn it came in, for the tool's trace span.
    since_ms: i64,
    turn: Option<Arc<crate::traces::Turn>>,
}

/// The cards waiting, and the turn running now: a card keeps the turn it came in, so a late click
/// never files it under the next one. Then where the turn works, for what the user chose to
/// answer before a card (ADR 0004).
#[derive(Debug, Default)]
struct Waiting(
    std::sync::Mutex<HashMap<String, Asked>>,
    std::sync::Mutex<Option<Arc<crate::traces::Turn>>>,
    std::sync::Mutex<Option<Scope>>,
);

/// Where the turn running now works, and which agent the chat's CLI is.
#[derive(Debug)]
struct Scope {
    app: AppHandle,
    folder: String,
    agent: vults_core::AgentKind,
    profile_access: Option<settings::ChatAccess>,
    profile_key: Option<String>,
}

impl Waiting {
    fn subject(&self) -> String {
        self.2
            .lock()
            .ok()
            .and_then(|scope| scope.as_ref().and_then(|scope| scope.profile_key.clone()))
            .unwrap_or_else(|| "zeca".into())
    }
    fn pending_count(&self) -> usize {
        self.0
            .lock()
            .map(|m| m.values().filter(|a| !a.answer.is_closed()).count())
            .unwrap_or_default()
    }

    fn shown(&self) -> Vec<WaitingView> {
        self.0
            .lock()
            .map(|m| {
                m.iter()
                    .filter(|(_, a)| !a.answer.is_closed())
                    .map(|(id, a)| WaitingView {
                        id: id.clone(),
                        tool: a.tool.clone(),
                        target: a.target.clone(),
                        detail: Default::default(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every card still waiting is closed unanswered (a no, for the CLI); each gets its span.
    fn close_all(&self) {
        let closed: Vec<Asked> = self
            .0
            .lock()
            .map(|mut m| m.drain().map(|(_, a)| a).collect())
            .unwrap_or_default();
        for a in closed {
            if let Some(turn) = &a.turn {
                crate::traces::tool(turn, &a.tool, &a.target, a.since_ms, "stopped");
            }
        }
    }
}

impl Approver for Waiting {
    fn wait(&self, id: &str, tool: &str, target: &str) -> oneshot::Receiver<vults_chat::Answer> {
        let (answer, rx) = oneshot::channel();
        if let Ok(mut map) = self.0.lock() {
            let asked = Asked {
                answer,
                tool: tool.to_owned(),
                target: target.to_owned(),
                since_ms: crate::traces::now_ms(),
                turn: self.1.lock().ok().and_then(|t| t.clone()),
            };
            map.insert(id.to_string(), asked);
        }
        rx
    }

    /// The user's rule for this conversation answered: on the record as the rule's, with its span.
    fn ruled(&self, tool: &str, target: &str) {
        crate::audit::keep(Audit::new(
            Actor::Rule,
            Act::Allow,
            &self.subject(),
            "",
            tool,
            target,
        ));
        if let Some(turn) = self.1.lock().ok().and_then(|t| t.clone()) {
            let now = crate::traces::now_ms();
            crate::traces::tool(&turn, tool, target, now, "ok");
        }
    }

    /// The user's choice for the chat, read now: a change in Settings holds for the next call.
    fn settle(&self, ask: &vults_agents::driver::Ask, outside: bool) -> Option<vults_chat::Settled> {
        let scope = self.2.lock().ok()?;
        let scope = scope.as_ref()?;
        let access = settings::chat_access(&scope.app);
        let profile_access = scope
            .profile_key
            .as_deref()
            .map(|key| settings::specialist_chat_access(&scope.app, key))
            .or(scope.profile_access);
        settle_in_route(
            profile_access,
            &crate::policies::active(),
            access,
            &scope.folder,
            scope.agent,
            ask,
            outside,
        )
    }

    fn settled(&self, tool: &str, target: &str, s: vults_chat::Settled) {
        let (actor, act) = match (s.by, s.answer.allows()) {
            (vults_chat::SettledBy::Policy, true) => (Actor::Policy, Act::Allow),
            (vults_chat::SettledBy::Policy, false) => (Actor::Policy, Act::Deny),
            (vults_chat::SettledBy::FullAccess, allow) => {
                (Actor::FullAccess, if allow { Act::Allow } else { Act::Deny })
            }
        };
        let audit = Audit::new(actor, act, &self.subject(), "", tool, target);
        crate::audit::keep(match s.consent {
            Some(consent) => audit.by_consent(consent),
            None => audit,
        });
        if let Some(turn) = self.1.lock().ok().and_then(|t| t.clone()) {
            let status = if s.answer.allows() { "ok" } else { "denied" };
            crate::traces::tool(&turn, tool, target, crate::traces::now_ms(), status);
        }
    }
}

fn settle_in_route(
    profile_access: Option<settings::ChatAccess>,
    policies: &[vults_core::policies::Active],
    access: settings::ChatAccess,
    folder: &str,
    agent: vults_core::AgentKind,
    ask: &vults_agents::driver::Ask,
    outside: bool,
) -> Option<vults_chat::Settled> {
    let access = profile_access.unwrap_or(access);
    if access == settings::ChatAccess::Ask {
        return None;
    }
    settle(policies, access, folder, agent, ask, outside)
}

/// What the user's choice answers before a card (ADR 0004). Their policies speak first, as on the
/// island: a deny denies, an allow allows, an ask asks. Full access then allows the rest, but never
/// a destructive call (ADR 0004), nor what a policy asks first, nor anything once the conversation
/// read content from outside: a page's words could be steering the call.
fn settle(
    policies: &[vults_core::policies::Active],
    access: settings::ChatAccess,
    folder: &str,
    agent: vults_core::AgentKind,
    ask: &vults_agents::driver::Ask,
    outside: bool,
) -> Option<vults_chat::Settled> {
    use vults_chat::{Answer, Settled, SettledBy};
    use vults_core::policies::{self, Verdict};
    // Read from the call's whole input, as the island reads an agent's: the whole command, and
    // whether the card's text cut it.
    let core_ask = vults_agents::ask(&ask.tool, &ask.input);
    let call = policies::Call {
        agent,
        root: Some(folder),
        cwd: Some(folder),
        tool: &ask.tool,
        target: &ask.target,
        ask: &core_ask,
    };
    let by_policy = |answer, consent| Settled {
        answer,
        by: SettledBy::Policy,
        consent: Some(consent),
    };
    match policies::decide(policies, &call) {
        Verdict::Deny { consent } => Some(by_policy(Answer::Deny, consent)),
        Verdict::Allow { consent } => Some(by_policy(Answer::Allow, consent)),
        Verdict::Ask
            if access == settings::ChatAccess::Full
                && !outside
                && policies::why_ask(policies, &call).is_none() =>
        {
            Some(Settled {
                answer: Answer::Allow,
                by: SettledBy::FullAccess,
                consent: None,
            })
        }
        Verdict::Ask => None,
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self(Arc::new(ChatInner {
            chat: Mutex::new(Chat::new(Provider::Claude, paths::chat_dir())),
            waiting: Arc::default(),
            stop: std::sync::Mutex::new(None),
            current: std::sync::Mutex::new(None),
            generation: std::sync::atomic::AtomicU32::new(1),
            profile: Default::default(),
            specialist_choice: Default::default(),
            live: Default::default(),
            draft: Default::default(),
        }))
    }

    fn specialist(profile: &Snapshot) -> Result<Self, String> {
        let state = Self::new();
        let dir = paths::chat_dir();
        let spec = vults_agents::driver::Spec::in_dir(dir);
        *state.chat.try_lock().map_err(|_| "Specialist Chat is busy.")? =
            Chat::specialist(spec, None, specialist_brief(profile))?;
        state.set_profile(Some(profile.clone()));
        Ok(state)
    }

    fn live(&self, change: impl FnOnce(&mut Live)) {
        if let Ok(mut live) = self.live.lock() {
            change(&mut live);
        }
    }

    fn current(&self) -> Option<ConversationId> {
        self.current.lock().ok().and_then(|c| *c)
    }

    fn profile(&self) -> Result<Option<Snapshot>, String> {
        self.profile
            .lock()
            .map(|p| p.clone())
            .map_err(|_| "The conversation recipient is unavailable.".into())
    }

    fn set_profile(&self, profile: Option<Snapshot>) {
        if let Ok(mut current) = self.profile.lock() {
            *current = profile;
        }
    }

    fn generation(&self) -> u32 {
        self.generation.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn check_generation(&self, expected: u32) -> Result<(), String> {
        if self.generation() != expected {
            return Err("The conversation changed before sending. Review your draft and try again.".into());
        }
        Ok(())
    }

    fn navigation(&self, generation: u32) -> Result<tokio::sync::MutexGuard<'_, Chat>, String> {
        let chat = self.chat.try_lock().map_err(|_| "Chat is busy.")?;
        self.check_generation(generation)?;
        Ok(chat)
    }

    pub(crate) fn check_companion(&self, generation: u32) -> Result<(), String> {
        self.check_generation(generation)?;
        if self.profile()?.is_some() || self.chat.try_lock().is_ok_and(|chat| chat.is_specialist()) {
            return Err("This action belongs to Zeca's conversation. Open Zeca before trying again.".into());
        }
        Ok(())
    }

    pub(crate) fn companion_command(
        &self,
        generation: u32,
        folder: Option<&str>,
    ) -> Result<tokio::sync::MutexGuard<'_, Chat>, String> {
        let chat = self.chat.try_lock().map_err(|_| "Chat is busy.")?;
        self.check_companion(generation)?;
        if chat.is_specialist() {
            return Err("This action belongs to Zeca's conversation. Open Zeca before trying again.".into());
        }
        if chat.started() && folder_key(folder.map(Path::new)) != folder_key(Some(chat.folder())) {
            return Err("The conversation changed before sending. Review your draft and try again.".into());
        }
        Ok(chat)
    }

    fn set_current(&self, id: Option<ConversationId>) {
        // A startup draft read must observe its owner and contents together.
        if let Ok(mut draft) = self.draft.lock() {
            self.set_saved(id);
            *draft = Draft::default();
            if let Ok(mut choice) = self.specialist_choice.lock() {
                *choice = None;
            }
            self.generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn set_saved(&self, id: Option<ConversationId>) {
        if let Ok(mut c) = self.current.lock() {
            *c = id;
        }
    }
}

pub(crate) fn attention(app: &AppHandle) -> Option<vults_core::AttentionItem> {
    let count = app.try_state::<ChatState>()?.waiting.pending_count()
        + app
            .try_state::<SpecialistChatState>()
            .map_or(0, |state| state.pending_count());
    (count > 0).then_some(vults_core::AttentionItem {
        level: vults_core::Attention::NeedsYou,
        source: vults_core::AttentionSource::Chat { count },
    })
}

/// To both of the chat's windows: the island and the Nest (if it is open).
fn emit_chat<S: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: &S) {
    // Once, to every page: a page's `listen` takes events sent to any target, so one copy sent
    // to the island and one to the Nest reached each page twice (every word doubled).
    let _ = app.emit(event, payload.clone());
}

/// What one of the chat's windows did, for the other to show too (`chat-sync`): each window
/// skips its own.
#[derive(serde::Serialize, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Sync {
    Draft {
        generation: u32,
        from: String,
        draft: Draft,
    },
    /// With where the turn runs, so a window that had not started shows it as it is.
    Sent {
        from: String,
        text: String,
        files: Vec<String>,
        provider: Provider,
        folder: Option<String>,
        model: Option<String>,
        effort: Option<String>,
        mentions: Vec<crate::mentions::Mention>,
    },
    Reset {
        draft: Option<Draft>,
        generation: u32,
        from: String,
        provider: Provider,
    },
    Opened {
        from: String,
        conversation: Opened,
    },
    Answered {
        from: String,
        id: String,
        answer: ChatAnswer,
    },
}

fn sync(app: &AppHandle, s: Sync) {
    if !matches!(s, Sync::Draft { .. }) {
        crate::runtime::recheck(app);
    }
    emit_chat(app, "chat-sync", &s);
}

fn specialist_sync(app: &AppHandle, profile: &str, s: Sync) {
    if !matches!(s, Sync::Draft { .. }) {
        crate::runtime::recheck(app);
    }
    let payload = serde_json::json!({ "profile": profile, "sync": s });
    let _ = app.emit_to(crate::NEST, "specialist-chat-sync", payload);
}

fn route_sync(app: &AppHandle, profile: Option<&str>, s: Sync) {
    if let Some(profile) = profile {
        specialist_sync(app, profile, s);
    } else {
        sync(app, s);
    }
}

fn route_delta(app: &AppHandle, profile: Option<&str>, delta: &Delta) {
    if let Some(profile) = profile {
        let _ = app.emit_to(
            crate::NEST,
            "specialist-chat",
            serde_json::json!({ "profile": profile, "delta": delta }),
        );
    } else {
        emit_chat(app, "chat", delta);
    }
}

/// Streams the reply to the island as `chat` events: text, permissions to ask, then done or
/// error. `folder` (the focused session's project) is where a new conversation works.
#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
    window: tauri::Webview,
    text: String,
    files: Vec<String>,
    folder: Option<String>,
    mentions: Option<Vec<crate::mentions::Mention>>,
    generation: u32,
) -> Result<(), String> {
    send(
        app,
        (*window.app_handle().state::<ChatState>()).clone(),
        Origin::Window(window.label().into(), generation),
        text,
        files,
        folder,
        mentions,
    )
    .await
}

enum Origin {
    Window(String, u32),
    SpecialistWindow(String, u32, String),
    #[cfg(target_os = "linux")]
    Voice(crate::dictation::worker::Ticket, u32),
}

impl Origin {
    fn label(&self) -> &str {
        match self {
            Self::Window(label, _) | Self::SpecialistWindow(label, ..) => label,
            #[cfg(target_os = "linux")]
            Self::Voice(..) => "voice-command",
        }
    }

    fn is_voice(&self) -> bool {
        match self {
            Self::Window(..) | Self::SpecialistWindow(..) => false,
            #[cfg(target_os = "linux")]
            Self::Voice(..) => true,
        }
    }

    fn specialist_profile(&self) -> Option<&str> {
        match self {
            Self::SpecialistWindow(_, _, profile) => Some(profile),
            Self::Window(..) => None,
            #[cfg(target_os = "linux")]
            Self::Voice(..) => None,
        }
    }

    fn reserve<'a>(&self, state: &'a ChatState) -> Result<Option<tokio::sync::MutexGuard<'a, Chat>>, String> {
        if self.is_voice() || self.specialist_profile().is_some() {
            state
                .chat
                .try_lock()
                .map(Some)
                .map_err(|_| "Chat is busy.".into())
        } else {
            Ok(None)
        }
    }

    fn check<R: tauri::Runtime>(&self, app: &tauri::AppHandle<R>) -> Result<(), String> {
        let settings = app.state::<SettingsState>();
        let saved = settings.0.lock().map_err(|_| "settings are busy")?;
        // Window sends resolve their recipient before checking the Zeca switch: profile Chat has
        // its own availability. Voice commands always target the companion.
        if !saved.zeca && self.is_voice() {
            return Err("Zeca is off (Settings → Flock & Zeca).".into());
        }
        #[cfg(target_os = "linux")]
        if let Self::Voice(ticket, generation) = self {
            app.state::<ChatState>().check_companion(*generation)?;
            if !saved.command_chat() || crate::lock::locked(app) || ticket.dispatch(|| ()).is_none() {
                return Err("Voice command handoff was cancelled.".into());
            }
            let state = app.state::<ChatState>();
            let draft = state.draft.lock().map_err(|_| "Chat draft is busy.")?;
            if draft.has_content() {
                return Err("Finish the Chat draft before sending a voice command.".into());
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn voice_command(app: AppHandle, text: String, ticket: crate::dictation::worker::Ticket) {
    let generation = app.state::<ChatState>().generation();
    tauri::async_runtime::spawn(async move {
        let origin = Origin::Voice(ticket.clone(), generation);
        if send(
            app.clone(),
            (*app.state::<ChatState>()).clone(),
            origin,
            text.clone(),
            Vec::new(),
            None,
            None,
        )
        .await
        .is_err()
        {
            voice_result(
                &app,
                &ticket,
                &text,
                vults_core::i18n::VoiceFeedback::ChatUnavailable,
            );
        }
    });
}

#[cfg(target_os = "linux")]
fn voice_result(
    app: &AppHandle,
    ticket: &crate::dictation::worker::Ticket,
    heard: &str,
    feedback: vults_core::i18n::VoiceFeedback,
) {
    let lang = crate::settings::lang(app);
    crate::dictation::command_result(
        app,
        ticket,
        heard,
        crate::voice_commands::Report {
            feedback,
            text: vults_core::i18n::voice_feedback(lang, feedback).into(),
            choices: Vec::new(),
            effects: Vec::new(),
            media: Vec::new(),
        },
        None,
    );
}

#[cfg(target_os = "linux")]
fn check_voice_source(state: &ChatState, expected: &Draft, provider: Provider) -> Result<(), String> {
    let current = state.draft.lock().map_err(|_| "Chat draft is busy.")?;
    if current.folder != expected.folder
        || current.provider != expected.provider
        || expected.provider.is_some_and(|picked| picked != provider)
    {
        return Err("Chat source changed. Say the voice command again.".into());
    }
    Ok(())
}

async fn send(
    app: AppHandle,
    state: ChatState,
    origin: Origin,
    text: String,
    files: Vec<String>,
    folder: Option<String>,
    mentions: Option<Vec<crate::mentions::Mention>>,
) -> Result<(), String> {
    origin.check(&app)?;
    // Voice never queues behind a turn or a conversation change, including local memory reads.
    let reserved = origin.reserve(&state)?;
    #[cfg(target_os = "linux")]
    let voice_source = if let Some(chat) = &reserved {
        let draft = state.draft.lock().map_err(|_| "Chat draft is busy.")?.clone();
        check_voice_source(&state, &draft, chat.provider())?;
        Some(draft)
    } else {
        None
    };
    #[cfg(target_os = "linux")]
    let folder = voice_source
        .as_ref()
        .and_then(|draft| draft.folder.clone())
        .or(folder);

    use crate::flock_tools::Source as _;
    let mentions = mentions.unwrap_or_default();
    let targets = crate::mentions::resolve(&text, mentions.clone(), &app.view().unwrap_or_default())?;
    let source: Arc<dyn crate::flock_tools::Source> = match &targets {
        Some(targets) => Arc::new(crate::mentions::Source {
            source: Arc::new(app.clone()),
            targets: targets.clone(),
        }),
        None => Arc::new(app.clone()),
    };
    // Renderer paths are untrusted even when they look like inbox paths.
    let files = inbox_files(&paths::inbox_dir(), &files)?;
    let (tx, mut rx) = mpsc::channel::<Delta>(64);
    // What the turn cost and how it ended, for its trace span; the usage never reaches the chat.
    let ended = Arc::new(std::sync::Mutex::new((None::<vults_chat::Usage>, "ok")));
    // What Zeca said, and the error if one came, for the saved conversation.
    let said = Arc::new(std::sync::Mutex::new((String::new(), None::<String>)));
    let forward = {
        let app = app.clone();
        let ended = ended.clone();
        let said = said.clone();
        let state = state.clone();
        let specialist = origin.specialist_profile().map(str::to_owned);
        tauri::async_runtime::spawn(async move {
            let mut terminal = None;
            while let Some(delta) = rx.recv().await {
                if let Ok(mut s) = said.lock() {
                    match &delta {
                        Delta::Text { text } => {
                            s.0.push_str(text);
                            state.live(|l| l.text.push_str(text));
                        }
                        Delta::ToolState { active } => state.live(|l| l.tools = *active),
                        Delta::Error { message } => s.1 = Some(message.clone()),
                        _ => {}
                    }
                }
                if let Ok(mut e) = ended.lock() {
                    match &delta {
                        Delta::Usage(u) => {
                            e.0 = Some(u.clone());
                            continue;
                        }
                        Delta::Error { .. } => e.1 = "error",
                        Delta::Stopped => e.1 = "stopped",
                        _ => {}
                    }
                }
                state.live(|l| match &delta {
                    Delta::Activity { event } => l.activity.apply(event),
                    Delta::Allowed { tool, target } => {
                        l.activity.apply(&vults_chat::activity::Event::Decision {
                            tool: tool.clone(),
                            target: target.clone(),
                            answer: "allow".into(),
                            by: "conversation".into(),
                        })
                    }
                    Delta::Settled {
                        tool,
                        target,
                        allow,
                        by,
                    } => l.activity.apply(&vults_chat::activity::Event::Decision {
                        tool: tool.clone(),
                        target: target.clone(),
                        answer: if *allow { "allow" } else { "deny" }.into(),
                        by: match by {
                            vults_chat::SettledBy::Policy => "policy",
                            vults_chat::SettledBy::FullAccess => "full-access",
                        }
                        .into(),
                    }),
                    _ => {}
                });
                if let Delta::Permission { id, detail, .. } = &delta {
                    let outside = detail.outside.clone();
                    state.live(|l| {
                        l.outside = outside;
                        l.permissions.insert(id.clone(), detail.clone());
                    });
                }
                if matches!(
                    delta,
                    Delta::Permission { .. } | Delta::Done | Delta::Error { .. } | Delta::Stopped
                ) {
                    crate::runtime::recheck(&app);
                }
                if matches!(delta, Delta::Done | Delta::Stopped | Delta::Error { .. }) {
                    if !matches!(terminal.as_ref(), Some(Delta::Error { .. } | Delta::Stopped)) {
                        terminal = Some(delta);
                    }
                } else {
                    route_delta(&app, specialist.as_deref(), &delta);
                }
            }
            terminal
        })
    };
    // Keep the recipient stable before any memory read, including asynchronous recall.
    let mut chat = match reserved {
        Some(chat) => chat,
        None => state.chat.lock().await,
    };
    match &origin {
        Origin::Window(_, generation) => state.check_generation(*generation)?,
        Origin::SpecialistWindow(_, generation, _) => state.check_generation(*generation)?,
        #[cfg(target_os = "linux")]
        Origin::Voice(_, generation) => state.check_companion(*generation)?,
    }
    let (stop_tx, mut stop) = oneshot::channel();
    *state.stop.lock().map_err(|_| "Chat is busy.")? = Some(stop_tx);
    let stop_guard = TurnStop(&state);
    let record = if let Some(id) = state.current() {
        store(move |s| s.conversation(id)).await?
    } else {
        None
    };
    let expected_profile = state.profile()?;
    let profile = turn_profile(&chat, record.as_ref(), expected_profile.as_ref())?;
    let companion = profile.is_none();
    if let Some(profile) = &profile {
        let key = profile_access_key(profile);
        let mut preferences = store(move |s| s.agent_profile_state(&key)).await?;
        if record.is_none()
            && !chat.started()
            && let Some(choice) = state
                .specialist_choice
                .lock()
                .map_err(|_| "Chat is busy.")?
                .clone()
        {
            preferences.defaults = choice;
        }
        check_specialist_state(&preferences)?;
        let selected = specialist_project(
            folder.as_deref(),
            &preferences.defaults,
            record.is_some() || chat.started(),
            chat.folder(),
        )?;
        if record.is_none() && !chat.started() {
            *chat = new_specialist(&app, profile, &preferences.defaults, selected)?;
        }
    }
    if !companion && (origin.is_voice() || !mentions.is_empty()) {
        return Err("Specialist conversations do not accept voice commands or flock mentions.".into());
    }
    if zeca_switch_blocks_turn(crate::settings::zeca(&app), !companion) {
        return Err("Zeca is off (Settings → Flock & Zeca).".into());
    }
    let memories = if companion {
        crate::recall::passages(&app, &text, crate::recall::PREFETCH, true).await
    } else {
        Vec::new()
    };
    origin.check(&app)?;
    // Turned off while this turn waited for the lock: `stop` found no turn to end.
    // Profile Chat has its own opt-in and identity; only Zeca's chat follows his switch.
    if zeca_switch_blocks_turn(crate::settings::zeca(&app), !companion) {
        return Err("Zeca is off (Settings → Flock & Zeca).".into());
    }
    // A conversation keeps the provider and model it began with, as a CLI keeps its pick.
    if !chat.started() && companion {
        let (api, model) = api_choice(&app);
        chat.set_api(api, model);
    }
    // Ignored once the conversation has started: it stays where it began. None is the chat's own
    // folder, so a pick made for it (and shown) is the one that runs.
    if companion {
        let dir = folder
            .map(PathBuf::from)
            .filter(|d| d.is_absolute())
            .unwrap_or_else(|| {
                let own = paths::chat_dir();
                let _ = std::fs::create_dir_all(&own);
                own
            });
        chat.set_folder(dir);
    }
    // After the folder: the memory's tool proposes for the roost the conversation is in.
    // The API chat reads the flock through Zeca's own tools (Z9); the CLIs bring theirs.
    if api_companion_tools_enabled(companion, chat.provider(), chat.api().0.supports_tools) {
        let mut tools = crate::flock_tools::all(source.clone());
        tools.extend(crate::recall::tools(&app));
        tools.extend(crate::memory::tools(&app, chat.folder()));
        // The web only while the user has it on: off, the model is never offered it (Z7).
        if crate::settings::web(&app) {
            tools.extend(crate::web_tools::all(Arc::new(crate::web_tools::AppNet(
                app.clone(),
            ))));
        }
        chat.set_tools(tools);
    }
    if !companion {
        let tools = profile
            .as_ref()
            .filter(|_| chat.provider() != Provider::ClaudeApi || chat.api().0.supports_tools)
            .map(|p| crate::specialist_github::tools(&app, &profile_access_key(p)))
            .unwrap_or_default();
        chat.set_tools(tools);
    }
    if companion && chat.provider() == Provider::Claude {
        crate::mcp::keep_scope(targets.as_deref())?;
    }
    if !chat.started() && companion {
        let pick = pick_for(&app, chat.folder(), chat.provider());
        let effort = pick.effort.as_deref().and_then(vults_chat::Effort::parse);
        let effort = effort.filter(|e| effort_fits(chat.provider(), pick.model.as_deref(), *e));
        chat.set_pick(pick.model, effort);
        let native = settings::native_chat(&app);
        let dir = chat.folder().to_path_buf();
        let state = vults_agents::claude_state_file(&paths::agent_dirs());
        let read = tauri::async_runtime::spawn_blocking(move || cli_settings(native, &dir, &state));
        chat.set_settings(read.await.unwrap_or_default());
        // Zeca's own tools for Claude Code (M7), the same file for every conversation.
        if chat.provider() == Provider::Claude {
            chat.set_mcp_config(crate::mcp::config_file());
        }
        // His memory as it is now, fixed for the whole conversation (M2).
        let folder = chat.folder().to_path_buf();
        let memory = crate::settings::memory(&app).then(paths::memory_dir);
        let frozen =
            tauri::async_runtime::spawn_blocking(move || crate::context::frozen(memory.as_deref(), &folder));
        chat.set_frozen(frozen.await.ok());
    }
    let names: Vec<String> = files
        .iter()
        .filter_map(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let conversation = match state.current() {
        // Moved before its first reply (a failed first turn): the record follows.
        Some(id) if !chat.started() => {
            let new = new_conversation(&chat, &text, &names, profile.clone());
            let saved = store(move |s| s.conversation_restart(id, &new)).await;
            if !companion {
                saved?;
            }
            Some(id)
        }
        Some(id) => Some(id),
        None => {
            let new = new_conversation(&chat, &text, &names, profile.clone());
            let made = tauri::async_runtime::spawn_blocking(move || {
                vults_store::Store::open(&crate::history::dir())
                    .and_then(|mut s| s.conversation_new(&new, crate::traces::now_ms()))
            })
            .await;
            match made {
                Ok(Ok(id)) => {
                    state.set_saved(Some(id));
                    Some(id)
                }
                // The chat still works; it just won't be there after a restart.
                _ => {
                    if !companion {
                        return Err("Could not save the specialist conversation. Try again.".into());
                    }
                    tracing::warn!("conversation not saved");
                    None
                }
            }
        }
    };
    let shown: Vec<String> = files.iter().map(|f| f.to_string_lossy().into_owned()).collect();
    let (model, effort) = chat.pick();
    if !companion && state.current() != conversation {
        return Err("The profile conversation changed before the turn started.".into());
    }
    // Preflight can await local IO; an old recording cannot inherit a later opt-in.
    origin.check(&app)?;
    #[cfg(target_os = "linux")]
    if let Some(expected) = &voice_source {
        check_voice_source(&state, expected, chat.provider())?;
    }
    if !matches!(stop.try_recv(), Err(oneshot::error::TryRecvError::Empty)) {
        return Err("Chat turn was stopped before sending.".into());
    }
    state.live(|l| {
        l.busy = true;
        l.started = true;
        l.sent = Some((text.clone(), shown.clone()));
        l.text.clear();
    });
    if let Some(id) = conversation {
        let now = crate::traces::now_ms();
        crate::writer::send(crate::writer::Job::Say(id, "you", text.clone(), names, now));
    }
    route_sync(
        &app,
        origin.specialist_profile(),
        Sync::Sent {
            from: origin.label().to_string(),
            text: text.clone(),
            files: shown,
            provider: chat.provider(),
            folder: Some(folder_key(Some(chat.folder()))).filter(|f| !f.is_empty()),
            model: if chat.provider() == Provider::ClaudeApi {
                Some(chat.api().1.to_string())
            } else {
                model.map(str::to_string)
            },
            effort: effort.map(|e| e.name().to_string()),
            mentions: mentions.clone(),
        },
    );
    if let Ok(mut scope) = state.waiting.2.lock() {
        let agent = match chat.provider() {
            Provider::Codex => vults_core::AgentKind::Codex,
            // The API chat asks nothing; Claude Code is the agent the policies know whole.
            Provider::Claude | Provider::ClaudeApi => vults_core::AgentKind::Claude,
        };
        *scope = chat.folder().to_str().map(|folder| Scope {
            app: app.clone(),
            folder: folder.to_owned(),
            agent,
            profile_key: profile.as_ref().map(profile_access_key),
            profile_access: profile
                .as_ref()
                .map(|profile| settings::specialist_chat_access(&app, &profile_access_key(profile))),
        });
    }
    let approver: Arc<dyn Approver> = state.waiting.clone();
    tracing::info!(provider = ?chat.provider(), files = files.len(), "chat turn");
    let (provider, model) = chat.engine();
    let turn = Arc::new(crate::traces::Turn::start(provider, model));
    if let Ok(mut t) = state.waiting.1.lock() {
        *t = Some(turn.clone());
    }
    // Older conversations omit general context; an explicit reference still carries its scope.
    let context = (companion && (chat.frozen().is_some() || targets.is_some()))
        .then(|| crate::context::volatile(source.as_ref(), &memories));
    // The journal keeps the words (M4), redacted and cut by the store, when they are said.
    if companion {
        crate::journal::chat(&app, "you", chat.folder(), &text, false);
    }
    #[cfg(target_os = "linux")]
    if let Origin::Voice(ticket, _) = &origin {
        voice_result(&app, ticket, &text, vults_core::i18n::VoiceFeedback::SentToChat);
    }
    chat.send(Turn { text, files, context }, tx, approver, stop).await;
    drop(stop_guard);
    let (session, history, outside) = (chat.session(), chat.history(), chat.outside());
    let started = chat.started();
    let folder = chat.folder().to_path_buf();
    let claude = chat.provider() == Provider::Claude;
    // Keep the runtime reserved until its session and transcript are persisted.
    // Zeca's MCP server may have kept a proposal during the turn: the Memory tab reads again.
    if companion && claude {
        crate::memory::changed(&app);
    }
    let terminal = forward.await.ok().flatten();
    if let Some(id) = conversation {
        let now = crate::traces::now_ms();
        let (reply, error) = said.lock().map(|s| s.clone()).unwrap_or_default();
        if companion && !reply.is_empty() {
            crate::journal::chat(&app, "zeca", &folder, &reply, outside.is_some());
        }
        let stopped = ended.lock().is_ok_and(|e| e.1 == "stopped");
        let activity = state.live.lock().ok().and_then(|l| {
            (!l.activity.is_empty())
                .then(|| serde_json::to_string(&l.activity.for_history()).ok())
                .flatten()
        });
        for (who, text) in [
            ("activity", activity),
            ("zeca", Some(reply).filter(|r| !r.is_empty())),
            ("error", error),
            ("stopped", stopped.then(String::new)),
        ] {
            if let Some(text) = text {
                crate::writer::send(crate::writer::Job::Say(id, who, text, Vec::new(), now));
            }
        }
        crate::writer::send(crate::writer::Job::Save(id, session, history, outside, now));
    }
    if !companion {
        crate::writer::flush().await;
    }
    state.live(|l| {
        *l = Live {
            started,
            ..Live::default()
        }
    });
    if let Ok(mut t) = state.waiting.1.lock() {
        *t = None;
    }
    if let Ok(e) = ended.lock() {
        turn.end(e.0.as_ref(), e.1);
    }
    if let Some(terminal) = terminal {
        route_delta(&app, origin.specialist_profile(), &terminal);
    }
    crate::runtime::recheck(&app);
    drop(chat);
    Ok(())
}

/// What the chat's CLI reads: nothing of the user's, their own setup, or with it the project's
/// where Claude Code already trusts the folder (as `claude` there in a terminal would).
/// `state` is Claude Code's state file ([`vults_agents::claude_state_file`]).
pub(crate) fn cli_settings(native: bool, dir: &Path, state: &Path) -> vults_chat::Settings {
    use vults_chat::Settings;
    if !native {
        return Settings::Isolated;
    }
    let state = std::fs::read(state)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    // Claude Code keys its projects by the real path.
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if state.is_some_and(|s| vults_agents::claude_trusts(&s, &dir)) {
        Settings::Project
    } else {
        Settings::User
    }
}

/// Where a pick is kept: the folder, or `""` for the chat's own empty one.
fn folder_key(dir: Option<&Path>) -> String {
    match dir {
        Some(d) if d != paths::chat_dir() => d.to_string_lossy().into_owned(),
        _ => String::new(),
    }
}

fn engine_key(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::Claude => Some("claude"),
        Provider::Codex => Some("codex"),
        Provider::ClaudeApi => None,
    }
}

/// The model and effort a new conversation in `dir` starts with.
fn pick_for(app: &AppHandle, dir: &Path, provider: Provider) -> settings::Pick {
    let Some(engine) = engine_key(provider) else {
        return settings::Pick::default();
    };
    let state = app.state::<SettingsState>();
    let picks = state.0.lock().map(|s| s.chat_picks.clone()).unwrap_or_default();
    picks
        .get(&folder_key(Some(dir)))
        .and_then(|p| p.get(engine))
        .cloned()
        .unwrap_or_default()
}

/// Whether the model takes the effort, as far as its list says (a hand-edited or stale pick would
/// fail every turn); a model not listed, or a list not read yet, is let through to the CLI.
fn effort_fits(provider: Provider, model: Option<&str>, effort: vults_chat::Effort) -> bool {
    let Some(model) = model else { return true };
    let known = match provider {
        Provider::Claude => Some(vults_chat::claude_models()),
        Provider::Codex => CODEX_MODELS.lock().ok().and_then(|m| m.clone()),
        Provider::ClaudeApi => None,
    };
    known
        .and_then(|list| list.into_iter().find(|m| m.id == model))
        .is_none_or(|m| m.efforts.contains(&effort.name()))
}

/// A pick the settings may keep: an absolute folder, a model name the CLI can only read as one,
/// and a known effort.
fn check_pick(folder: Option<&str>, model: Option<&str>, effort: Option<&str>) -> Result<(), String> {
    if folder.is_some_and(|f| !Path::new(f).is_absolute()) {
        return Err("Unknown folder.".into());
    }
    if model.is_some_and(|m| !vults_chat::valid_model(m)) {
        return Err("Unknown model.".into());
    }
    if effort.is_some_and(|e| vults_chat::Effort::parse(e).is_none()) {
        return Err("Unknown effort.".into());
    }
    Ok(())
}

/// What the user's Codex lists, asked once per run: each ask starts an app-server.
static CODEX_MODELS: std::sync::Mutex<Option<Vec<vults_chat::ModelChoice>>> = std::sync::Mutex::new(None);

/// The models the chat's CLI offers: Claude Code's aliases, or the user's Codex's own list (asked
/// when the picker first opens with Codex; the codex they logged into fetches it).
#[tauri::command]
pub async fn chat_models(provider: Provider) -> Result<Vec<vults_chat::ModelChoice>, String> {
    match provider {
        Provider::Claude => Ok(vults_chat::claude_models()),
        Provider::Codex => {
            if let Some(known) = CODEX_MODELS.lock().ok().and_then(|m| m.clone()) {
                return Ok(known);
            }
            let models = vults_chat::codex_models(&paths::chat_dir()).await?;
            if let Ok(mut m) = CODEX_MODELS.lock() {
                *m = Some(models.clone());
            }
            Ok(models)
        }
        Provider::ClaudeApi => Ok(Vec::new()),
    }
}

/// The picks for a folder (none: the chat's own), by engine.
#[tauri::command]
pub fn chat_pick(
    app: AppHandle,
    folder: Option<String>,
) -> std::collections::BTreeMap<String, settings::Pick> {
    let key = folder_key(folder.as_deref().map(Path::new));
    let state = app.state::<SettingsState>();
    state
        .0
        .lock()
        .ok()
        .and_then(|s| s.chat_picks.get(&key).cloned())
        .unwrap_or_default()
}

/// Remembers the model and effort for new conversations in `folder` with `provider`.
#[tauri::command]
pub fn set_chat_pick(
    app: AppHandle,
    folder: Option<String>,
    provider: Provider,
    model: Option<String>,
    effort: Option<String>,
) -> Result<(), String> {
    let engine = engine_key(provider).ok_or("The API chat's model is chosen in Settings → Chat & voice.")?;
    check_pick(folder.as_deref(), model.as_deref(), effort.as_deref())?;
    let key = folder_key(folder.as_deref().map(Path::new));
    settings::edit(&app, |s| {
        let picks = s.chat_picks.entry(key.clone()).or_default();
        if model.is_none() && effort.is_none() {
            picks.remove(engine);
        } else {
            picks.insert(engine.to_string(), settings::Pick { model, effort });
        }
        if picks.is_empty() {
            s.chat_picks.remove(&key);
        }
    })
}

/// A click on the chat's card.
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatAnswer {
    Deny,
    Allow,
    /// *Always in this conversation*: this whole command again, until the conversation ends.
    Always,
}

/// The user's answer to a permission card. Only a click calls this.
#[tauri::command]
pub fn chat_decide(
    window: tauri::Webview,
    state: tauri::State<'_, ChatState>,
    id: String,
    answer: ChatAnswer,
) {
    // Like an agent's card, the chat's is answered on the island or in the Nest (ADR 0021).
    if !crate::runtime::card_host(&window) {
        return;
    }
    decide_for(window.app_handle(), &window, &state, &id, answer, "zeca");
}

fn decide_for(
    app: &AppHandle,
    window: &tauri::Webview,
    state: &ChatState,
    id: &str,
    answer: ChatAnswer,
    subject: &str,
) {
    tracing::info!(?answer, "chat permission answered");
    let asked = state.waiting.0.lock().ok().and_then(|mut m| m.remove(id));
    if let Some(asked) = asked {
        let (reply, act) = match answer {
            ChatAnswer::Deny => (vults_chat::Answer::Deny, Act::Deny),
            ChatAnswer::Allow => (vults_chat::Answer::Allow, Act::Allow),
            ChatAnswer::Always => (vults_chat::Answer::AllowInConversation, Act::AllowInConversation),
        };
        let allow = reply.allows();
        let full_target = state
            .live
            .lock()
            .ok()
            .and_then(|mut l| l.permissions.remove(id))
            .and_then(|d| d.full)
            .unwrap_or_else(|| asked.target.clone());
        state.live(|l| {
            l.activity.apply(&vults_chat::activity::Event::Decision {
                tool: asked.tool.clone(),
                target: full_target,
                answer: match answer {
                    ChatAnswer::Deny => "deny",
                    ChatAnswer::Allow => "allow",
                    ChatAnswer::Always => "always",
                }
                .into(),
                by: "human".into(),
            })
        });
        let _ = asked.answer.send(reply);
        route_sync(
            app,
            (subject != "zeca").then_some(subject),
            Sync::Answered {
                from: window.label().to_string(),
                id: id.to_owned(),
                answer,
            },
        );
        // The chat's own CLI asked: Zeca's, not one of the flock's sessions.
        crate::audit::keep(Audit::new(
            Actor::Human,
            act,
            subject,
            "",
            &asked.tool,
            &asked.target,
        ));
        if let Some(turn) = &asked.turn {
            let status = if allow { "ok" } else { "denied" };
            crate::traces::tool(turn, &asked.tool, &asked.target, asked.since_ms, status);
        }
    }
}

/// Stop on the chat: ends the turn running now. Whatever it was waiting on is a no.
#[tauri::command]
pub fn chat_stop(state: tauri::State<'_, ChatState>) {
    end_turn(&state);
}

/// Zeca switched off: ends the turn running now, then drops the conversation, and with it the
/// Codex app-server a chat may keep alive.
pub fn stop(app: &AppHandle) {
    end_turn(&app.state::<ChatState>());
    // An open Nest stays (ADR 0021): it hides its Chat tab when it hears Zeca is off.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<ChatState>();
        let mut chat = state.chat.lock().await;
        let provider = chat.provider();
        *chat = Chat::new(provider, paths::chat_dir());
        let draft = state.draft.lock().ok().map(|d| d.clone());
        state.set_current(None);
        if let Some(draft) = &draft
            && let Ok(mut saved) = state.draft.lock()
        {
            *saved = draft.clone();
        }
        state.set_profile(None);
        state.live(|l| *l = Live::default());
        sync(
            &app,
            Sync::Reset {
                from: "settings".into(),
                provider,
                generation: state.generation(),
                draft,
            },
        );
    });
}

fn end_turn(state: &ChatState) {
    tracing::info!("chat turn stopped");
    state.waiting.close_all();
    if let Some(tx) = state.stop.lock().ok().and_then(|mut s| s.take()) {
        let _ = tx.send(());
    }
}

// A failed preflight must not leave a stop sender that can cancel a later turn.
struct TurnStop<'a>(&'a ChatState);
impl Drop for TurnStop<'_> {
    fn drop(&mut self) {
        if let Ok(mut stop) = self.0.stop.lock() {
            *stop = None;
        }
    }
}

/// A chat turn runs now (the sleep waits for it, M8).
pub fn busy(app: &AppHandle) -> bool {
    app.state::<ChatState>().live.lock().is_ok_and(|l| l.busy)
}

/// The saved record of the chat's conversation was removed (Clear history): the next message
/// saves it anew rather than into a record that is gone.
pub async fn forget_current(app: &AppHandle) {
    app.state::<ChatState>().set_saved(None);
    let runtimes = app
        .state::<SpecialistChatState>()
        .0
        .lock()
        .map(|states| {
            states
                .iter()
                .map(|(key, state)| (key.clone(), state.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for (_, state) in &runtimes {
        end_turn(state);
    }
    for (profile, state) in runtimes {
        let mut chat = state.chat.lock().await;
        if let Ok(Some(snapshot)) = state.profile()
            && let Ok(fresh) = Chat::specialist(
                vults_agents::driver::Spec::in_dir(paths::chat_dir()),
                None,
                specialist_brief(&snapshot),
            )
        {
            *chat = fresh;
        }
        state.set_current(None);
        state.live(|live| *live = Live::default());
        specialist_sync(
            app,
            &profile,
            Sync::Reset {
                from: "history".into(),
                provider: Provider::Claude,
                generation: state.generation(),
                draft: None,
            },
        );
    }
}

#[derive(serde::Serialize)]
pub struct Reset {
    provider: Provider,
    generation: u32,
}

/// A new conversation, optionally with the other provider. The one before stays saved.
#[tauri::command]
pub async fn chat_reset(
    window: tauri::Webview,
    state: tauri::State<'_, ChatState>,
    provider: Option<Provider>,
    generation: u32,
) -> Result<Reset, String> {
    if !crate::runtime::card_host(&window) {
        return Err("Chat history belongs to the island or Nest.".into());
    }
    let mut chat = state.navigation(generation)?;
    state.waiting.close_all();
    crate::voice::cancel(window.app_handle());
    let provider = provider.unwrap_or(chat.provider());
    let outside_before = chat.outside().is_some();
    let companion = !chat.is_specialist();
    *chat = Chat::new(provider, paths::chat_dir());
    // The one left may hold something worth remembering (M6).
    if companion && let Some(left) = state.current() {
        crate::memory::conversation_ended(window.app_handle(), left, outside_before);
    }
    state.set_current(None);
    state.set_profile(None);
    *state.draft.lock().map_err(|_| "Chat draft is busy.")? = Draft::default();
    state.live(|l| *l = Live::default());
    let from = window.label().to_string();
    sync(
        window.app_handle(),
        Sync::Reset {
            from,
            provider,
            generation: state.generation(),
            draft: None,
        },
    );
    Ok(Reset {
        provider,
        generation: state.generation(),
    })
}

/// The conversation the chat is in now, as saved, for a window that opens on it (the Nest).
#[tauri::command]
pub async fn chat_current(state: tauri::State<'_, ChatState>) -> Result<Option<Opened>, String> {
    let generation = state.generation();
    let Some(id) = state.current() else {
        return Ok(None);
    };
    // Never the chat's lock: a turn holds it until it ends.
    let (busy, tools, started, sent, text, outside, activity, permissions) = state
        .live
        .lock()
        .map(|l| {
            (
                l.busy,
                l.tools,
                l.started,
                l.sent.clone(),
                l.text.clone(),
                l.outside.clone(),
                l.activity.clone(),
                l.permissions.clone(),
            )
        })
        .unwrap_or_default();
    let (c, lines) = store(move |s| Ok((s.conversation(id)?, s.conversation_messages(id)?))).await?;
    let Some(mut opened) = c.and_then(|c| opened(c, lines, started)) else {
        return Ok(None);
    };
    if busy {
        // The turn's own message may not be written yet.
        if let Some((text, files)) = sent
            && !opened
                .lines
                .last()
                .is_some_and(|l| l.who == "you" && l.text == text)
        {
            opened.lines.push(LineView {
                who: "you".into(),
                text,
                files,
            });
        }
        opened.busy = true;
        opened.tools = tools;
        opened.partial = text;
        opened.activity = activity;
        opened.waiting = state.waiting.shown();
        for w in &mut opened.waiting {
            if let Some(detail) = permissions.get(&w.id) {
                w.detail = detail.clone();
            }
            w.detail.outside.clone_from(&outside);
        }
    }
    state.check_generation(generation)?;
    opened.generation = generation;
    Ok(Some(opened))
}

/// A saved conversation in the list.
#[derive(serde::Serialize)]
pub struct ConversationView {
    id: i64,
    title: String,
    provider: Provider,
    /// Who it talks to: Claude, Codex, or the API provider it was pinned to.
    label: String,
    /// The folder's name, or none for the chat's own.
    folder: Option<String>,
    #[serde(rename = "updatedMs")]
    updated_ms: i64,
    /// The one the chat is in now.
    current: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistConversationView {
    id: i64,
    current: bool,
    title: String,
    provider: Provider,
    label: String,
    folder: Option<String>,
    updated_ms: i64,
    profile_title: String,
    profile_revision: u32,
}

fn specialist_conversation_view(
    conversation: conversations::Conversation,
    profile: &str,
) -> Option<SpecialistConversationView> {
    let bound = conversation.profile.as_ref()?;
    if profile_access_key(bound) != profile {
        return None;
    }
    Some(SpecialistConversationView {
        current: false,
        provider: engine_provider(&conversation.engine)?,
        label: engine_label(&conversation.engine),
        id: conversation.id.0,
        title: conversation.title,
        folder: (!conversation.folder.is_empty()).then_some(conversation.folder),
        updated_ms: conversation.updated_ms,
        profile_title: bound.title.clone(),
        profile_revision: match &bound.source {
            vults_store::profiles::Source::Preset { revision, .. }
            | vults_store::profiles::Source::Custom { revision, .. } => *revision,
        },
    })
}

/// A card the chat waits on, for a window that opens while it waits.
#[derive(serde::Serialize, Clone)]
pub struct WaitingView {
    id: String,
    tool: String,
    target: String,
    #[serde(flatten)]
    detail: vults_chat::Detail,
}

/// One line of a saved conversation, as the chat shows it.
#[derive(serde::Serialize, Clone)]
pub struct LineView {
    who: String,
    text: String,
    files: Vec<String>,
}

/// A saved conversation, opened: what the chat needs to go on with it.
#[derive(serde::Serialize, Clone)]
pub struct Opened {
    generation: u32,
    /// A turn runs: what Zeca said so far, and the cards it waits on.
    busy: bool,
    tools: bool,
    partial: String,
    activity: vults_chat::activity::Activity,
    waiting: Vec<WaitingView>,
    provider: Provider,
    label: String,
    folder: Option<String>,
    started: bool,
    model: Option<String>,
    effort: Option<String>,
    lines: Vec<LineView>,
    #[serde(rename = "profileTitle", skip_serializing_if = "Option::is_none")]
    profile_title: Option<String>,
    #[serde(rename = "profileRevision", skip_serializing_if = "Option::is_none")]
    profile_revision: Option<u32>,
    #[serde(rename = "profileSpecies", skip_serializing_if = "Option::is_none")]
    profile_species: Option<String>,
    #[serde(rename = "profileLook", skip_serializing_if = "Option::is_none")]
    profile_look: Option<String>,
}

/// `claude`, `codex`, or `api:<provider>`: what a saved conversation runs on.
fn engine_name(chat: &Chat) -> String {
    match chat.provider() {
        Provider::Claude => "claude".into(),
        Provider::Codex => "codex".into(),
        Provider::ClaudeApi => format!("api:{}", chat.api().0.id),
    }
}

/// The name the chat shows for a saved conversation's engine.
fn engine_label(engine: &str) -> String {
    match engine {
        "claude" => "Claude".into(),
        "codex" => "Codex".into(),
        e => e
            .strip_prefix("api:")
            .and_then(providers::find)
            .map_or_else(|| e.to_string(), |p| p.label.to_string()),
    }
}

fn engine_provider(engine: &str) -> Option<Provider> {
    match engine {
        "claude" => Some(Provider::Claude),
        "codex" => Some(Provider::Codex),
        e if e.starts_with("api:") => Some(Provider::ClaudeApi),
        _ => None,
    }
}

fn settings_name(s: vults_chat::Settings) -> &'static str {
    match s {
        vults_chat::Settings::Isolated => "isolated",
        vults_chat::Settings::User => "user",
        vults_chat::Settings::Project => "project",
    }
}

fn settings_of(name: &str) -> vults_chat::Settings {
    match name {
        "user" => vults_chat::Settings::User,
        "project" => vults_chat::Settings::Project,
        _ => vults_chat::Settings::Isolated,
    }
}

/// The record a conversation starts with, from its first message (or its files' names).
fn new_conversation(
    chat: &Chat,
    text: &str,
    files: &[String],
    profile: Option<Snapshot>,
) -> conversations::New {
    let (model, effort) = chat.pick();
    let model = match chat.provider() {
        Provider::ClaudeApi => Some(chat.api().1.to_string()),
        _ => model.map(str::to_string),
    };
    conversations::New {
        engine: engine_name(chat),
        folder: folder_key(Some(chat.folder())),
        title: if text.trim().is_empty() {
            files.join(", ")
        } else {
            text.to_string()
        },
        model,
        effort: effort.map(|e| e.name().to_string()),
        settings: settings_name(chat.settings()).into(),
        frozen: chat.frozen().map(str::to_string),
        profile,
    }
}

/// A specialist turn must still belong to the exact retained recipient, source and executor.
fn turn_profile(
    chat: &Chat,
    record: Option<&conversations::Conversation>,
    expected: Option<&Snapshot>,
) -> Result<Option<Snapshot>, String> {
    if !chat.is_specialist() {
        if expected.is_some() || record.is_some_and(|c| c.profile.is_some()) {
            return Err("The conversation recipient does not match its runtime.".into());
        }
        return Ok(None);
    }
    let Some(c) = record else {
        return if !chat.started()
            && expected
                .is_some_and(|profile| chat.specialist_brief() == Some(specialist_brief(profile).as_str()))
        {
            expected
                .cloned()
                .ok_or_else(|| "The conversation recipient does not match its runtime.".into())
                .map(Some)
        } else {
            Err("The profile conversation is gone.".into())
        };
    };
    let profile = c
        .profile
        .as_ref()
        .ok_or("The profile conversation has no recipient.")?;
    if expected != Some(profile)
        || c.engine != engine_name(chat)
        || c.frozen.is_some()
        || (chat.provider() != Provider::ClaudeApi && c.history.is_some())
        || (chat.provider() == Provider::ClaudeApi && c.session.is_some())
        || c.settings != "isolated"
        || chat.specialist_brief() != Some(specialist_brief(profile).as_str())
        || folder_key(Some(chat.folder())) != c.folder
        || c.model.as_deref()
            != if chat.provider() == Provider::ClaudeApi {
                Some(chat.api().1)
            } else {
                chat.pick().0
            }
        || c.effort.as_deref().and_then(vults_chat::Effort::parse) != chat.pick().1
        || c.session
            .as_ref()
            .is_some_and(|id| Some(id.as_str()) != chat.session().as_deref())
    {
        return Err("The profile conversation does not match its pinned runtime.".into());
    }
    Ok(Some(profile.clone()))
}

fn check_specialist_state(state: &vults_store::profiles::ProfileState) -> Result<(), String> {
    if state.archived {
        return Err("This agent is archived. Restore it before sending a new message.".into());
    }
    Ok(())
}

fn specialist_project(
    folder: Option<&str>,
    defaults: &vults_store::profiles::ExecutionDefaults,
    bound: bool,
    current: &Path,
) -> Result<PathBuf, String> {
    if matches!(defaults, vults_store::profiles::ExecutionDefaults::Api { .. }) && !bound && folder.is_some()
    {
        return Err(
            "API specialists cannot access projects. Start a new conversation to clear the project.".into(),
        );
    }
    let selected = match folder {
        Some(folder) if Path::new(folder).is_absolute() && Path::new(folder).is_dir() => {
            std::fs::canonicalize(folder).map_err(|_| "That project is unavailable.")?
        }
        Some(_) => return Err("That project is unavailable.".into()),
        None => paths::chat_dir(),
    };
    if bound && selected != current {
        return Err(
            "The conversation project changed. Start a new conversation to choose another project.".into(),
        );
    }
    Ok(selected)
}

fn specialist_from_defaults(
    profile: &Snapshot,
    defaults: &vults_store::profiles::ExecutionDefaults,
    dir: PathBuf,
    api_model: Option<&str>,
) -> Result<Chat, String> {
    use vults_store::profiles::ExecutionDefaults;
    let mut spec = vults_agents::driver::Spec::in_dir(dir);
    spec.state_dir = Some(
        paths::data_dir()
            .join("specialists")
            .join(profile_access_key(profile))
            .join("codex"),
    );
    let (provider, api) = match defaults {
        ExecutionDefaults::Claude { model, effort } => {
            spec.model = model.clone();
            spec.effort = effort.as_deref().and_then(vults_chat::Effort::parse);
            if spec
                .effort
                .is_some_and(|e| !effort_fits(Provider::Claude, model.as_deref(), e))
            {
                return Err("The saved effort is not supported by this model.".into());
            }
            (Provider::Claude, None)
        }
        ExecutionDefaults::Api { provider, model } => {
            let provider = self::provider(provider)?;
            let model = model.as_deref().or(api_model).unwrap_or(provider.default_model);
            if !vults_chat::valid_model(model) {
                return Err("Choose a model in this agent's conversation defaults.".into());
            }
            (Provider::ClaudeApi, Some((provider, model.to_owned())))
        }
        ExecutionDefaults::Codex { model, effort } => {
            spec.model = model.clone();
            spec.effort = effort.as_deref().and_then(vults_chat::Effort::parse);
            if spec
                .effort
                .is_some_and(|e| !effort_fits(Provider::Codex, model.as_deref(), e))
            {
                return Err("The saved effort is not supported by this model.".into());
            }
            (Provider::Codex, None)
        }
    };
    Chat::specialist_for(provider, spec, None, None, api, specialist_brief(profile))
}

fn new_specialist(
    app: &AppHandle,
    profile: &Snapshot,
    defaults: &vults_store::profiles::ExecutionDefaults,
    dir: PathBuf,
) -> Result<Chat, String> {
    if dir == paths::chat_dir() {
        std::fs::create_dir_all(&dir).map_err(|_| "Could not prepare the agent conversation folder.")?;
    }
    let model = if let vults_store::profiles::ExecutionDefaults::Api { provider, .. } = defaults {
        app.state::<SettingsState>()
            .0
            .lock()
            .ok()
            .and_then(|s| s.api_models.get(provider).cloned())
    } else {
        None
    };
    specialist_from_defaults(profile, defaults, dir, model.as_deref())
}

fn profile_access_key(profile: &Snapshot) -> String {
    match &profile.source {
        vults_store::profiles::Source::Preset { id, .. } => id.clone(),
        vults_store::profiles::Source::Custom { id, .. } => format!("custom-{id}"),
    }
}

fn snapshot_revision(profile: &Snapshot) -> u32 {
    match &profile.source {
        vults_store::profiles::Source::Preset { revision, .. }
        | vults_store::profiles::Source::Custom { revision, .. } => *revision,
    }
}

fn api_companion_tools_enabled(companion: bool, provider: Provider, supports_tools: bool) -> bool {
    companion && provider == Provider::ClaudeApi && supports_tools
}

fn zeca_switch_blocks_turn(zeca_on: bool, specialist: bool) -> bool {
    !zeca_on && !specialist
}

/// Restore the retained executor and source without importing companion context.
fn restore_conversation(
    c: &conversations::Conversation,
    mut spec: vults_agents::driver::Spec,
    api: Option<(&'static ApiProvider, String)>,
) -> Result<(Chat, Option<Snapshot>), String> {
    let provider = engine_provider(&c.engine).ok_or("That conversation's engine is unknown.")?;
    if let Some(profile) = &c.profile {
        spec.state_dir = Some(
            paths::data_dir()
                .join("specialists")
                .join(profile_access_key(profile))
                .join("codex"),
        );
        if !spec.dir.is_absolute()
            || c.frozen.is_some()
            || (provider != Provider::ClaudeApi && (c.history.is_some() || api.is_some()))
            || (provider == Provider::ClaudeApi
                && (api.is_none() || c.session.is_some() || c.effort.is_some() || !c.folder.is_empty()))
            || c.settings != "isolated"
            || c.session.as_ref().is_some_and(|id| !valid_session(id))
            || c.model
                .as_ref()
                .is_some_and(|model| !vults_chat::valid_model(model))
            || c.effort
                .as_deref()
                .is_some_and(|effort| vults_chat::Effort::parse(effort).is_none())
        {
            return Err("This profile conversation cannot be resumed by the isolated runtime.".into());
        }
        spec.model = if provider == Provider::ClaudeApi {
            None
        } else {
            c.model.clone()
        };
        spec.effort = c.effort.as_deref().and_then(vults_chat::Effort::parse);
        let mut chat = Chat::specialist_for(
            provider,
            spec,
            c.session.clone(),
            c.history.as_deref(),
            api,
            specialist_brief(profile),
        )?;
        chat.set_outside(c.outside.clone());
        turn_profile(&chat, Some(c), Some(profile))?;
        Ok((chat, Some(profile.clone())))
    } else {
        spec.role = vults_chat::Role::Chat;
        Ok((
            Chat::resume(
                provider,
                spec,
                c.session.clone().filter(|s| valid_session(s)),
                c.history.as_deref(),
                api,
            ),
            None,
        ))
    }
}

pub(crate) fn specialist_brief(profile: &Snapshot) -> String {
    let mut brief = profile.instructions.clone();
    if !profile.skills.is_empty() {
        brief.push_str("\n\nPinned skills:\n");
        for skill in &profile.skills {
            brief.push_str(&format!(
                "\n## {} (revision {})\n{}\n",
                skill.title, skill.revision, skill.content
            ));
        }
    }
    brief
}

/// One thing done with the database, off the async runtime.
async fn store<T: Send + 'static>(
    work: impl FnOnce(&mut vults_store::Store) -> Result<T, vults_store::Error> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        vults_store::Store::open(&crate::history::dir()).and_then(|mut s| work(&mut s))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// The saved conversations, the latest used first.
#[tauri::command]
pub async fn chat_conversations(state: tauri::State<'_, ChatState>) -> Result<Vec<ConversationView>, String> {
    let current = state.current();
    let list = store(|s| s.conversations(50)).await?;
    Ok(list
        .into_iter()
        .filter_map(|c| {
            if c.profile.is_some() {
                return None;
            }
            Some(ConversationView {
                provider: engine_provider(&c.engine)?,
                label: engine_label(&c.engine),
                current: current == Some(c.id),
                id: c.id.0,
                title: c.title,
                folder: (!c.folder.is_empty()).then_some(c.folder),
                updated_ms: c.updated_ms,
            })
        })
        .collect())
}

/// Specialist history is read only in the Nest and only for the selected profile.
#[tauri::command]
pub async fn specialist_chat_conversations(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
) -> Result<Vec<SpecialistConversationView>, String> {
    if window.label() != crate::NEST {
        return Err("Specialist conversations are available in the Nest only.".into());
    }
    let preset = vults_zeca::profiles::PRESETS
        .iter()
        .any(|entry| entry.id == profile);
    let custom_id = profile
        .strip_prefix("custom-")
        .and_then(|id| id.parse::<i64>().ok())
        .filter(|id| *id > 0 && format!("custom-{id}") == profile);
    if !preset && custom_id.is_none() {
        return Err("That profile is unavailable.".into());
    }
    let profile_key = profile.clone();
    let (known_custom, conversations) = store(move |s| {
        let conversations = s.conversations(vults_store::conversations::KEEP as u32)?;
        let retained = custom_id.is_some_and(|_| {
            conversations.iter().any(|conversation| {
                conversation
                    .profile
                    .as_ref()
                    .is_some_and(|snapshot| profile_access_key(snapshot) == profile_key)
            })
        });
        let current = if let Some(id) = custom_id {
            s.agent_profiles()?.iter().any(|entry| entry.id == id)
        } else {
            false
        };
        Ok((retained || current, conversations))
    })
    .await?;
    if !preset && !known_custom {
        return Err("That profile is unavailable.".into());
    }
    let current = app
        .state::<SpecialistChatState>()
        .existing(&profile)?
        .and_then(|state| state.current());
    Ok(conversations
        .into_iter()
        .filter_map(|conversation| specialist_conversation_view(conversation, &profile))
        .map(|mut view| {
            view.current = current == Some(ConversationId(view.id));
            view
        })
        .collect())
}

#[derive(serde::Serialize)]
pub struct SpecialistCurrent {
    generation: u32,
    choice: Option<vults_store::profiles::ExecutionDefaults>,
    conversation: Option<Opened>,
    #[serde(rename = "profileTitle")]
    profile_title: String,
    #[serde(rename = "profileRevision")]
    profile_revision: u32,
    #[serde(rename = "profileSpecies")]
    profile_species: String,
    #[serde(rename = "profileLook")]
    profile_look: String,
    draft: DraftView,
}

fn specialist_current_view(
    state: &ChatState,
    conversation: Option<Opened>,
) -> Result<SpecialistCurrent, String> {
    let profile = state
        .profile()?
        .ok_or("The conversation recipient is unavailable.")?;
    let draft = state.draft.lock().map_err(|_| "Chat draft is busy.")?;
    Ok(SpecialistCurrent {
        generation: state.generation(),
        choice: state
            .specialist_choice
            .lock()
            .map_err(|_| "Chat is busy.")?
            .clone(),
        draft: DraftView {
            draft: draft.clone(),
            generation: state.generation(),
        },
        profile_title: profile.title.clone(),
        profile_revision: snapshot_revision(&profile),
        profile_species: profile.species.clone(),
        profile_look: profile.look.clone(),
        conversation,
    })
}

#[tauri::command]
pub async fn specialist_chat_current(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
) -> Result<SpecialistCurrent, String> {
    specialist_route(&window)?;
    let state = specialist_runtime(&app, &profile).await?;
    let Some(id) = state.current() else {
        return specialist_current_view(&state, None);
    };
    let generation = state.generation();
    let (busy, tools, started, sent, text, outside, activity, permissions) = state
        .live
        .lock()
        .map(|l| {
            (
                l.busy,
                l.tools,
                l.started,
                l.sent.clone(),
                l.text.clone(),
                l.outside.clone(),
                l.activity.clone(),
                l.permissions.clone(),
            )
        })
        .unwrap_or_default();
    let (c, lines) = store(move |s| Ok((s.conversation(id)?, s.conversation_messages(id)?))).await?;
    let Some(c) = c.filter(|c| {
        c.profile
            .as_ref()
            .is_some_and(|p| profile_access_key(p) == profile)
    }) else {
        return specialist_current_view(&state, None);
    };
    let pinned = c.profile.as_ref().expect("filtered specialist conversation");
    let profile_title = pinned.title.clone();
    let profile_revision = snapshot_revision(pinned);
    let mut opened = opened_specialist(c, lines, started).ok_or("That conversation's engine is unknown.")?;
    opened.profile_title = Some(profile_title);
    opened.profile_revision = Some(profile_revision);
    if busy {
        if let Some((text, files)) = sent
            && !opened
                .lines
                .last()
                .is_some_and(|l| l.who == "you" && l.text == text)
        {
            opened.lines.push(LineView {
                who: "you".into(),
                text,
                files,
            });
        }
        opened.busy = true;
        opened.tools = tools;
        opened.partial = text;
        opened.activity = activity;
        opened.waiting = state.waiting.shown();
        for waiting in &mut opened.waiting {
            if let Some(detail) = permissions.get(&waiting.id) {
                waiting.detail = detail.clone();
            }
            waiting.detail.outside.clone_from(&outside);
        }
    }
    state.check_generation(generation)?;
    opened.generation = generation;
    specialist_current_view(&state, Some(opened))
}

fn choice_editable(state: &ChatState, chat: &Chat, generation: u32) -> Result<(), String> {
    state.check_generation(generation)?;
    if state.profile()?.is_none() || !chat.is_specialist() {
        return Err("The conversation recipient does not match its runtime.".into());
    }
    if state.current().is_some() || chat.started() {
        return Err("Start a new conversation before changing its executor or model.".into());
    }
    Ok(())
}

fn install_specialist_choice(
    state: &ChatState,
    chat: &mut Chat,
    fresh: Chat,
    choice: Option<vults_store::profiles::ExecutionDefaults>,
    generation: u32,
) -> Result<(), String> {
    choice_editable(state, chat, generation)?;
    let snapshot = state
        .profile()?
        .ok_or("The conversation recipient is unavailable.")?;
    turn_profile(&fresh, None, Some(&snapshot))?;
    let mut draft = state.draft.lock().map_err(|_| "Chat draft is busy.")?;
    let mut selected = state.specialist_choice.lock().map_err(|_| "Chat is busy.")?;
    *chat = fresh;
    *selected = choice;
    draft.provider = Some(chat.provider());
    state.generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn specialist_chat_choice_set(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    generation: u32,
    choice: Option<vults_store::profiles::ExecutionDefaults>,
) -> Result<SpecialistCurrent, String> {
    specialist_route(&window)?;
    if let Some(choice) = &choice {
        crate::profiles::validate_defaults(choice)?;
    }
    let state = specialist_runtime(&app, &profile).await?;
    let mut chat = state.chat.try_lock().map_err(|_| "Chat is busy.")?;
    choice_editable(&state, &chat, generation)?;
    let key = profile.clone();
    let mut preferences = store(move |s| s.agent_profile_state(&key)).await?;
    if let Some(choice) = &choice {
        preferences.defaults = choice.clone();
    }
    // Clearing an override may restore unavailable defaults, but never executes them.
    check_specialist_state(&preferences)?;
    let snapshot = state
        .profile()?
        .ok_or("The conversation recipient is unavailable.")?;
    let folder = state
        .draft
        .lock()
        .map_err(|_| "Chat draft is busy.")?
        .folder
        .clone();
    let dir = specialist_project(folder.as_deref(), &preferences.defaults, false, chat.folder())?;
    let fresh = if choice.is_some() {
        new_specialist(&app, &snapshot, &preferences.defaults, dir)?
    } else {
        Chat::specialist(
            vults_agents::driver::Spec::in_dir(dir),
            None,
            specialist_brief(&snapshot),
        )?
    };
    install_specialist_choice(&state, &mut chat, fresh, choice, generation)?;
    specialist_current_view(&state, None)
}

#[tauri::command]
pub async fn specialist_chat_draft_set(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    draft: Draft,
    generation: u32,
) -> Result<(), String> {
    specialist_route(&window)?;
    draft.validate()?;
    if !draft.mentions.is_empty() {
        return Err("Specialist conversations do not accept voice commands or flock mentions.".into());
    }
    let state = specialist_runtime(&app, &profile).await?;
    let mut saved = state.draft.lock().map_err(|_| "Chat draft is busy.")?;
    state.check_generation(generation)?;
    *saved = draft;
    Ok(())
}

fn history_owner(
    store: &vults_store::Store,
    id: ConversationId,
    profile: Option<&str>,
) -> Result<(), vults_store::Error> {
    use vults_store::Error;
    let record = store
        .conversation(id)?
        .ok_or(Error::Invalid("That conversation is gone."))?;
    match (record.profile.as_ref(), profile) {
        (None, None) => Ok(()),
        (Some(snapshot), Some(key)) if profile_access_key(snapshot) == key => Ok(()),
        _ => Err(Error::Invalid("That conversation belongs to another profile.")),
    }
}

#[tauri::command]
pub async fn specialist_chat_rename(
    window: tauri::Webview,
    profile: String,
    id: i64,
    title: String,
) -> Result<(), String> {
    specialist_route(&window)?;
    store(move |s| {
        history_owner(s, ConversationId(id), Some(&profile))?;
        s.conversation_rename(ConversationId(id), &title)
    })
    .await
}

#[derive(serde::Serialize)]
pub struct SpecialistDeleted {
    current: bool,
    reset: Option<SpecialistReset>,
}

#[tauri::command]
pub async fn specialist_chat_delete(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    id: i64,
) -> Result<SpecialistDeleted, String> {
    specialist_route(&window)?;
    let state = specialist_runtime(&app, &profile).await?;
    let mut chat = state.chat.try_lock().map_err(|_| "Chat is busy.")?;
    let current = state.current() == Some(ConversationId(id));
    // Prepare the replacement before deleting; archived history can still be removed.
    let fresh = if current {
        let snapshot = specialist_snapshot(&profile).await?;
        let chat = Chat::specialist(
            vults_agents::driver::Spec::in_dir(paths::chat_dir()),
            None,
            specialist_brief(&snapshot),
        )?;
        let key = profile.clone();
        let preferences = store(move |s| s.agent_profile_state(&key)).await?;
        let provider = if matches!(
            preferences.defaults,
            vults_store::profiles::ExecutionDefaults::Api { .. }
        ) {
            Provider::ClaudeApi
        } else {
            Provider::Claude
        };
        Some((snapshot, chat, provider))
    } else {
        None
    };
    let key = profile.clone();
    store(move |s| {
        history_owner(s, ConversationId(id), Some(&key))?;
        s.conversation_delete(ConversationId(id))
    })
    .await?;
    let reset = if let Some((snapshot, fresh, provider)) = fresh {
        *chat = fresh;
        state.waiting.close_all();
        state.set_profile(Some(snapshot));
        state.set_current(None);
        state.live(|l| *l = Live::default());
        specialist_sync(
            &app,
            &profile,
            Sync::Reset {
                from: window.label().into(),
                provider,
                generation: state.generation(),
                draft: None,
            },
        );
        Some(SpecialistReset {
            provider,
            current: specialist_current_view(&state, None)?,
        })
    } else {
        None
    };
    Ok(SpecialistDeleted { current, reset })
}

#[tauri::command]
pub async fn specialist_chat_send(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    text: String,
    files: Vec<String>,
    generation: u32,
    folder: Option<String>,
) -> Result<(), String> {
    specialist_route(&window)?;
    if text.trim().is_empty() && files.is_empty() {
        return Err("Write a message first.".into());
    }
    if text.len() > 1_000_000 {
        return Err("Message is too large.".into());
    }
    let state = specialist_runtime(&app, &profile).await?;
    send(
        app,
        state,
        Origin::SpecialistWindow(window.label().into(), generation, profile),
        text,
        files,
        folder,
        None,
    )
    .await
}

#[tauri::command]
pub async fn specialist_chat_open(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    id: i64,
    generation: u32,
) -> Result<Opened, String> {
    specialist_route(&window)?;
    let state = specialist_runtime(&app, &profile).await?;
    let mut chat = state.navigation(generation)?;
    let id = ConversationId(id);
    let (c, lines) = store(move |s| Ok((s.conversation(id)?, s.conversation_messages(id)?))).await?;
    let c = c.ok_or("That conversation is gone.")?;
    let pinned = c
        .profile
        .as_ref()
        .ok_or("That is not a specialist conversation.")?;
    if profile_access_key(pinned) != profile {
        return Err("That conversation belongs to another profile.".into());
    }
    let profile_title = pinned.title.clone();
    let profile_revision = snapshot_revision(pinned);
    if !c.folder.is_empty() && !Path::new(&c.folder).is_dir() {
        return Err("That conversation's runtime folder is gone.".into());
    }
    let spec = vults_agents::driver::Spec {
        model: c.model.clone(),
        effort: c.effort.as_deref().and_then(vults_chat::Effort::parse),
        ..vults_agents::driver::Spec::in_dir(if c.folder.is_empty() {
            paths::chat_dir()
        } else {
            PathBuf::from(&c.folder)
        })
    };
    let (restored, snapshot) = restore_conversation(
        &c,
        spec,
        c.engine
            .strip_prefix("api:")
            .map(|id| self::provider(id).map(|p| (p, c.model.clone().unwrap_or_default())))
            .transpose()?,
    )?;
    state.waiting.close_all();
    *chat = restored;
    chat.set_outside(c.outside.clone());
    state.set_profile(snapshot);
    state.set_current(Some(id));
    *state.draft.lock().map_err(|e| e.to_string())? = Draft::default();
    let started = chat.started();
    state.live(|l| {
        *l = Live {
            started,
            ..Live::default()
        }
    });
    let mut conversation =
        opened_specialist(c, lines, started).ok_or("That conversation's engine is unknown.")?;
    conversation.profile_title = Some(profile_title);
    conversation.profile_revision = Some(profile_revision);
    conversation.generation = state.generation();
    specialist_sync(
        &app,
        &profile,
        Sync::Opened {
            from: window.label().into(),
            conversation: conversation.clone(),
        },
    );
    Ok(conversation)
}

#[derive(serde::Serialize)]
pub struct SpecialistReset {
    provider: Provider,
    #[serde(flatten)]
    current: SpecialistCurrent,
}

#[tauri::command]
pub async fn specialist_chat_reset(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    generation: u32,
) -> Result<SpecialistReset, String> {
    specialist_route(&window)?;
    let key = profile.clone();
    let preferences = store(move |s| s.agent_profile_state(&key)).await?;
    check_specialist_state(&preferences)?;
    let state = specialist_runtime(&app, &profile).await?;
    let mut chat = state.navigation(generation)?;
    let snapshot = specialist_snapshot(&profile).await?;
    *chat = Chat::specialist(
        vults_agents::driver::Spec::in_dir(paths::chat_dir()),
        None,
        specialist_brief(&snapshot),
    )?;
    let provider = if matches!(
        preferences.defaults,
        vults_store::profiles::ExecutionDefaults::Api { .. }
    ) {
        Provider::ClaudeApi
    } else {
        Provider::Claude
    };
    state.waiting.close_all();
    state.set_profile(Some(snapshot));
    state.set_current(None);
    *state.draft.lock().map_err(|e| e.to_string())? = Draft::default();
    state.live(|l| *l = Live::default());
    specialist_sync(
        &app,
        &profile,
        Sync::Reset {
            from: window.label().into(),
            provider,
            generation: state.generation(),
            draft: None,
        },
    );
    Ok(SpecialistReset {
        provider,
        current: specialist_current_view(&state, None)?,
    })
}

#[tauri::command]
pub fn specialist_chat_stop(app: AppHandle, window: tauri::Webview, profile: String) -> Result<(), String> {
    specialist_route(&window)?;
    if let Some(state) = app.state::<SpecialistChatState>().existing(&profile)? {
        end_turn(&state);
    }
    Ok(())
}

#[tauri::command]
pub fn specialist_chat_decide(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    id: String,
    answer: ChatAnswer,
) -> Result<(), String> {
    specialist_route(&window)?;
    let Some(state) = app.state::<SpecialistChatState>().existing(&profile)? else {
        return Ok(());
    };
    decide_for(&app, &window, &state, &id, answer, &profile);
    Ok(())
}

/// Goes on with a saved conversation: the chat becomes that one, as it was.
#[tauri::command]
pub async fn chat_open(
    app: AppHandle,
    window: tauri::Webview,
    id: i64,
    generation: u32,
) -> Result<Opened, String> {
    if !crate::runtime::card_host(&window) {
        return Err("Chat history belongs to the island or Nest.".into());
    }
    let state = app.state::<ChatState>();
    let mut chat = state.navigation(generation)?;
    let id = ConversationId(id);
    let (c, lines) = store(move |s| Ok((s.conversation(id)?, s.conversation_messages(id)?))).await?;
    let c = c.ok_or("That conversation is gone.")?;
    if c.profile.is_some() {
        return Err("Specialist conversations are not available in Zeca's chat.".into());
    }
    let provider = engine_provider(&c.engine).ok_or("That conversation's engine is unknown.")?;
    let dir = if c.folder.is_empty() {
        paths::chat_dir()
    } else {
        PathBuf::from(&c.folder)
    };
    // Never brought back: a folder the user removed stays removed.
    if !c.folder.is_empty() && !dir.is_dir() {
        return Err(format!("That conversation's folder is gone: {}", c.folder));
    }
    let api = c
        .engine
        .strip_prefix("api:")
        .map(|p| {
            self::provider(p).map(|p| (p, c.model.clone().unwrap_or_else(|| p.default_model.to_string())))
        })
        .transpose()?;
    let spec = vults_agents::driver::Spec {
        model: c
            .model
            .clone()
            .filter(|m| vults_chat::valid_model(m) && api.is_none()),
        effort: c.effort.as_deref().and_then(vults_chat::Effort::parse),
        settings: settings_of(&c.settings),
        // The same system prompt it began with (M2); with the memory off now, the same without his
        // facts: off means none of it reaches a model.
        system_append: if c.profile.is_some() {
            None
        } else if crate::settings::memory(&app) {
            c.frozen.clone()
        } else {
            c.frozen.as_ref().map(|_| crate::context::frozen(None, &dir))
        },
        mcp_config: (c.profile.is_none() && provider == Provider::Claude)
            .then(crate::mcp::config_file)
            .flatten(),
        ..vults_agents::driver::Spec::in_dir(dir)
    };
    // A session id reaches Claude Code as `--resume`'s value: never anything that reads as a flag.
    let restored = restore_conversation(&c, spec, api)?;
    state.waiting.close_all();
    if !chat.is_specialist()
        && let Some(left) = state.current().filter(|l| *l != c.id)
    {
        crate::memory::conversation_ended(&app, left, chat.outside().is_some());
    }
    crate::voice::cancel(&app);
    *chat = restored.0;
    state.set_profile(restored.1);
    // It read from outside before: its cards still say so.
    chat.set_outside(c.outside.clone());
    state.set_current(Some(c.id));
    *state.draft.lock().map_err(|e| e.to_string())? = Draft::default();
    let started = chat.started();
    state.live(|l| {
        *l = Live {
            started,
            ..Live::default()
        }
    });
    let mut conversation = opened(c, lines, started).ok_or("That conversation's engine is unknown.")?;
    conversation.generation = state.generation();
    let from = window.label().to_string();
    sync(
        &app,
        Sync::Opened {
            from,
            conversation: conversation.clone(),
        },
    );
    Ok(conversation)
}

/// A saved conversation as the chat shows it.
fn opened(
    c: conversations::Conversation,
    lines: Vec<conversations::Message>,
    started: bool,
) -> Option<Opened> {
    if c.profile.is_some() {
        return None;
    }
    opened_record(c, lines, started)
}

fn opened_specialist(
    c: conversations::Conversation,
    lines: Vec<conversations::Message>,
    started: bool,
) -> Option<Opened> {
    let profile = c.profile.as_ref()?;
    let species = profile.species.clone();
    let look = profile.look.clone();
    let mut opened = opened_record(c, lines, started)?;
    opened.profile_species = Some(species);
    opened.profile_look = Some(look);
    Some(opened)
}

fn opened_record(
    c: conversations::Conversation,
    lines: Vec<conversations::Message>,
    started: bool,
) -> Option<Opened> {
    Some(Opened {
        generation: 0,
        busy: false,
        tools: false,
        partial: String::new(),
        activity: Default::default(),
        waiting: Vec::new(),
        provider: engine_provider(&c.engine)?,
        label: engine_label(&c.engine),
        folder: (!c.folder.is_empty()).then_some(c.folder),
        started,
        model: c.model,
        effort: c.effort,
        lines: lines
            .into_iter()
            .map(|l| LineView {
                who: l.who,
                text: l.text,
                files: l.files,
            })
            .collect(),
        profile_title: None,
        profile_revision: None,
        profile_species: None,
        profile_look: None,
    })
}

/// A CLI's session or thread id: letters, digits, `-` and `_`, never first a `-`.
pub(crate) fn valid_session(id: &str) -> bool {
    (1..=100).contains(&id.len())
        && !id.starts_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[tauri::command]
pub async fn chat_rename(window: tauri::Webview, id: i64, title: String) -> Result<(), String> {
    if !crate::runtime::card_host(&window) {
        return Err("Chat history belongs to the island or Nest.".into());
    }
    store(move |s| {
        history_owner(s, ConversationId(id), None)?;
        s.conversation_rename(ConversationId(id), &title)
    })
    .await
}

/// Deletes a saved conversation; clears the runtime only after the database succeeds.
#[tauri::command]
pub async fn chat_delete(
    window: tauri::Webview,
    state: tauri::State<'_, ChatState>,
    id: i64,
) -> Result<bool, String> {
    if !crate::runtime::card_host(&window) {
        return Err("Chat history belongs to the island or Nest.".into());
    }
    let id = ConversationId(id);
    let mut chat = state.chat.try_lock().map_err(|_| "Chat is busy.")?;
    let was_current = state.current() == Some(id);
    store(move |s| {
        history_owner(s, id, None)?;
        s.conversation_delete(id)
    })
    .await?;
    if was_current {
        state.waiting.close_all();
        crate::voice::cancel(window.app_handle());
        let provider = chat.provider();
        *chat = Chat::new(provider, paths::chat_dir());
        state.set_current(None);
        state.set_profile(None);
        state.live(|l| *l = Live::default());
        sync(
            window.app_handle(),
            Sync::Reset {
                from: window.label().into(),
                provider,
                generation: state.generation(),
                draft: None,
            },
        );
    }
    Ok(was_current)
}

/// The API chat as the island sees it: usable now (a key saved, or a local server), and by whom.
#[derive(serde::Serialize, Clone)]
pub struct ApiStatus {
    ready: bool,
    label: &'static str,
}

/// One provider in Settings → Chat. Whether a key is saved, never the key itself.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProviderView {
    #[serde(flatten)]
    provider: &'static ApiProvider,
    has_key: bool,
    model: String,
}

#[derive(serde::Serialize)]
pub struct ApiProviders {
    selected: &'static str,
    providers: Vec<ApiProviderView>,
}

fn provider(id: &str) -> Result<&'static ApiProvider, String> {
    providers::find(id).ok_or_else(|| format!("Unknown provider {id}."))
}

/// The selected provider and its model, from the settings.
pub(crate) fn api_choice(app: &AppHandle) -> (&'static ApiProvider, String) {
    let state = app.state::<SettingsState>();
    let s = state.0.lock().map(|s| s.clone()).unwrap_or_default();
    let p = providers::find(&s.api_provider).unwrap_or(&providers::PROVIDERS[0]);
    let model = s
        .api_models
        .get(p.id)
        .cloned()
        .unwrap_or_else(|| p.default_model.to_string());
    (p, model)
}

async fn has_key(p: &'static ApiProvider) -> bool {
    p.local
        || tauri::async_runtime::spawn_blocking(move || secrets::has(p.secret()))
            .await
            .unwrap_or(false)
}

#[tauri::command]
pub async fn api_key_status(app: AppHandle) -> ApiStatus {
    let (p, _) = api_choice(&app);
    ApiStatus {
        ready: has_key(p).await,
        label: p.label,
    }
}

/// How each CLI engine of the chat stands, for its menu: `missing` off the PATH the chat starts
/// it with, `signed-in` with its login file there (only looked for, never read), else
/// `installed`.
#[derive(serde::Serialize, Debug, PartialEq, Eq)]
pub struct Engines {
    claude: &'static str,
    codex: &'static str,
}

#[tauri::command]
pub async fn chat_engines() -> Engines {
    // The folders may wait on the login shell (up to 2 s at start-up): never on the main thread.
    tokio::task::spawn_blocking(|| {
        let dirs = crate::paths::agent_dirs();
        let path = std::env::var_os("PATH");
        let on_path = |program: &str| {
            path.as_ref()
                .is_some_and(|p| std::env::split_paths(p).any(|d| d.join(program).is_file()))
        };
        Engines {
            claude: engine(
                on_path("claude"),
                &dirs
                    .agent("CLAUDE_CONFIG_DIR", ".claude")
                    .join(".credentials.json"),
            ),
            codex: engine(
                on_path("codex"),
                &vults_agents::codex_home(&dirs).join("auth.json"),
            ),
        }
    })
    .await
    .unwrap_or(Engines {
        claude: "missing",
        codex: "missing",
    })
}

fn engine(installed: bool, login: &Path) -> &'static str {
    match (installed, login.is_file()) {
        (false, _) => "missing",
        (true, true) => "signed-in",
        (true, false) => "installed",
    }
}

/// Tells the windows what the API chat can do now.
async fn announce(app: &AppHandle) {
    let status = api_key_status(app.clone()).await;
    let _ = app.emit("settings", serde_json::json!({ "api": status }));
}

#[tauri::command]
pub async fn api_providers(app: AppHandle) -> ApiProviders {
    let (selected, _) = api_choice(&app);
    let models = app
        .state::<SettingsState>()
        .0
        .lock()
        .map(|s| s.api_models.clone())
        .unwrap_or_default();
    let mut list = Vec::new();
    for p in providers::PROVIDERS {
        list.push(ApiProviderView {
            provider: p,
            has_key: !p.local && has_key(p).await,
            model: models
                .get(p.id)
                .cloned()
                .unwrap_or_else(|| p.default_model.to_string()),
        });
    }
    ApiProviders {
        selected: selected.id,
        providers: list,
    }
}

#[tauri::command]
pub async fn api_provider_set(app: AppHandle, provider: String) -> Result<(), String> {
    let p = self::provider(&provider)?;
    settings::edit(&app, |s| s.api_provider = p.id.to_string())?;
    #[cfg(target_os = "linux")]
    crate::dictation::cleanup::invalidate(&app);
    crate::meetings::summary::invalidate(&app);
    announce(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn api_model_set(app: AppHandle, provider: String, model: String) -> Result<(), String> {
    let p = self::provider(&provider)?;
    let model = model.trim().to_string();
    if model.is_empty() || model.len() > 200 {
        return Err("Choose a model from the list.".into());
    }
    settings::edit(&app, |s| {
        s.api_models.insert(p.id.to_string(), model);
    })?;
    #[cfg(target_os = "linux")]
    crate::dictation::cleanup::invalidate(&app);
    crate::meetings::summary::invalidate(&app);
    announce(&app).await;
    Ok(())
}

/// The provider's chat models, asked live with the saved key.
#[tauri::command]
pub async fn api_models(provider: String) -> Result<Vec<String>, String> {
    let p = self::provider(&provider)?;
    let key = if p.local {
        None
    } else {
        tauri::async_runtime::spawn_blocking(move || secrets::get(p.secret()))
            .await
            .map_err(|_| "Can't read the key.".to_string())??
    };
    if !p.local && key.is_none() {
        return Err(format!("Save a {} key first.", p.label));
    }
    providers::models(p, key.as_deref()).await
}

/// Saves the key in the OS keyring, the only place it is ever written.
#[tauri::command]
pub async fn api_key_set(
    window: tauri::Webview,
    app: AppHandle,
    provider: String,
    key: String,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let p = self::provider(&provider)?;
    let key = key.trim().to_string();
    if p.local || !p.accepts_key(&key) {
        return Err(if p.key_hint.is_empty() {
            format!("That doesn't look like a {} API key.", p.label)
        } else {
            format!(
                "That doesn't look like a {} API key (they start with {}).",
                p.label, p.key_hint
            )
        });
    }
    let changed = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = secrets::set(p.secret(), &key);
        crate::realtime::key_changed(&changed, p.id);
        result
    })
    .await
    .map_err(|_| "Can't save the key.".to_string())??;
    #[cfg(target_os = "linux")]
    crate::dictation::cleanup::invalidate(&app);
    crate::meetings::summary::invalidate(&app);
    tracing::info!(provider = p.id, "api key saved");
    announce(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn api_key_clear(window: tauri::Webview, app: AppHandle, provider: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    let p = self::provider(&provider)?;
    let changed = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = secrets::delete(p.secret());
        crate::realtime::key_changed(&changed, p.id);
        result
    })
    .await
    .map_err(|_| "Can't remove the key.".to_string())??;
    #[cfg(target_os = "linux")]
    crate::dictation::cleanup::invalidate(&app);
    crate::meetings::summary::invalidate(&app);
    tracing::info!(provider = p.id, "api key removed");
    announce(&app).await;
    Ok(())
}

/// What became of the files dropped on the island: the inbox copies, and the ones refused with why.
#[derive(serde::Serialize)]
struct Dropped {
    copied: Vec<String>,
    refused: Vec<Refused>,
    /// The page whose chat takes them: the island's drop and paperclip, or the Nest's paperclip.
    /// Every page hears the event; only that one attaches them, so nothing waits twice.
    to: &'static str,
}

#[derive(serde::Serialize)]
struct Refused {
    name: String,
    /// "folder", "too-big" or "unreadable".
    reason: &'static str,
}

/// Copies dropped or picked files into the inbox and tells the page `to` (every page hears it)
/// about the copies and the refusals.
pub fn on_drop(app: &AppHandle, dropped: &[PathBuf], to: &'static str) {
    // Files go to Zeca's chat: with him off, nothing is copied.
    if !crate::settings::zeca(app) {
        return;
    }
    // Up to MAX_FILE each, maybe from a slow disk: never on the main thread, where the island
    // would freeze until the last copy.
    let app = app.clone();
    let dropped = dropped.to_vec();
    tauri::async_runtime::spawn_blocking(move || {
        let out = copy_dropped(&dropped, to);
        let _ = app.emit("files", &out);
    });
}

/// A file chooser is open (the paperclip's).
static PICKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The chat's paperclip: the system's file chooser, and what it picks goes the way of a drop (the
/// inbox copy, then Zeca swallows it). Only files the user picked; nothing is read before that.
#[tauri::command]
pub fn chat_pick_files(app: AppHandle, window: tauri::Webview, title: String) {
    use std::sync::atomic::Ordering;
    use tauri_plugin_dialog::DialogExt;
    if !crate::settings::zeca(&app) {
        return;
    }
    // The island's chat or the Nest's: the files go back to the one that asked.
    let to = match window.label() {
        l if l == ISLAND => ISLAND,
        l if l == crate::NEST => crate::NEST,
        _ => return,
    };
    // One chooser at a time: a double click would open two, both able to hand files in.
    if PICKING.swap(true, Ordering::SeqCst) {
        return;
    }
    // Its title is the page's own words, in the user's language; kept short.
    let title: String = title.chars().filter(|c| !c.is_control()).take(80).collect();
    // The chooser blocks until the user picks or cancels: off the main thread.
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app
            .dialog()
            .file()
            .set_title(title)
            .blocking_pick_files()
            .unwrap_or_default();
        PICKING.store(false, Ordering::SeqCst);
        let paths: Vec<PathBuf> = picked.into_iter().filter_map(|p| p.into_path().ok()).collect();
        if !paths.is_empty() {
            on_drop(&app, &paths, to);
        }
    });
}

#[derive(serde::Serialize)]
pub struct SpecialistFiles {
    generation: u32,
    #[serde(flatten)]
    files: Dropped,
}

struct Picking;
impl Drop for Picking {
    fn drop(&mut self) {
        PICKING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn specialist_chat_pick_files(
    app: AppHandle,
    window: tauri::Webview,
    profile: String,
    generation: u32,
    title: String,
) -> Result<SpecialistFiles, String> {
    use tauri_plugin_dialog::DialogExt;
    specialist_route(&window)?;
    let state = specialist_runtime(&app, &profile).await?;
    state.check_generation(generation)?;
    let key = profile.clone();
    let preferences = store(move |s| s.agent_profile_state(&key)).await?;
    check_specialist_state(&preferences)?;
    if PICKING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err("A file chooser is already open.".into());
    }
    let picking = Picking;
    let title: String = title.chars().filter(|c| !c.is_control()).take(80).collect();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let _picking = picking;
        app.dialog()
            .file()
            .set_title(title)
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| p.into_path().ok())
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|_| "Could not choose files. Your draft stays here.")?;
    state.check_generation(generation)?;
    let preferences = store(move |s| s.agent_profile_state(&profile)).await?;
    check_specialist_state(&preferences)?;
    if picked.len() > MAX_ATTACHMENTS {
        return Err("Attach at most 16 files and 20 MB in total.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        state.check_generation(generation)?;
        let files = copy_dropped(&picked, crate::NEST);
        if let Err(error) = state.check_generation(generation) {
            for copy in files.copied {
                let _ = std::fs::remove_file(copy);
            }
            return Err(error);
        }
        Ok(SpecialistFiles { generation, files })
    })
    .await
    .map_err(|_| "Could not choose files. Your draft stays here.".to_owned())?
}

fn inbox_files(inbox: &Path, files: &[String]) -> Result<Vec<PathBuf>, String> {
    if files.len() > MAX_ATTACHMENTS {
        return Err("Attach at most 16 files and 20 MB in total.".into());
    }
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let root = std::fs::canonicalize(inbox)
        .map_err(|_| "An attachment is unavailable. Remove it and choose the file again.")?;
    let mut total = 0u64;
    let mut selected = Vec::with_capacity(files.len());
    for file in files {
        let path = Path::new(file);
        let valid = path.is_absolute()
            && path.parent() == Some(inbox)
            && !path
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir));
        if !valid {
            return Err("Only files selected through Vults can be attached. Choose the file again.".into());
        }
        let meta = std::fs::symlink_metadata(path)
            .map_err(|_| "An attachment is unavailable. Remove it and choose the file again.")?;
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| "An attachment is unavailable. Remove it and choose the file again.")?;
        if !meta.file_type().is_file() || canonical.parent() != Some(root.as_path()) {
            return Err("Only files selected through Vults can be attached. Choose the file again.".into());
        }
        total = total.saturating_add(meta.len());
        if total > MAX_FILE {
            return Err("Attach at most 16 files and 20 MB in total.".into());
        }
        if selected.contains(&canonical) {
            return Err("Choose each attachment only once.".into());
        }
        selected.push(canonical);
    }
    Ok(selected)
}

fn copy_dropped(paths: &[PathBuf], to: &'static str) -> Dropped {
    let mut out = Dropped {
        copied: Vec::new(),
        refused: Vec::new(),
        to,
    };
    for path in paths {
        match copy_to_inbox(path) {
            Ok(copy) => out.copied.push(copy),
            Err(reason) => out.refused.push(Refused {
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                reason,
            }),
        }
    }
    out
}

/// Something is being dragged over the island (true) or left it (false).
pub fn on_drag(app: &AppHandle, over: bool) {
    if !crate::settings::zeca(app) {
        return;
    }
    let _ = app.emit_to(ISLAND, "drag", over);
}

fn copy_to_inbox(src: &Path) -> Result<String, &'static str> {
    let meta = std::fs::metadata(src).map_err(|_| "unreadable")?;
    if meta.is_dir() {
        return Err("folder");
    }
    if !meta.is_file() {
        return Err("unreadable");
    }
    if meta.len() > MAX_FILE {
        return Err("too-big");
    }
    copy_file(src).ok_or("unreadable")
}

fn copy_file(src: &Path) -> Option<String> {
    copy_file_into(src, &paths::inbox_dir())
}

fn copy_file_into(src: &Path, dir: &Path) -> Option<String> {
    use std::io::Read;
    std::fs::create_dir_all(dir).ok()?;
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let name = src.file_name()?.to_string_lossy();
    let dest = dir.join(format!("{stamp}-{name}"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options.open(&dest).ok()?;
    let copied =
        std::fs::File::open(src).and_then(|file| std::io::copy(&mut file.take(MAX_FILE + 1), &mut output));
    if copied.is_err() || copied.is_ok_and(|len| len > MAX_FILE) {
        drop(output);
        let _ = std::fs::remove_file(dest);
        return None;
    }
    Some(dest.to_string_lossy().into_owned())
}

/// Deletes inbox copies older than a week.
pub fn clean_inbox() {
    let Ok(entries) = std::fs::read_dir(paths::inbox_dir()) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|age| age > INBOX_KEEP);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn specialist_archive_blocks_new_turns_with_every_executor() {
        use vults_store::profiles::{ExecutionDefaults, ProfileState};
        let mut state = ProfileState::default();
        assert!(super::check_specialist_state(&state).is_ok());
        for defaults in [
            ExecutionDefaults::Codex {
                model: Some("test-model".into()),
                effort: None,
            },
            ExecutionDefaults::Api {
                provider: "ollama".into(),
                model: None,
            },
        ] {
            state.defaults = defaults;
            assert!(super::check_specialist_state(&state).is_ok());
            assert!(super::check_specialist_state(&state).is_ok());
        }
        state.archived = true;
        assert!(super::check_specialist_state(&state).is_err());
        assert!(super::check_specialist_state(&state).is_err());
        state.archived = false;
        assert!(super::check_specialist_state(&state).is_ok());
    }
    #[test]
    fn specialist_api_resume_pins_provider_model_messages_and_profile() {
        use vults_store::profiles::ExecutionDefaults;
        let profile = Snapshot {
            source: vults_store::profiles::Source::Preset {
                id: "developer".into(),
                revision: 1,
            },
            title: "Developer".into(),
            about: "Description".into(),
            instructions: "PINNED_ONLY".into(),
            species: "atratus".into(),
            look: "none".into(),
            skills: Vec::new(),
        };
        let dir = std::env::temp_dir();
        let defaults = ExecutionDefaults::Api {
            provider: "ollama".into(),
            model: Some("local-model".into()),
        };
        let chat =
            specialist_from_defaults(&profile, &defaults, paths::chat_dir(), Some("other-model")).unwrap();
        assert_eq!(chat.api().0.id, "ollama");
        assert_eq!(chat.api().1, "local-model");
        assert!(chat.frozen().is_none());
        let directory = std::env::temp_dir().join(format!("vults-specialist-api-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let mut store = vults_store::Store::open(&directory).unwrap();
        let id = store
            .conversation_new(&new_conversation(&chat, "Hello", &[], Some(profile.clone())), 1)
            .unwrap();
        store
            .conversation_save(
                id,
                None,
                Some(r#"[{"role":"user","content":"own"},{"role":"assistant","content":"answer"}]"#),
                None,
                2,
            )
            .unwrap();
        let record = store.conversation(id).unwrap().unwrap();
        let (restored, pinned) = restore_conversation(
            &record,
            vults_agents::driver::Spec::in_dir(paths::chat_dir()),
            Some((providers::find("ollama").unwrap(), "local-model".into())),
        )
        .unwrap();
        assert!(restored.started());
        assert_eq!(pinned, Some(profile.clone()));
        assert_eq!(
            turn_profile(&restored, Some(&record), Some(&profile)).unwrap(),
            Some(profile)
        );
        let mut changed = record.clone();
        changed.engine = "api:openai".into();
        assert!(turn_profile(&restored, Some(&changed), pinned.as_ref()).is_err());
        changed = record.clone();
        changed.frozen = Some("PRIVATE".into());
        assert!(
            restore_conversation(
                &changed,
                vults_agents::driver::Spec::in_dir(dir.clone()),
                Some((providers::find("ollama").unwrap(), "local-model".into()))
            )
            .is_err()
        );
        assert!(specialist_project(Some("relative"), &ExecutionDefaults::default(), false, &dir).is_err());
        assert!(
            specialist_project(
                Some("/nonexistent-vults-project"),
                &ExecutionDefaults::default(),
                false,
                &dir
            )
            .is_err()
        );
        let canonical = std::fs::canonicalize(&dir).unwrap();
        assert_eq!(
            specialist_project(dir.to_str(), &ExecutionDefaults::default(), false, &dir).unwrap(),
            canonical
        );
        assert!(specialist_project(Some("/"), &ExecutionDefaults::default(), true, &canonical).is_err());
        assert!(specialist_project(dir.to_str(), &defaults, false, &dir).is_err());
        assert!(
            specialist_from_defaults(
                pinned.as_ref().unwrap(),
                &ExecutionDefaults::Api {
                    provider: "unknown".into(),
                    model: Some("model".into())
                },
                dir,
                None
            )
            .is_err()
        );
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn conversation_choices_preserve_the_draft_and_profile_and_fence_old_actions() {
        use vults_store::profiles::ExecutionDefaults;
        let profile = Snapshot {
            source: vults_store::profiles::Source::Preset {
                id: "developer".into(),
                revision: 1,
            },
            title: "Developer".into(),
            about: "Description".into(),
            instructions: "Pinned".into(),
            species: "atratus".into(),
            look: "none".into(),
            skills: Vec::new(),
        };
        let defaults = ExecutionDefaults::Codex {
            model: Some("codex-test-model".into()),
            effort: Some("high".into()),
        };
        let codex = specialist_from_defaults(&profile, &defaults, paths::chat_dir(), None).unwrap();
        assert_eq!(codex.provider(), Provider::Codex);
        let directory =
            std::env::temp_dir().join(format!("vults-codex-profile-resume-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let mut store = vults_store::Store::open(&directory).unwrap();
        let record = new_conversation(&codex, "Codex question", &[], Some(profile.clone()));
        assert_eq!(record.engine, "codex");
        assert_eq!(record.profile, Some(profile.clone()));
        let id = store.conversation_new(&record, 1).unwrap();
        store
            .conversation_save(id, Some("codex-profile-thread"), None, None, 2)
            .unwrap();
        let saved = store.conversation(id).unwrap().unwrap();
        let (restored, pinned) = restore_conversation(
            &saved,
            vults_agents::driver::Spec::in_dir(paths::chat_dir()),
            None,
        )
        .unwrap();
        assert_eq!(restored.provider(), Provider::Codex);
        assert_eq!(restored.session().as_deref(), Some("codex-profile-thread"));
        assert_eq!(pinned, Some(profile.clone()));
        assert_eq!(
            restored.specialist_brief(),
            Some(specialist_brief(&profile).as_str())
        );
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
        let state = ChatState::specialist(&profile).unwrap();
        state.draft.lock().unwrap().text = "Keep my words".into();
        let choice = ExecutionDefaults::Api {
            provider: "ollama".into(),
            model: Some("local-model".into()),
        };
        let fresh = specialist_from_defaults(&profile, &choice, paths::chat_dir(), None).unwrap();
        let generation = state.generation();
        let mut chat = state.chat.try_lock().unwrap();
        install_specialist_choice(&state, &mut chat, fresh, Some(choice.clone()), generation).unwrap();
        assert_eq!(state.profile().unwrap(), Some(profile.clone()));
        assert_eq!(state.draft.lock().unwrap().text, "Keep my words");
        assert_eq!(state.specialist_choice.lock().unwrap().as_ref(), Some(&choice));
        assert_eq!(chat.provider(), Provider::ClaudeApi);
        assert!(choice_editable(&state, &chat, generation).is_err());
        assert!(choice_editable(&state, &chat, state.generation()).is_ok());
        state.set_saved(Some(ConversationId(99)));
        assert!(
            choice_editable(&state, &chat, state.generation()).is_err(),
            "A failed first model turn still binds the saved executor"
        );
        state.set_current(None);
        assert!(state.specialist_choice.lock().unwrap().is_none());
        assert!(state.draft.lock().unwrap().text.is_empty());
        let mut other = profile.clone();
        other.instructions = "Other source".into();
        let foreign = Chat::specialist(
            vults_agents::driver::Spec::in_dir(paths::chat_dir()),
            None,
            specialist_brief(&other),
        )
        .unwrap();
        assert!(install_specialist_choice(&state, &mut chat, foreign, None, state.generation()).is_err());
        assert_eq!(
            chat.provider(),
            Provider::ClaudeApi,
            "A rejected source cannot replace the existing runtime"
        );
        let companion = ChatState::new();
        assert!(
            choice_editable(
                &companion,
                &companion.chat.try_lock().unwrap(),
                companion.generation()
            )
            .is_err()
        );
    }

    #[test]
    fn attachments_are_private_inbox_copies_with_bounded_unambiguous_paths() {
        let root = std::env::temp_dir().join(format!("vults-attachments-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let inbox = root.join("inbox");
        let source = root.join("notes.txt");
        std::fs::write(&source, "USER_FILE").unwrap();
        let copied = copy_file_into(&source, &inbox).unwrap();
        let second = copy_file_into(&source, &inbox).unwrap();
        assert_ne!(copied, second);
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "USER_FILE");
        assert_eq!(std::fs::read_to_string(&copied).unwrap(), "USER_FILE");
        assert_eq!(
            inbox_files(&inbox, std::slice::from_ref(&copied)).unwrap(),
            vec![std::fs::canonicalize(&copied).unwrap()]
        );
        assert!(inbox_files(&inbox, &[copied.clone(), copied.clone()]).is_err());
        assert!(inbox_files(&inbox, &vec![copied.clone(); MAX_ATTACHMENTS + 1]).is_err());
        for invalid in [
            source.to_string_lossy().into_owned(),
            format!("{}/../notes.txt", inbox.display()),
            "relative.txt".into(),
            inbox.join("missing").to_string_lossy().into_owned(),
        ] {
            assert!(inbox_files(&inbox, &[invalid]).is_err());
        }
        let child = inbox.join("child");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("nested.txt"), "NESTED").unwrap();
        assert!(inbox_files(&inbox, &[child.to_string_lossy().into_owned()]).is_err());
        assert!(inbox_files(&inbox, &[child.join("nested.txt").to_string_lossy().into_owned()]).is_err());
        let large = inbox.join("large");
        let file = std::fs::File::create(&large).unwrap();
        file.set_len(MAX_FILE).unwrap();
        drop(file);
        assert!(inbox_files(&inbox, &[copied.clone(), large.to_string_lossy().into_owned()]).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::{fs::PermissionsExt, fs::symlink};
            assert_eq!(
                std::fs::metadata(&copied).unwrap().permissions().mode() & 0o077,
                0
            );
            let link = inbox.join("link");
            symlink(&source, &link).unwrap();
            assert!(inbox_files(&inbox, &[link.to_string_lossy().into_owned()]).is_err());
            std::fs::remove_file(&link).unwrap();
            symlink(&copied, &link).unwrap();
            assert!(inbox_files(&inbox, &[link.to_string_lossy().into_owned()]).is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stop_during_preflight_cancels_once_and_failed_preflight_cleans_up() {
        let state = ChatState::new();
        let (tx, mut rx) = oneshot::channel();
        *state.stop.lock().unwrap() = Some(tx);
        let guard = TurnStop(&state);
        end_turn(&state);
        assert!(rx.try_recv().is_ok());
        drop(guard);
        let (tx, mut rx) = oneshot::channel();
        *state.stop.lock().unwrap() = Some(tx);
        drop(TurnStop(&state));
        assert!(matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed)));
    }

    #[test]
    fn specialist_empty_runtime_keeps_its_generation_and_exact_brief() {
        let profile = Snapshot {
            source: vults_store::profiles::Source::Preset {
                id: "developer".into(),
                revision: 1,
            },
            title: "Developer".into(),
            about: "Description".into(),
            instructions: "Pinned brief".into(),
            species: "atratus".into(),
            look: "none".into(),
            skills: vec![],
        };
        let registry = SpecialistChatState::default();
        let first = registry.runtime("developer", &profile).unwrap();
        first.set_current(None);
        let mut edited = profile.clone();
        edited.instructions = "Later brief".into();
        let second = registry.runtime("developer", &edited).unwrap();
        assert!(Arc::ptr_eq(&first.0, &second.0));
        let view = specialist_current_view(&second, None).unwrap();
        assert_eq!(view.generation, first.generation());
        let reset = serde_json::to_value(SpecialistReset {
            provider: Provider::Claude,
            current: view,
        })
        .unwrap();
        assert_eq!(reset["generation"], first.generation());
        assert_eq!(reset["profileSpecies"], "atratus");
        assert_eq!(reset["profileLook"], "none");
        assert_eq!(reset["profileRevision"], 1);
        assert_eq!(reset["provider"], "claude");
        assert!(reset["conversation"].is_null());
        assert_eq!(second.profile().unwrap(), Some(profile.clone()));
        let chat = second.chat.try_lock().unwrap();
        assert!(turn_profile(&chat, None, Some(&edited)).is_err());
        assert_eq!(turn_profile(&chat, None, Some(&profile)).unwrap(), Some(profile));
    }

    #[test]
    fn specialist_brief_includes_exact_pinned_skill_content() {
        let profile = Snapshot {
            source: vults_store::profiles::Source::Preset {
                id: "developer".into(),
                revision: 1,
            },
            title: "Developer".into(),
            about: "Description".into(),
            instructions: "Pinned instructions".into(),
            species: "atratus".into(),
            look: "none".into(),
            skills: vec![vults_store::profiles::SkillSnapshot {
                id: "verify-change".into(),
                revision: 2,
                title: "Verify".into(),
                content: "Pinned check text".into(),
            }],
        };
        let brief = super::specialist_brief(&profile);
        assert!(brief.starts_with("Pinned instructions\n\nPinned skills:"));
        assert!(brief.contains("## Verify (revision 2)\nPinned check text"));
    }

    #[test]
    fn companion_actions_reject_stale_sources_and_specialist_ownership() {
        let state = ChatState::new();
        let generation = state.generation();
        let held = state.companion_command(generation, None).unwrap();
        assert!(
            state.chat.try_lock().is_err(),
            "Keep the recipient until the action completes"
        );
        assert!(
            state.companion_command(generation, None).is_err(),
            "Do not queue local actions"
        );
        drop(held);
        state.set_current(None);
        assert!(state.companion_command(generation, None).is_err());
        assert!(state.check_companion(generation).is_err());
        let generation = state.generation();
        let profile = Snapshot {
            source: vults_store::profiles::Source::Preset {
                id: "developer".into(),
                revision: 1,
            },
            title: "Developer".into(),
            about: "Description".into(),
            instructions: "Pinned instructions".into(),
            species: "atratus".into(),
            look: "none".into(),
            skills: vec![],
        };
        let dir = std::env::temp_dir();
        *state.chat.try_lock().unwrap() = Chat::resume(
            Provider::Claude,
            vults_agents::driver::Spec::in_dir(dir.clone()),
            Some("retained-session".into()),
            None,
            None,
        );
        assert!(state.chat.try_lock().unwrap().started());
        assert!(state.companion_command(generation, dir.to_str()).is_ok());
        assert!(
            state
                .companion_command(generation, Some("/different-project"))
                .is_err()
        );
        state.set_profile(Some(profile.clone()));
        assert!(state.companion_command(generation, None).is_err());
        assert!(state.check_companion(generation).is_err());
        state.set_profile(None);
        *state.chat.try_lock().unwrap() = Chat::specialist(
            vults_agents::driver::Spec::in_dir(std::env::temp_dir()),
            None,
            profile.instructions,
        )
        .unwrap();
        assert!(
            state.companion_command(generation, None).is_err(),
            "Inconsistent metadata never grants companion access"
        );
    }

    #[test]
    fn navigation_never_waits_for_a_turn_or_accepts_an_obsolete_source() {
        let state = ChatState::new();
        state.set_current(Some(ConversationId(42)));
        state.draft.lock().unwrap().text = "Retain this draft".into();
        let generation = state.generation();
        let running = state.chat.try_lock().unwrap();
        assert_eq!(state.navigation(generation).err().unwrap(), "Chat is busy.");
        assert_eq!(state.current(), Some(ConversationId(42)));
        assert_eq!(state.draft.lock().unwrap().text, "Retain this draft");
        drop(running);
        state.set_current(Some(ConversationId(43)));
        state.draft.lock().unwrap().text = "The newer source".into();
        assert!(state.navigation(generation).is_err());
        assert_eq!(state.current(), Some(ConversationId(43)));
        assert_eq!(state.draft.lock().unwrap().text, "The newer source");
        assert!(state.navigation(state.generation()).is_ok());
    }

    #[test]
    fn sends_stay_bound_across_empty_reset_open_and_first_save() {
        let state = super::ChatState::new();
        let initial = state.generation();
        assert!(state.check_generation(initial).is_ok());
        state.set_saved(Some(super::ConversationId(42)));
        assert!(state.check_generation(initial).is_ok());
        state.set_current(None);
        assert!(state.check_generation(initial).is_err());
        let empty = state.generation();
        state.set_current(None);
        assert!(state.check_generation(empty).is_err());
        let second_empty = state.generation();
        state.set_current(Some(super::ConversationId(42)));
        assert!(state.check_generation(second_empty).is_err());
        let opened = state.generation();
        state.set_current(Some(super::ConversationId(42)));
        assert!(state.check_generation(opened).is_err());
    }

    use super::*;
    use vults_chat::Settings;

    #[cfg(target_os = "linux")]
    #[test]
    fn voice_handoff_rechecks_opt_in_draft_ticket_and_never_queues() {
        let app = tauri::test::mock_app();
        app.manage(SettingsState(std::sync::Mutex::new(
            crate::settings::Settings::default(),
        )));
        app.manage(ChatState::new());
        let ticket = crate::dictation::worker::Ticket::test_command();
        let origin = Origin::Voice(ticket.clone(), app.state::<ChatState>().generation());
        assert!(origin.check(app.handle()).is_err(), "No implicit model grant");
        let settings = app.state::<SettingsState>();
        settings.0.lock().unwrap().dictation_command_chat = true;
        assert!(origin.check(app.handle()).is_ok());
        let chat = app.state::<ChatState>();
        let reserved = origin.reserve(&chat).unwrap().unwrap();
        assert!(
            origin.reserve(&chat).is_err(),
            "A busy Chat cannot queue a voice turn"
        );
        drop(reserved);
        for draft in [Draft { text: "unfinished".into(), ..Draft::default() },
            Draft { files: vec!["/inbox/file".into()], ..Draft::default() },
            serde_json::from_value::<Draft>(serde_json::json!({"text":"", "files":[], "folder":null,"provider":null,"mentions":[{"token":"@api", "target":{"kind":"session", "agent":"codex", "id":"a"}}]})).unwrap()] {
            *chat.draft.lock().unwrap() = draft;
            assert!(origin.check(app.handle()).is_err(), "Never inherit a composer draft");
        }
        *chat.draft.lock().unwrap() = Draft::default();
        settings.0.lock().unwrap().zeca = false;
        assert!(origin.check(app.handle()).is_err());
        settings.0.lock().unwrap().zeca = true;
        settings.0.lock().unwrap().dictation_commands = false;
        assert!(origin.check(app.handle()).is_err());
        settings.0.lock().unwrap().dictation_commands = true;
        assert!(origin.check(app.handle()).is_ok());
        chat.set_current(None);
        assert!(
            origin.check(app.handle()).is_err(),
            "An identical empty composer is a different recipient generation"
        );
        let origin = Origin::Voice(ticket.clone(), chat.generation());
        assert!(origin.check(app.handle()).is_ok());
        ticket.test_cancel();
        assert!(
            origin.check(app.handle()).is_err(),
            "A later grant cannot revive a cancelled ticket"
        );
        assert!(!chat.live.lock().unwrap().busy);
        assert_eq!(chat.current(), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn voice_preflight_keeps_the_selected_folder_and_rejects_changed_sources() {
        let state = ChatState::new();
        let expected = Draft {
            folder: Some("/work/api".into()),
            provider: Some(Provider::Codex),
            ..Draft::default()
        };
        *state.draft.lock().unwrap() = expected.clone();
        assert!(check_voice_source(&state, &expected, Provider::Codex).is_ok());
        assert!(check_voice_source(&state, &expected, Provider::Claude).is_err());
        state.draft.lock().unwrap().folder = Some("/work/web".into());
        assert!(check_voice_source(&state, &expected, Provider::Codex).is_err());
        *state.draft.lock().unwrap() = expected.clone();
        state.draft.lock().unwrap().provider = Some(Provider::Claude);
        assert!(check_voice_source(&state, &expected, Provider::Codex).is_err());
    }

    #[test]
    fn timed_out_chat_offers_leave_the_inbox_and_reopened_chat() {
        let waiting = Waiting::default();
        let live = waiting.wait("live", "Bash", "git status");
        let expired = waiting.wait("expired", "Bash", "git diff");
        assert_eq!(waiting.pending_count(), 2);
        drop(expired);
        assert_eq!(waiting.pending_count(), 1);
        assert_eq!(waiting.shown().len(), 1);
        assert_eq!(waiting.shown()[0].id, "live");
        drop(live);
        assert_eq!(waiting.pending_count(), 0);
        assert!(waiting.shown().is_empty());
    }

    #[test]
    fn draft_roundtrip_keeps_multiline_text_files_and_context() {
        let draft = Draft {
            text: "Review this change\nThen run its tests".into(),
            files: vec!["/inbox/change.patch".into()],
            folder: Some("/project".into()),
            provider: Some(Provider::Codex),
            mentions: vec![crate::mentions::Mention {
                token: "@\"site\"".into(),
                target: crate::mentions::Target::Session {
                    agent: "codex".into(),
                    id: "exact-session".into(),
                },
            }],
        };
        draft.validate().unwrap();
        let restored: Draft = serde_json::from_str(&serde_json::to_string(&draft).unwrap()).unwrap();
        assert_eq!(restored.text, draft.text);
        assert_eq!(restored.files, draft.files);
        assert_eq!(restored.folder, draft.folder);
        assert_eq!(restored.provider, draft.provider);
        assert_eq!(restored.mentions, draft.mentions);
        let old: Draft =
            serde_json::from_str(r#"{"text":"old draft","files":[],"folder":null,"provider":null}"#).unwrap();
        assert!(old.mentions.is_empty());
        assert!(
            Draft {
                text: "x".repeat(1024 * 1024 + 1),
                ..Draft::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            Draft {
                files: vec!["file".into(); 257],
                ..Draft::default()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn an_engine_is_missing_installed_or_signed_in() {
        let gone = Path::new("/nonexistent/vults/auth.json");
        let here = std::env::current_exe().unwrap();
        assert_eq!(engine(false, &here), "missing");
        assert_eq!(engine(true, gone), "installed");
        assert_eq!(engine(true, &here), "signed-in");
    }

    fn policy(text: &str) -> Vec<vults_core::policies::Active> {
        vec![vults_core::policies::Active {
            consent: 9,
            policy: vults_core::policies::parse(text, Some("/home/u")).unwrap(),
        }]
    }

    fn call(tool: &str, field: &str, value: &str) -> vults_agents::driver::Ask {
        vults_agents::driver::Ask {
            id: "1".into(),
            tool: tool.into(),
            target: format!("{tool} · {value}"),
            full: None,
            input: serde_json::json!({ field: value }),
        }
    }

    /// (allowed, by full access) of what the choice settles; None asks.
    fn settled(
        p: &[vults_core::policies::Active],
        access: settings::ChatAccess,
        agent: vults_core::AgentKind,
        ask: &vults_agents::driver::Ask,
        outside: bool,
    ) -> Option<(bool, bool)> {
        settle(p, access, "/home/u/site", agent, ask, outside)
            .map(|s| (s.answer.allows(), s.by == vults_chat::SettledBy::FullAccess))
    }

    #[test]
    fn specialist_actions_use_the_profile_access_choice() {
        use settings::ChatAccess::{Ask, Full, Policies};
        use vults_core::AgentKind::Claude;
        let allow = policy("allow read");
        let read = call("Read", "file_path", "/home/u/site/src/main.rs");
        assert!(settle_in_route(Some(Ask), &allow, Full, "/home/u/site", Claude, &read, false).is_none());
        assert_eq!(
            settled_profile(&allow, Policies, Claude, &read, false),
            Some((true, false))
        );
        assert_eq!(
            settled_profile(&allow, Full, Claude, &read, false),
            Some((true, false))
        );
        let list = call("Bash", "command", "ls");
        assert_eq!(
            settled_profile(&[], Full, Claude, &list, false),
            Some((true, true))
        );
        let wipe = call("Bash", "command", "rm -rf /tmp/work");
        assert!(settle_in_route(Some(Full), &[], Ask, "/home/u/site", Claude, &wipe, false).is_none());
        assert!(settle_in_route(Some(Full), &[], Ask, "/home/u/site", Claude, &list, true).is_none());
    }

    #[test]
    fn zeca_off_blocks_only_companion_turns() {
        assert!(zeca_switch_blocks_turn(false, false));
        assert!(!zeca_switch_blocks_turn(false, true));
        assert!(!zeca_switch_blocks_turn(true, false));
        assert!(!zeca_switch_blocks_turn(true, true));
    }

    #[test]
    fn built_in_api_tools_require_a_supported_companion_provider() {
        assert!(api_companion_tools_enabled(true, Provider::ClaudeApi, true));
        assert!(!api_companion_tools_enabled(true, Provider::ClaudeApi, false));
        assert!(!api_companion_tools_enabled(false, Provider::ClaudeApi, true));
        assert!(!api_companion_tools_enabled(true, Provider::Claude, true));
        assert!(!api_companion_tools_enabled(true, Provider::Codex, true));
    }

    fn settled_profile(
        policies: &[vults_core::policies::Active],
        access: settings::ChatAccess,
        agent: vults_core::AgentKind,
        ask: &vults_agents::driver::Ask,
        outside: bool,
    ) -> Option<(bool, bool)> {
        settle_in_route(
            Some(access),
            policies,
            settings::ChatAccess::Ask,
            "/home/u/site",
            agent,
            ask,
            outside,
        )
        .map(|s| (s.answer.allows(), s.by == vults_chat::SettledBy::FullAccess))
    }

    #[test]
    fn the_chats_access_answers_what_the_user_chose_and_nothing_more() {
        use settings::ChatAccess::{Full, Policies};
        use vults_core::AgentKind::{Claude, Codex};
        let ls = call("Bash", "command", "ls");
        let read = call("Read", "file_path", "/home/u/site/src/main.rs");
        let push = call("Bash", "command", "git push origin main");
        let fetch = call("Bash", "command", "curl https://example.com");
        let wipe = call("Bash", "command", "rm -rf build");
        let p = policy("allow read\nask command git push\ndeny command curl");
        // Their policies: what they allow and deny, as on the island; the rest asks.
        assert_eq!(settled(&p, Policies, Claude, &read, false), Some((true, false)));
        assert_eq!(settled(&p, Policies, Claude, &fetch, false), Some((false, false)));
        assert_eq!(settled(&p, Policies, Claude, &ls, false), None);
        assert_eq!(
            settled(&p, Policies, Codex, &read, false),
            None,
            "a policy allows only Claude Code's calls"
        );
        // Full access: the rest runs, but a destructive command still asks (ADR 0004), and what a
        // policy denies or asks first still does.
        assert_eq!(settled(&p, Full, Claude, &ls, false), Some((true, true)));
        assert_eq!(settled(&p, Full, Codex, &ls, false), Some((true, true)));
        assert_eq!(settled(&p, Full, Claude, &wipe, false), None);
        assert_eq!(settled(&p, Full, Claude, &push, false), None);
        assert_eq!(settled(&p, Full, Claude, &fetch, false), Some((false, false)));
        assert_eq!(settled(&[], Full, Claude, &ls, false), Some((true, true)));
        // Once the conversation read from outside, full access gives way to a card; a policy's
        // own answer still holds.
        assert_eq!(settled(&p, Full, Claude, &ls, true), None);
        assert_eq!(settled(&p, Full, Claude, &read, true), Some((true, false)));
    }

    #[test]
    fn a_saved_conversation_names_its_engine_and_settings_the_way_back() {
        for p in [Provider::Claude, Provider::Codex, Provider::ClaudeApi] {
            let chat = Chat::new(p, paths::chat_dir());
            assert_eq!(engine_provider(&engine_name(&chat)), Some(p));
        }
        assert_eq!(engine_provider("gemini"), None);
        for s in [
            vults_chat::Settings::Isolated,
            vults_chat::Settings::User,
            vults_chat::Settings::Project,
        ] {
            assert_eq!(settings_of(settings_name(s)), s);
        }
        // An unknown word reads nothing of the user's.
        assert_eq!(settings_of("everything"), vults_chat::Settings::Isolated);
    }

    #[test]
    fn a_saved_session_never_reads_as_a_flag() {
        assert!(valid_session("480b2798-5d6b-4fa2-9bcd-7fbe117db81b"));
        assert!(!valid_session("--dangerously-skip-permissions"));
        assert!(!valid_session("a b"));
        assert!(!valid_session(""));
    }

    #[test]
    fn a_conversation_is_named_by_its_first_message_or_its_files() {
        let mut chat = Chat::new(Provider::ClaudeApi, paths::chat_dir());
        chat.set_api(&vults_chat::providers::PROVIDERS[1], "m-1".into());
        let new = new_conversation(&chat, "  ", &["shot.png".into()], None);
        assert_eq!(new.title, "shot.png");
        assert_eq!(
            new.engine,
            format!("api:{}", vults_chat::providers::PROVIDERS[1].id)
        );
        assert_eq!(new.model.as_deref(), Some("m-1"));
        assert_eq!(new.folder, "");
        assert_eq!(new_conversation(&chat, "Hi", &[], None).title, "Hi");
    }

    #[test]
    fn a_pick_is_kept_by_folder_with_the_chats_own_as_empty() {
        assert_eq!(folder_key(None), "");
        assert_eq!(folder_key(Some(&paths::chat_dir())), "");
        assert_eq!(folder_key(Some(Path::new("/home/me/site"))), "/home/me/site");
    }

    #[test]
    fn only_a_pick_the_cli_can_read_is_kept() {
        // Absolute on every OS (a path with no drive is relative on Windows).
        let site = std::env::temp_dir();
        assert!(check_pick(site.to_str(), Some("sonnet"), Some("high")).is_ok());
        assert!(check_pick(None, None, None).is_ok());
        assert!(check_pick(Some("site"), None, None).is_err());
        assert!(check_pick(None, Some("-x"), None).is_err());
        assert!(check_pick(None, None, Some("ultra")).is_err());
    }

    #[test]
    fn an_effort_the_model_does_not_take_is_left_to_the_cli() {
        use vults_chat::Effort;
        assert!(effort_fits(Provider::Claude, Some("sonnet"), Effort::Max));
        assert!(!effort_fits(Provider::Claude, Some("haiku"), Effort::Max));
        assert!(effort_fits(Provider::Claude, None, Effort::Max));
        assert!(
            effort_fits(Provider::Claude, Some("claude-sonnet-5-5"), Effort::Low),
            "not listed: the CLI decides"
        );
    }

    #[test]
    fn the_cli_reads_a_projects_settings_only_where_claude_code_trusts_it() {
        let tmp = std::env::temp_dir().join(format!("vults-cli-settings-{}", std::process::id()));
        let real = tmp.join("real");
        std::fs::create_dir_all(&real).unwrap();
        let real = real.canonicalize().unwrap();
        let state = tmp.join(".claude.json");
        let trusted =
            serde_json::json!({ "projects": { real.to_str().unwrap(): { "hasTrustDialogAccepted": true } } });
        std::fs::write(&state, trusted.to_string()).unwrap();
        assert_eq!(cli_settings(false, &real, &state), Settings::Isolated);
        assert_eq!(cli_settings(true, &real, &state), Settings::Project);
        // Claude Code keys the folder by its real path: a link to it is the same folder.
        #[cfg(unix)]
        {
            let link = tmp.join("link");
            std::os::unix::fs::symlink(&real, &link).unwrap();
            assert_eq!(cli_settings(true, &link, &state), Settings::Project);
        }
        assert_eq!(cli_settings(true, &tmp, &state), Settings::User);
        std::fs::write(&state, "not json").unwrap();
        assert_eq!(cli_settings(true, &real, &state), Settings::User);
        assert_eq!(
            cli_settings(true, &real, &tmp.join("missing.json")),
            Settings::User
        );
        std::fs::remove_dir_all(&tmp).unwrap();
    }
    #[test]
    fn specialist_records_are_hidden_from_zeca_and_listed_only_under_their_profile() {
        let dir = std::env::temp_dir().join(format!("vults-chat-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = vults_store::Store::open(&dir).unwrap();
        let chat = Chat::new(Provider::Claude, dir.clone());
        let mut request = new_conversation(&chat, "Hello", &[], None);
        assert!(request.profile.is_none());
        let normal = store.conversation_new(&request, 1).unwrap();
        assert!(opened(store.conversation(normal).unwrap().unwrap(), vec![], false).is_some());
        request.frozen = None;
        request.profile = Some(vults_store::profiles::Snapshot {
            source: vults_store::profiles::Source::Preset {
                id: "developer".into(),
                revision: 1,
            },
            title: "Developer".into(),
            about: "Description".into(),
            instructions: "Implement changes".into(),
            species: "gyps-rueppelli".into(),
            look: "crown".into(),
            skills: vec![],
        });
        let specialist = store.conversation_new(&request, 2).unwrap();
        assert!(history_owner(&store, normal, None).is_ok());
        assert!(history_owner(&store, normal, Some("developer")).is_err());
        assert!(history_owner(&store, specialist, Some("developer")).is_ok());
        assert!(history_owner(&store, specialist, Some("researcher")).is_err());
        assert!(history_owner(&store, specialist, None).is_err());
        assert!(history_owner(&store, ConversationId(i64::MAX), None).is_err());
        let retained = store.conversation(specialist).unwrap().unwrap();
        assert!(opened(retained.clone(), vec![], false).is_none());
        let opened = opened_specialist(retained.clone(), vec![], false).unwrap();
        assert_eq!(opened.profile_species.as_deref(), Some("gyps-rueppelli"));
        assert_eq!(opened.profile_look.as_deref(), Some("crown"));
        let listed = specialist_conversation_view(retained.clone(), "developer").unwrap();
        assert_eq!(listed.id, specialist.0);
        assert_eq!(listed.profile_title, "Developer");
        assert_eq!(listed.profile_revision, 1);
        assert!(specialist_conversation_view(retained, "researcher").is_none());
        assert!(
            specialist_conversation_view(store.conversation(normal).unwrap().unwrap(), "developer").is_none()
        );
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn native_restore_and_turn_keep_the_exact_specialist_recipient_and_source() {
        let dir = std::env::temp_dir().join(format!("vults-native-profile-source-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = vults_store::Store::open(&dir).unwrap();
        let profile = vults_store::profiles::Snapshot {
            source: vults_store::profiles::Source::Preset {
                id: "developer".into(),
                revision: 1,
            },
            title: "Developer".into(),
            about: "Description".into(),
            instructions: "Pinned specialist instructions".into(),
            species: "atratus".into(),
            look: "none".into(),
            skills: vec![],
        };
        let id = store
            .conversation_new(
                &conversations::New {
                    engine: "claude".into(),
                    folder: dir.to_string_lossy().into_owned(),
                    title: "Specialist".into(),
                    model: Some("sonnet".into()),
                    settings: "isolated".into(),
                    profile: Some(profile.clone()),
                    ..Default::default()
                },
                1,
            )
            .unwrap();
        let mut record = store.conversation(id).unwrap().unwrap();
        record.outside = Some("External evidence".into());
        let unsafe_spec = vults_agents::driver::Spec {
            settings: vults_chat::Settings::Project,
            system_append: Some("PRIVATE_MEMORY".into()),
            mcp_config: Some("private-memory.json".into()),
            ..vults_agents::driver::Spec::in_dir(dir.clone())
        };
        let (chat, expected) = restore_conversation(&record, unsafe_spec, None).unwrap();
        assert!(chat.is_specialist());
        assert_eq!(chat.outside().as_deref(), Some("External evidence"));
        assert_eq!(chat.frozen(), None);
        assert_eq!(chat.settings(), vults_chat::Settings::Isolated);
        assert_eq!(
            turn_profile(&chat, Some(&record), expected.as_ref()).unwrap(),
            Some(profile.clone())
        );
        assert_eq!(
            turn_profile(&chat, None, expected.as_ref()).unwrap(),
            Some(profile.clone())
        );
        let started = Chat::specialist(
            vults_agents::driver::Spec::in_dir(dir.clone()),
            Some("session-1".into()),
            specialist_brief(&profile),
        )
        .unwrap();
        assert!(turn_profile(&started, None, expected.as_ref()).is_err());
        let fresh = Chat::specialist(
            vults_agents::driver::Spec::in_dir(dir.clone()),
            None,
            specialist_brief(&profile),
        )
        .unwrap();
        assert_eq!(
            turn_profile(&fresh, None, Some(&profile)).unwrap(),
            Some(profile.clone())
        );
        let plain = Chat::new(Provider::Claude, dir.clone());
        assert!(turn_profile(&plain, Some(&record), None).is_err());
        assert!(turn_profile(&chat, Some(&record), None).is_err());
        let state = ChatState::new();
        state.set_profile(expected.clone());
        assert_eq!(state.profile().unwrap(), expected);
        state.set_profile(None);
        assert!(state.profile().unwrap().is_none());
        let saved = new_conversation(&chat, "Hello", &[], Some(profile.clone()));
        assert_eq!(saved.profile, Some(profile));
        assert!(saved.frozen.is_none());
        store.conversation_restart(id, &saved).unwrap();
        for field in [
            "identity",
            "revision",
            "title",
            "appearance",
            "brief",
            "source",
            "model",
            "effort",
            "session",
            "frozen",
            "history",
            "engine",
        ] {
            let mut changed = record.clone();
            match field {
                "identity" => {
                    changed.profile.as_mut().unwrap().source = vults_store::profiles::Source::Preset {
                        id: "reviewer".into(),
                        revision: 1,
                    }
                }
                "revision" => {
                    changed.profile.as_mut().unwrap().source = vults_store::profiles::Source::Preset {
                        id: "developer".into(),
                        revision: 2,
                    }
                }
                "title" => changed.profile.as_mut().unwrap().title = "Changed name".into(),
                "appearance" => changed.profile.as_mut().unwrap().look = "crown".into(),
                "brief" => changed.profile.as_mut().unwrap().instructions = "Changed".into(),
                "source" => changed.folder = "different-source".into(),
                "model" => changed.model = Some("different-model".into()),
                "effort" => changed.effort = Some("max".into()),
                "session" => changed.session = Some("different-session".into()),
                "frozen" => changed.frozen = Some("PRIVATE_MEMORY".into()),
                "history" => changed.history = Some("PRIVATE_HISTORY".into()),
                _ => changed.engine = "codex".into(),
            }
            assert!(
                turn_profile(&chat, Some(&changed), expected.as_ref()).is_err(),
                "{field}"
            );
        }
        let mut invalid = record.clone();
        invalid.session = Some("--wrong".into());
        assert!(
            restore_conversation(&invalid, vults_agents::driver::Spec::in_dir(dir.clone()), None).is_err()
        );
        invalid = record.clone();
        invalid.frozen = Some("PRIVATE_MEMORY".into());
        assert!(
            restore_conversation(&invalid, vults_agents::driver::Spec::in_dir(dir.clone()), None).is_err()
        );
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
