// The only file that talks to Tauri: views in, intents out.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  ready: boolean;
}
export type MediaAction = "playpause" | "next" | "previous";

/** `other`: any other tool, named by `agent_name`. */
export type AgentKind = "claude" | "codex" | "gemini" | "other";
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

export type Status =
  | "idle"
  | "thinking"
  | "working"
  | "approval"
  | "question"
  | "finished"
  | "failed"
  | "ratelimited";
export type Activity = "read" | "search" | "edit" | "run" | "web" | "plan" | "subagent" | "think" | "work";

export interface SessionView {
  id: string;
  agent: AgentKind;
  /** Another tool's own name, with `agent: "other"`. */
  agent_name?: string | null;
  project: string;
  cwd: string | null;
  status: Status;
  activity: Activity | null;
  step: string | null;
  /** The latest steps, oldest first. */
  steps: string[];
  step_count: number;
  subagents: number;
  /** The question, the last reply or the error that goes with the status. */
  note: string | null;
  /** The editor whose terminal the session runs in ("Cursor", "VS Code"). */
  editor: string | null;
}

export interface ApprovalView {
  request: string;
  agent: AgentKind;
  /** The session that asked. */
  session: string;
  project: string;
  tool: string;
  target: string;
  /** The agent's own words for the action ("Run the test suite"). */
  description: string | null;
  /** The whole command, when `target` had to cut it. */
  full: string | null;
  /** Lines an edit adds and removes; both 0 when it is not an edit. */
  added: number;
  removed: number;
  /** How many permissions wait, this one included. */
  queue: number;
}

export interface AlertView {
  key: string;
  connector: string;
  level: "info" | "ok" | "warn" | "error";
  title: string;
  detail: string;
  link: boolean;
}

export interface ViewModel {
  sessions: SessionView[];
  approval: ApprovalView | null;
  alerts: AlertView[];
}

export interface ConnectorStatus {
  id: string;
  enabled: boolean;
  lastOk: number | null;
  error: string | null;
  watching: number;
}

/** A permission the user chose to always allow: this exact tool and target, in this folder. */
export interface Rule {
  agent: AgentKind;
  cwd: string;
  tool: string;
  target: string;
}

export interface InstallStatus {
  agent: AgentKind;
  configPath: string;
  hookPath: string;
  hookReady: boolean;
  installed: boolean;
  error: string | null;
  /** Codex only: whether it will run our hooks. */
  codex: { hooksDisabled: boolean; untrusted: number; total: number } | null;
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

export const Bridge = {
  onView(cb: (v: ViewModel) => void): void {
    void listen<ViewModel>("view", (e) => cb(e.payload));
    void invoke<ViewModel | null>("current_view").then((v) => v && cb(v));
  },
  decide: (request: string, decision: "allow" | "deny") => invoke<void>("decide", { request, decision }),
  layout: (x: number, y: number, width: number, height: number) =>
    invoke<void>("layout", { x, y, width, height }),
  installStatus: (agent: AgentKind) => invoke<InstallStatus>("install_status", { agent }),
  installPreview: (agent: AgentKind, install: boolean) =>
    invoke<InstallPreview>("install_preview", { agent, install }),
  installApply: (agent: AgentKind, install: boolean, fingerprint: string) =>
    invoke<string | null>("install_apply", { agent, install, fingerprint }),
  decideAlways: (request: string) => invoke<void>("decide_always", { request }),
  rulesList: () => invoke<Rule[]>("rules_list"),
  ruleRemove: (index: number) => invoke<void>("rule_remove", { index }),
  sessionJump: (agent: AgentKind, id: string) => invoke<void>("session_jump", { agent, id }),
  alertOpen: (key: string) => invoke<void>("alert_open", { key }),
  alertDismiss: (key: string) => invoke<void>("alert_dismiss", { key }),
  connectorsStatus: () => invoke<ConnectorStatus[]>("connectors_status"),
  connectorEnable: (id: string, on: boolean) => invoke<void>("connector_enable", { id, on }),
  openSettings: () => invoke<void>("open_settings_window"),
  appSettings: () =>
    invoke<{ sounds: boolean; autostart: boolean; foldAfter: number; monitor: string | null; nowPlaying: boolean }>("app_settings"),
  setNowPlaying: (on: boolean) => invoke<void>("set_now_playing", { on }),
  mediaControl: (action: MediaAction) => invoke<void>("media_control", { action }),
  voiceStatus: () => invoke<VoiceStatus>("voice_status"),
  voiceDownload: (id: string) => invoke<void>("voice_download", { id }),
  voiceSelect: (id: string) => invoke<void>("voice_select", { id }),
  voiceOff: () => invoke<void>("voice_off"),
  voiceStart: () => invoke<void>("voice_start"),
  voiceStop: () => invoke<string>("voice_stop"),
  voiceCancel: () => invoke<void>("voice_cancel"),
  /** Connected monitors, by maker and model. */
  monitors: () => invoke<{ name: string; label: string }[]>("monitors"),
  /** A screen was plugged in or removed. */
  onMonitors(cb: () => void): void {
    void listen("monitors", () => cb());
  },
  /** `null` lets the desktop choose. */
  setMonitor: (name: string | null) => invoke<void>("set_monitor", { name }),
  setFoldAfter: (seconds: number) => invoke<void>("set_fold_after", { seconds }),
  setAutostart: (on: boolean) => invoke<void>("set_autostart", { on }),
  setSounds: (on: boolean) => invoke<void>("set_sounds", { on }),
  /** A setting changed somewhere; only the fields that changed are present. */
  onSettings(cb: (s: { sounds?: boolean; api?: ApiStatus; foldAfter?: number; voice?: boolean }) => void): void {
    void listen<{ sounds?: boolean; api?: ApiStatus; foldAfter?: number }>("settings", (e) => cb(e.payload));
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
  islandKeyboard: (on: boolean) => invoke<void>("island_keyboard", { on }),
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
  onVoiceDownload(cb: (p: { id: string; done: number; total: number }) => void): void {
    void listen<{ id: string; done: number; total: number }>("voice-download", (e) => cb(e.payload));
  },
  /** What is playing, while the setting is on; null when nothing is. */
  onMedia(cb: (now: NowPlaying | null) => void): void {
    void listen<NowPlaying | null>("media", (e) => cb(e.payload));
  },
  onPointer(cb: (inside: boolean) => void): void {
    void listen<boolean>("pointer", (e) => cb(e.payload));
  },
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
