// The island: renders the ViewModel and nothing else. The character arrives in M2.
import { Bridge, type ApprovalView, type SessionView, type ViewModel } from "./bridge";
import { el } from "./dom";

const root = document.getElementById("island")!;

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

function approval(a: ApprovalView): HTMLElement {
  let sent = false;
  const answer = (decision: "allow" | "deny") => () => {
    if (sent) return;
    sent = true;
    void Bridge.decide(a.request, decision);
  };
  return el(
    "section",
    { class: `approval ${a.agent}` },
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

function render(v: ViewModel): void {
  root.replaceChildren(...v.sessions.slice(0, 4).map(session), ...(v.approval ? [approval(v.approval)] : []));
  // Measure synchronously: requestAnimationFrame is paused while the page counts as hidden.
  // The rectangle becomes the only part of the window that takes the mouse.
  const r = root.getBoundingClientRect();
  void Bridge.layout(Math.floor(r.x), Math.floor(r.y), Math.ceil(r.width), Math.ceil(r.height));
}

render({ sessions: [], approval: null });
Bridge.onView(render);
