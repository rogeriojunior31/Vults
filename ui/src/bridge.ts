// The only file that talks to Tauri: views in, intents out.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type AgentKind = "claude" | "codex";
/** Who the chat talks through: a CLI, or Claude with the user's API key. */
export type ChatProvider = AgentKind | "api";
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
  project: string;
  cwd: string | null;
  status: Status;
  activity: Activity | null;
  step: string | null;
  /** The latest steps, oldest first. */
  steps: string[];
  step_count: number;
  subagents: number;
}

export interface ApprovalView {
  request: string;
  agent: AgentKind;
  project: string;
  tool: string;
  target: string;
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
  | { kind: "permission"; id: string; tool: string; target: string }
  | { kind: "done" }
  | { kind: "error"; message: string };

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
  appSettings: () => invoke<{ sounds: boolean; autostart: boolean }>("app_settings"),
  setAutostart: (on: boolean) => invoke<void>("set_autostart", { on }),
  setSounds: (on: boolean) => invoke<void>("set_sounds", { on }),
  /** A setting changed somewhere; only the fields that changed are present. */
  onSettings(cb: (s: { sounds?: boolean; apiKey?: boolean }) => void): void {
    void listen<{ sounds?: boolean; apiKey?: boolean }>("settings", (e) => cb(e.payload));
  },
  /** Whether an Anthropic API key is saved; the key itself never comes back. */
  apiKeyStatus: () => invoke<boolean>("api_key_status"),
  apiKeySet: (key: string) => invoke<void>("api_key_set", { key }),
  apiKeyClear: () => invoke<void>("api_key_clear"),
  chatSend: (text: string, files: string[], folder: string | null) => invoke<void>("chat_send", { text, files, folder }),
  chatDecide: (id: string, allow: boolean) => invoke<void>("chat_decide", { id, allow }),
  chatReset: (provider: ChatProvider | null) => invoke<ChatProvider>("chat_reset", { provider }),
  islandKeyboard: (on: boolean) => invoke<void>("island_keyboard", { on }),
  onChat(cb: (d: ChatDelta) => void): void {
    void listen<ChatDelta>("chat", (e) => cb(e.payload));
  },
  /** Inbox copies of files dropped on the island. */
  onFiles(cb: (paths: string[]) => void): void {
    void listen<string[]>("files", (e) => cb(e.payload));
  },
  /** A global shortcut was pressed: "allow" or "deny". */
  onShortcut(cb: (id: string) => void): void {
    void listen<string>("shortcut", (e) => cb(e.payload));
  },
  /** The keys the desktop bound, by shortcut id, to show on the buttons. */
  onShortcutKeys(cb: (keys: Record<string, string>) => void): void {
    void listen<Record<string, string>>("shortcut-keys", (e) => cb(e.payload));
  },
  onOpenChat(cb: () => void): void {
    void listen("open-chat", () => cb());
  },
};
