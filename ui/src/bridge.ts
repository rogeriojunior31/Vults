// The only file that talks to Tauri: views in, intents out.
import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import type { AgentKind, Diff, ViewModel } from "./view.gen";
import type { Activity } from "./surfaces/settings/activity";

// The core's view, generated from crates/core (see `mod ts` in view.rs); the island imports it from here.
export type {
  Activity,
  AgentKind,
  AlertLevel,
  AlertView,
  ApprovalView,
  Attention,
  BoardView,
  CardView,
  Checks,
  Choice,
  Diff,
  DiffSummary,
  DigestView,
  EndedView,
  FileDiff,
  Group,
  Hunk,
  Outcome,
  Outfit,
  Question,
  RowView,
  SessionRef,
  SessionView,
  Silence,
  Status,
  Verdict,
  ViewModel,
} from "./view.gen";

/** The song on screen, from the system's media players. */
export interface NowPlaying {
  title: string;
  artist: string | null;
  playing: boolean;
}
/** Speech to text in the chat: the models, and whether one is ready. */
export interface VoiceStatus {
  models: { id: string; label: string; size: number; installed: boolean }[];
  selected: string | null;
  /** As chosen: a code, `auto`, or null to follow the system. */
  language: string | null;
  /** The system's language code. */
  system: string | null;
  downloading: string[];
  ready: boolean;
}
export type MediaAction = "playpause" | "next" | "previous";

/** One rate-limit window of a subscription, as its CLI reports it. */
export interface UsageWindow {
  agent: AgentKind;
  /** The window's length: 300 is 5 hours, 10080 a week. */
  minutes: number;
  used_percent: number;
  /** Epoch seconds. */
  resets_at: number | null;
}

/** Who the chat talks through: a CLI, or a provider's API with the user's key (or a local model). */
export type ChatProvider = "claude" | "codex" | "api";
/** The API chat as the island sees it: usable now, and through whom. */
export interface ApiStatus {
  ready: boolean;
  label: string;
}

/** One provider in Settings → Chat. Whether a key is saved, never the key itself. */
export interface ApiProvider {
  id: string;
  label: string;
  local: boolean;
  keyHint: string;
  defaultModel: string;
  hasKey: boolean;
  model: string;
}

/** Where the flock draws its birds from. */
export type Flock = "brazil" | "americas" | "world";

/** A reply to one question: a choice or the user's own words, or several for a multi-select. */
export type Answer = string | string[];

export interface ConnectorStatus {
  id: string;
  enabled: boolean;
  lastOk: number | null;
  error: string | null;
  watching: number;
}

/** What the user chose for one project folder; a choice left out is off. */
export interface ProjectPrefs {
  mute?: boolean;
  pin?: boolean;
  hide?: boolean;
  /** Its flock's species, chosen by the user; absent, the pool draws it. */
  species?: string;
}
export type ProjectPref = "mute" | "pin" | "hide";
/** The answer to a quiet bird (crates/core `silence::Hush`). */
export type Hush = "snooze" | "keep-going" | "dismiss";

/** A permission the user chose to always allow: this exact tool and target, in this folder. */
export interface Rule {
  agent: AgentKind;
  cwd: string;
  tool: string;
  target: string;
}

/** An agent the installer sets up, by the name its hooks run with. Antigravity's and OpenCode's sessions are another tool's on
 *  the wire; OpenCode gets a plugin file instead of hooks. */
export type InstallAgent = Exclude<AgentKind, "other"> | "antigravity" | "opencode";

export interface InstallStatus {
  agent: InstallAgent;
  configPath: string;
  hookPath: string;
  hookReady: boolean;
  installed: boolean;
  /** Installed, but older than what this version writes: reinstalling brings what is new. */
  outdated: boolean;
  /** Outdated only because the hooks run the hook at this other path (another data folder). */
  otherHookPath: string | null;
  error: string | null;
  /** Why installing or updating would be refused now (the user's status line sits beside another data folder's hook). */
  installBlocked: string | null;
  /** Codex only: whether it will run our hooks. */
  codex: { hooksDisabled: boolean; untrusted: number; total: number } | null;
  /** Claude Code only: whose statusLine the config has. Installing over theirs keeps it running behind ours. */
  statusLine: "none" | "ours" | "theirs" | null;
}

export interface InstallPreview {
  diff: string;
  fingerprint: string;
}

export type ChatDelta =
  | { kind: "text"; text: string }
  | {
      kind: "permission";
      id: string;
      tool: string;
      target: string;
      description: string | null;
      full: string | null;
      added: number;
      removed: number;
    }
  | { kind: "done" }
  | { kind: "stopped" }
  | { kind: "error"; message: string };

/** Files dropped on the island: the inbox copies, and the ones refused with why. */
export interface Dropped {
  copied: string[];
  refused: { name: string; reason: "folder" | "too-big" | "unreadable" }[];
}

/** A setting that changed somewhere (the island, the tray, Settings); only those present changed. */
export type SettingsChange = { sounds?: boolean; volume?: number; api?: ApiStatus; foldAfter?: number; openOnHover?: boolean; voice?: boolean; zecaSpecies?: string; visitors?: boolean; presence?: Presence; zeca?: boolean; zecaLook?: string; widget?: Corner | null; projects?: Record<string, ProjectPrefs>; dndUntil?: number | null };

/** The presence preset (crates/core `Presence`). */
export type Presence = "island" | "panel" | "quiet" | "paused";
export type Place = { presence: Presence; dock: "top" | "bottom" };
/** The screen corner of the corner widget. */
export type Corner = "top-left" | "top-right" | "bottom-left" | "bottom-right";

export const Bridge = {
  onView(cb: (v: ViewModel) => void): void {
    void listen<ViewModel>("view", (e) => cb(e.payload));
    void invoke<ViewModel | null>("current_view").then((v) => v && cb(v));
  },
  decide: (request: string, decision: "allow" | "deny") => invoke<void>("decide", { request, decision }),
  layout: (x: number, y: number, width: number, height: number) =>
    invoke<void>("layout", { x, y, width, height }),
  installStatus: (agent: InstallAgent) => invoke<InstallStatus>("install_status", { agent }),
  installPreview: (agent: InstallAgent, install: boolean) =>
    invoke<InstallPreview>("install_preview", { agent, install }),
  installApply: (agent: InstallAgent, install: boolean, fingerprint: string) =>
    invoke<string | null>("install_apply", { agent, install, fingerprint }),
  decideAlways: (request: string) => invoke<void>("decide_always", { request }),
  questionAnswer: (request: string, answers: Answer[]) => invoke<void>("question_answer", { request, answers }),
  questionRelease: (request: string) => invoke<void>("question_release", { request }),
  rulesList: () => invoke<Rule[]>("rules_list"),
  ruleRemove: (index: number) => invoke<void>("rule_remove", { index }),
  sessionJump: (agent: AgentKind, id: string) => invoke<void>("session_jump", { agent, id }),
  /** Puts a session in front; null gives the choice back to core's rule. */
  sessionFocus: (s: { agent: AgentKind; id: string } | null) => invoke<void>("session_focus", { agent: s?.agent ?? null, id: s?.id ?? null }),
  /** Quick actions: the session's folder, or a file of a step's diff, in VS Code or the file manager. */
  sessionOpenFolder: (agent: AgentKind, id: string) => invoke<void>("session_open_folder", { agent, id }),
  sessionOpenFile: (agent: AgentKind, id: string, step: number, file: number) => invoke<void>("session_open_file", { agent, id, step, file }),
  /** A quick action: mute, pin or hide the session's project (its folder), or undo it. */
  sessionProjectPref: (agent: AgentKind, id: string, pref: ProjectPref, on: boolean) => invoke<void>("session_project_pref", { agent, id, pref, on }),
  sessionProjectBird: (agent: AgentKind, id: string, species: string | null) => invoke<void>("session_project_bird", { agent, id, species }),
  /** The answer to a quiet bird: it changes only its flag, never the agent. */
  sessionHush: (agent: AgentKind, id: string, hush: Hush) => invoke<void>("session_hush", { agent, id, hush }),
  /** Every project with a choice on, by folder. */
  projectsList: () => invoke<Record<string, ProjectPrefs>>("projects_list"),
  /** One project's choices from Settings; all off forgets it. */
  projectSet: (cwd: string, prefs: ProjectPrefs) => invoke<void>("project_set", { cwd, prefs: { mute: !!prefs.mute, pin: !!prefs.pin, hide: !!prefs.hide, species: prefs.species ?? null } }),
  /** Whether `code` is on the PATH, for the quick actions' words. */
  editorFound: () => invoke<boolean>("editor_found"),
  stepDiff: (agent: AgentKind, id: string, step: number) => invoke<Diff | null>("step_diff", { agent, id, step }),
  alertOpen: (key: string) => invoke<void>("alert_open", { key }),
  alertDismiss: (key: string) => invoke<void>("alert_dismiss", { key }),
  boardOpen: (connector: string, item: string) => invoke<void>("board_open", { connector, item }),
  connectorsStatus: () => invoke<ConnectorStatus[]>("connectors_status"),
  connectorEnable: (id: string, on: boolean) => invoke<void>("connector_enable", { id, on }),
  connectorsRefresh: () => invoke<void>("connectors_refresh"),
  /** Opens Settings, at one page (`agents`…) when given. */
  openSettings: (section?: string) => invoke<void>("open_settings_window", { section: section ?? null }),
  /** The app asks an open Settings window to show one page. */
  onSettingsSection(cb: (section: string) => void): void {
    void listen<string>("settings-section", (e) => cb(e.payload));
  },
  /** Zeca on or off: off, no chat, mic or talk shortcut; the flock keeps working. */
  setZeca: (on: boolean) => invoke<void>("set_zeca", { on }),
  appSettings: () =>
    invoke<{ sounds: boolean; volume: number; autostart: boolean; foldAfter: number; openOnHover: boolean; monitor: string | null; nowPlaying: boolean; zecaSpecies: string; zecaLook: string; flock: Flock; visitors: boolean; presence: Presence; notifications: boolean; zeca: boolean; widget: Corner | null; dndUntil: number | null; settingsPath: string; dataPath: string }>("app_settings"),
  /** "While you were away" read: it goes. */
  digestDismiss: () => invoke<void>("digest_dismiss"),
  recapDismiss: () => invoke<void>("recap_dismiss"),
  /** Do not disturb for this many minutes, or off with null: no sounds or notifications; cards still show. */
  setDnd: (minutes: number | null) => invoke<void>("set_dnd", { minutes }),
  /** Desktop notifications (finished, failed, a card waiting): on or off. */
  setNotifications: (on: boolean) => invoke<void>("set_notifications", { on }),
  /** The presence preset: Island, Panel (by the tray), Quiet or Paused. Switches at once. */
  setPresence: (presence: Presence) => invoke<void>("set_presence", { presence }),
  /** Where the island is now, for its page on load. */
  islandPlace: () => invoke<Place>("island_place"),
  onPlace(cb: (p: Place) => void): void {
    void listen<Place>("place", (e) => cb(e.payload));
  },
  /** Rare visitors crossing the sky: on or off. */
  setVisitors: (on: boolean) => invoke<void>("set_visitors", { on }),
  /** Zeca's species, by id (ui/src/character/flock/species.ts). */
  setZecaSpecies: (id: string) => invoke<void>("set_zeca_species", { id }),
  /** Zeca's look: "auto" (the calendar), "none", or a look id. The island gets it in the view. */
  setZecaLook: (look: string) => invoke<void>("set_zeca_look", { look }),
  /** Where the other sessions' birds are drawn from; their birds are drawn again at once. */
  setFlock: (flock: Flock) => invoke<void>("set_flock", { flock }),
  setNowPlaying: (on: boolean) => invoke<void>("set_now_playing", { on }),
  mediaControl: (action: MediaAction) => invoke<void>("media_control", { action }),
  /** The song on screen now, for an island that loads after it was sent. */
  mediaNow: () => invoke<NowPlaying | null>("media_now"),
  voiceStatus: () => invoke<VoiceStatus>("voice_status"),
  voiceDownload: (id: string) => invoke<void>("voice_download", { id }),
  voiceSelect: (id: string) => invoke<void>("voice_select", { id }),
  voiceLanguageSet: (language: string | null) => invoke<void>("voice_language_set", { language }),
  voiceOff: () => invoke<void>("voice_off"),
  /** `tap`: a click started it, so `onVoiceSilence` says when the user stopped talking. */
  voiceStart: (tap: boolean) => invoke<void>("voice_start", { tap }),
  voiceStop: () => invoke<string>("voice_stop"),
  voiceCancel: () => invoke<void>("voice_cancel"),
  /** The last usage read, for an island that loads after it. */
  usage: () => invoke<UsageWindow[]>("usage"),
  /** Connected monitors, by maker and model. */
  monitors: () => invoke<{ name: string; label: string }[]>("monitors"),
  /** A screen was plugged in or removed. */
  onMonitors(cb: () => void): void {
    void listen("monitors", () => cb());
  },
  /** `null` lets the desktop choose. */
  setMonitor: (name: string | null) => invoke<void>("set_monitor", { name }),
  setFoldAfter: (seconds: number) => invoke<void>("set_fold_after", { seconds }),
  setOpenOnHover: (on: boolean) => invoke<void>("set_open_on_hover", { on }),
  /** Settings → Activity: the week of `monday` (this week's when null), the weeks, the grid. */
  activity: (monday: string | null) => invoke<Activity>("activity", { monday }),
  setHistory: (on: boolean) => invoke<void>("set_history", { on }),
  clearHistory: () => invoke<void>("clear_history"),
  setAutostart: (on: boolean) => invoke<void>("set_autostart", { on }),
  setSounds: (on: boolean) => invoke<void>("set_sounds", { on }),
  /** Percent, 0 to 100. */
  setVolume: (percent: number) => invoke<void>("set_volume", { percent }),
  /** A setting changed somewhere; only the fields that changed are present. */
  onSettings(cb: (s: SettingsChange) => void): void {
    void listen<SettingsChange>("settings", (e) => cb(e.payload));
  },
  /** Whether the API chat can be used now; keys themselves never come back. */
  apiKeyStatus: () => invoke<ApiStatus>("api_key_status"),
  apiKeySet: (provider: string, key: string) => invoke<void>("api_key_set", { provider, key }),
  apiKeyClear: (provider: string) => invoke<void>("api_key_clear", { provider }),
  apiProviders: () => invoke<{ selected: string; providers: ApiProvider[] }>("api_providers"),
  apiProviderSet: (provider: string) => invoke<void>("api_provider_set", { provider }),
  apiModels: (provider: string) => invoke<string[]>("api_models", { provider }),
  apiModelSet: (provider: string, model: string) => invoke<void>("api_model_set", { provider, model }),
  chatSend: (text: string, files: string[], folder: string | null) => invoke<void>("chat_send", { text, files, folder }),
  chatDecide: (id: string, allow: boolean) => invoke<void>("chat_decide", { id, allow }),
  chatStop: () => invoke<void>("chat_stop"),
  chatReset: (provider: ChatProvider | null) => invoke<ChatProvider>("chat_reset", { provider }),
  islandKeyboard: (on: boolean) => invoke<void>("surface_keyboard", { on }),
  firstName: () => invoke<string | null>("first_name"),
  onChat(cb: (d: ChatDelta) => void): void {
    void listen<ChatDelta>("chat", (e) => cb(e.payload));
  },
  /** Files dropped on the island. */
  onFiles(cb: (dropped: Dropped) => void): void {
    void listen<Dropped>("files", (e) => cb(e.payload));
  },
  /** Something is dragged over the island (true), or it left (false). */
  onDrag(cb: (over: boolean) => void): void {
    void listen<boolean>("drag", (e) => cb(e.payload));
  },
  /** A global shortcut was pressed: "allow" or "deny". */
  /** The pointer came onto or left the island's window (GTK's word, in order; see `pointer`). */
  /** The mic's loudness (0..1) while recording, every 50 ms. */
  onVoiceLevel(cb: (level: number) => void): void {
    void listen<number>("voice-level", (e) => cb(e.payload));
  },
  /** A tap-to-talk recording heard speech and then a pause: time to stop it. */
  onVoiceSilence(cb: () => void): void {
    void listen<null>("voice-silence", () => cb());
  },
  /** While listening on a GPU: the text heard so far, about every 0.8 s. Not the final text. */
  onVoicePartial(cb: (text: string) => void): void {
    void listen<string>("voice-partial", (e) => cb(e.payload));
  },
  onVoiceDownload(cb: (p: { id: string; done: number; total: number }) => void): void {
    void listen<{ id: string; done: number; total: number }>("voice-download", (e) => cb(e.payload));
  },
  /** What is playing, while the setting is on; null when nothing is. */
  onMedia(cb: (now: NowPlaying | null) => void): void {
    void listen<NowPlaying | null>("media", (e) => cb(e.payload));
  },
  /** A fresh usage read: every few minutes, while a CLI answers. */
  onUsage(cb: (windows: UsageWindow[]) => void): void {
    void listen<UsageWindow[]>("usage", (e) => cb(e.payload));
  },
  onPointer(cb: (inside: boolean) => void): void {
    void listen<boolean>("pointer", (e) => cb(e.payload));
  },
  /** The user went to another of the app's windows (Settings): by the panel, the island folds. */
  onAway(cb: () => void): void {
    void listen("away", () => cb());
  },
  /** The corner widget was clicked: the island comes up (on the card, when one waits). */
  openIsland: () => emitTo("island", "open-island"),
  onOpenIsland(cb: () => void): void {
    void listen("open-island", () => cb());
  },
  /** The corner widget in a corner, or none. It comes, moves or goes at once. */
  setWidget: (corner: Corner | null) => invoke<void>("set_widget", { corner }),
  /** Settings was clicked: the island hears `onAway` ("island" is `ISLAND` in app/src/lib.rs). */
  away: () => emitTo("island", "away"),
  onShortcut(cb: (id: string) => void): void {
    void listen<string>("shortcut", (e) => cb(e.payload));
  },
  /** The keys bound so far (the binding may have happened before this window loaded). */
  shortcutKeys: () => invoke<Record<string, string>>("shortcut_keys"),
  /** The keys the desktop bound, by shortcut id, to show on the buttons. */
  onShortcutKeys(cb: (keys: Record<string, string>) => void): void {
    void listen<Record<string, string>>("shortcut-keys", (e) => cb(e.payload));
  },
  /** Open terminal found nothing to bring forward. */
  onJumpFailed(cb: () => void): void {
    void listen("jump-failed", () => cb());
  },
  onOpenChat(cb: () => void): void {
    void listen("open-chat", () => cb());
  },
};
