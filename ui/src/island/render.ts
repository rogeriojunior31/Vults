// The island's DOM, from a ViewModel. No Tauri here: actions come in, so the lab can render the
// real island with made-up views.
//
// Three modes (fsm.ts). Hidden, only an invisible strip at the top edge is left, to wake it.
// Compact (Mini), it is a fixed-size pill with no text: the session in front on the left (Zeca
// with none), up to eight vults on the right with a badge each. Open (Quick: a click, a
// permission, the chat), it has a header, the habitat (every other session on one wire) over the
// focus card (the session in front, or Zeca), and connector news; a connector's tab swaps the
// overview for its card (what is open on GitHub).
// The two layers cross-fade; the black shape springs when it grows and eases when it shrinks.
import { Clock } from "../clock";
import type { AlertView, Answer, CardView, DigestView, Hush, ProjectPref, ApprovalView, Attention, ConnectorStatus, Diff, MediaAction, NowPlaying, Size, Presence, Flock, Outcome, SessionRef, SessionView, UsageWindow, ViewModel } from "../bridge";
import type { WeekView } from "../view.gen";
import { el } from "../dom";
import { Sound, type Cue } from "../sound";
import { Tracked } from "./anim";
import { idleClip, setMusic } from "./behavior";
import { onDayPart } from "./daypart";
import { ChatPanel, type ChatBackend } from "./chat";
import { IslandMachine, nestHoldsCards, nestIsOpen, presenceNow, type Mode } from "./fsm";
import { icon, type IconName } from "./icons";
import {
  BIRD_W,
  COMPACT_H,
  COMPACT_SCENE,
  COMPACT_W,
  FOCUS_SCENE,
  compactAt,
  HABITAT_PLACES,
  HABITAT_SCENE,
  HABITAT_W,
  Scene,
} from "./scene";
import { Ticker, tickerSteps } from "./ticker";
import { Sky, type SkyBox, type SkyPerch } from "./sky";
import { assignSpecies, setZeca as setZecaShown, setZecaLook, zecaLook, zecaShown, zecaSpecies } from "./flock";
import { lookPicker } from "./looks";
import { birdCard, type BirdActions } from "./birds";
import { connectionsCard, mergeEnded, mergeSig, mergeStarted, staleNote } from "./board";
import { onSeen, seenSig, unseen } from "./seen";
import { settingsQuick, weekQuick, type QuickPage, type QuickSettings } from "./quick";
import { createPalette, paletteItems } from "../palette";
import { sizeSwitch } from "../sizes";
import { CONNECTORS } from "../connectors";
import { activityCard, agentName, badgeOf, diffCard, focusCard, greetingCard, menuCard, needsYou, settledCard, shownKeys, statusText, usageMeters, type MenuActions, type Settled } from "./views";
import { currentLang, t } from "../i18n";

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
  /** Last week's recap read, or opened in Settings → Activity (absent in tests). */
  dismissRecap?(): void;
  openActivity?(): void;
  /** Ends do not disturb (the moon in the header; absent in tests). */
  endDnd?(): void;
  /** Whether VS Code is there now: asked each time the menu opens, so its words stay true. */
  editorFound?(): Promise<boolean>;
  /** A step's whole diff; null once the step is gone. */
  stepDiff(agent: SessionView["agent"], id: string, step: number): Promise<Diff | null>;
  /** Settings in the Nest, at that page (General when none). */
  openSettings(page?: string): void;
  /** The size switch's Room and Full (ADR 0021), at the Nest's tab for what the island shows. */
  setSize?(size: Size, tab: string): void;
  /** Quick's short views: this week, and the settings its gear shows (ADR 0021). */
  /** This week, or (`previous`) the one before, when the history has it. */
  week?(previous?: boolean): Promise<{ week: WeekView; history: boolean }>;
  quickSettings?(): Promise<QuickSettings>;
  setPresence?(presence: Presence): void;
  setFoldAfter?(seconds: number): void;
  setHistory?(on: boolean): Promise<void>;
  setFlock?(flock: Flock): Promise<void>;
  /** Zeca on or off from the gear (the same setting as Settings → Flock & Zeca). */
  setZecaSetting?(on: boolean): Promise<void>;
  /** Zeca's look picked on the island: the same setting as Settings → Flock → Look (absent in
   *  tests that do not pick). */
  setLook?(look: string): void;
  /** The island or a connector's card opened: connectors fetch again if their news is old. */
  opened(): void;
  /** A click on a row of a connector's card. */
  openRow(connector: string, item: string): void;
  /** *Squash and merge* confirmed on a row (absent in tests that do not merge). */
  mergeRow?(connector: string, item: string): void;
  /** How the connector's last poll went (absent in tests that do not care: never stale). */
  connectorStatus?(connector: string): Promise<Pick<ConnectorStatus, "lastOk" | "error"> | null>;
  setSounds(on: boolean): void;
  /** Play/pause or skip the song on screen. */
  media(action: MediaAction): void;
}

/** Quick's short views (ADR 0021): this week, the gear's settings, and Connections with none on. */
type QuickKind = "activity" | "settings" | "connections";

/** Quick (ADR 0021): the open island's width, in its own CSS pixels (before its zoom). */
const OPEN_WIDTH = 440;
/** The open island drawn larger, words and buttons alike (the user's ask, 2026-10-12): about 500
 *  pixels wide on screen. Its scenes compensate, so the birds stay crisp at their pixel size. */
const OPEN_ZOOM = 1.14;
/** News rows shown at once; the rest are counted. */
const MAX_ALERTS = 3;
/** An open connector's card asks how its polls go this often: the view carries rows, not errors,
 *  and a poll that fails or recovers with the same rows changes nothing in it. A local read. */
const STATUS_EVERY_MS = 5000;
/** How long "couldn't find the terminal" stays on the card. */
const JUMP_NOTE_MS = 2600;
/** A permission goes back to its terminal this long after it arrived: the core's `PENDING_TTL`,
 *  the server's decision deadline less a second's grace. Counted from when the island saw it, at
 *  most a moment after the hook's line was read. */
const EXPIRES_MS = 107_000;
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
  /** Presence changed elsewhere (the tray, Settings): the gear's view, if open, shows it. */
  setPresence(presence: Presence): void;
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
  /** A setting the gear shows changed elsewhere. */
  settingsChanged(change: Partial<QuickSettings>): void;
  /** How a confirmed merge went: GitHub's refusal, or none once merged. */
  merged(connector: string, item: string, error: string | null): void;
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
    compactScene.refresh(); focusScene.refresh(); habitatScene.refresh();
  });
  const hidden = (key: string) => sky.owns(key);
  const compactScene = new Scene(COMPACT_SCENE, hidden);
  const focusScene = new Scene(FOCUS_SCENE, hidden, OPEN_ZOOM);
  const habitatScene = new Scene(HABITAT_SCENE, hidden, OPEN_ZOOM);
  let habitatPlaces = HABITAT_PLACES;
  let habitatColumns = HABITAT_PLACES;
  let compactPerched = -1;
  const ticker = new Ticker();
  /** Zeca's place in the focus card: his canvas and the glow behind him. It moves with the card. */
  const perch = el("div", { class: "perch" }, el("span", { class: "glow" }), focusScene.canvas);

  const badges = el("div", { class: "badges" });
  // Mini has no text (ADR 0021): what it would say is its label, for a screen reader.
  // The prototype's fuel: each plan's most-used window as a hairline under the pill, in its color.
  const fuel = el("div", { class: "pill-fuel" });
  fuel.setAttribute("aria-hidden", "true");
  // A connector's news: a cyan dot at the pill's corner, as the prototype has it.
  const newsDot = el("span", { class: "pill-dot" });
  newsDot.setAttribute("aria-hidden", "true");
  const compact = el("div", { class: "layer compact" }, compactScene.canvas, badges, fuel, newsDot);
  compact.setAttribute("role", "button");
  compact.tabIndex = 0;
  compact.addEventListener("keydown", (e) => {
    if ((e.key === "Enter" || e.key === " ") && fsm.mode === "compact") {
      e.preventDefault();
      fsm.click();
    }
  });
  // The open layer is built once; its parts are refilled on each render.
  const headerSlot = el("div", { class: "header-slot" });
  const focusHost = el("div", { class: "focus-host" });
  // Quick as the prototype draws it: at rest the flock alone, each name under its bird, and a chip
  // for each session that needs you; a bird (or a card that comes) opens its session, with the way
  // back to the flock and a dot for each session.
  const needStrip = el("div", { class: "need-strip" });
  const sessionBar = el("div", { class: "session-bar" });
  // The habitat over the card: the rest of the flock on its wire, each with its badge.
  const habitatBadges = el("div", { class: "badges" });
  const habitat = el("div", { class: "habitat" }, habitatScene.canvas, habitatBadges);
  const overview = el("div", { class: "overview" }, habitat, needStrip, sessionBar, focusHost);
  const main = el("div", { class: "main" });
  const alertsSlot = el("div", { class: "alerts-slot" });
  // Over the other tabs, the same chips under the tabs: who needs you stays in sight.
  const topNeeds = el("div", { class: "need-strip top" });
  const inner = el("div", { class: "layer inner" }, headerSlot, topNeeds, main, alertsSlot);
  inner.style.setProperty("zoom", String(OPEN_ZOOM));
  const countdown = el("div", { class: "countdown" });
  root.replaceChildren(compact, inner, countdown);
  root.after(sky.canvas);
  // Outside the island, so it can be hovered while the island is retracted.
  const wake = el("div", { class: "wake" });
  root.after(wake);
  // What is news for the ear is news for a screen reader too: a card interrupts, the rest waits.
  // Regions of their own: the cards are drawn anew on every view and would be read out each time.
  const urgent = el("div", { class: "sr-only" });
  urgent.setAttribute("aria-live", "assertive");
  const polite = el("div", { class: "sr-only" });
  polite.setAttribute("role", "status");
  root.after(urgent, polite);
  const announce = (s: SessionView, card: boolean) => {
    const region = card ? urgent : polite;
    region.textContent = `${s.project || agentName(s)} · ${statusText(s)}`;
  };

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
  /** Who holds the keyboard: the chat, a question card's "Other" field, the island's keys. The
   *  island asks for it while any of them does, and gives it back once none does. */
  const holding = new Set<"chat" | "card" | "keys" | "palette">();
  const holdKeys = (who: "chat" | "card" | "keys" | "palette", on: boolean) => {
    const before = `${holding.size > 0}|${holding.has("keys")}`;
    if (on) holding.add(who);
    else holding.delete(who);
    // Opened from the keyboard there is no click to give the island the focus: it asks for it
    // whole (exclusive) while its keys are on, and only then.
    if (before !== `${holding.size > 0}|${holding.has("keys")}`) actions.chat.keyboard(holding.size > 0, holding.has("keys"));
  };
  const chat = new ChatPanel({ ...actions.chat, keyboard: (on) => holdKeys("chat", on) }, () => render(raw));
  /** Opened from the keyboard (the open shortcut): the island takes the keys until it folds. */
  let keysMode = false;
  const keysOff = () => {
    if (!keysMode) return;
    keysMode = false;
    holdKeys("keys", false);
  };
  /** The sessions in the flock's order, as last drawn: what the arrow keys walk. */
  let order: SessionView[] = [];

  /** The lab holds the island open: nothing folds or unpins it. */
  let held = false;
  /** The session just clicked, until a view carrying core's focus comes back. */
  let picked: string | null = null;
  /** The session the user opened from the flock (Quick's session view); null: the flock at rest. */
  let drilled: string | null = null;
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
  /** The connector last opened: the Connections tab opens on it again. */
  let lastBoard: string | null = null;
  const boardHost = el("div", { class: "board-host" });
  /** Quick's short view the user opened from its tab (ADR 0021), in place of the overview. */
  let quickOpen: QuickKind | null = null;
  let quickShown: QuickKind | null = null;
  /** What the short views show, read when their tab opens; null while it is read. */
  let quickSettings: QuickSettings | null = null;
  let quickWeek: { week: WeekView; history: boolean } | null = null;
  /** Quick's Activity shows the week before (the prototype's ‹ Previous week). */
  let quickPrevious = false;
  /** The gear's page drawn in place. */
  let quickPage: QuickPage = "general";
  let quickSig = "";
  /** The last read failed; and which ask is the latest. */
  let quickFailed = false;
  let quickAsked = 0;
  const quickHost = el("div", { class: "quick-host" });
  /** The palette (Ctrl+K) holds the body until a choice or Esc. */
  let paletteOpen = false;
  const closePalette = () => {
    if (!paletteOpen) return;
    paletteOpen = false;
    holdKeys("palette", false);
  };
  const palette = createPalette(
    () => {
      closePalette();
      render(raw);
    },
    // A filtered list is shorter or longer: the island measures itself again around it.
    () => render(raw),
  );
  const paletteHost = el("div", { class: "palette-island" }, palette.element);
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
    quickOpen = null;
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
    quickOpen = null;
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
    // A click on a vult opens the island on its session; a waiting card keeps the front.
    const k = cardWaits ? null : vultAt(e.clientX);
    const s = k ? order.find((o) => key(o) === k) : undefined;
    if (s) pick(s, false);
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
    // Folded, the island opens next on the flock, as the prototype's Quick does.
    if (from === "open") drilled = null;
    if (to === "open" && from !== "open") actions.opened();
    if (to === "open" && from !== "open" && chatWhenOpened && !chat.isOpen())
      chat.toggle(true);
    if (from === "open" && to !== "open") {
      closePalette();
      keysOff();
      diffOpen = null;
      boardOpen = null;
      quickOpen = null;
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
  const pick = (s: SessionView, tap = true, drill = true) => {
    picked = key(s);
    if (drill) drilled = key(s);
    diffOpen = null;
    activityOpen = null;
    birdsOpen = null;
    actions.focus?.(s.agent, s.id);
    jumpNoteUntil = 0;
    if (tap) Sound.play("tap");
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
          if (preset !== "paused" && (now?.card || (!now?.muted && preset !== "quiet"))) {
            Sound.play(cue, !!now?.card);
            announce(now ?? s, !!now?.card);
          }
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
    const boards = (last.boards ?? []).map((b) => [b.connector, unseen(b).length]);
    return JSON.stringify([view, boardShown, quickShown, boards, order.filter(needsYou).length, cardWaits, media, live, Sound.isEnabled(), fsm.pinned && !held, zecaShown(), !!raw.dnd]);
  }

  const connectorName = (id: string) => CONNECTORS.find((c) => c.id === id)?.name ?? id;

  /** Opens one of Quick's short views, reading what it shows anew. */
  function openQuick(kind: QuickKind): void {
    if (chat.isOpen()) chat.toggle(false);
    // It takes the overview's place: what was open over it closes, as for a connector's card.
    boardOpen = null;
    diffOpen = null;
    menuOpen = activityOpen = birdsOpen = null;
    if (looksOpen) closeLooks();
    quickOpen = kind;
    quickFailed = false;
    // Opened from its tab, a view starts at its own start: General, this week.
    if (kind === "settings") quickPage = "general";
    if (kind === "activity") quickPrevious = false;
    Sound.play("tap");
    actions.opened();
    // Only the latest ask is drawn: a slower answer to an earlier one is dropped.
    const asked = ++quickAsked;
    const failed = () => {
      if (asked !== quickAsked) return;
      quickFailed = true;
      render(raw);
    };
    if (kind === "settings") {
      quickSettings = null;
      void actions.quickSettings?.().then((s) => {
        if (asked !== quickAsked) return;
        quickSettings = s;
        render(raw);
      }, failed);
    } else if (kind === "activity") {
      quickWeek = null;
      void actions.week?.(quickPrevious).then((w) => {
        if (asked !== quickAsked) return;
        quickWeek = w;
        render(raw);
      }, failed);
    }
    render(raw);
  }

  function openBoard(connector: string): void {
    if (chat.isOpen()) chat.toggle(false);
    quickOpen = null;
    boardOpen = connector;
    lastBoard = connector;
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
    // The tab lit for it: the chat's drop zone is the chat's, every connector is Connections.
    const view = chat.isOpen() ? "chat" : boardShown ? "connections" : (quickShown ?? "flock");
    // The prototype's Quick: the tabs as icons spread across the island, a count where it matters.
    const tab = (name: string, glyph: IconName, label: string, go: () => void, count = 0) => {
      const b = el("button", { class: `tab${name === view ? " on" : ""}`, onclick: go }, icon(glyph, 15, 1.6), ...(count > 0 ? [el("span", { class: "tab-count", text: String(count) })] : []));
      b.title = label;
      b.setAttribute("aria-label", count > 0 ? `${label} ${count}` : label);
      return b;
    };
    const quickTab = (kind: QuickKind, glyph: IconName, label: string) => {
      const b = tab(kind, glyph, label, () => openQuick(kind));
      if (cardWaits) b.disabled = true;
      return b;
    };
    // One tab for every connector, as the prototype has it: their rows counted, the last one opened
    // first; with none on, a view that says so and leads to the Nest's Connections.
    const connectionsTab = () => {
      const boards = last.boards ?? [];
      // Rows marked seen leave the count, as they leave Quick.
      const count = boards.reduce((n, b) => n + unseen(b).length, 0);
      const go = () => {
        const next = boards.find((b) => b.connector === lastBoard) ?? boards[0];
        if (next) openBoard(next.connector);
        else openQuick("connections");
      };
      if (!cardWaits) return tab("connections", "plug", t("Connections"), go, count);
      // Clicked now, it would open only once the permission is answered: say so instead.
      const waiting = tab("connections", "plug", t("Connections opens once the permission is answered"), () => {}, count);
      waiting.disabled = true;
      return waiting;
    };
    // As the badges count it: settled, and not a state you already said OK to.
    const needs = order.filter(needsYou).length;
    // The gear opens Quick's settings (General's island rows, any page in the Nest).
    const gear = quickTab("settings", "gear", t("Settings"));
    gear.classList.add("tab-gear");
    const tabs = el(
      "div",
      { class: "tabs" },
      tab(
        "flock",
        "flock",
        t("Flock"),
        () => {
          boardOpen = null;
          quickOpen = null;
          // The Flock tab is the flock: a session open goes back to it, as ‹ Flock does.
          drilled = null;
          menuOpen = activityOpen = birdsOpen = null;
          diffOpen = null;
          if (chat.isOpen()) chat.toggle(false);
          else render(raw);
        },
        needs,
      ),
      // The chat and its drop zone are Zeca's (ADR 0010).
      ...(zecaShown()
        ? [
            tab("chat", "chat", t("Chat"), () => {
              boardOpen = null;
              quickOpen = null;
              chat.toggle(true);
            }),
          ]
        : []),
      // Quick's short views (ADR 0021); a waiting card keeps the island, as for a connector's card.
      quickTab("activity", "list", t("Activity")),
      connectionsTab(),
      gear,
    );
    const soundOn = Sound.isEnabled();
    const sound = el(
      "button",
      { class: "icon-btn", onclick: () => actions.setSounds(!soundOn) },
      icon(soundOn ? "sound" : "mute", 14, 1.6),
    );
    sound.title = t("Mute");
    sound.setAttribute("aria-label", t("Mute"));
    sound.setAttribute("aria-pressed", String(!soundOn));
    // Shown only while do not disturb lasts: a click ends it.
    const moon = raw.dnd ? el("button", { class: "icon-btn dnd", onclick: () => actions.endDnd?.() }, icon("moon", 14)) : null;
    if (moon) moon.title = t("Do not disturb is on: no sounds or notifications. Click to end it.");
    // The palette, as Ctrl+K opens it; a card waiting is read first.
    const search = el("button", { class: "icon-btn", onclick: () => !cardWaits && !boardOpen && openPalette() }, icon("search", 14, 1.6));
    search.title = t("Command palette (Ctrl+K)");
    search.setAttribute("aria-label", t("Command palette (Ctrl+K)"));
    // Ctrl+K waits for a card or a connector's card to be read: so does the button.
    if (cardWaits || boardOpen) search.disabled = true;
    // Mini folds, as the fold button did; Room and Full take what the island shows to the Nest.
    const nestTab = view === "settings" ? "flock" : view;
    const sizes = sizeSwitch(
      "quick",
      // From the gear's view, Room is Settings itself in the Nest.
      (s) => (s === "mini" ? foldNow() : view === "settings" && s === "room" ? actions.openSettings(quickPage === "general" ? undefined : quickPage) : actions.setSize?.(s, nestTab)),
      fsm.pinned && !held ? ["mini"] : [],
      ["mini", "quick", "room"],
    );
    const full = el("button", { class: "icon-btn island-fullscreen", onclick: () => actions.setSize?.("full", nestTab) }, icon("fullscreen", 14, 1.6));
    full.title = t("Full: the Nest on the whole screen; Esc goes back");
    full.setAttribute("aria-label", t("Full"));
    const mark = el("img", { class: "app-mark" });
    mark.src = "/icon.png";
    mark.alt = "";
    // At Quick's width the song and the plans' usage get a line of their own under the tabs.
    const meters = usageMeters(usage, Date.now() / 1000);
    const sub = media || meters ? el("div", { class: "header-sub" }, media ? nowPlaying(media) : el("span"), meters) : null;
    return el(
      "div",
      { class: "header-wrap" },
      el(
        "div",
        { class: "header" },
        el("div", { class: "header-brand" }, mark, el("span", { class: "app-logo", text: "vults" })),
        el("div", { class: "header-actions" }, moon, sound, search, sizes, full),
      ),
      tabs,
      sub,
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
        control("previous", "previous", t("Previous")),
        control("playpause", m.playing ? "pause" : "play", m.playing ? t("Pause") : t("Play")),
        control("next", "next", t("Next")),
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
    const detail = song ? (song.artist ?? "") : front ? statusText(front) : paused ? t("Agents ask in their terminals") : zecaShown() ? t("Nothing running") : t("Start an agent and it lands here");
    const vults = compactScene.slots().filter((s) => s.key !== "zeca");
    const name = song ? `♪ ${song.title}` : front ? front.project || agentName(front) : paused ? t("Paused") : zecaShown() ? "Zeca" : t("Nothing running");
    const failed = shown.filter((s) => s.status === "failed" || s.attention === "failed").length;
    compact.setAttribute(
      "aria-label",
      [name, detail, failed ? (failed === 1 ? t("1 failed") : t("{n} failed", { n: failed })) : "", alerts ? t("{n} new", { n: alerts }) : ""].filter(Boolean).join(" · "),
    );
    // Someone needs you: the pill's border pulses amber.
    // Not while paused: every card is answered in its terminal then.
    const needs = !paused && shown.some((s) => s.attention === "needs-you");
    compact.classList.toggle("needs", needs);
    // A session failed (and none needs you): the edge stays red, still, until it is seen.
    compact.classList.toggle("failed", !needs && failed > 0);
    newsDot.hidden = alerts === 0;
    const plans = new Map<string, number>();
    // As the meters: a window that has reset says nothing until the next read.
    const nowSecs = Date.now() / 1000;
    for (const w of usage) if (w.resets_at === null || w.resets_at > nowSecs) plans.set(w.agent, Math.max(plans.get(w.agent) ?? 0, w.used_percent));
    fuel.replaceChildren(
      ...[...plans].map(([agent, used]) => {
        const bar = el("b");
        bar.style.width = `${Math.min(100, Math.round(used))}%`;
        // Amber from 70%, red from 90%, as the island's meters say it.
        bar.style.background = used >= 90 ? "var(--error)" : used >= 70 ? "var(--warn)" : `var(--agent-${agent}, var(--agent-other))`;
        return el("i", {}, bar);
      }),
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
  /** The Monday card already shown: it opens the island once. */
  let recapSeen: number | null = null;
  /** The reminders the card on screen has sounded, by request (the attention ladder). */
  let reminded: { request: string; n: number } | null = null;

  function render(view: ViewModel): void {
    raw = view;
    Sound.setHushed(!!view.dnd);
    // The chat, a connector's card or a permission took the island: the looks close, so an
    // unpicked preview is never worn elsewhere nor comes back by itself.
    if (looksOpen && (chat.isOpen() || boardOpen || quickOpen || view.approval)) {
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
    const shown = (order = v.sessions
      .map((s): SessionView => {
        const k = key(s);
        if (SETTLE_MS[s.attention] && !announced.has(k)) return { ...s, status: "working", attention: "quiet", card: false };
        if (seen.get(k) === s.status) return { ...s, status: "idle", note: null, attention: "quiet", card: false };
        return s;
      }))
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
    // A card took the island: the palette closes, rather than come back later with old choices.
    if (pending && paletteOpen) closePalette();
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
    if (pending || chat.isOpen() || boardOpen || quickOpen) menuOpen = activityOpen = birdsOpen = null;
    if (menuOpen && !shownByKey.has(menuOpen)) menuOpen = null;
    if (activityOpen && !shownByKey.has(activityOpen)) activityOpen = null;
    if (birdsOpen && !shownByKey.has(birdsOpen)) birdsOpen = null;
    // What the user opened on a session holds it in front: its menu, its steps, its bird, a diff.
    if (drilled && !shownByKey.has(drilled)) drilled = null;
    const asked = menuOpen ?? activityOpen ?? birdsOpen ?? diffOpen?.session ?? drilled ?? null;
    const front = settledSession ?? pending ?? (asked ? shownByKey.get(asked) : null) ?? chosen ?? asCore ?? ref(v.focus) ?? shown[0] ?? null;
    // The diff belongs to its session's card: anything else in front, or a card to answer, closes it.
    if (diffOpen && (!front || key(front) !== diffOpen.session || settledSession || front === pending)) diffOpen = null;
    inFront = front;

    // A settled permission opens the island and keeps it open until it is answered, unless the
    // user is in the Nest, which answers it there (ADR 0030). Going there folds the island.
    const here = pending && v.approval && !nestHoldsCards();
    if (here && !fsm.pinned) fsm.openPinned();
    else if (!here && fsm.pinned && !held) {
      fsm.unpin(now);
      if (pending) fsm.fold(now);
    }
    fsm.setOccupied(v.sessions.length > 0 || v.alerts.length > 0 || !!v.digest || !!v.recap, now);
    // Back from away: the digest opens the island once; it folds as usual after.
    if (v.digest && v.digest.seq !== digestSeen) {
      // Only the top island opens for news at rest: by the panel or in Quiet it waits there.
      // The chat would hide it: back from away, the news comes first (the chat keeps its place).
      if ((digestSeen !== null || primedDigest) && presenceNow() === "island" && !nestIsOpen()) {
        if (chat.isOpen()) chat.toggle(false);
        fsm.open(now);
      }
      digestSeen = v.digest.seq;
    }
    primedDigest = true;
    // Monday morning: last week's recap opens the top island once, as the digest does.
    if (v.recap && v.recap.seq !== recapSeen) {
      if (presenceNow() === "island" && !chat.isOpen() && !nestIsOpen()) fsm.open(now);
      recapSeen = v.recap.seq;
    }
    if (chat.isOpen() && fsm.mode !== "open") fsm.open(now);
    fsm.setEngaged(chat.isOpen(), now);
    const mode: Mode = fsm.mode;

    chat.setFolders(
      shown.flatMap((s) => (s.cwd ? [{ cwd: s.cwd, project: s.project }] : [])),
      front?.cwd ?? null,
    );
    // At rest (nothing to answer, nothing opened, no hello) Quick shows the flock alone, every
    // session on the wire, the one in front marked; otherwise the session in front, opened.
    const atRest = shown.length > 0 && !pending && !settledSession && !asked && Clock.now() >= greetUntil;
    const others = atRest ? shown : shown.filter((s) => s !== front);
    const chatShown = chat.isOpen() && !pending;
    // Connections with none answered yet: the first board to come takes the view's place.
    if (quickOpen === "connections" && (v.boards ?? []).length) {
      quickOpen = null;
      boardOpen = lastBoard = v.boards![0].connector;
      boardStatus = null;
      askStatus(boardOpen);
    }
    const board = !chat.isOpen() && !pending && boardOpen ? (v.boards ?? []).find((b) => b.connector === boardOpen) : undefined;
    const picking = looksOpen && !chat.isOpen() && !pending && zecaShown();
    // Switched off meanwhile: back to the flock.
    if (boardOpen && !(v.boards ?? []).some((b) => b.connector === boardOpen)) boardOpen = null;
    boardShown = board ? board.connector : null;
    // What became of a card shows first, as over the overview.
    const quick = !chat.isOpen() && !pending && !board && !settledSession ? quickOpen : null;
    quickShown = quick;
    // Zeca stays on the wire with nobody there, so the island is never a blank shape. In the chat
    // he is the chat: thinking, swallowing a file, waiting for an answer.
    // With Zeca off (ADR 0010) an empty wire stays empty. Paused, he sleeps: the pill has no words,
    // and asleep is how it tells a pause from nothing running.
    const idle = zecaShown() ? { clip: presenceNow() === "paused" ? "sleep" : idleClip(), agent: "claude" as const, alone: true } : null;
    assignSpecies(shown);
    // The pill's vults spread over the wire after Zeca, as many as there are (up to eight).
    const perched = Math.min(COMPACT_SCENE.vults.max, shown.filter((s) => s !== front).length);
    if (perched !== compactPerched) {
      compactPerched = perched;
      compactScene.resize(COMPACT_W, COMPACT_SCENE.vults.max, compactAt(perched));
    }
    compactScene.update(shown, front, front ? null : idle);
    if (chatShown) focusScene.update([], null, { clip: chat.clip(now), agent: chat.agent() });
    else focusScene.update([], front, front ? null : idle);
    // Columns as the prototype's: four wide ones for up to four sessions, up to six; past six the
    // last column is +N's, five birds keeping their places, none under the chip.
    const columns = Math.min(HABITAT_PLACES, Math.max(4, others.length));
    const places = others.length > HABITAT_PLACES ? HABITAT_PLACES - 1 : columns;
    if (places !== habitatPlaces || columns !== habitatColumns) {
      habitatPlaces = places;
      habitatColumns = columns;
      const col = HABITAT_W / columns;
      habitatScene.resize(HABITAT_W, places, (slot) => ({ x: Math.round(slot * col + (col - BIRD_W * 2) / 2), wire: HABITAT_SCENE.wire! }));
    }
    habitatScene.update(others, null);
    // Locked, nobody is looking: the scene and the sky rest (no frames, no timers).
    const awake = !v.locked;
    compactScene.setActive(awake && mode === "compact");
    // Zeca is on the focus card or in the chat: drawn either way while the island is open.
    focusScene.setActive(awake && mode === "open" && !board && !quick && (!atRest || chatShown || picking));
    // The flock shows at rest; a session (or a card) takes its place, its sessions as dots.
    const listShown = others.length > 0;
    habitatScene.setActive(awake && mode === "open" && !chatShown && !board && !quick && !picking && listShown && atRest);
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
      paintNeeds(chatShown || board || quick ? shown : [], topNeeds);
      if (chatShown) {
        perch.className = "perch chatting";
        if (perch.parentElement !== chat.perchSlot) chat.perchSlot.append(perch);
        show(chat.element);
      } else if (board) {
        // Every board: its chips count the other connectors' rows too.
        const sig = JSON.stringify([last.boards ?? [], board.connector, seenSig(), mergeSig(), zecaShown()]);
        const stale = boardStatus ? staleNote(boardStatus, Date.now() / 1000) : null;
        if (sig !== boardSig || stale !== boardStale || !boardHost.firstChild) {
          boardSig = sig;
          boardStale = stale;
          boardHost.replaceChildren(
            connectionsCard(
              last.boards ?? [],
              board,
              connectorName,
              {
                open: (c, item) => actions.openRow(c, item),
                show: openBoard,
                add: () => actions.setSize?.("room", "connections"),
                room: () => actions.setSize?.("room", "connections"),
                merge: actions.mergeRow
                  ? (c, item) => {
                      mergeStarted(c, item);
                      actions.mergeRow?.(c, item);
                      render(raw);
                    }
                  : undefined,
                // The question waits in the composer: nothing is asked until the user sends it.
                ask: zecaShown()
                  ? (text) => {
                      boardOpen = null;
                      quickOpen = null;
                      chat.toggle(true);
                      chat.draft(text);
                    }
                  : undefined,
                // The marked row is gone: focus goes to the next one, not to the page.
                seen: () => {
                  render(raw);
                  boardHost.querySelector<HTMLElement>(".qconn-done, .qconn-empty button")?.focus();
                },
              },
              stale,
            ),
          );
        }
        show(boardHost);
      } else if (paletteOpen) {
        show(paletteHost);
      } else if (quick) {
        // Rebuilt only when what it shows changes.
        const sig = JSON.stringify([quick, quick === "settings" ? [quickSettings, quickPage] : quick === "activity" ? [quickWeek, quickPrevious] : null, quickFailed, zecaShown(), currentLang()]);
        if (sig !== quickSig || !quickHost.firstChild) {
          quickSig = sig;
          const settingsActions = {
            setPresence: (p: Presence) => {
              if (quickSettings) quickSettings = { ...quickSettings, presence: p };
              actions.setPresence?.(p);
              render(raw);
            },
            setFoldAfter: (n: number) => {
              if (quickSettings) quickSettings = { ...quickSettings, foldAfter: n };
              actions.setFoldAfter?.(n);
              render(raw);
            },
            setHistory: (on: boolean) => {
              if (quickSettings) quickSettings = { ...quickSettings, history: on };
              actions.setHistory?.(on).catch(rereadSettings);
              render(raw);
            },
            setZeca: (on: boolean) => {
              if (quickSettings) quickSettings = { ...quickSettings, zeca: on };
              actions.setZecaSetting?.(on).catch(rereadSettings);
              render(raw);
            },
            setFlock: (f: Flock) => {
              if (quickSettings) quickSettings = { ...quickSettings, flock: f };
              actions.setFlock?.(f).catch(rereadSettings);
              render(raw);
            },
            show: (page: QuickPage) => {
              quickPage = page;
              render(raw);
            },
            open: (page?: string) => actions.openSettings(page),
          };
          quickHost.replaceChildren(
            quick === "settings"
              ? settingsQuick(quickSettings, settingsActions, () => reshape(fsm.mode), quickFailed, quickPage)
              : quick === "connections"
                ? connectionsCard([], null, connectorName, { open: () => {}, show: () => {}, add: () => actions.setSize?.("room", "connections") })
                : weekQuick(quickWeek?.week ?? null, quickWeek?.history ?? true, () => actions.setSize?.("room", "activity"), quickFailed, {
                    previous: quickPrevious,
                    go: (previous) => {
                      if (previous === quickPrevious) return;
                      quickPrevious = previous;
                      // The card keeps the week shown until the other one comes: no "Reading…" in
                      // between, so the island does not shrink and grow under the pointer.
                      const asked = ++quickAsked;
                      void actions.week?.(previous).then(
                        (w) => {
                          if (asked !== quickAsked) return;
                          quickWeek = w;
                          render(raw);
                        },
                        () => {},
                      );
                      render(raw);
                    },
                  }),
          );
        }
        show(quickHost);
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
        overview.classList.toggle("at-rest", atRest);
        paintFocus(front, approval, now, settledSession ? recent : null);
        tickExpiry();
        habitat.classList.toggle("none", !listShown || !atRest);
        paintHabitat(others, atRest ? front : null);
        paintNeeds(atRest ? shown : []);
        paintSessionBar(atRest ? [] : shown, front, pending !== null || settledSession !== null || Clock.now() < greetUntil);
      }
      // The looks fill the island: news waits under the overview, so it never grows past its surface.
      const news = !chat.isOpen() && !picking;
      alertsSlot.replaceChildren(
        ...(keysMode && !chat.isOpen() ? [keysHint()] : []),
        ...(news && v.recap ? [recapBox(v.recap)] : []),
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
    const scenes = fsm.mode === "compact" ? [compactScene] : [focusScene, habitatScene];
    for (const scene of scenes) {
      if (!scene.canvas.isConnected) continue;
      const rect = scene.canvas.getBoundingClientRect();
      // A hidden scene (the habitat in a session, the focus card at rest) measures zero: its birds
      // land at the island's edge, as those past the habitat's last place do.
      if (!rect.width) continue;
      for (const at of scene.anchors()) anchors.set(at.key, {
        x: rect.left - skyRect.left + at.x, y: rect.top - skyRect.top + at.y, scale: at.scale,
      });
    }
    // Sessions past the habitat's last place land at the island's edge.
    // A connector's card takes the whole overview, and that edge is its corner: every bird lands
    // in the header instead, side by side in the gap after the name (the rest share its last spot).
    const rect = root.getBoundingClientRect();
    const brand = boardShown && fsm.mode === "open" ? headerSlot.querySelector(".header-brand") : null;
    const tools = brand?.parentElement?.querySelector(".header-actions");
    const gap = brand && tools && { from: brand.getBoundingClientRect().right, bottom: brand.getBoundingClientRect().bottom,
      to: tools.getBoundingClientRect().left };
    let spare = 0;
    for (const session of last.sessions) if (!anchors.has(key(session))) anchors.set(key(session), gap
      ? { x: Math.max(gap.from + 30, Math.min(gap.from + 30 + 32 * spare++, gap.to - 30)) - skyRect.left, y: gap.bottom - skyRect.top - 16, scale: 1 }
      : { x: rect.right - skyRect.left - 20, y: rect.bottom - skyRect.top - 16, scale: 1 });
    sky.place(anchors, {
      left: rect.left - skyRect.left, top: rect.top - skyRect.top, width: rect.width, height: rect.height,
      radius: parseFloat(getComputedStyle(root).borderBottomLeftRadius) || 0,
      // Only the open layer's: the folded one keeps its cards in the tree, faded out.
      cards: fsm.mode === "open" ? cardRects(skyRect) : [],
    });
  }
  /** The cards in the sky's coordinates (placeSky runs every frame of a change). A card fading out
   *  sits right over its successor: it is left out, the two would cancel out. */
  function cardRects(sky: DOMRect): NonNullable<SkyBox["cards"]> {
    const out: NonNullable<SkyBox["cards"]> = [];
    const radius = new Map<string, number>();
    for (const card of inner.querySelectorAll<HTMLElement>(".card:not(.leaving), .board")) {
      const c = card.getBoundingClientRect();
      const kind = card.classList.contains("board") ? "board" : "card";
      if (!radius.has(kind)) radius.set(kind, parseFloat(getComputedStyle(card).borderTopLeftRadius) || 0);
      out.push({ left: c.left - sky.left, top: c.top - sky.top, width: c.width, height: c.height, radius: radius.get(kind)! });
    }
    return out;
  }
  window.addEventListener("resize", placeSky);

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

  /** Over each vult in the habitat, a button: its name and state for a screen reader and on hover,
   *  a click puts it in front, a right-click opens its quick actions; and its badge, when its state
   *  asks. Past the last place, +N opens the Nest's Flock, amber when one of them needs you. */
  function paintHabitat(others: SessionView[], chosen: SessionView | null = null): void {
    const byKey = new Map(others.map((s) => [key(s), s]));
    const placed = new Set(habitatScene.slots().map((s) => s.key));
    const rest = others.filter((s) => !placed.has(key(s)));
    const more = rest.length
      ? el("button", {
          class: `habitat-more${rest.some(needsYou) ? " needs" : ""}`,
          text: `+${rest.length}`,
          onclick: () => actions.setSize?.("room", "flock"),
        })
      : null;
    if (more) more.title = t("{n} more: see them all in the Nest", { n: rest.length });
    // Each badge by its bird's head, wherever its species puts it. The scene speaks screen pixels
    // (OPEN_ZOOM); the labels and badges sit in the island's own CSS pixels.
    const heads = new Map(habitatScene.anchors().map((a) => [a.key, a.y]));
    habitatBadges.replaceChildren(
      // In wire order, so a screen reader and the keyboard meet them left to right.
      ...[...habitatScene.slots()].sort((a, b) => a.x - b.x).flatMap((slot) => {
        const s = byKey.get(slot.key);
        if (!s) return [];
        const name = s.project || agentName(s);
        const said = [name, agentName(s), statusText(s)].join(" · ");
        // The prototype's roost: the bird, and its name under it; "!" when it needs you.
        const mark = el("span", { class: "marker", text: needsYou(s) ? "!" : "" });
        mark.setAttribute("aria-hidden", "true");
        const b = el("button", { class: `habitat-bird ${s.status}${chosen && key(chosen) === key(s) ? " chosen" : ""}`, onclick: () => pick(s) }, el("span", { class: "label" }, el("span", { class: "name", text: name }), mark));
        b.dataset.key = slot.key;
        const column = HABITAT_W / habitatColumns;
        const x = slot.x / OPEN_ZOOM;
        const width = slot.width / OPEN_ZOOM;
        b.style.left = `${Math.round(x - (column - width) / 2)}px`;
        b.style.width = `${Math.round(column)}px`;
        b.setAttribute("aria-label", said);
        b.title = `${said}\n${t("Right-click for more")}`;
        const kind = badgeOf(s);
        const badge = kind ? el("span", { class: `badge ${kind}` }) : null;
        if (badge) {
          badge.style.left = `${x + width - 6}px`;
          // 30 screen pixels over the head, as before the zoom: the bird keeps its size.
          badge.style.top = `${Math.max(2, ((heads.get(slot.key) ?? 34) - 30) / OPEN_ZOOM)}px`;
        }
        return badge ? [b, badge] : [b];
      }),
      ...(more ? [more] : []),
    );
  }

  /** At rest, a chip for each session that needs you, as the prototype's: a click opens it. */
  /** From any tab, to that session on the flock's tab. */
  function toSession(s: SessionView): void {
    boardOpen = null;
    quickOpen = null;
    if (chat.isOpen()) chat.toggle(false);
    pick(s);
  }

  function paintNeeds(sessions: SessionView[], strip = needStrip): void {
    // What a look would answer: a question or a card, a failure, a bird gone quiet for too long.
    const wanting = sessions.filter((s) => needsYou(s) || s.status === "failed" || s.attention === "failed" || s.silent === "loud");
    strip.hidden = !wanting.length;
    strip.replaceChildren(
      ...wanting.map((s) => {
        const failed = s.status === "failed" || s.attention === "failed";
        const b = el("button", { class: `need-tag${failed ? " failed" : ""}`, onclick: () => toSession(s) }, el("i", { text: failed ? "×" : "!" }), el("span", { text: s.project || agentName(s) }));
        b.title = [s.project || agentName(s), statusText(s)].join(" · ");
        return b;
      }),
    );
  }

  /** In a session: the way back to the flock, and a dot per session in the agent's color, the one
   *  open ringed; a click on a dot opens that one. */
  function paintSessionBar(sessions: SessionView[], open: SessionView | null, held: boolean): void {
    sessionBar.hidden = !sessions.length || !open;
    if (sessionBar.hidden) return;
    const back = el("button", {
      class: "session-back",
      onclick: () => {
        drilled = null;
        menuOpen = activityOpen = birdsOpen = null;
        diffOpen = null;
        Sound.play("tap");
        render(raw);
      },
    }, icon("chevron", 12, 2), el("span", { text: t("Flock") }));
    // A waiting card is answered before the flock comes back: ‹ Flock would do nothing there.
    back.hidden = held;
    const dots = el(
      "div",
      { class: "session-dots" },
      ...sessions.map((s) => {
        const d = el("button", { class: `session-dot ${s.agent}${open && key(open) === key(s) ? " on" : ""}`, onclick: () => pick(s) });
        const said = [s.project || agentName(s), statusText(s)].join(" · ");
        d.title = said;
        d.setAttribute("aria-label", said);
        return d;
      }),
    );
    sessionBar.replaceChildren(back, dots);
  }

  /** The vult in the pill at this x, by its key. */
  function vultAt(clientX: number): string | null {
    const r = compactScene.canvas.getBoundingClientRect();
    const x = clientX - r.left;
    return compactScene.slots().find((s) => s.key !== "zeca" && x >= s.x && x <= s.x + s.width)?.key ?? null;
  }

  /** Zeca looks toward the pointer, at you when it is on him, and preens if it rests there. */
  function notice(e: PointerEvent | null): void {
    const under = fsm.mode === "compact" && e ? vultAt(e.clientX) : null;
    compactScene.hover(under);
    if (under) sky.call(under);
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
    const row = (e.target as Element).closest?.(".habitat-bird") as HTMLElement | null;
    let k = row?.dataset.key ?? null;
    if (!k && fsm.mode === "compact") k = vultAt(e.clientX);
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
      box.querySelector(".expiry-text")!.textContent = t("Goes back to the terminal in {time}", { time: `0:${String(secs).padStart(2, "0")}` });
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
      holdKeys("card", on);
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
    close.title = t("Dismiss");
    return el("div", { class: "digest" }, icon("flock", 13), el("span", { class: "digest-text", text: d.text }), close);
  }

  /** What the island's keys do, while it holds them. */
  function keysHint(): HTMLElement {
    const parts = [t("↑ ↓ sessions"), t("⏎ terminal"), t("M actions")];
    if (cardOnScreen) parts.push(cardOnScreen.questions.length ? t("1–9 choose") : t("Y / N answer"));
    if (zecaShown()) parts.push(t("C chat"));
    parts.push(t("Esc fold"));
    return el("div", { class: "keys-hint", text: parts.join(" · ") });
  }

  /** Last week's recap, on Monday morning: Open Activity shows the whole week, × reads it. */
  function recapBox(r: CardView): HTMLElement {
    const open = el("button", { class: "look-chip", text: t("Open Activity"), onclick: () => actions.openActivity?.() });
    const close = el("button", { class: "icon-btn", onclick: () => actions.dismissRecap?.() }, icon("close", 11));
    close.title = t("Dismiss");
    return el("div", { class: "digest recap" }, icon("flock", 13), el("span", { class: "digest-text", text: r.text }), open, close);
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
    // On screen, its zoom included: the shape and the input region are screen pixels. Not its
    // bounding box: that holds the opening's scale(0.97), measured before the transition runs.
    return {
      w: Math.ceil(inner.offsetWidth * OPEN_ZOOM),
      h: Math.ceil(inner.offsetHeight * OPEN_ZOOM),
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
    // Unfolds it, as a click on the pill would, and takes the keys: it was asked for from the
    // keyboard. It folds as usual once the pointer is away, and gives them back then.
    // Mini from the size switch or its key (ADR 0021): the pill, as the fold button does.
    if (id === "fold") {
      foldNow();
      return;
    }
    if (id === "open") {
      fsm.open(Clock.now());
      keysMode = true;
      holdKeys("keys", true);
      render(raw);
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
    keys = shownKeys(bound);
    chat.setKeys(keys);
    render(raw);
  };
  /** A change the app refused: the gear shows what is saved again. */
  const rereadSettings = () => {
    void actions.quickSettings?.().then((s) => {
      quickSettings = s;
      render(raw);
    }, () => {});
  };
  // The gear's view reads its settings when it opens; a change made elsewhere meanwhile shows too.
  const quickSettingsChanged = (change: Partial<QuickSettings>) => {
    if (!quickSettings) return;
    quickSettings = { ...quickSettings, ...change };
    render(raw);
  };
  const setFoldAfter = (seconds: number) => {
    fsm.foldAfterMs = Math.max(3, seconds) * 1000;
    quickSettingsChanged({ foldAfter: seconds });
  };
  const setPresence = (presence: Presence) => quickSettingsChanged({ presence });
  const setOpenOnHover = (on: boolean) => {
    fsm.openOnHover = on;
  };
  // The island's own keys, while it holds them (opened from the keyboard) and no field is typed in.
  // None of them answers a card but Y / N on a permission and a number on a question's choice,
  // the same answers a click gives (ADR 0004).
  window.addEventListener("keydown", (e) => {
    if (!keysMode || fsm.mode !== "open" || chat.isOpen() || e.ctrlKey || e.altKey || e.metaKey) return;
    const target = e.target as HTMLElement | null;
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) return;
    const k = e.key.length === 1 ? e.key.toLowerCase() : e.key;
    const walk = (step: number) => {
      if (!order.length) return;
      const i = inFront ? order.findIndex((s) => key(s) === key(inFront!)) : -1;
      // At rest the arrows move the choice along the flock; in a session they go to the next one.
      pick(order[(i + step + order.length) % order.length], true, drilled !== null);
    };
    const choice = /^[1-9]$/.test(k) ? card?.querySelectorAll<HTMLButtonElement>(".question-card .choice")[Number(k) - 1] : undefined;
    if (k === "ArrowDown" || k === "j") walk(1);
    else if (k === "ArrowUp" || k === "k") walk(-1);
    else if (k === "Enter" && inFront && !cardOnScreen) actions.jump(inFront.agent, inFront.id);
    else if (k === "m" && inFront) openMenu(key(inFront));
    else if ((k === "y" || k === "n") && cardOnScreen && cardOnScreen.questions.length === 0) {
      const { request } = cardOnScreen;
      cardOnScreen = null;
      cardActions.decide(request, k === "y" ? "allow" : "deny");
    } else if (choice) choice.click();
    else if (k === "c" && zecaShown()) chat.toggle(true);
    else return;
    e.preventDefault();
  });
  // Ctrl+K: the palette in the island's body (it never answers a card: a waiting one is already here).
  window.addEventListener("keydown", (e) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.key.toLowerCase() !== "k" || fsm.mode !== "open") return;
    // A card or a connector's card is the one thing to read: the palette waits for it.
    if (cardWaits || boardOpen) return;
    e.preventDefault();
    openPalette();
  });
  /** The palette in the island's body: Ctrl+K, or the header's search. */
  function openPalette(): void {
    // It takes the keyboard before the chat lets it go, so typing still lands in it.
    holdKeys("palette", true);
    if (chat.isOpen()) chat.toggle(false);
    paletteOpen = true;
    render(raw);
    palette.open(
      paletteItems({
        sessions: order,
        focus: (s) => pick(s),
        review: (s) => pick(s),
        command: zecaShown()
          ? (text) => {
              chat.toggle(true);
              chat.draft(text);
            }
          : null,
        tab: (id) => (id === "chat" ? chat.toggle(true) : actions.setSize?.("room", id)),
      }),
    );
    // Its list is in now: the island measures itself again around it.
    render(raw);
  }
  // Another window took the focus: the keys go back with it.
  window.addEventListener("blur", () => {
    if (!keysMode) return;
    keysOff();
    render(raw);
  });
  // Escape drops a recording first, then closes the chat; with the chat closed, it folds the island.
  window.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || fsm.mode !== "open") return;
    if (paletteOpen) {
      closePalette();
      render(raw);
      return;
    }
    if (chat.cancelVoice()) return;
    if (chat.isOpen()) chat.toggle(false);
    else if (looksOpen) closeLooks();
    else if (menuOpen) menuActions.close();
    else if (diffOpen) closeDiff();
    else if (activityOpen) closeActivity();
    else if (birdsOpen) birdActions.close();
    else foldNow();
  });
  /** How a confirmed merge went (`row-merged`). */
  const merged = (connector: string, item: string, error: string | null) => {
    mergeEnded(connector, item, error);
    render(raw);
  };
  const jumpFailed = () => {
    jumpNoteUntil = Clock.now() + JUMP_NOTE_MS;
    render(raw);
    window.setTimeout(() => render(raw), JUMP_NOTE_MS + 20);
  };
  const greet = (name: string | null = null) => {
    greetName = name;
    if (calm() || !zecaShown() || nestIsOpen()) return;
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
  // A row marked seen in the Nest leaves Quick here too.
  onSeen(() => render(raw));
  // The hour turning brings the idle flock down at dusk, to sleep at night, and up by day.
  onDayPart(() => render(raw));
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
    quickSettingsChanged({ zeca: on });
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
    render, last: () => raw, hold, shortcut, setKeys, setFoldAfter, setOpenOnHover, setPresence, setVisitors, setZeca, setLookSetting, openLooks, visitNow, jumpFailed, merged, settingsChanged: quickSettingsChanged, greet, chat, pointer, away, setMedia, setUsage,
    openMenu: (agent, id) => openMenu(`${agent}:${id}`),
    setEditor,
  };
}

export { OPEN_WIDTH };
