// The island's DOM, from a ViewModel. No Tauri here: actions come in, so the lab can render the
// real island with made-up views.
//
// Three modes (fsm.ts). Hidden, only an invisible strip at the top edge is left, to wake it.
// Compact, it is a fixed-size pill: Zeca and the session in front on the left, up to four vults
// on the right with a badge each. Open (a click, a permission, the chat), it has a header, the
// focus card (Zeca and the session in front) beside the flock list (every other session), and
// connector news. The two layers cross-fade; the black shape springs when it grows and eases when
// it shrinks.
import { Clock } from "../clock";
import type { AlertView, Answer, ApprovalView, MediaAction, NowPlaying, SessionView, UsageWindow, ViewModel } from "../bridge";
import { el } from "../dom";
import { Sound, type Cue } from "../sound";
import { Tracked } from "./anim";
import { idleClip, setMusic } from "./behavior";
import { ChatPanel, type ChatBackend } from "./chat";
import { IslandMachine, type Mode } from "./fsm";
import { icon, type IconName } from "./icons";
import {
  COMPACT_H,
  COMPACT_SCENE,
  COMPACT_W,
  FOCUS_SCENE,
  LIST_ROW,
  LIST_SCENE,
  Scene,
} from "./scene";
import { Ticker } from "./ticker";
import { Sky, type SkyPerch } from "./sky";
import { agentName, BADGE, flockRows, focusCard, greetingCard, settledCard, statusText, usageMeters, type Settled } from "./views";

export interface Actions {
  chat: ChatBackend;
  decide(request: string, decision: "allow" | "deny"): void;
  decideAlways(request: string): void;
  /** The replies to a question card, one per question. */
  answer(request: string, answers: Answer[]): void;
  /** A question card goes back to the agent's terminal. */
  release(request: string): void;
  /** The part of the window that takes the pointer; everything else passes through. */
  layout(x: number, y: number, width: number, height: number): void;
  openAlert(key: string): void;
  dismissAlert(key: string): void;
  /** Brings the session's terminal forward. */
  jump(agent: SessionView["agent"], id: string): void;
  openSettings(): void;
  setSounds(on: boolean): void;
  /** Play/pause or skip the song on screen. */
  media(action: MediaAction): void;
}

/** Width of the open island's content. */
const OPEN_WIDTH = 640;
/** The flock list shows this many rows, then scrolls. */
const LIST_ROWS = 4;
/** News rows shown at once; the rest are counted. */
const MAX_ALERTS = 3;
/** How long "couldn't find the terminal" stays on the card. */
const JUMP_NOTE_MS = 2600;
/** A permission goes back to its terminal this long after it arrived (the server's decision
 *  deadline, `protocol::limits::SERVER_DECISION_TIMEOUT`). */
const EXPIRES_MS = 108_000;
/** The card counts down over the last part of that. */
const EXPIRY_SHOWN_MS = 30_000;
/** How long the card says what became of a permission before the next thing shows. */
const SETTLED_MS: Record<Settled, number> = { allow: 700, deny: 700, answered: 700, released: 1600, terminal: 1600, expired: 2600 };
/** The hello at start-up: Zeca lands and waves, then the island folds. */
const GREET_MS = 5200;
/** Resting the pointer on Zeca this long, he preens; not again before the cooldown. */
const PREEN_AFTER_MS = 1900;
const PREEN_EVERY_MS = 6000;
/** Clicks on Zeca this close together add up: the third annoys him. */
const CLICKS_MS = 1700;
/** A click on the pill calls soaring birds down; the island opens this much later, mid-swoop. */
/** Keys the desktop bound without saying which: the ones we asked for. */
const DEFAULT_KEYS: Record<string, string> = { allow: "Ctrl+Alt+Y", deny: "Ctrl+Alt+N" };
/** Corner radius of the bottom corners, compact and open. */
const RADIUS = { compact: 14, open: 24 };
/** Hidden: the shape retracts into the top edge, this wide. */
const HIDDEN_W = 184;
/** The invisible strip that wakes a hidden island. */
const WAKE = { width: 240, height: 6 };
/** The countdown before folding shows in the last part of the wait, at most this long. */
const COUNTDOWN_MS = 10_000;
/** The countdown moves in steps this long: a hairline needs no more, and each step costs a paint. */
const COUNTDOWN_STEP_MS = 250;
/** Geometry steps this often while it moves (timers: WebKit pauses rAF on a hidden surface). */
const FRAME_MS = 16;

const STATUS_CUE: Partial<Record<SessionView["status"], Cue>> = {
  approval: "approval",
  question: "question",
  finished: "done",
  failed: "fail",
};

/**
 * How long a state must hold before it is news. In auto mode every tool call passes through a
 * permission request the classifier clears in a blink, and a turn's Stop is often followed at once
 * by the next prompt: only a state that stays is worth a sound, a badge or opening the island.
 */
const SETTLE_MS: Partial<Record<SessionView["status"], number>> = {
  approval: 1500,
  question: 1500,
  failed: 1500,
  finished: 3000,
};

const key = (s: SessionView) => `${s.agent}:${s.id}`;
/** The session is the one the card in line is for, and its state says it waits on it. */
const onCard = (s: SessionView, a: ApprovalView | null): boolean =>
  !!a && s.agent === a.agent && s.id === a.session && (s.status === "approval" || (s.status === "question" && a.questions.length > 0));
/** What a settled card names: the command for a permission, the question for a question. */
const cardTarget = (a: ApprovalView): string => a.questions[0]?.question ?? a.target;
/** No motion when the user asked for less (or the lab takes a still). */
const calm = () =>
  window.matchMedia("(prefers-reduced-motion: reduce)").matches ||
  document.body.classList.contains("still");

function alertRow(a: AlertView, actions: Actions): HTMLElement {
  const text = el(
    "span",
    {
      class: "alert-text",
      onclick: a.link ? () => actions.openAlert(a.key) : undefined,
    },
    el("span", { class: "alert-title", text: a.title }),
    el("span", { class: "alert-detail", text: a.detail }),
  );
  return el(
    "div",
    { class: `alert ${a.level}${a.link ? " link" : ""}` },
    el("span", { class: "dot" }),
    text,
    el(
      "button",
      { class: "icon-btn", onclick: () => actions.dismissAlert(a.key) },
      icon("close", 11),
    ),
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
  /** Holds the island open (the lab, screenshots). */
  hold(open: boolean): void;
  /** A global shortcut: answers the permission card on screen, or holds the chat's mic (`talk`, `talk-up`). */
  shortcut(id: string): void;
  /** The keys the desktop bound for the shortcuts. */
  setKeys(keys: Record<string, string>): void;
  /** How long the open island waits before folding once the pointer leaves. */
  setFoldAfter(seconds: number): void;
  /** Open terminal found nothing to bring forward. */
  jumpFailed(): void;
  /** Zeca lands and says hello at start-up, by the user's first name when there is one. */
  greet(name?: string | null): void;
  /** The window says the pointer came onto it or left it (Linux: GTK's crossings, in order). */
  pointer(inside: boolean): void;
  /** What is playing; null when nothing is (or the setting is off). */
  setMedia(now: NowPlaying | null): void;
  /** The subscriptions' usage windows, from the last read. */
  setUsage(windows: UsageWindow[]): void;
  chat: ChatPanel;
}

export function createIsland(root: HTMLElement, actions: Actions): Island {
  const fsm = new IslandMachine();
  const sky = new Sky(() => {
    compactScene.refresh(); focusScene.refresh(); listScene.refresh();
  });
  const hidden = (key: string) => sky.owns(key);
  const compactScene = new Scene(COMPACT_SCENE, hidden);
  const focusScene = new Scene(FOCUS_SCENE, hidden);
  const listScene = new Scene(LIST_SCENE, hidden);
  const ticker = new Ticker();
  /** Zeca's place in the focus card: his canvas and the glow behind him. It moves with the card. */
  const perch = el("div", { class: "perch" }, el("span", { class: "glow" }), focusScene.canvas);

  const compactText = el("div", { class: "pill-text" });
  const badges = el("div", { class: "badges" });
  const compact = el(
    "div",
    { class: "layer compact" },
    compactScene.canvas,
    compactText,
    badges,
  );
  // The open layer is built once; its parts are refilled on each render.
  const headerSlot = el("div", { class: "header-slot" });
  const focusHost = el("div", { class: "focus-host" });
  const rows = el("div", { class: "flock-rows" });
  const flock = el("div", { class: "flock" }, rows);
  const overview = el("div", { class: "overview" }, focusHost, flock);
  const main = el("div", { class: "main" });
  const alertsSlot = el("div", { class: "alerts-slot" });
  const inner = el("div", { class: "layer inner" }, headerSlot, main, alertsSlot);
  const countdown = el("div", { class: "countdown" });
  root.replaceChildren(compact, inner, countdown);
  root.after(sky.canvas);
  // Outside the island, so it can be hovered while the island is retracted.
  const wake = el("div", { class: "wake" });
  root.after(wake);

  let last: ViewModel = { sessions: [], approval: null, alerts: [] };
  let media: NowPlaying | null = null;
  let usage: UsageWindow[] = [];
  const chat = new ChatPanel(actions.chat, () => render(last));

  /** The lab holds the island open: nothing folds or unpins it. */
  let held = false;
  /** A session the user put in front by clicking its bird or tag. */
  let pinned: string | null = null;
  /** The card actually on screen, the only thing a shortcut may answer (a permission, not a question). */
  let cardOnScreen: ApprovalView | null = null;
  /** The question card's "Other" field took the keyboard. */
  let cardKeyboard = false;
  let keys: Record<string, string> = {};
  /** The chat was open when the island folded: it comes back with it. */
  let chatWhenOpened = false;
  /** Finished or failed states the user said OK to, by session: they show as idle. */
  const seen = new Map<string, SessionView["status"]>();
  /** Until when the focus card says Open terminal found nothing. */
  let jumpNoteUntil = 0;
  /** When each permission was first seen, for its countdown. */
  const requestSeen = new Map<string, number>();
  /** Permissions answered here (a click, a shortcut), so their going away is not news. */
  const answeredHere = new Set<string>();
  /** The permission card last on screen. */
  let lastShown: { request: string; session: string; target: string } | null = null;
  /** What became of the permission just settled, shown on its session's card for a moment. */
  let settled: { session: string; request: string; how: Settled; target: string; until: number } | null = null;
  let expiryTimer: number | undefined;

  function settle(session: string, request: string, target: string, how: Settled): void {
    const until = Clock.now() + SETTLED_MS[how];
    settled = { session, request, how, target, until };
    requestSeen.delete(request);
    window.setTimeout(() => render(last), SETTLED_MS[how] + 20);
  }

  // ── Pointer and modes ──────────────────────────────────────────────────────

  const pointerIn = () => fsm.pointerEntered();
  const pointerOut = () => {
    fsm.pointerLeft(Clock.now());
    frame();
  };
  /** The window reports crossings (Linux): from then on the page's own enter and leave are
   *  ignored, since they can miss a leave and arrive out of order with the window's. */
  let windowPointer = false;
  root.addEventListener("pointerenter", () => {
    if (!windowPointer) pointerIn();
  });
  root.addEventListener("pointerleave", () => {
    if (!windowPointer) pointerOut();
    notice(null);
  });
  root.addEventListener("pointermove", (e) => notice(e));
  wake.addEventListener("pointerenter", () => {
    if (!windowPointer) pointerIn();
  });
  // Left the strip without ever reaching the island (it grows under the pointer otherwise).
  wake.addEventListener("pointerleave", () => {
    if (!windowPointer && !root.matches(":hover")) pointerOut();
  });
  // A click on the compact pill opens it. Only on the pill: a click on Fold must not bubble up
  // and open the island again.
  root.addEventListener("click", (e) => {
    if (fsm.mode !== "compact" || !compact.contains(e.target as Node)) return;
    fsm.click();
  });
  fsm.onChange = (from, to) => {
    // A permission opening the island has its own sound; the chat opening is the user's own doing.
    if (to === "open" && !fsm.pinned && !chat.isOpen()) Sound.play("open");
    else if (from === "open") Sound.play("close");
    else if (from === "hidden") Sound.play("peek");
    if (to === "open" && from !== "open" && chatWhenOpened && !chat.isOpen())
      chat.toggle(true);
    if (from === "open" && to !== "open") {
      chatWhenOpened = chat.isOpen();
      if (chat.isOpen()) chat.toggle(false);
    }
    render(last);
  };

  /** Puts a session in front. */
  const pick = (s: SessionView) => {
    pinned = key(s);
    jumpNoteUntil = 0;
    Sound.play("tap");
    render(last);
  };

  // ── News ───────────────────────────────────────────────────────────────────

  // What was already on screen, so only changes make a sound.
  const statuses = new Map<string, SessionView["status"]>();
  const alertsSeen = new Set<string>();
  let primed = false;
  /** Status changes waiting to settle, by session. */
  const settling = new Map<string, number>();
  /** Sessions whose current state was announced: it shows, and a permission opens the island. */
  const announced = new Set<string>();
  /** When each session was first seen: the wire keeps that order, so birds don't shuffle. */
  const firstSeen = new Map<string, number>();

  function cues(v: ViewModel): void {
    const now = Clock.now();
    for (const s of v.sessions) {
      const k = key(s);
      if (!firstSeen.has(k)) {
        firstSeen.set(k, now);
        if (primed) fsm.reveal(now);
      }
      if (statuses.get(k) === s.status) continue;
      statuses.set(k, s.status);
      announced.delete(k);
      seen.delete(k);
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
          fsm.reveal(Clock.now());
          render(last);
        }, SETTLE_MS[status] ?? 0),
      );
    }
    const present = new Set(v.sessions.map(key));
    for (const k of firstSeen.keys()) if (!present.has(k)) firstSeen.delete(k);
    for (const a of v.alerts) {
      if (primed && !alertsSeen.has(a.key)) {
        Sound.play(
          a.level === "ok" || a.level === "info" ? "alertOk" : "alert",
        );
        fsm.reveal(now);
      }
      alertsSeen.add(a.key);
    }
    primed = true;
  }

  // ── Open layer ─────────────────────────────────────────────────────────────

  function header(): HTMLElement {
    // Which part of the island shows: the flock, the chat, or the chat waiting for a file.
    const view = !chat.isOpen() ? "flock" : chat.isShowingDrop() ? "drop" : "chat";
    const tab = (name: typeof view, glyph: IconName, label: string, go: () => void) => {
      const b = el("button", { class: `tab${name === view ? " on" : ""}`, onclick: go }, icon(glyph, 14));
      b.title = label;
      b.setAttribute("aria-label", label);
      return b;
    };
    const tabs = el(
      "div",
      { class: "tabs" },
      tab("flock", "flock", "Flock", () => chat.toggle(false)),
      tab("chat", "chat", "Chat", () => {
        chat.hideDrop();
        chat.toggle(true);
      }),
      tab("drop", "plus", "Drop a file", () => chat.showDrop()),
    );
    const soundOn = Sound.isEnabled();
    const sound = el(
      "button",
      { class: "icon-btn", onclick: () => actions.setSounds(!soundOn) },
      icon(soundOn ? "sound" : "mute", 14),
    );
    sound.title = soundOn ? "Mute" : "Unmute";
    const gear = el(
      "button",
      { class: "icon-btn", onclick: () => actions.openSettings() },
      icon("gear", 14),
    );
    gear.title = "Settings";
    const fold = el(
      "button",
      { class: "icon-btn", onclick: () => foldNow() },
      icon("fold", 14),
    );
    fold.title = "Fold";
    if (fsm.pinned && !held) fold.toggleAttribute("disabled", true);
    return el(
      "div",
      { class: "header" },
      tabs,
      ...(media ? [nowPlaying(media)] : []),
      el("div", { class: "header-actions" }, usageMeters(usage, Date.now() / 1000), sound, gear, fold),
    );
  }

  /** The song, and its controls on hover. */
  function nowPlaying(m: NowPlaying): HTMLElement {
    const control = (action: MediaAction, glyph: IconName, label: string) => {
      const b = el("button", { class: "icon-btn", onclick: () => actions.media(action) }, icon(glyph, 13));
      b.title = label;
      b.setAttribute("aria-label", label);
      return b;
    };
    const song = m.artist ? `${m.title} · ${m.artist}` : m.title;
    const line = el("span", { class: "song", text: song });
    line.title = song;
    return el(
      "div",
      { class: `now-playing${m.playing ? " playing" : ""}` },
      icon("note", 12),
      line,
      el(
        "div",
        { class: "controls" },
        control("previous", "previous", "Previous"),
        control("playpause", m.playing ? "pause" : "play", m.playing ? "Pause" : "Play"),
        control("next", "next", "Next"),
      ),
    );
  }

  const foldNow = () => {
    if (held) return;
    fsm.fold(Clock.now());
  };

  // ── Compact layer ──────────────────────────────────────────────────────────

  function paintCompact(
    front: SessionView | null,
    shown: SessionView[],
    alerts: number,
  ): void {
    const status = front ? front.status : null;
    // Nothing to report: the island says what is playing instead.
    const song = media?.playing && (!front || status === "idle") ? media : null;
    const detail = song ? (song.artist ?? "") : front ? statusText(front) : "Nothing running";
    const vults = compactScene.slots().filter((s) => s.key !== "zeca");
    // The text runs between Zeca and the leftmost vult.
    const right = vults.length
      ? Math.min(...vults.map((s) => s.x)) - 10
      : COMPACT_W - 14;
    compactText.style.width = `${Math.max(40, right - 44)}px`;
    compactText.replaceChildren(
      el("span", {
        class: song ? "name song" : "name",
        text: song ? `♪ ${song.title}` : front ? front.project || agentName(front) : "Zeca",
      }),
      el("span", { class: `status ${song ? "music" : (status ?? "none")}`, text: detail }),
      ...(alerts ? [el("span", { class: "news", text: `${alerts} new` })] : []),
    );
    const byKey = new Map(shown.map((s) => [key(s), s]));
    badges.replaceChildren(
      ...vults.flatMap((slot) => {
        const kind = BADGE[byKey.get(slot.key)?.status ?? "idle"];
        if (!kind) return [];
        const b = el("span", { class: `badge ${kind}` });
        b.style.left = `${slot.x + slot.width - 4}px`;
        return [b];
      }),
    );
  }

  // ── Render ─────────────────────────────────────────────────────────────────

  function render(v: ViewModel): void {
    if (v !== last) cues(v);
    last = v;
    const now = Clock.now();
    const byKey = new Map(v.sessions.map((s) => [key(s), s]));
    if (pinned && !byKey.has(pinned)) pinned = null;
    // States that have not settled yet show as plain work: no wings, no badge, no card. A state
    // the user said OK to shows as idle.
    const shown = v.sessions
      .map((s): SessionView => {
        const k = key(s);
        if (SETTLE_MS[s.status] && !announced.has(k)) return { ...s, status: "working" };
        if (seen.get(k) === s.status) return { ...s, status: "idle", note: null };
        return s;
      })
      .sort((a, b) => (firstSeen.get(key(a)) ?? 0) - (firstSeen.get(key(b)) ?? 0));
    const approval = v.approval;
    if (approval && !requestSeen.has(approval.request)) requestSeen.set(approval.request, now);
    // The card on screen went away without a click here: answered in the terminal, or nobody
    // answered in time and the terminal asks now.
    if (lastShown && approval?.request !== lastShown.request) {
      if (!answeredHere.has(lastShown.request)) {
        const age = now - (requestSeen.get(lastShown.request) ?? now);
        settle(lastShown.session, lastShown.request, lastShown.target, age >= EXPIRES_MS - 3000 ? "expired" : "terminal");
      }
      answeredHere.delete(lastShown.request);
      lastShown = null;
    }
    // Only the first permission in line has a card; it shows once its session's state settled.
    const pending = shown.find((s) => onCard(s, approval)) ?? null;
    const shownByKey = new Map(shown.map((s) => [key(s), s]));
    const recent = settled && now < settled.until ? settled : null;
    const settledSession = recent ? (shownByKey.get(recent.session) ?? null) : null;
    const active = shown.find(s => s.status !== "idle");
    const front = settledSession ?? pending ?? (pinned ? shownByKey.get(pinned)! : null) ?? active ?? shown[0] ?? null;

    // A settled permission opens the island and keeps it open until it is answered.
    if (pending && v.approval && !fsm.pinned) fsm.openPinned();
    else if (!pending && fsm.pinned && !held) fsm.unpin(now);
    fsm.setOccupied(v.sessions.length > 0 || v.alerts.length > 0, now);
    if (chat.isOpen() && fsm.mode !== "open") fsm.open(now);
    fsm.setEngaged(chat.isOpen(), now);
    const mode: Mode = fsm.mode;

    chat.setFolders(
      shown.flatMap((s) => (s.cwd ? [{ cwd: s.cwd, project: s.project }] : [])),
      front?.cwd ?? null,
    );
    const others = shown.filter((s) => s !== front);
    const chatShown = chat.isOpen() && !pending;
    // Zeca stays on the wire with nobody there, so the island is never a blank shape. In the chat
    // he is the chat: thinking, swallowing a file, waiting for an answer.
    const idle = { clip: idleClip(), agent: "claude" as const };
    compactScene.update(shown, front, front ? null : idle);
    if (chatShown) focusScene.update([], null, { clip: chat.clip(now), agent: chat.agent() });
    else focusScene.update([], front, front ? null : idle);
    listScene.update(others, null);
    listScene.setHeight(Math.max(1, others.length) * LIST_ROW);
    compactScene.setActive(mode === "compact");
    // Zeca is on the focus card or in the chat: drawn either way while the island is open.
    focusScene.setActive(mode === "open");
    // A permission (or what became of it) takes the whole width: it is the one thing to read.
    const wide = settledSession !== null || (pending !== null && front === pending);
    const listShown = others.length > 0 && !wide;
    listScene.setActive(mode === "open" && !chatShown && listShown);
    sky.update(shown, mode !== "hidden");
    paintCompact(front, shown, v.alerts.length);

    const asking = mode === "open" && !chatShown && !settledSession && front !== null && front === pending;
    cardOnScreen = asking ? approval : null;
    if (asking && approval) lastShown = { request: approval.request, session: key(front), target: cardTarget(approval) };
    // The card that took the keyboard is gone (answered elsewhere, expired): give it back.
    if (cardKeyboard && !asking) cardActions.keyboard(false);
    if (mode === "open") {
      headerSlot.replaceChildren(header());
      if (chatShown) {
        perch.className = "perch chatting";
        if (perch.parentElement !== chat.perchSlot) chat.perchSlot.append(perch);
        show(chat.element);
      } else {
        show(overview);
        paintFocus(front, approval, now, settledSession ? recent : null);
        tickExpiry();
        flock.classList.toggle("none", !listShown);
        rows.style.maxHeight = `${LIST_ROWS * LIST_ROW}px`;
        rows.replaceChildren(...flockRows(others, listScene.canvas, pick));
      }
      alertsSlot.replaceChildren(...(v.alerts.length && !chat.isOpen() ? [alertsBox(v.alerts)] : []));
    }
    compact.classList.toggle("on", mode === "compact");
    inner.classList.toggle("on", mode === "open");
    root.classList.toggle("is-open", mode === "open");
    root.classList.toggle("is-hidden", mode === "hidden");
    reshape(mode);
  }

  function placeSky(): void {
    const skyRect = sky.canvas.getBoundingClientRect();
    const anchors = new Map<string, SkyPerch>();
    const scenes = fsm.mode === "compact" ? [compactScene] : [focusScene, listScene];
    for (const scene of scenes) {
      if (!scene.canvas.isConnected) continue;
      const rect = scene.canvas.getBoundingClientRect();
      for (const at of scene.anchors()) anchors.set(at.key, {
        x: rect.left - skyRect.left + at.x, y: rect.top - skyRect.top + at.y, scale: at.scale,
      });
    }
    // Sessions beyond the visible list land at the island's edge, then their row owns the bird.
    const rect = root.getBoundingClientRect();
    for (const session of last.sessions) if (!anchors.has(key(session))) anchors.set(key(session), {
      x: rect.right - skyRect.left - 20, y: rect.bottom - skyRect.top - 16, scale: 1,
    });
    sky.place(anchors, {
      left: rect.left - skyRect.left, top: rect.top - skyRect.top, width: rect.width, height: rect.height,
      radius: parseFloat(getComputedStyle(root).borderBottomLeftRadius) || 0,
    });
  }
  window.addEventListener("resize", placeSky);
  rows.addEventListener("scroll", placeSky);

  /**
   * Puts the chat or the overview in the island's body. Only when it is not there already: taking
   * the chat out of the page and back, on every render, would blur its input after each key.
   */
  function show(part: HTMLElement): void {
    if (main.childElementCount !== 1 || main.firstElementChild !== part) main.replaceChildren(part);
  }

  // ── Zeca notices you ───────────────────────────────────────────────────────

  let overZeca = false;
  let restTimer: number | undefined;
  let lastPreen = 0;
  let clicks: number[] = [];
  /** Until when the start-up hello shows. */
  let greetUntil = 0;
  let greetName: string | null = null;

  /** Zeca looks toward the pointer, at you when it is on him, and preens if it rests there. */
  function notice(e: PointerEvent | null): void {
    const box = fsm.mode === "open" && e ? focusScene.zecaBox() : null;
    if (!box || !e || !focusScene.canvas.isConnected) {
      if (overZeca || e === null) focusScene.lookAt(null);
      overZeca = false;
      window.clearTimeout(restTimer);
      root.classList.remove("over-zeca");
      return;
    }
    const r = focusScene.canvas.getBoundingClientRect();
    const x = e.clientX - r.left;
    const y = e.clientY - r.top;
    const over = x >= box.x && x <= box.x + box.w && y >= box.y && y <= box.y + box.h;
    // His head is at the front (right) of his body: past its middle to the left, he looks back.
    focusScene.lookAt(over ? "front" : x < box.x + box.w * 0.4 ? "left" : "right");
    root.classList.toggle("over-zeca", over);
    if (over && !overZeca) {
      restTimer = window.setTimeout(() => {
        if (!overZeca || Clock.now() - lastPreen < PREEN_EVERY_MS) return;
        if (focusScene.react("preen")) lastPreen = Clock.now();
      }, PREEN_AFTER_MS);
    }
    if (!over) window.clearTimeout(restTimer);
    overZeca = over;
  }

  /** A click on Zeca startles him; the third in a row annoys him. */
  root.addEventListener("click", () => {
    if (fsm.mode !== "open" || !overZeca) return;
    const now = Clock.now();
    clicks = [...clicks.filter((t) => now - t < CLICKS_MS), now];
    if (clicks.length >= 3) {
      clicks = [];
      if (focusScene.react("fail")) Sound.play("hiss");
    } else if (focusScene.react("startle")) Sound.play("squawk");
  });

  // ── Focus card ─────────────────────────────────────────────────────────────

  let cardKey = "";
  let cardSig = "";
  let card: HTMLElement | null = null;

  /**
   * Rebuilds the focus card only when what it shows changed: a rebuild between mousedown and
   * mouseup would swallow a click on Allow. A new session or a new state cross-fades.
   */
  function paintFocus(s: SessionView | null, approval: ViewModel["approval"], now: number, done: typeof settled): void {
    const jumpFailed = now < jumpNoteUntil;
    // The hello gives way to anything that needs the user.
    const greeting = now < greetUntil && !(s && onCard(s, approval)) && !done;
    const k = greeting ? "greeting" : done ? `settled|${done.request}|${done.how}` : s ? `${key(s)}|${s.status}` : "empty";
    const sig = JSON.stringify([
      s?.status,
      s?.project,
      s?.note,
      s?.steps.slice(-2),
      s?.step_count,
      s?.subagents,
      s?.cwd,
      s && onCard(s, approval) ? approval : null,
      keys,
      jumpFailed,
    ]);
    // Zeca's perch may be in the chat: a card without him is repainted to take him back.
    if (card && k === cardKey && sig === cardSig && card.contains(perch)) return;
    const next = greeting
      ? greetingCard(perch, greetName)
      : done && s
        ? settledCard(s, done.how, done.target, perch)
        : focusCard(s, approval, ticker, perch, cardActions, jumpFailed);
    if (card && k === cardKey) card.replaceWith(next);
    else {
      if (card && !calm()) {
        const old = card;
        old.classList.add("leaving");
        window.setTimeout(() => old.remove(), 170);
      } else card?.remove();
      if (card && !calm()) next.classList.add("entering");
      focusHost.append(next);
    }
    card = next;
    cardKey = k;
    cardSig = sig;
  }

  /** The countdown on the permission card, in its last half minute. */
  function tickExpiry(): void {
    window.clearTimeout(expiryTimer);
    const box = card?.querySelector<HTMLElement>(".expiry");
    if (!box || !cardOnScreen) return;
    const left = (requestSeen.get(cardOnScreen.request) ?? Clock.now()) + EXPIRES_MS - Clock.now();
    const on = left < EXPIRY_SHOWN_MS;
    if (box.classList.contains("on") !== on) {
      box.classList.toggle("on", on);
      // It adds two lines to the card: the island grows with it.
      reshape(fsm.mode);
    }
    if (on) {
      const secs = Math.max(0, Math.ceil(left / 1000));
      box.querySelector(".expiry-text")!.textContent = `Goes back to the terminal in 0:${String(secs).padStart(2, "0")}`;
      box.querySelector<HTMLElement>(".expiry-bar")!.style.width = `${Math.max(0, (left / EXPIRY_SHOWN_MS) * 100)}%`;
    }
    expiryTimer = window.setTimeout(tickExpiry, on ? 1000 : Math.max(1000, left - EXPIRY_SHOWN_MS));
  }

  /** A click or a shortcut answered the card on screen. */
  function answered(request: string, how: "allow" | "deny" | "answered" | "released"): void {
    answeredHere.add(request);
    if (lastShown?.request === request) settle(lastShown.session, request, lastShown.target, how);
    if (how !== "released") Sound.play(how === "answered" ? "allow" : how);
    render(last);
  }

  const cardActions = {
    get keys() {
      return keys;
    },
    decide: (request: string, decision: "allow" | "deny") => {
      answered(request, decision);
      actions.decide(request, decision);
    },
    decideAlways: (request: string) => {
      answered(request, "allow");
      actions.decideAlways(request);
    },
    answer: (request: string, answers: Answer[]) => {
      answered(request, "answered");
      actions.answer(request, answers);
    },
    release: (request: string) => {
      answered(request, "released");
      actions.release(request);
    },
    keyboard: (on: boolean) => {
      cardKeyboard = on;
      // The chat may hold the keyboard too: it keeps it.
      if (!chat.isOpen()) actions.chat.keyboard(on);
    },
    relayout: () => render(last),
    jump: actions.jump,
    dismiss: (s: SessionView) => {
      seen.set(key(s), s.status);
      render(last);
    },
    openChat: () => chat.toggle(true),
  };

  function alertsBox(alerts: AlertView[]): HTMLElement {
    const more = alerts.length - MAX_ALERTS;
    return el(
      "div",
      { class: "alerts" },
      ...alerts.slice(0, MAX_ALERTS).map((a) => alertRow(a, actions)),
      ...(more > 0 ? [el("div", { class: "alerts-more", text: `+${more} more` })] : []),
    );
  }

  // ── Geometry ───────────────────────────────────────────────────────────────

  const width = new Tracked(COMPACT_W);
  const height = new Tracked(COMPACT_H);
  const radius = new Tracked(RADIUS.compact);
  let frameTimer: number | undefined;
  /** What the shape and the countdown last got, so an unchanged frame writes nothing. */
  let drawnShape = "";
  let drawnCountdown = "";
  let lastStep = 0;
  let sent = "";

  function target(mode: Mode): { w: number; h: number; r: number } {
    if (mode === "hidden") return { w: HIDDEN_W, h: 0, r: RADIUS.compact };
    if (mode === "compact")
      return { w: COMPACT_W, h: COMPACT_H, r: RADIUS.compact };
    return {
      w: Math.ceil(inner.offsetWidth),
      h: Math.ceil(inner.offsetHeight),
      r: RADIUS.open,
    };
  }

  /** Moves the shape toward the mode's size, and tells the window which part takes the pointer. */
  function reshape(mode: Mode): void {
    const t = target(mode);
    const now = performance.now();
    if (calm()) {
      width.jump(t.w);
      height.jump(t.h);
      radius.jump(t.r);
    } else {
      width.moveTo(t.w, now);
      height.moveTo(t.h, now);
      radius.moveTo(t.r, now);
    }
    // The final rectangle: hit-testing must not wait for the animation. Hidden, only the strip.
    const rect =
      mode === "hidden"
        ? { w: WAKE.width, h: WAKE.height }
        : { w: t.w, h: t.h };
    const next = `${rect.w}x${rect.h}`;
    if (next !== sent) {
      sent = next;
      actions.layout(
        Math.floor((window.innerWidth - rect.w) / 2),
        0,
        rect.w,
        rect.h,
      );
    }
    wake.classList.toggle("on", mode === "hidden");
    frame();
  }

  function frame(): void {
    window.clearTimeout(frameTimer);
    const now = performance.now();
    const dt = lastStep
      ? Math.min(0.05, (now - lastStep) / 1000)
      : FRAME_MS / 1000;
    lastStep = now;
    width.step(dt, now);
    height.step(dt, now);
    radius.step(dt, now);
    // Only what changed: a style write, even of the same value, can cost a layout.
    const shape = `${width.value}|${height.value}|${radius.value}`;
    if (shape !== drawnShape) {
      drawnShape = shape;
      root.style.width = `${width.value}px`;
      root.style.height = `${Math.max(0, height.value)}px`;
      root.style.borderRadius = `0 0 ${radius.value}px ${radius.value}px`;
    }
    placeSky();
    const counting = paintCountdown();
    if (width.animating || height.animating || radius.animating || counting) {
      frameTimer = window.setTimeout(
        frame,
        counting && !width.animating && !height.animating ? COUNTDOWN_STEP_MS : FRAME_MS,
      );
    } else {
      lastStep = 0;
      // Wake up again when the countdown starts showing.
      const at = fsm.foldAt;
      if (at !== null && fsm.mode === "open") {
        frameTimer = window.setTimeout(
          frame,
          Math.max(0, at - COUNTDOWN_MS - Clock.now()),
        );
      }
    }
  }

  /** The hairline that shrinks before the island folds. True while it shows. */
  function paintCountdown(): boolean {
    const at = fsm.foldAt;
    const span = Math.min(COUNTDOWN_MS, fsm.foldAfterMs * 0.6);
    const left = at === null ? Infinity : at - Clock.now();
    const on = fsm.mode === "open" && left > 0 && left < span;
    // A scale, not a width: it shrinks without laying the island out again.
    const look = on ? `translateX(-50%) scaleX(${(left / span).toFixed(3)})` : "";
    if (look !== drawnCountdown) {
      drawnCountdown = look;
      countdown.style.transform = look;
      countdown.classList.toggle("on", on);
    }
    return on;
  }

  // ── Public ─────────────────────────────────────────────────────────────────

  const hold = (open: boolean) => {
    held = open;
    fsm.hold(open);
  };
  const shortcut = (id: string) => {
    if (id === "talk" || id === "talk-up") {
      chat.holdToTalk(id === "talk");
      return;
    }
    const allow = id === "allow";
    if (id !== "allow" && id !== "deny") return;
    if (chat.answerWaiting(allow)) return;
    // Y / N answer a permission; a question needs its own choice.
    if (cardOnScreen && cardOnScreen.questions.length === 0) {
      const { request } = cardOnScreen;
      cardOnScreen = null;
      cardActions.decide(request, allow ? "allow" : "deny");
    }
  };
  const setKeys = (bound: Record<string, string>) => {
    keys = Object.fromEntries(Object.entries(bound).map(([k, v]) => [k, v.trim() ? shortKeys(v) : (DEFAULT_KEYS[k] ?? "")]));
    chat.setKeys(keys);
    render(last);
  };
  const setFoldAfter = (seconds: number) => {
    fsm.foldAfterMs = Math.max(3, seconds) * 1000;
  };
  // Escape drops a recording first, then closes the chat; with the chat closed, it folds the island.
  window.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || fsm.mode !== "open") return;
    if (chat.cancelVoice()) return;
    if (chat.isOpen()) chat.toggle(false);
    else foldNow();
  });
  const jumpFailed = () => {
    jumpNoteUntil = Clock.now() + JUMP_NOTE_MS;
    render(last);
    window.setTimeout(() => render(last), JUMP_NOTE_MS + 20);
  };
  const greet = (name: string | null = null) => {
    greetName = name;
    if (calm()) return;
    const now = Clock.now();
    greetUntil = now + GREET_MS;
    fsm.open(now);
    render(last);
    focusScene.dropIn("hello");
    window.setTimeout(() => Sound.play("hello"), 1500);
    window.setTimeout(() => {
      greetUntil = 0;
      // Unless the user took over meanwhile (the pointer, the chat, a permission).
      if (!fsm.pinned && !fsm.pointerInside && !chat.isOpen()) fsm.fold(Clock.now());
      render(last);
    }, GREET_MS);
  };
  const pointer = (inside: boolean) => {
    windowPointer = true;
    if (inside === fsm.pointerInside) return;
    if (inside) pointerIn();
    else {
      pointerOut();
      notice(null);
    }
  };
  const setMedia = (now: NowPlaying | null) => {
    media = now;
    setMusic(!!now?.playing);
    render(last);
  };
  const setUsage = (windows: UsageWindow[]) => {
    usage = windows;
    render(last);
  };
  return { render, last: () => last, hold, shortcut, setKeys, setFoldAfter, jumpFailed, greet, chat, pointer, setMedia, setUsage };
}

export { OPEN_WIDTH };
