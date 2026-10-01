// The island's DOM, from a ViewModel. No Tauri here: actions come in, so the lab can render the
// real island with made-up views.
//
// Folded, it is the wire with the flock and one line for the bird in front. Open (hover, news,
// a card, the chat), it adds a header, name tags under the birds, the card of the session in
// front, and connector news. Its size springs open and eases shut.
import type { AlertView, SessionView, ViewModel } from "../bridge";
import { el } from "../dom";
import { Sound, type Cue } from "../sound";
import { ChatPanel, type ChatBackend } from "./chat";
import { icon } from "./icons";
import { Scene } from "./scene";
import { Ticker } from "./ticker";
import { AGENT_NAME, sessionCard } from "./views";

export interface Actions {
  chat: ChatBackend;
  decide(request: string, decision: "allow" | "deny"): void;
  /** The island's rectangle; width 0 means nothing is shown. */
  layout(x: number, y: number, width: number, height: number): void;
  openAlert(key: string): void;
  dismissAlert(key: string): void;
  /** Brings the session's terminal forward. */
  jump(agent: SessionView["agent"], id: string): void;
  openSettings(): void;
  setSounds(on: boolean): void;
}

/** Hover this long before the island opens, and away this long before it folds back. */
const OPEN_AFTER_MS = 200;
const FOLD_AFTER_MS = 600;
/** News (a session finished, failed or asks; a connector alert) opens it this long. */
const PEEK_MS = 5200;
/** Width of the open island's content. */
const OPEN_WIDTH = 520;

const STATUS_CUE: Partial<Record<SessionView["status"], Cue>> = {
  approval: "approval",
  question: "question",
  finished: "done",
  failed: "fail",
};

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

const key = (s: SessionView) => `${s.agent}:${s.id}`;

function line(s: SessionView, onclick: () => void): HTMLElement {
  return el(
    "div",
    { class: `line ${s.status}`, onclick },
    el("span", { class: `dot ${s.agent}` }),
    el("span", { class: "name", text: s.project || AGENT_NAME[s.agent] }),
    el("span", { class: "step", text: s.step ?? STATUS_TEXT[s.status] }),
  );
}

function alertRow(a: AlertView, actions: Actions): HTMLElement {
  const text = el(
    "span",
    { class: "alert-text", onclick: a.link ? () => actions.openAlert(a.key) : undefined },
    el("span", { class: "alert-title", text: a.title }),
    el("span", { class: "alert-detail", text: a.detail }),
  );
  return el(
    "div",
    { class: `alert ${a.level}${a.link ? " link" : ""}` },
    el("span", { class: "dot" }),
    text,
    el("button", { class: "icon-btn", onclick: () => actions.dismissAlert(a.key) }, icon("close", 11)),
  );
}

export interface Island {
  render(v: ViewModel): void;
  /** The view on screen, to re-render after a setting changes. */
  last(): ViewModel;
  /** Holds the island open as if hovered (the lab, screenshots). */
  hold(open: boolean): void;
  chat: ChatPanel;
}

export function createIsland(root: HTMLElement, actions: Actions): Island {
  const scene = new Scene();
  const ticker = new Ticker();
  const inner = el("div", { class: "inner" });
  root.replaceChildren(inner);
  let last: ViewModel = { sessions: [], approval: null, alerts: [] };
  const chat = new ChatPanel(actions.chat, () => render(last));

  let hovered = false;
  let wasOpen = false;
  let peekUntil = 0;
  let hoverTimer: number | undefined;
  /** A session the user put in front by clicking its bird or tag. */
  let pinned: string | null = null;

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
  const pick = (k: string) => {
    if (k === "zeca") chat.toggle();
    else {
      pinned = k;
      if (chat.isOpen()) chat.toggle(false);
    }
    render(last);
  };
  scene.canvas.addEventListener("click", (e) => {
    const x = e.offsetX;
    const hit = scene.slots().find((s) => x >= s.x - 4 && x <= s.x + s.width + 4);
    if (hit) pick(hit.key);
  });

  // What was already on screen, so only changes make a sound or a peek.
  const statuses = new Map<string, SessionView["status"]>();
  const alertsSeen = new Set<string>();
  let primed = false;
  function cues(v: ViewModel): void {
    for (const s of v.sessions) {
      const cue = STATUS_CUE[s.status];
      if (primed && cue && statuses.get(key(s)) !== s.status) {
        Sound.play(cue);
        peek();
        // News is about that session: put it in front.
        pinned = key(s);
      }
      statuses.set(key(s), s.status);
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

  function header(): HTMLElement {
    const tab = (name: "sessions" | "chat", glyph: Node, label: string) => {
      const on = (name === "chat") === chat.isOpen();
      const b = el("button", { class: `tab${on ? " on" : ""}`, onclick: () => chat.toggle(name === "chat") }, glyph, el("span", { text: label }));
      return b;
    };
    const soundOn = Sound.isEnabled();
    const sound = el("button", { class: "icon-btn", onclick: () => actions.setSounds(!soundOn) }, icon(soundOn ? "sound" : "mute", 14));
    sound.title = soundOn ? "Mute" : "Unmute";
    const gear = el("button", { class: "icon-btn", onclick: () => actions.openSettings() }, icon("gear", 14));
    gear.title = "Settings";
    return el(
      "div",
      { class: "header" },
      el("div", { class: "tabs" }, tab("sessions", icon("flock", 13), "Flock"), tab("chat", icon("chat", 13), "Chat")),
      el("div", { class: "header-actions" }, sound, gear),
    );
  }

  function tags(v: ViewModel, focus: SessionView | null): HTMLElement {
    const byKey = new Map(v.sessions.map((s) => [key(s), s]));
    return el(
      "div",
      { class: "tags" },
      ...scene.slots().map((slot) => {
        const s = slot.key === "zeca" ? focus : byKey.get(slot.key);
        const name = slot.key === "zeca" ? (chat.isOpen() && !focus ? "Zeca" : (s?.project ?? "Zeca")) : (s?.project ?? "");
        const t = el("button", { class: `tag${slot.key === "zeca" ? " front" : ""}`, text: name, onclick: () => pick(slot.key) });
        t.style.left = `${slot.x + slot.width / 2}px`;
        return t;
      }),
    );
  }

  function render(v: ViewModel): void {
    if (v !== last) cues(v);
    last = v;
    const byKey = new Map(v.sessions.map((s) => [key(s), s]));
    if (pinned && !byKey.has(pinned)) pinned = null;
    const pending = v.sessions.find((s) => s.status === "approval") ?? null;
    const front = pending ?? (pinned ? byKey.get(pinned)! : null) ?? v.sessions[0] ?? null;
    // With the chat open (and nobody waiting on a card), Zeca on the wire is the chat.
    const focus = pending ?? (chat.isOpen() ? null : front);
    chat.setFolder(front?.cwd ?? null);
    const talking = !pending && chat.isOpen() ? { clip: chat.clip(performance.now()), agent: chat.agent() } : null;
    scene.update(v.sessions, focus, talking);

    const open = hovered || chat.isOpen() || v.approval !== null || performance.now() < peekUntil;
    const hasBird = focus !== null || talking !== null;
    const body: (Node | null)[] = [];
    if (open) {
      body.push(header());
      if (hasBird) body.push(el("div", { class: "stage" }, scene.canvas, tags(v, focus)));
      if (chat.isOpen() && !pending) body.push(chat.element);
      else if (front) {
        body.push(
          sessionCard(front, v.approval, ticker, {
            decide: actions.decide,
            jump: actions.jump,
            dismiss: () => {
              peekUntil = 0;
              hovered = false;
              render(last);
            },
          }),
        );
      } else body.push(el("section", { class: "card" }, el("div", { class: "title dim", text: "Nothing running. Ask Zeca anything from the Chat tab." })));
      if (v.alerts.length && !chat.isOpen()) body.push(el("div", { class: "alerts" }, ...v.alerts.slice(0, 3).map((a) => alertRow(a, actions))));
    } else if (hasBird) {
      body.push(el("div", { class: "stage" }, scene.canvas));
      if (front) body.push(line(front, () => actions.jump(front.agent, front.id)));
      if (v.alerts.length) body.push(el("div", { class: "more", text: `${v.alerts.length} new` }));
    } else if (v.alerts.length) {
      body.push(el("div", { class: "more", text: `${v.alerts.length} new` }));
    }

    inner.classList.toggle("open", open);
    inner.replaceChildren(...body.filter((n): n is Node => n !== null));
    if (open && !wasOpen) {
      root.classList.add("opening");
      window.setTimeout(() => root.classList.remove("opening"), 360);
    }
    wasOpen = open;
    resize();
  }

  /** Springs the island to the content's size and tells the window which part takes the mouse. */
  function resize(): void {
    const empty = inner.childElementCount === 0;
    const w = empty ? 0 : Math.ceil(inner.offsetWidth);
    const h = empty ? 0 : Math.ceil(inner.offsetHeight);
    const before = root.offsetWidth * root.offsetHeight;
    root.classList.toggle("shrinking", w * h < before);
    root.style.width = `${w}px`;
    root.style.height = `${h}px`;
    root.classList.toggle("empty", empty);
    // The final rectangle: hit-testing must not wait for the animation.
    actions.layout(Math.floor((window.innerWidth - w) / 2), 0, w, h);
  }

  const hold = (open: boolean) => {
    hovered = open;
    render(last);
  };
  return { render, last: () => last, hold, chat };
}

export { OPEN_WIDTH };
