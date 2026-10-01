// The island's DOM, from a ViewModel. No Tauri here: the actions come in, so the lab can render
// the real island with made-up views.
import type { AlertView, ApprovalView, SessionView, ViewModel } from "../bridge";
import { el } from "../dom";
import { Sound, type Cue } from "../sound";
import { ChatPanel, type ChatBackend } from "./chat";
import { Scene } from "./scene";

export interface Actions {
  chat: ChatBackend;
  openAlert(key: string): void;
  /** Brings the session's terminal forward. */
  jump(agent: SessionView["agent"], id: string): void;
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

function session(s: SessionView, actions: Actions): HTMLElement {
  return el(
    "div",
    { class: `session ${s.agent} ${s.status}`, onclick: () => actions.jump(s.agent, s.id) },
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

/** The cue for a session entering a state, if any. */
const STATUS_CUE: Partial<Record<SessionView["status"], Cue>> = {
  approval: "approval",
  question: "question",
  finished: "done",
  failed: "fail",
};

/** Hover this long before the island opens, and away this long before it folds back. */
const OPEN_AFTER_MS = 200;
const FOLD_AFTER_MS = 600;
/** News (a session finished, failed or asks; a connector alert) opens it this long. */
const PEEK_MS = 5200;

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

  // What was already on screen, so only changes make a sound or a peek.
  const statuses = new Map<string, SessionView["status"]>();
  const alertsSeen = new Set<string>();
  let primed = false;

  // Compact by default; open while hovered, for news, for a card, or with the chat.
  let hovered = false;
  let wasOpen = false;
  let peekUntil = 0;
  let hoverTimer: number | undefined;
  root.addEventListener("pointerenter", () => {
    window.clearTimeout(hoverTimer);
    hoverTimer = window.setTimeout(() => {
      hovered = true;
      render(last);
    }, OPEN_AFTER_MS);
  });
  root.addEventListener("pointerleave", () => {
    window.clearTimeout(hoverTimer);
    hoverTimer = window.setTimeout(() => {
      hovered = false;
      render(last);
    }, FOLD_AFTER_MS);
  });
  const peek = () => {
    peekUntil = performance.now() + PEEK_MS;
    window.setTimeout(() => render(last), PEEK_MS + 20);
  };

  function cues(v: ViewModel): void {
    for (const s of v.sessions) {
      const k = `${s.agent}:${s.id}`;
      const cue = STATUS_CUE[s.status];
      if (primed && cue && statuses.get(k) !== s.status) {
        Sound.play(cue);
        peek();
      }
      statuses.set(k, s.status);
    }
    for (const a of v.alerts) {
      if (primed && !alertsSeen.has(a.key)) {
        Sound.play(a.level === "ok" || a.level === "info" ? "alertOk" : "alert");
        peek();
      }
      alertsSeen.add(a.key);
    }
    primed = true;
  }

  function render(v: ViewModel): void {
    if (v !== last) cues(v);
    last = v;
    const pending = v.sessions.find((s) => s.status === "approval");
    // The bird on the wire is the session that needs you; with the chat open, it is the chat;
    // otherwise the latest session.
    const focus = pending ?? (chat.isOpen() ? null : (v.sessions[0] ?? null));
    // A new chat works in the folder of the session in front.
    chat.setFolder((pending ?? v.sessions[0])?.cwd ?? null);
    const talking = !pending && chat.isOpen() ? { clip: chat.clip(performance.now()), agent: chat.agent() } : null;
    scene.update(v.sessions, focus, talking);
    const open = hovered || chat.isOpen() || v.approval !== null || performance.now() < peekUntil;
    root.classList.toggle("open", open);
    // Rows rise in only when the island opens, not on every update while it is open.
    if (open && !wasOpen) {
      root.classList.add("opening");
      window.setTimeout(() => root.classList.remove("opening"), 320);
    }
    wasOpen = open;
    // Folded: the wire and one line for the bird in front. Open: everything.
    const lines = open ? v.sessions.slice(0, 4) : (focus ?? v.sessions[0]) ? [(focus ?? v.sessions[0])!] : [];
    root.replaceChildren(
      ...(focus || talking ? [scene.canvas] : []),
      ...lines.map((s) => session(s, actions)),
      ...(open ? v.alerts.slice(0, 3).map((a) => alert(a, actions)) : []),
      ...(!open && v.alerts.length ? [el("div", { class: "more", text: `${v.alerts.length} new` })] : []),
      ...(v.approval ? [approval(v.approval, actions)] : []),
      ...(chat.isOpen() ? [chat.element] : []),
    );
    // Measured synchronously: rAF may be paused (see scene.ts).
    const r = root.getBoundingClientRect();
    actions.layout(Math.floor(r.x), Math.floor(r.y), Math.ceil(r.width), Math.ceil(r.height));
  }
  return { render, chat };
}
