// The island's DOM, from a ViewModel. No Tauri here: the actions come in, so the lab can render
// the real island with made-up views.
import type { ApprovalView, SessionView, ViewModel } from "../bridge";
import { el } from "../dom";
import { Scene } from "./scene";

export interface Actions {
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

export function createIsland(root: HTMLElement, actions: Actions): (v: ViewModel) => void {
  const scene = new Scene();
  return (v) => {
    // The bird on the wire stands for the session that needs you, or the latest one.
    const focus = v.sessions.find((s) => s.status === "approval") ?? v.sessions[0] ?? null;
    scene.update(v.sessions, focus);
    root.replaceChildren(
      ...(focus ? [scene.canvas] : []),
      ...v.sessions.slice(0, 4).map(session),
      ...(v.approval ? [approval(v.approval, actions)] : []),
    );
    // Measured synchronously: rAF may be paused (see scene.ts).
    const r = root.getBoundingClientRect();
    actions.layout(Math.floor(r.x), Math.floor(r.y), Math.ceil(r.width), Math.ceil(r.height));
  };
}
