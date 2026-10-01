// The only file that talks to Tauri: views in, intents out.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type AgentKind = "claude" | "codex";
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
  status: Status;
  activity: Activity | null;
  step: string | null;
  subagents: number;
}

export interface ApprovalView {
  request: string;
  agent: AgentKind;
  project: string;
  tool: string;
  target: string;
}

export interface ViewModel {
  sessions: SessionView[];
  approval: ApprovalView | null;
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

export type ChatDelta = { kind: "text"; text: string } | { kind: "done" } | { kind: "error"; message: string };

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
  chatSend: (text: string, files: string[]) => invoke<void>("chat_send", { text, files }),
  chatReset: (provider: AgentKind | null) => invoke<AgentKind>("chat_reset", { provider }),
  islandKeyboard: (on: boolean) => invoke<void>("island_keyboard", { on }),
  onChat(cb: (d: ChatDelta) => void): void {
    void listen<ChatDelta>("chat", (e) => cb(e.payload));
  },
  /** Inbox copies of files dropped on the island. */
  onFiles(cb: (paths: string[]) => void): void {
    void listen<string[]>("files", (e) => cb(e.payload));
  },
  onOpenChat(cb: () => void): void {
    void listen("open-chat", () => cb());
  },
};
