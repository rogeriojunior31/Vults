// The island's DOM, from a ViewModel. No Tauri here: actions come in, so the lab can render the
// real island with made-up views.
//
// Three modes (fsm.ts). Hidden, only an invisible strip at the top edge is left, to wake it.
// Compact, it is a fixed-size pill: the session in front on the left (Zeca with none), up to four
// vults on the right with a badge each. Open (a click, a permission, the chat), it has a header, the
// focus card (the session in front, or Zeca) beside the flock list (every other session), and
// connector news; a connector's tab swaps the overview for its card (what is open on GitHub).
// The two layers cross-fade; the black shape springs when it grows and eases when it shrinks.
import { Clock } from "../clock";
import type { AlertView, Answer, DigestView, Hush, ProjectPref, ApprovalView, Attention, ConnectorStatus, Diff, MediaAction, NowPlaying, Outcome, SessionRef, SessionView, UsageWindow, ViewModel } from "../bridge";
import { el } from "../dom";
import { Sound, type Cue } from "../sound";
import { Tracked } from "./anim";
import { idleClip, setMusic } from "./behavior";
import { ChatPanel, type ChatBackend } from "./chat";
import { IslandMachine, presenceNow, type Mode } from "./fsm";
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
import { Ticker, tickerSteps } from "./ticker";
import { Sky, type SkyBox, type SkyPerch } from "./sky";
import { assignSpecies, setZeca as setZecaShown, setZecaLook, zecaLook, zecaShown, zecaSpecies } from "./flock";
import { lookPicker } from "./looks";
import { birdCard, type BirdActions } from "./birds";
import { boardCard, staleNote } from "./board";
import { CONNECTORS } from "../connectors";
import { activityCard, agentName, badgeOf, diffCard, flockRows, focusCard, greetingCard, menuCard, settledCard, statusClass, statusText, usageMeters, type MenuActions, type Settled } from "./views";

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
  /** Puts the session in front: core keeps the choice, the next view carries it (absent in tests
   *  that build their own island: the click alone holds it there). */
  focus?(agent: SessionView["agent"], id: string): void;
  /** Gives the choice of the session in front back to core's rule. */
  unfocus?(): void;
  /** Quick actions: the session's folder, or one file of a step's diff, in the editor (absent in
   *  tests that do not open things). */
  openFolder?(agent: SessionView["agent"], id: string): void;
  openFile?(agent: SessionView["agent"], id: string, step: number, file: number): void;
  /** A quick action on the session's project: mute, pin or hide it, or undo it (absent in tests
   *  that do not set them). */
  projectPref?(agent: SessionView["agent"], id: string, pref: ProjectPref, on: boolean): void;
  /** A quick action: the session's project's bird, or null for the pool's draw (absent in tests
   *  that do not set it). */
  projectBird?(agent: SessionView["agent"], id: string, species: string | null): void;
  /** The answer to a quiet bird (absent in tests that do not answer one). */
  hush?(agent: SessionView["agent"], id: string, hush: Hush): void;
  /** The digest read and closed (absent in tests). */
  dismissDigest?(): void;
  /** Ends do not disturb (the moon in the header; absent in tests). */
  endDnd?(): void;
  /** Whether VS Code is there now: asked each time the menu opens, so its words stay true. */
  editorFound?(): Promise<boolean>;
  /** A step's whole diff; null once the step is gone. */
  stepDiff(agent: SessionView["agent"], id: string, step: number): Promise<Diff | null>;
  openSettings(): void;
  /** Zeca's look picked on the island: the same setting as Settings → Flock → Look (absent in
   *  tests that do not pick). */
  setLook?(look: string): void;
  /** The island or a connector's card opened: connectors fetch again if their news is old. */
  opened(): void;
  /** A click on a row of a connector's card. */
  openRow(connector: string, item: string): void;
  /** How the connector's last poll went (absent in tests that do not care: never stale). */
  connectorStatus?(connector: string): Promise<Pick<ConnectorStatus, "lastOk" | "error"> | null>;
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
/** An open connector's card asks how its polls go this often: the view carries rows, not errors,
 *  and a poll that fails or recovers with the same rows changes nothing in it. A local read. */
const STATUS_EVERY_MS = 5000;
/** How long "couldn't find the terminal" stays on the card. */
const JUMP_NOTE_MS = 2600;
/** A permission goes back to its terminal this long after it arrived (the server's decision
 *  deadline, `protocol::limits::SERVER_DECISION_TIMEOUT`). */
const EXPIRES_MS = 108_000;
/** The card counts down over the last part of that. */
const EXPIRY_SHOWN_MS = 30_000;
/** How long the card says what became of a permission before the next thing shows. */
const SETTLED_MS: Record<Settled, number> = { allow: 700, deny: 700, answered: 700, released: 1600, terminal: 1600, expired: 2600 };
/** How core says a card ended, as the settled card shows it (a rule's answer reads as an Allow). */
const SETTLED_AS: Record<Outcome, Settled> = {
  allowed: "allow",
  denied: "deny",
  answered: "answered",
  released: "released",
  terminal: "terminal",
  expired: "expired",
  rule: "allow",
};
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

/** The sound of a session's news, by how much it wants the user (core says, the island sounds). */
const CUE: Partial<Record<Attention, Cue>> = {
  "needs-you": "approval",
  done: "done",
  failed: "fail",
};
const cueFor = (s: SessionView): Cue | undefined =>
  s.attention === "needs-you" && s.status === "question" ? "question" : CUE[s.attention];

/**
 * How long a state must hold before it is news. In auto mode every tool call passes through a
 * permission request the classifier clears in a blink, and a turn's Stop is often followed at once
 * by the next prompt: only a state that stays is worth a sound, a badge or opening the island.
 */
const SETTLE_MS: Partial<Record<Attention, number>> = {
  "needs-you": 1500,
  failed: 1500,
  done: 3000,
};

const key = (s: SessionView) => `${s.agent}:${s.id}`;
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
  /** A global shortcut: answers the permission card on screen, holds the chat's mic (`talk`, `talk-up`), or opens the island (`open`). */
  shortcut(id: string): void;
  /** The keys the desktop bound for the shortcuts. */
  setKeys(keys: Record<string, string>): void;
  /** How long the open island waits before folding once the pointer leaves. */
  setFoldAfter(seconds: number): void;
  /** Hovering opens the island (Settings → General). */
  setOpenOnHover(on: boolean): void;
  /** Rare visitors on or off (Settings → Flock). */
  setVisitors(on: boolean): void;
  /** Zeca on or off (Settings → Flock). */
  setZeca(on: boolean): void;
  /** The look setting as saved (`auto`, `none` or a look), for the picker to mark. */
  setLookSetting(look: string): void;
  /** Right-click on Zeca: his looks, in place of the overview (the lab opens it directly). */
  openLooks(): void;
  /** A rare visitor now (the lab). */
  visitNow(): void;
  /** A session's quick actions, as a right-click on its bird or row opens them (the lab). */
  openMenu(agent: SessionView["agent"], id: string): void;
  /** Whether VS Code (`code`) is there to open folders and files in, for the menu's words. */
  setEditor(found: boolean): void;
  /** Open terminal found nothing to bring forward. */
  jumpFailed(): void;
  /** Zeca lands and says hello at start-up, by the user's first name when there is one. */
  greet(name?: string | null): void;
  /** The window says the pointer came onto it or left it (Linux: GTK's crossings, in order). */
  pointer(inside: boolean): void;
  /** Another of the app's windows took the focus: by the panel, the open island folds. */
  away(): void;
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

  /** The view as drawn: empty while paused. */
  let last: ViewModel = { sessions: [], approval: null, alerts: [] };
  /** The view as core sent it, drawn again whole once the pause ends. */
  let raw: ViewModel = last;
  /** The paused view, made once per view (a new one would be news to `cues` each time). */
  let pausedFrom: ViewModel | null = null;
  let pausedAs: ViewModel = last;
  /** Paused (ADR 0009), the island is empty: core sends every card to its terminal, the
   *  connectors stop, and the pill says so. */
  function drawn(v: ViewModel): ViewModel {
    if (presenceNow() !== "paused") return v;
    if (v !== pausedFrom) {
      pausedFrom = v;
      pausedAs = { ...v, sessions: [], approval: null, alerts: [], boards: [], attention: "quiet", focus: null, front: null, digest: null };
    }
    return pausedAs;
  }
  let media: NowPlaying | null = null;
  let usage: UsageWindow[] = [];
  const chat = new ChatPanel(actions.chat, () => render(raw));

  /** The lab holds the island open: nothing folds or unpins it. */
  let held = false;
  /** The session just clicked, until a view carrying core's focus comes back. */
  let picked: string | null = null;
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
  /** The permission card last on screen. */
  let lastShown: { request: string; session: string; target: string } | null = null;
  /** What became of the permission just settled, shown on its session's card for a moment. */
  let settled: {
    session: string;
    request: string;
    how: Settled;
    target: string;
    until: number;
    /** The session as it last was: an agent that quit takes its session away with the card. */
    view: SessionView | null;
  } | null = null;
  let expiryTimer: number | undefined;
  /** The session in front, whose steps the ticker shows. */
  let inFront: SessionView | null = null;
  /** The diff shown in place of the focus card: its lines are undefined until they come. */
  let diffOpen: { session: string; step: number; text: string; diff: Diff | null | undefined } | null = null;
  /** The session whose quick actions are open, in place of the focus card. */
  let menuOpen: string | null = null;
  /** The session whose kept steps are listed, in place of the focus card. */
  let activityOpen: string | null = null;
  /** The session whose project's bird is being picked, in place of the focus card. */
  let birdsOpen: string | null = null;
  let editorFound = false;
  /** The connector whose card the user opened from its tab, in place of the overview. */
  let boardOpen: string | null = null;
  /** The card on screen right now (a permission takes its place while it waits). */
  let boardShown: string | null = null;
  const boardHost = el("div", { class: "board-host" });
  /** Zeca's looks are open (a right-click on him); they give way to a card or the chat. */
  let looksOpen = false;
  /** The look under the pointer, worn as a preview; undefined when none is. "none" wears nothing. */
  let lookPreview: string | undefined;
  /** What he wore when the picker opened: worn again when it closes, unless a view says what he
   *  wears now (a pick in Settings, a new day). */
  let lookBefore: string | null = null;
  const worn = (): string | null => ("look" in raw ? (raw.look ?? null) : lookBefore);
  /** The saved setting, marked in the picker. */
  let lookSetting = "auto";
  const looksHost = el("div", { class: "looks-host" });
  let looksSig = "";
  const wear = (look: string | undefined) => setZecaLook(look === undefined ? worn() : look === "none" ? null : look);
  const openLooks = () => {
    // A card waiting is the one thing to read; the looks can wait for it.
    if (!zecaShown() || looksOpen || cardWaits) return;
    looksOpen = true;
    lookBefore = zecaLook();
    lookPreview = undefined;
    boardOpen = null;
    diffOpen = null;
    if (chat.isOpen()) chat.toggle(false);
    Sound.play("tap");
    if (fsm.mode !== "open") fsm.open(Clock.now());
    render(raw);
  };
  const closeLooks = () => {
    if (!looksOpen) return;
    looksOpen = false;
    lookPreview = undefined;
    wear(undefined);
    render(raw);
  };
  const lookActions = {
    preview: (look: string | null) => {
      lookPreview = look ?? undefined;
      wear(lookPreview);
      render(raw);
    },
    pick: (look: string) => {
      lookSetting = look;
      actions.setLook?.(look);
      Sound.play("tap");
      // Worn at once; Auto waits for the view, which brings the calendar's look.
      if (look !== "auto") lookBefore = look === "none" ? null : look;
      closeLooks();
    },
    close: closeLooks,
  };
  /** What the card on screen shows: rebuilt only when it changes, or a row loses its click. */
  let boardSig = "";
  /** A permission card waits: it wins over a connector's card (ADR 0009), so their tabs rest. */
  let cardWaits = false;
  /** How the open card's connector last polled, once asked. */
  let boardStatus: Pick<ConnectorStatus, "lastOk" | "error"> | null = null;
  let boardStale: string | null = null;
  let statusTimer: number | undefined;
  const closeDiff = () => {
    diffOpen = null;
    render(raw);
  };
  /** A kept step's diff in place of the session's card; its lines come from the loop. */
  const openDiff = (s: SessionView, step: number) => {
    const k = key(s);
    const text = tickerSteps(s).find((t) => t.n === step)?.text ?? "Changes";
    menuOpen = null;
    diffOpen = { session: k, step, text, diff: undefined };
    Sound.play("tap");
    render(raw);
    void actions.stepDiff(s.agent, s.id, step).then((diff) => {
      if (diffOpen?.session !== k || diffOpen.step !== step) return;
      diffOpen = { ...diffOpen, diff };
      render(raw);
    });
  };
  ticker.onDiff = (step) => {
    if (inFront) openDiff(inFront, step);
  };
  const closeActivity = () => {
    activityOpen = null;
    render(raw);
  };
  const birdActions: BirdActions = {
    pick: (s, species) => {
      birdsOpen = null;
      actions.projectBird?.(s.agent, s.id, species);
      Sound.play("tap");
      render(raw);
    },
    close: () => {
      birdsOpen = null;
      render(raw);
    },
  };
  /** Core's focus, as a session key. */
  const focusKey = (): string | null => (last.focus ? `${last.focus.agent}:${last.focus.id}` : null);
  /** A session's quick actions, in place of its card; the island opens for them. A card waiting
   *  is the one thing to read: the menu waits for it, like the looks. */
  const openMenu = (k: string) => {
    if (cardWaits || !last.sessions.some((s) => key(s) === k)) return;
    menuOpen = k;
    activityOpen = null;
    birdsOpen = null;
    diffOpen = null;
    boardOpen = null;
    if (looksOpen) closeLooks();
    if (chat.isOpen()) chat.toggle(false);
    Sound.play("tap");
    if (fsm.mode !== "open") fsm.open(Clock.now());
    render(raw);
    void actions.editorFound?.().then((found) => {
      if (found === editorFound) return;
      editorFound = found;
      render(raw);
    }, () => {});
  };
  const menuActions: MenuActions = {
    jump: (s) => {
      menuOpen = null;
      actions.jump(s.agent, s.id);
      render(raw);
    },
    openFolder: (s) => {
      menuOpen = null;
      actions.openFolder?.(s.agent, s.id);
      Sound.play("tap");
      render(raw);
    },
    openFile: (s, step) => {
      menuOpen = null;
      actions.openFile?.(s.agent, s.id, step, 0);
      Sound.play("tap");
      render(raw);
    },
    activity: (s) => {
      menuOpen = null;
      activityOpen = key(s);
      Sound.play("tap");
      render(raw);
    },
    bird: (s) => {
      menuOpen = null;
      birdsOpen = key(s);
      Sound.play("tap");
      render(raw);
    },
    diff: (s, step) => openDiff(s, step),
    focus: (s) => {
      menuOpen = null;
      if (s) pick(s);
      else {
        picked = null;
        actions.unfocus?.();
        Sound.play("tap");
        render(raw);
      }
    },
    projectPref: (s, pref, on) => {
      menuOpen = null;
      actions.projectPref?.(s.agent, s.id, pref, on);
      Sound.play("tap");
      render(raw);
    },
    close: () => {
      menuOpen = null;
      render(raw);
    },
  };

  function settle(session: string, request: string, target: string, how: Settled, view: SessionView | null): void {
    const until = Clock.now() + SETTLED_MS[how];
    settled = { session, request, how, target, until, view };
    requestSeen.delete(request);
    window.setTimeout(() => render(raw), SETTLED_MS[how] + 20);
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
  // Any press inside the open island: one a hover opened stays open like any other.
  root.addEventListener("pointerdown", () => {
    if (fsm.mode === "open") fsm.interacted();
  });
  fsm.onChange = (from, to) => {
    // A permission opening the island has its own sound; the chat opening is the user's own doing.
    if (to === "open" && !fsm.pinned && !chat.isOpen()) Sound.play("open");
    else if (from === "open") Sound.play("close");
    else if (from === "hidden") Sound.play("peek");
    if (to === "open" && from !== "open") actions.opened();
    if (to === "open" && from !== "open" && chatWhenOpened && !chat.isOpen())
      chat.toggle(true);
    if (from === "open" && to !== "open") {
      diffOpen = null;
      boardOpen = null;
      menuOpen = null;
      activityOpen = null;
      birdsOpen = null;
      closeLooks();
      chatWhenOpened = chat.isOpen();
      if (chat.isOpen()) chat.toggle(false);
    }
    render(raw);
  };

  /** Puts a session in front: core keeps the choice, and the next view brings it. */
  const pick = (s: SessionView) => {
    picked = key(s);
    diffOpen = null;
    activityOpen = null;
    birdsOpen = null;
    actions.focus?.(s.agent, s.id);
    jumpNoteUntil = 0;
    Sound.play("tap");
    render(raw);
  };

  // ── News ───────────────────────────────────────────────────────────────────

  // What was already on screen, so only changes make a sound.
  const statuses = new Map<string, SessionView["status"]>();
  /** The `seq` of each alert already announced; only those still on screen are kept. */
  let alertsSeen = new Set<number>();
  let primed = false;
  /** Status changes waiting to settle, by session. */
  const settling = new Map<string, number>();
  /** Sessions whose current state was announced: it shows, and a permission opens the island. */
  const announced = new Set<string>();
  /** Each quiet bird's flag, so only turning loud sounds. */
  const silences = new Map<string, NonNullable<SessionView["silent"]>>();
  /** When each session was first seen: the wire keeps that order, so birds don't shuffle. */
  const firstSeen = new Map<string, number>();

  function cues(v: ViewModel): void {
    const now = Clock.now();
    for (const s of v.sessions) {
      const k = key(s);
      const fresh = !firstSeen.has(k);
      if (fresh) {
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
      // So is a session that comes into view already in it (its project shown again), unless it
      // brings a card.
      if (!primed || (fresh && !s.card && s.attention !== "needs-you")) {
        if (SETTLE_MS[s.attention]) announced.add(k);
        continue;
      }
      const cue = cueFor(s);
      if (!cue) continue;
      const status = s.status;
      const wait = SETTLE_MS[s.attention] ?? 0;
      settling.set(
        k,
        window.setTimeout(() => {
          settling.delete(k);
          // Still in that state after the wait: now it is news.
          if (statuses.get(k) !== status) return;
          announced.add(k);
          // Quiet keeps only a card's sound (ADR 0009); the rest of the wire stays silent.
          // Paused, a state that started settling before the pause stays silent too. A muted
          // project is quiet at rest; its card keeps its sound (ADR 0009).
          const preset = presenceNow();
          const now = raw.sessions.find((x) => key(x) === k);
          if (preset !== "paused" && (now?.card || (!now?.muted && preset !== "quiet"))) Sound.play(cue, !!now?.card);
          fsm.reveal(Clock.now());
          render(raw);
        }, wait),
      );
    }
    const present = new Set(v.sessions.map(key));
    // A session that left the view (gone, or its project hidden) is forgotten whole: back on the
    // wire, it starts quietly (see `fresh`).
    for (const k of firstSeen.keys()) if (!present.has(k)) firstSeen.delete(k);
    for (const k of statuses.keys()) {
      if (present.has(k)) continue;
      statuses.delete(k);
      announced.delete(k);
      seen.delete(k);
      window.clearTimeout(settling.get(k));
      settling.delete(k);
    }
    // A quiet bird turning loud is worth one sound, like news (not muted, not at rest in Quiet).
    for (const s of v.sessions) {
      const k = key(s);
      const was = silences.get(k);
      if (s.silent) silences.set(k, s.silent);
      else silences.delete(k);
      const preset = presenceNow();
      if (primed && s.silent === "loud" && was !== "loud" && !s.muted && preset !== "paused" && preset !== "quiet") {
        Sound.play("alert");
        fsm.reveal(now);
      }
    }
    for (const k of silences.keys()) if (!present.has(k)) silences.delete(k);
    const alertsNow = new Set<number>();
    for (const a of v.alerts) {
      alertsNow.add(a.seq);
      if (primed && !alertsSeen.has(a.seq) && presenceNow() !== "quiet") {
        Sound.play(
          a.level === "ok" || a.level === "info" ? "alertOk" : "alert",
        );
        fsm.reveal(now);
      }
    }
    alertsSeen = alertsNow;
    primed = true;
  }

  // ── Open layer ─────────────────────────────────────────────────────────────

  /** Everything [`header`] draws, as one string: the same string, the same header. */
  let headerShown = "";
  function headerShows(): string {
    const view = !chat.isOpen() ? "flock" : chat.isShowingDrop() ? "drop" : "chat";
    const nowSecs = Date.now() / 1000;
    const live = usage.filter((w) => w.resets_at === null || w.resets_at > nowSecs);
    const boards = (last.boards ?? []).map((b) => b.connector);
    return JSON.stringify([view, boardShown, boards, cardWaits, media, live, Sound.isEnabled(), fsm.pinned && !held, zecaShown(), !!raw.dnd]);
  }

  const connectorName = (id: string) => CONNECTORS.find((c) => c.id === id)?.name ?? id;

  function openBoard(connector: string): void {
    if (chat.isOpen()) chat.toggle(false);
    boardOpen = connector;
    Sound.play("tap");
    actions.opened();
    boardStatus = null;
    window.clearInterval(statusTimer);
    askStatus(connector);
    statusTimer = window.setInterval(() => (boardOpen === connector ? askStatus(connector) : window.clearInterval(statusTimer)), STATUS_EVERY_MS);
    render(raw);
  }

  /** The card keeps its rows after a failed poll; this says they are old, and why. */
  function askStatus(connector: string): void {
    void actions.connectorStatus?.(connector).then((status) => {
      if (boardOpen !== connector || JSON.stringify(status) === JSON.stringify(boardStatus)) return;
      boardStatus = status;
      render(raw);
    }, () => {});
  }

  function header(): HTMLElement {
    // Which part of the island shows: the flock, the chat, or the chat waiting for a file.
    const view = !chat.isOpen() ? (boardShown ? `board:${boardShown}` : "flock") : chat.isShowingDrop() ? "drop" : "chat";
    const tab = (name: string, glyph: IconName, label: string, go: () => void) => {
      const b = el("button", { class: `tab${name === view ? " on" : ""}`, onclick: go }, icon(glyph, 14));
      b.title = label;
      b.setAttribute("aria-label", label);
      return b;
    };
    const tabs = el(
      "div",
      { class: "tabs" },
      tab("flock", "flock", "Flock", () => {
        boardOpen = null;
        if (chat.isOpen()) chat.toggle(false);
        else render(raw);
      }),
      // The chat and its drop zone are Zeca's (ADR 0010).
      ...(zecaShown()
        ? [
            tab("chat", "chat", "Chat", () => {
              boardOpen = null;
              chat.hideDrop();
              chat.toggle(true);
            }),
            tab("drop", "plus", "Drop a file", () => {
              boardOpen = null;
              chat.showDrop();
            }),
          ]
        : []),
      ...(last.boards ?? []).map((b) => {
        const name = connectorName(b.connector);
        if (!cardWaits) return tab(`board:${b.connector}`, "pull", name, () => openBoard(b.connector));
        // Clicked now, it would open only once the permission is answered: say so instead.
        const t = tab(`board:${b.connector}`, "pull", `${name} opens once the permission is answered`, () => {});
        t.disabled = true;
        return t;
      }),
    );
    const soundOn = Sound.isEnabled();
    const sound = el(
      "button",
      { class: "icon-btn", onclick: () => actions.setSounds(!soundOn) },
      icon(soundOn ? "sound" : "mute", 14),
    );
    sound.title = soundOn ? "Mute" : "Unmute";
    // Shown only while do not disturb lasts: a click ends it.
    const moon = raw.dnd ? el("button", { class: "icon-btn dnd", onclick: () => actions.endDnd?.() }, icon("moon", 14)) : null;
    if (moon) moon.title = "Do not disturb is on: no sounds or notifications. Click to end it.";
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
      el("div", { class: "header-actions" }, usageMeters(usage, Date.now() / 1000), moon, sound, gear, fold),
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
    const paused = presenceNow() === "paused";
    const detail = song ? (song.artist ?? "") : front ? statusText(front) : paused ? "Agents ask in their terminals" : zecaShown() ? "Nothing running" : "Start an agent and it lands here";
    const vults = compactScene.slots().filter((s) => s.key !== "zeca");
    // The text runs between Zeca and the leftmost vult.
    const right = vults.length
      ? Math.min(...vults.map((s) => s.x)) - 10
      : COMPACT_W - 14;
    // No bird on Zeca's spot (he is off, nobody runs): the text takes it.
    const left = !front && !zecaShown() ? 14 : 44;
    compactText.style.left = `${left}px`;
    compactText.style.width = `${Math.max(40, right - left)}px`;
    compactText.replaceChildren(
      el("span", {
        class: song ? "name song" : "name",
        text: song ? `♪ ${song.title}` : front ? front.project || agentName(front) : paused ? "Paused" : zecaShown() ? "Zeca" : "Nothing running",
      }),
      el("span", { class: `status ${song ? "music" : front ? statusClass(front) : "none"}`, text: detail }),
      ...(alerts ? [el("span", { class: "news", text: `${alerts} new` })] : []),
    );
    const byKey = new Map(shown.map((s) => [key(s), s]));
    badges.replaceChildren(
      ...vults.flatMap((slot) => {
        const kind = badgeOf(byKey.get(slot.key));
        if (!kind) return [];
        const b = el("span", { class: `badge ${kind}` });
        b.style.left = `${slot.x + slot.width - 4}px`;
        return [b];
      }),
    );
  }

  // ── Render ─────────────────────────────────────────────────────────────────

  /** The digest already shown (it opens the island once), and whether a first view came. */
  let digestSeen: number | null = null;
  let primedDigest = false;
  /** The reminders the card on screen has sounded, by request (the attention ladder). */
  let reminded: { request: string; n: number } | null = null;

  function render(view: ViewModel): void {
    raw = view;
    Sound.setHushed(!!view.dnd);
    // The chat, a connector's card or a permission took the island: the looks close, so an
    // unpicked preview is never worn elsewhere nor comes back by itself.
    if (looksOpen && (chat.isOpen() || boardOpen || view.approval)) {
      looksOpen = false;
      lookPreview = undefined;
      wear(undefined);
    }
    // A view sets what he wears today (island.ts); the picker's preview wins while it is open.
    if (looksOpen) wear(lookPreview);
    const v = drawn(view);
    // The click holds until core's focus names it: a view sent before core took the click must
    // not swing the front back for a moment.
    if (picked && v.focus && `${v.focus.agent}:${v.focus.id}` === picked) picked = null;
    if (v !== last) cues(v);
    const prev = last;
    last = v;
    const now = Clock.now();
    const byKey = new Map(v.sessions.map((s) => [key(s), s]));
    if (picked && !byKey.has(picked)) picked = null;
    // States that have not settled yet show as plain work: no wings, no badge, no card. A state
    // the user said OK to shows as idle.
    const shown = v.sessions
      .map((s): SessionView => {
        const k = key(s);
        if (SETTLE_MS[s.attention] && !announced.has(k)) return { ...s, status: "working", attention: "quiet", card: false };
        if (seen.get(k) === s.status) return { ...s, status: "idle", note: null, attention: "quiet", card: false };
        return s;
      })
      // Pinned projects first (core's order), then as they arrived: birds don't shuffle.
      .sort((a, b) => Number(!!b.pinned) - Number(!!a.pinned) || (firstSeen.get(key(a)) ?? 0) - (firstSeen.get(key(b)) ?? 0));
    const approval = v.approval;
    if (approval && !requestSeen.has(approval.request)) requestSeen.set(approval.request, now);
    // The card on screen went away: core says how (here, in the terminal, expired), and the card
    // says it for a moment.
    if (lastShown && approval?.request !== lastShown.request) {
      const shownRequest = lastShown.request;
      const end = v.ended?.find((e) => e.request === shownRequest);
      const was = prev?.sessions.find((s) => key(s) === lastShown?.session) ?? null;
      if (end) settle(lastShown.session, shownRequest, lastShown.target, SETTLED_AS[end.outcome], was);
      lastShown = null;
    }
    // Only the first permission in line has a card; it shows once its session's state settled.
    const pending = (approval && shown.find((s) => s.card)) || null;
    cardWaits = pending !== null;
    // The card still waits: each reminder core counts sounds its cue again (C4), except under do
    // not disturb. Its first one came when it opened the island.
    if (approval && pending) {
      const n = approval.reminders ?? 0;
      if (reminded?.request === approval.request && n > reminded.n && presenceNow() !== "paused") {
        Sound.play(approval.questions.length ? "question" : "approval");
      }
      reminded = { request: approval.request, n };
    } else reminded = null;
    const shownByKey = new Map(shown.map((s) => [key(s), s]));
    const recent = settled && now < settled.until ? settled : null;
    const settledSession = recent ? (shownByKey.get(recent.session) ?? (byKey.has(recent.session) ? null : recent.view)) : null;
    // Core says who is in front (the card's session, the user's choice, the first at work). While
    // the island holds a state back (not settled yet, or said OK to) it holds back core's reason
    // for it too, and picks among what it shows.
    const ref = (r: SessionRef | null | undefined) => (r ? shownByKey.get(`${r.agent}:${r.id}`) : undefined);
    const coreFront = ref(v.front);
    const asCore = coreFront && coreFront === byKey.get(key(coreFront)) ? coreFront : null;
    const chosen = picked ? shownByKey.get(picked) : null;
    // A card, the chat or a connector's card takes the island: the quick actions give way, as the
    // looks do. A session that left takes its menu with it.
    if (pending || chat.isOpen() || boardOpen) menuOpen = activityOpen = birdsOpen = null;
    if (menuOpen && !shownByKey.has(menuOpen)) menuOpen = null;
    if (activityOpen && !shownByKey.has(activityOpen)) activityOpen = null;
    if (birdsOpen && !shownByKey.has(birdsOpen)) birdsOpen = null;
    // What the user opened on a session holds it in front: its menu, its steps, its bird, a diff.
    const asked = menuOpen ?? activityOpen ?? birdsOpen ?? diffOpen?.session ?? null;
    const front = settledSession ?? pending ?? (asked ? shownByKey.get(asked) : null) ?? chosen ?? asCore ?? ref(v.focus) ?? shown[0] ?? null;
    // The diff belongs to its session's card: anything else in front, or a card to answer, closes it.
    if (diffOpen && (!front || key(front) !== diffOpen.session || settledSession || front === pending)) diffOpen = null;
    inFront = front;

    // A settled permission opens the island and keeps it open until it is answered.
    if (pending && v.approval && !fsm.pinned) fsm.openPinned();
    else if (!pending && fsm.pinned && !held) fsm.unpin(now);
    fsm.setOccupied(v.sessions.length > 0 || v.alerts.length > 0 || !!v.digest, now);
    // Back from away: the digest opens the island once; it folds as usual after.
    if (v.digest && v.digest.seq !== digestSeen) {
      // Only the top island opens for news at rest: by the panel or in Quiet it waits there.
      // The chat would hide it: back from away, the news comes first (the chat keeps its place).
      if ((digestSeen !== null || primedDigest) && presenceNow() === "island") {
        if (chat.isOpen()) chat.toggle(false);
        fsm.open(now);
      }
      digestSeen = v.digest.seq;
    }
    primedDigest = true;
    if (chat.isOpen() && fsm.mode !== "open") fsm.open(now);
    fsm.setEngaged(chat.isOpen(), now);
    const mode: Mode = fsm.mode;

    chat.setFolders(
      shown.flatMap((s) => (s.cwd ? [{ cwd: s.cwd, project: s.project }] : [])),
      front?.cwd ?? null,
    );
    const others = shown.filter((s) => s !== front);
    const chatShown = chat.isOpen() && !pending;
    const board = !chat.isOpen() && !pending && boardOpen ? (v.boards ?? []).find((b) => b.connector === boardOpen) : undefined;
    const picking = looksOpen && !chat.isOpen() && !pending && zecaShown();
    // Switched off meanwhile: back to the flock.
    if (boardOpen && !(v.boards ?? []).some((b) => b.connector === boardOpen)) boardOpen = null;
    boardShown = board ? board.connector : null;
    // Zeca stays on the wire with nobody there, so the island is never a blank shape. In the chat
    // he is the chat: thinking, swallowing a file, waiting for an answer.
    // With Zeca off (ADR 0010) an empty wire stays empty.
    const idle = zecaShown() ? { clip: idleClip(), agent: "claude" as const, alone: true } : null;
    assignSpecies(shown);
    compactScene.update(shown, front, front ? null : idle);
    if (chatShown) focusScene.update([], null, { clip: chat.clip(now), agent: chat.agent() });
    else focusScene.update([], front, front ? null : idle);
    listScene.update(others, null);
    listScene.setHeight(Math.max(1, others.length) * LIST_ROW);
    // Locked, nobody is looking: the scene and the sky rest (no frames, no timers).
    const awake = !v.locked;
    compactScene.setActive(awake && mode === "compact");
    // Zeca is on the focus card or in the chat: drawn either way while the island is open.
    focusScene.setActive(awake && mode === "open" && !board);
    // A permission (or what became of it), or a diff, takes the whole width: it is the one thing to read.
    const wide = settledSession !== null || (pending !== null && front === pending) || ((diffOpen !== null || asked !== null) && !chatShown);
    const listShown = others.length > 0 && !wide;
    listScene.setActive(awake && mode === "open" && !chatShown && !board && !picking && listShown);
    sky.update(shown, awake && mode !== "hidden");
    paintCompact(front, shown, v.alerts.length);

    const asking = mode === "open" && !chatShown && !settledSession && front !== null && front === pending;
    cardOnScreen = asking ? approval : null;
    if (asking && approval) lastShown = { request: approval.request, session: key(front), target: cardTarget(approval) };
    // The card that took the keyboard is gone (answered elsewhere, expired): give it back.
    if (cardKeyboard && !asking) cardActions.keyboard(false);
    if (mode === "open") {
      // Rebuilt only when what it shows changed: renders come with every agent event and every
      // keystroke, and a button replaced between press and release (Pause, Mute) loses the click.
      const shows = headerShows();
      if (shows !== headerShown || !headerSlot.firstChild) {
        headerShown = shows;
        headerSlot.replaceChildren(header());
      }
      if (chatShown) {
        perch.className = "perch chatting";
        if (perch.parentElement !== chat.perchSlot) chat.perchSlot.append(perch);
        show(chat.element);
      } else if (board) {
        const sig = JSON.stringify(board);
        const stale = boardStatus ? staleNote(boardStatus, Date.now() / 1000) : null;
        if (sig !== boardSig || stale !== boardStale || !boardHost.firstChild) {
          boardSig = sig;
          boardStale = stale;
          boardHost.replaceChildren(boardCard(board, connectorName(board.connector), (item) => actions.openRow(board.connector, item), stale));
        }
        show(boardHost);
      } else if (picking) {
        // Rebuilt only when the marked look or the species changes: hovering must not swap the tiles.
        const sig = `${lookSetting}|${zecaSpecies()}`;
        if (sig !== looksSig || !looksHost.firstChild || !looksHost.contains(perch)) {
          looksSig = sig;
          perch.className = "perch";
          looksHost.replaceChildren(lookPicker(lookSetting, perch, lookActions));
        }
        show(looksHost);
      } else {
        show(overview);
        paintFocus(front, approval, now, settledSession ? recent : null);
        tickExpiry();
        flock.classList.toggle("none", !listShown);
        rows.style.maxHeight = `${LIST_ROWS * LIST_ROW}px`;
        rows.replaceChildren(...flockRows(others, listScene.canvas, pick));
      }
      // The looks fill the island: news waits under the overview, so it never grows past its surface.
      const news = !chat.isOpen() && !picking;
      alertsSlot.replaceChildren(
        ...(news && v.digest ? [digestBox(v.digest)] : []),
        ...(news && v.alerts.length ? [alertsBox(v.alerts)] : []),
      );
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
    // A connector's card takes the whole overview, and that edge is its corner: every bird lands
    // in the header instead, side by side in the gap after the tabs (the rest share its last spot).
    const rect = root.getBoundingClientRect();
    const tabs = boardShown && fsm.mode === "open" ? headerSlot.querySelector(".tabs") : null;
    const gap = tabs && { from: tabs.getBoundingClientRect().right, bottom: tabs.getBoundingClientRect().bottom,
      to: (tabs.nextElementSibling ?? tabs).getBoundingClientRect().left };
    let spare = 0;
    for (const session of last.sessions) if (!anchors.has(key(session))) anchors.set(key(session), gap
      ? { x: Math.max(gap.from + 30, Math.min(gap.from + 30 + 32 * spare++, gap.to - 30)) - skyRect.left, y: gap.bottom - skyRect.top - 16, scale: 1 }
      : { x: rect.right - skyRect.left - 20, y: rect.bottom - skyRect.top - 16, scale: 1 });
    sky.place(anchors, {
      left: rect.left - skyRect.left, top: rect.top - skyRect.top, width: rect.width, height: rect.height,
      radius: parseFloat(getComputedStyle(root).borderBottomLeftRadius) || 0,
      // Only the open layer's: the folded one keeps its cards in the tree, faded out. Rows
      // scrolled out of the list are not there to hide behind.
      cards: fsm.mode === "open" ? cardRects(skyRect) : [],
    });
  }
  /** The cards and visible rows in the sky's coordinates (placeSky runs every frame of a change).
   *  A card fading out sits right over its successor: it is left out, the two would cancel out. */
  function cardRects(sky: DOMRect): NonNullable<SkyBox["cards"]> {
    const list = rows.getBoundingClientRect();
    const out: NonNullable<SkyBox["cards"]> = [];
    const radius = new Map<string, number>();
    for (const card of inner.querySelectorAll<HTMLElement>(".card:not(.leaving), .board, .flock-row")) {
      const c = card.getBoundingClientRect();
      const row = card.classList.contains("flock-row");
      if (row && (c.bottom <= list.top || c.top >= list.bottom)) continue;
      const kind = row ? "row" : card.classList.contains("board") ? "board" : "card";
      if (!radius.has(kind)) radius.set(kind, parseFloat(getComputedStyle(card).borderTopLeftRadius) || 0);
      out.push({ left: c.left - sky.left, top: c.top - sky.top, width: c.width, height: c.height, radius: radius.get(kind)! });
    }
    return out;
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

  /** A right-click on Zeca: his looks. Open, on his perch; folded, on his spot in the pill. */
  root.addEventListener("contextmenu", (e) => {
    // Text fields keep the web view's own menu (copy, paste); everywhere else it would offer
    // Back and Reload, which mean nothing here.
    if ((e.target as Element).closest?.("input, textarea, [contenteditable]")) return;
    e.preventDefault();
    let over = fsm.mode === "open" && overZeca;
    if (fsm.mode === "compact") {
      const slot = compactScene.slots().find((s) => s.key === "zeca");
      const r = compactScene.canvas.getBoundingClientRect();
      over = !!slot && e.clientX - r.left >= slot.x && e.clientX - r.left <= slot.x + slot.width;
    }
    // The front spot holds Zeca (the chat, an empty wire) or the session in front, in its own bird.
    const scene = fsm.mode === "compact" ? compactScene : focusScene;
    if (over && scene.zecaPerched()) {
      if (zecaShown()) openLooks();
      return;
    }
    if (over && inFront) {
      openMenu(key(inFront));
      return;
    }
    // Any other bird or row: that session's quick actions. On the open card, its session's.
    const row = (e.target as Element).closest?.(".flock-row") as HTMLElement | null;
    let k = row?.dataset.key ?? null;
    if (!k && fsm.mode === "compact") {
      const r = compactScene.canvas.getBoundingClientRect();
      k = compactScene.slots().find((s) => s.key !== "zeca" && e.clientX - r.left >= s.x && e.clientX - r.left <= s.x + s.width)?.key ?? null;
    }
    if (!k && fsm.mode === "open" && (e.target as Element).closest?.(".card.focus") && inFront) k = key(inFront);
    if (!k) return;
    openMenu(k);
  });

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
    const greeting = now < greetUntil && !s?.card && !done;
    const diff = !greeting && !done && s && diffOpen?.session === key(s) ? diffOpen : null;
    const menu = !greeting && !done && !diff && s && menuOpen === key(s);
    const activity = !greeting && !done && !diff && !menu && s && activityOpen === key(s);
    const birds = !greeting && !done && !diff && !menu && !activity && s && birdsOpen === key(s);
    const k = greeting
      ? "greeting"
      : done
        ? `settled|${done.request}|${done.how}`
        : diff
          ? `diff|${diff.session}|${diff.step}`
          : menu
            ? `menu|${key(s)}`
            : activity
              ? `activity|${key(s)}`
              : birds
                ? `birds|${key(s)}`
                : s
                ? `${key(s)}|${s.status}`
                : "empty";
    // New steps would scroll the diff back to its top: it is rebuilt only when its lines come.
    const sig = diff ? JSON.stringify([s?.status, s?.project, diff.diff === undefined ? "loading" : diff.diff === null ? "gone" : "lines"]) : JSON.stringify([
      s?.status,
      s?.project,
      s?.note,
      s?.steps.slice(-2),
      s?.diffs?.slice(-2),
      s?.step_count,
      s?.subagents,
      s?.cwd,
      s?.card ? approval : null,
      keys,
      jumpFailed,
      presenceNow(),
      zecaShown(),
      s?.waiting,
      s?.raise,
      s?.silent,
      menu || activity ? [s?.steps, s?.diffs, focusKey(), editorFound] : null,
      birds ? [s?.species, s?.bird_chosen] : null,
    ]);
    // Zeca's perch may be in the chat: a card without him is repainted to take him back.
    if (card && k === cardKey && sig === cardSig && card.contains(perch)) return;
    const next = greeting
      ? greetingCard(perch, greetName)
      : diff && s
        ? diffCard(s, diff.text, diff.diff, perch, closeDiff)
        : done && s
        ? settledCard(s, done.how, done.target, perch)
        : menu && s
          ? menuCard(s, perch, { editor: editorFound, focused: focusKey() === key(s) }, menuActions)
          : activity && s
            ? activityCard(s, perch, (step) => openDiff(s, step), closeActivity)
            : birds && s
              ? birdCard(s, perch, birdActions)
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

  /** The card last answered here: until the next view takes it away, a second click or key on it
   *  would only sound again (core ignores it). */
  let answeredLast: string | null = null;
  /** A click or a shortcut answered the card on screen: it sounds now; what became of it comes
   *  from core with the next view. False when that card was already answered. */
  function answered(request: string, how: "allow" | "deny" | "answered" | "released"): boolean {
    if (request === answeredLast) return false;
    answeredLast = request;
    if (how !== "released") Sound.play(how === "answered" ? "allow" : how);
    return true;
  }

  const cardActions = {
    get keys() {
      return keys;
    },
    decide: (request: string, decision: "allow" | "deny") => {
      if (answered(request, decision)) actions.decide(request, decision);
    },
    decideAlways: (request: string) => {
      if (answered(request, "allow")) actions.decideAlways(request);
    },
    answer: (request: string, answers: Answer[]) => {
      if (answered(request, "answered")) actions.answer(request, answers);
    },
    release: (request: string) => {
      if (answered(request, "released")) actions.release(request);
    },
    keyboard: (on: boolean) => {
      cardKeyboard = on;
      // The chat may hold the keyboard too: it keeps it.
      if (!chat.isOpen()) actions.chat.keyboard(on);
    },
    relayout: () => render(raw),
    jump: actions.jump,
    dismiss: (s: SessionView) => {
      seen.set(key(s), s.status);
      render(raw);
    },
    openChat: () => chat.toggle(true),
    hush: (s: SessionView, hush: Hush) => {
      actions.hush?.(s.agent, s.id, hush);
      Sound.play("tap");
    },
  };

  /** "While you were away", over the news; × reads it. */
  function digestBox(d: DigestView): HTMLElement {
    const close = el("button", { class: "icon-btn", onclick: () => actions.dismissDigest?.() }, icon("close", 11));
    close.title = "Dismiss";
    return el("div", { class: "digest" }, icon("flock", 13), el("span", { class: "digest-text", text: d.text }), close);
  }

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
    // Unfolds it, as a click on the pill would; it folds as usual once the pointer is away.
    if (id === "open") {
      fsm.open(Clock.now());
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
    render(raw);
  };
  const setFoldAfter = (seconds: number) => {
    fsm.foldAfterMs = Math.max(3, seconds) * 1000;
  };
  const setOpenOnHover = (on: boolean) => {
    fsm.openOnHover = on;
  };
  // Escape drops a recording first, then closes the chat; with the chat closed, it folds the island.
  window.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || fsm.mode !== "open") return;
    if (chat.cancelVoice()) return;
    if (chat.isOpen()) chat.toggle(false);
    else if (looksOpen) closeLooks();
    else if (menuOpen) menuActions.close();
    else if (diffOpen) closeDiff();
    else if (activityOpen) closeActivity();
    else if (birdsOpen) birdActions.close();
    else foldNow();
  });
  const jumpFailed = () => {
    jumpNoteUntil = Clock.now() + JUMP_NOTE_MS;
    render(raw);
    window.setTimeout(() => render(raw), JUMP_NOTE_MS + 20);
  };
  const greet = (name: string | null = null) => {
    greetName = name;
    if (calm() || !zecaShown()) return;
    const now = Clock.now();
    greetUntil = now + GREET_MS;
    fsm.open(now);
    render(raw);
    focusScene.dropIn("hello");
    window.setTimeout(() => Sound.play("hello"), 1500);
    window.setTimeout(() => {
      greetUntil = 0;
      // Unless the user took over meanwhile (the pointer, the chat, a permission).
      if (!fsm.pinned && !fsm.pointerInside && !chat.isOpen()) fsm.fold(Clock.now());
      render(raw);
    }, GREET_MS);
  };
  const away = () => {
    if (held) return;
    fsm.away(Clock.now());
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
    render(raw);
  };
  const setUsage = (windows: UsageWindow[]) => {
    usage = windows;
    render(raw);
  };
  const setVisitors = (on: boolean) => sky.setVisitors(on);
  /** Zeca on or off: off, his chat closes, his hello stops, the empty wire stays empty. */
  const setZeca = (on: boolean) => {
    setZecaShown(on);
    chat.setEnabled(on);
    if (!on) {
      greetUntil = 0;
      closeLooks();
    }
    render(raw);
  };
  const setLookSetting = (look: string) => {
    lookSetting = look;
    render(raw);
  };
  const visitNow = () => sky.visit();
  const setEditor = (found: boolean) => {
    editorFound = found;
    render(raw);
  };
  return {
    render, last: () => raw, hold, shortcut, setKeys, setFoldAfter, setOpenOnHover, setVisitors, setZeca, setLookSetting, openLooks, visitNow, jumpFailed, greet, chat, pointer, away, setMedia, setUsage,
    openMenu: (agent, id) => openMenu(`${agent}:${id}`),
    setEditor,
  };
}

export { OPEN_WIDTH };
