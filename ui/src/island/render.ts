// The island's DOM, from a ViewModel. No Tauri here: actions come in, so the lab can render the
// real island with made-up views.
//
// Folded, it is the wire with the flock and one line for the bird in front. Open (hover, news,
// a card, the chat), it adds a header, name tags under the birds, the card of the session in
// front, and connector news. Its size springs open and eases shut.
import { Clock } from "../clock";
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

/**
 * How long a state must hold before it is news. In auto mode every tool call passes through a
 * permission request the classifier clears in a blink, and a turn's Stop is often followed at once
 * by the next prompt: only a state that stays is worth a sound and a peek.
 */
const SETTLE_MS: Partial<Record<SessionView["status"], number>> = {
  approval: 1500,
  question: 1500,
  failed: 1500,
  finished: 3000,
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

/** The desktop's own description of the keys ("Ctrl+Alt+Y"), tidied for a button. */
function shortKeys(trigger: string): string {
  return trigger.replace(/Control/gi, "Ctrl").replace(/\s+/g, "");
}

export interface Island {
  render(v: ViewModel): void;
  /** The view on screen, to re-render after a setting changes. */
  last(): ViewModel;
  /** Holds the island open as if hovered (the lab, screenshots). */
  hold(open: boolean): void;
  /** A global shortcut: answers the permission card on screen, if there is one. */
  shortcut(id: string): void;
  /** The keys the desktop bound for the shortcuts. */
  setKeys(keys: Record<string, string>): void;
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
  /** The approval card actually on screen, the only thing a shortcut may answer. */
  let cardOnScreen: { request: string } | null = null;
  let keys: Record<string, string> = {};

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
    peekUntil = Clock.now() + PEEK_MS;
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
  /** Status changes waiting to settle, by session. */
  const settling = new Map<string, number>();
  /** Sessions whose current state was announced: their card may open the island. */
  const announced = new Set<string>();

  function cues(v: ViewModel): void {
    for (const s of v.sessions) {
      const k = key(s);
      if (statuses.get(k) === s.status) continue;
      statuses.set(k, s.status);
      announced.delete(k);
      window.clearTimeout(settling.get(k));
      settling.delete(k);
      // Already in that state when the island first draws (a webview reload, the app opening
      // on a waiting card): show it, but quietly; it is not news.
      if (!primed) {
        if (SETTLE_MS[s.status]) announced.add(k);
        continue;
      }
      const cue = STATUS_CUE[s.status];
      if (!cue) continue;
      const status = s.status;
      settling.set(
        k,
        window.setTimeout(() => {
          settling.delete(k);
          // Still in that state after the wait: now it is news.
          if (statuses.get(k) !== status) return;
          announced.add(k);
          Sound.play(cue);
          peek();
          // News is about that session: put it in front.
          pinned = k;
          render(last);
        }, SETTLE_MS[status] ?? 0),
      );
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
    // States that have not settled yet show as plain work: no wings, no jumping to the front.
    const shown = v.sessions.map((s) =>
      SETTLE_MS[s.status] && !announced.has(key(s)) ? { ...s, status: "working" as const } : s,
    );
    const pending = shown.find((s) => s.status === "approval") ?? null;
    const shownByKey = new Map(shown.map((s) => [key(s), s]));
    const front = pending ?? (pinned ? shownByKey.get(pinned)! : null) ?? shown[0] ?? null;
    // With the chat open (and nobody waiting on a card), Zeca on the wire is the chat.
    const focus = pending ?? (chat.isOpen() ? null : front);
    chat.setFolder(front?.cwd ?? null);
    const talking = !pending && chat.isOpen() ? { clip: chat.clip(Clock.now()), agent: chat.agent() } : null;
    scene.update(shown, focus, talking);

    // A card opens the island only once its request has settled (see SETTLE_MS).
    const open = hovered || chat.isOpen() || pending !== null || Clock.now() < peekUntil;
    const hasBird = focus !== null || talking !== null;
    const body: (Node | null)[] = [];
    if (open) {
      body.push(header());
      if (hasBird) body.push(el("div", { class: "stage" }, scene.canvas, tags(v, focus)));
      if (chat.isOpen() && !pending) body.push(chat.element);
      else if (front) {
        body.push(
          sessionCard(front, v.approval, ticker, {
            keys,
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

    cardOnScreen = open && front?.status === "approval" && v.approval && !chat.isOpen() ? v.approval : null;
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
  const shortcut = (id: string) => {
    const allow = id === "allow";
    if (id !== "allow" && id !== "deny") return;
    if (chat.answerWaiting(allow)) return;
    if (cardOnScreen) {
      const { request } = cardOnScreen;
      cardOnScreen = null;
      actions.decide(request, allow ? "allow" : "deny");
    }
  };
  const setKeys = (bound: Record<string, string>) => {
    keys = Object.fromEntries(Object.entries(bound).map(([k, v]) => [k, shortKeys(v)]));
    render(last);
  };
  return { render, last: () => last, hold, shortcut, setKeys, chat };
}

export { OPEN_WIDTH };
