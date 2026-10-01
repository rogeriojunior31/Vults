// The island's DOM, from a ViewModel. No Tauri here: the actions come in, so the lab can render
// the real island with made-up views.
import type { AlertView, ApprovalView, SessionView, ViewModel } from "../bridge";
import { el } from "../dom";
import { ChatPanel, type ChatBackend } from "./chat";
import { Scene } from "./scene";

export interface Actions {
  chat: ChatBackend;
  openAlert(key: string): void;
  dismissAlert(key: string): void;
  decide(request: string, decision: "allow" | "deny"): void;
  /** The island's rectangle; width 0 means nothing is shown. */
  layout(x: number, y: number, width: number, height: number): void;
}

const AGENT_NAME = { claude: "Claude Code", codex: "Codex" } as const;

const STATUS_TEXT: Record<SessionView["status"], string> = {
  idle: "Idle",
  thinking: "Thinking",
  working: "Working",
  approval: "Needs your approval",
  question: "Has a question",
  finished: "Done",
  failed: "Failed",
  ratelimited: "Rate limited",
};

function session(s: SessionView): HTMLElement {
  return el(
    "div",
    { class: `session ${s.agent} ${s.status}` },
    el("span", { class: "dot" }),
    el("span", { class: "project", text: s.project || AGENT_NAME[s.agent] }),
    el("span", { class: "step", text: s.step ?? STATUS_TEXT[s.status] }),
    s.subagents > 0 ? el("span", { class: "subagents", text: `+${s.subagents}` }) : null,
  );
}

function alert(a: AlertView, actions: Actions): HTMLElement {
  const row = el(
    "div",
    { class: `alert ${a.level}${a.link ? " link" : ""}` },
    el("span", { class: "dot" }),
    el("span", { class: "alert-text" }, el("span", { class: "alert-title", text: a.title }), el("span", { class: "step", text: a.detail })),
    el("button", {
      class: "ghost",
      text: "×",
      onclick: () => actions.dismissAlert(a.key),
    }),
  );
  if (a.link) row.querySelector(".alert-text")?.addEventListener("click", () => actions.openAlert(a.key));
  return row;
}

function approval(a: ApprovalView, actions: Actions): HTMLElement {
  let sent = false;
  const answer = (decision: "allow" | "deny") => () => {
    if (sent) return;
    sent = true;
    actions.decide(a.request, decision);
  };
  return el(
    "section",
    { class: `approval-card ${a.agent}` },
    el("div", { class: "who", text: `${AGENT_NAME[a.agent]} · ${a.project}` }),
    el("div", { class: "asks", text: "wants to use" }),
    el("pre", { class: "target", text: a.target }),
    el(
      "div",
      { class: "actions" },
      el("button", { class: "deny", text: "Deny", onclick: answer("deny") }),
      el("button", { class: "allow", text: "Allow", onclick: answer("allow") }),
    ),
  );
}

export interface Island {
  render(v: ViewModel): void;
  chat: ChatPanel;
}

export function createIsland(root: HTMLElement, actions: Actions): Island {
  const scene = new Scene();
  let last: ViewModel = { sessions: [], approval: null, alerts: [] };
  const chat = new ChatPanel(actions.chat, () => render(last));
  // A click on the bird opens the chat.
  scene.canvas.addEventListener("click", () => chat.toggle());

  function render(v: ViewModel): void {
    last = v;
    const pending = v.sessions.find((s) => s.status === "approval");
    // The bird on the wire is the session that needs you; with the chat open, it is the chat;
    // otherwise the latest session.
    const focus = pending ?? (chat.isOpen() ? null : (v.sessions[0] ?? null));
    const talking = !pending && chat.isOpen() ? { clip: chat.clip(performance.now()), agent: chat.agent() } : null;
    scene.update(v.sessions, focus, talking);
    root.replaceChildren(
      ...(focus || talking ? [scene.canvas] : []),
      ...v.sessions.slice(0, 4).map(session),
      ...v.alerts.slice(0, 3).map((a) => alert(a, actions)),
      ...(v.approval ? [approval(v.approval, actions)] : []),
      ...(chat.isOpen() ? [chat.element] : []),
    );
    // Measured synchronously: rAF may be paused (see scene.ts).
    const r = root.getBoundingClientRect();
    actions.layout(Math.floor(r.x), Math.floor(r.y), Math.ceil(r.width), Math.ceil(r.height));
  }
  return { render, chat };
}
