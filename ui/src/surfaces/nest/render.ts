// The Nest's page, with no Tauri in it: the backend comes in, so the lab can draw it too. Five tabs
// and the gear (ADR 0021): Flock, Tasks, Chat (with its Memory beside it), Activity (Week, By
// project, History, Cost, Traces, Audit) and Connections. Without Zeca there is no Chat tab.
import type { MediaAction, NowPlaying, Size } from "../../bridge";
import { el } from "../../dom";
import { createPalette, paletteItems } from "../../palette";
import { t } from "../../i18n";
import { ChatPanel, type ChatBackend } from "../../island/chat";
import { Clock } from "../../clock";
import { FOCUS_SCENE, Scene } from "../../island/scene";
import { setZecaLook, setZecaSpecies } from "../../island/flock";
import { icon } from "../../island/icons";
import { agentName, statusText } from "../../island/views";
import { renderFull } from "../../island/markdown";
import type { SessionView, ViewModel } from "../../view.gen";
import { createConnections, type ConnectionsActions } from "./connections";
import { createFlock, needsYou, type FlockActions } from "./flock";
import { createNestFoot, createNestHead, NEST_TABS, tabByKey, type NestTab } from "./head";
import { createBudget, type BudgetBackend } from "./budget";
import { createMemory, type MemoryBackend } from "./memory";
import { createRecent } from "./recent";
import {
  createAudit,
  createHistory,
  createTraces,
  createUsage,
  type RecordsBackend,
} from "./records";
import { createRoosts, type RoostsBackend } from "./roosts";
import { createTasks, type TasksBackend } from "./tasks";
import { createWeek, type WeekBackend } from "./week";

export interface Nest {
  flushState: () => Promise<void>;
  chat: ChatPanel;
  /** The flock's view, as each surface gets it. */
  view: (v: ViewModel) => void;
  /** The header's words again, in the language just set. */
  relabel: () => void;
  /** The memory changed: its panel reads it again, when open. */
  memoryChanged: () => void;
  /** The tasks changed (`/task`, another window): their tab reads them again, when open. */
  tasksChanged: () => void;
  /** Zeca switched on or off: the Chat tab comes or goes. */
  setZeca: (on: boolean) => void;
  /** Opens a tab by its id (`flock`, `tasks`, `chat`, `activity`, `connections`, or an Activity
   *  sub-tab such as `cost`), as `open_nest` asks; an unknown id is ignored. */
  open: (id: string) => void;
  /** The tab shown now. */
  current: () => string;
  /** The window went Full, or back to Room. */
  setFull: (full: boolean) => void;
  /** The sounds went on or off. */
  setSound: (on: boolean) => void;
  /** The song your player is playing, or none. */
  setMedia: (now: NowPlaying | null) => void;
  /** Zeca's species, for the footer's mark. */
  setSpecies: (id: string) => void;
  /** The plans' usage changed. */
  usageChanged: () => void;
  /** The keys the desktop bound for the shortcuts, for the cards' buttons. */
  setKeys: (keys: Record<string, string>) => void;
  /** Allow or Deny's key while the user is in the Nest: answers the card on screen, if one is
   *  (the Chat's in its tab, the session in front's on the Flock). */
  shortcut: (id: string) => void;
}

export interface NestBackend {
  chat: ChatBackend;
  roosts: RoostsBackend;
  memory: MemoryBackend;
  // Whether Zeca is on, the Nest knows itself.
  tasks: Omit<TasksBackend, "zeca">;
  records: RecordsBackend;
  budget: BudgetBackend;
  week: WeekBackend;
  flock: FlockActions;
  connections: ConnectionsActions;
  /** The gear: Settings. */
  settings: () => void;
  /** The footer's switch. */
  setZeca: (on: boolean) => void;
  /** The footer's way back. */
  island: () => void;
  /** The header's sound button. */
  sound: (on: boolean) => void;
  /** The header's song controls. */
  media: (action: MediaAction) => void;
  /** The header's moon: ends do not disturb. */
  endDnd: () => void;
  /** The footer's *Zeca's look*: Settings › Flock & Zeca. */
  look: () => void;
  /** The size switch (ADR 0021), with the tab shown now to keep the place. */
  size: (size: Size, tab: string) => void;
  savePlace?: (tab: string) => Promise<void>;
}

/** How often By project may ask again while the flock moves: each ask reads the repositories. */
const ROOSTS_EVERY_MS = 1500;

type Tab = NestTab;
type Sub = "week" | "projects" | "history" | "cost" | "traces" | "audit";
const TABS = NEST_TABS;
const SUBS: Sub[] = ["week", "projects", "history", "cost", "traces", "audit"];

/** Activity → Cost: today's spend and the daily budget over the records of what the agents cost. */
function costPart(
  budget: { element: HTMLElement; refresh: () => void },
  usage: { element: HTMLElement; refresh: () => void },
): { element: HTMLElement; refresh: () => void } {
  const title = el("h2");
  const about = el("p");
  return {
    element: el("div", { class: "nest-cost" },
      el("header", { class: "activity-heading" }, el("div", {}, title, about)),
      budget.element, usage.element,
    ),
    refresh: () => {
      title.textContent = t("Cost");
      about.textContent = t("Your plan windows, recorded cost and daily budget on this computer.");
      budget.refresh();
      usage.refresh();
    },
  };
}

export function createNest(
  root: HTMLElement,
  backend: NestBackend,
  zeca: boolean,
  startFull = false,
): Nest {
  const portrait = new Scene(FOCUS_SCENE);
  portrait.setActive(false);
  let mounted = false;
  // A change in the conversation (a message, New, another one opened) may change Recent.
  let recentTimer: number | undefined;
  const chatChanged = () => {
    if (mounted) paintPortrait();
    window.clearTimeout(recentTimer);
    recentTimer = window.setTimeout(() => refreshRecent(), 400);
  };
  const chat = new ChatPanel(backend.chat, chatChanged, {
    window: "nest",
    render: renderFull,
  });
  const portraitButton = el("button", { class: "chat-character", onclick: () => {
    if (chat.clip(Clock.now()) === "idle" && !window.matchMedia("(prefers-reduced-motion: reduce)").matches
      && !document.body.classList.contains("still")) portrait.react("hello");
  } }, el("span", { class: "perch" }, portrait.canvas));
  portraitButton.addEventListener("pointerenter", () => portrait.lookAt("front"));
  portraitButton.addEventListener("pointerleave", () => portrait.lookAt(null));
  portrait.canvas.setAttribute("aria-hidden", "true");
  chat.perchSlot.append(portraitButton);
  const flock = createFlock({
    ...backend.flock,
    // Ask Zeca about a session: the Chat, with the question ready to finish or send.
    ask: (s) => {
      show("chat");
      chat.draft(t("What is {project} doing now?", { project: s.project || agentName(s) }));
    },
  });
  const tasks = createTasks({ ...backend.tasks, zeca: () => zecaOn });
  const memory = createMemory(backend.memory);
  // A row's question for Zeca opens the Chat with it in the composer; nothing is sent.
  const connections = createConnections({
    ...backend.connections,
    zeca: () =>
      zecaOn
        ? (text: string) => {
            show("chat");
            chat.draft(text);
          }
        : null,
  });
  // Activity's parts, each read again when its sub-tab opens.
  const parts: Record<Sub, { element: HTMLElement; refresh: () => void; flush?: () => Promise<void> }> = {
    week: createWeek(backend.week),
    projects: createRoosts(backend.roosts),
    history: createHistory(backend.records.history, backend.records.query),
    cost: (() => {
      const usage = createUsage(backend.records.usage);
      return costPart(createBudget(backend.budget, usage.refresh), usage);
    })(),
    traces: createTraces(backend.records.traces),
    audit: createAudit(backend.records.audit, backend.records.query),
  };

  // Chat: the conversation, and Zeca's memory beside it when asked for.
  const memoryBack = el("button", { class: "nest-memory-back", text: t("Back to Chat") });
  const memoryPane = el("aside", { class: "nest-memory" }, memoryBack, memory.element);
  memoryPane.hidden = true;
  const memoryLabel = el("span", { class: "nest-memory-label", text: t("Memory") });
  const memoryButton = el(
    "button",
    { class: "chat-ico nest-memory-toggle" },
    icon("sideRight", 15, 1.6),
    memoryLabel,
  );
  const setMemory = (open: boolean, returnFocus = false) => {
    memoryPane.hidden = !open;
    memoryButton.classList.toggle("on", open);
    memoryButton.setAttribute("aria-pressed", String(open));
    if (open) {
      if (window.innerWidth <= 1100) setRecent(false);
      memory.refresh();
      if (matchMedia("(max-width: 600px)").matches) {
        chat.cancelVoice();
        memoryBack.focus();
      }
    } else if (returnFocus) memoryButton.focus();
    if (mounted) paintPortrait();
  };
  memoryButton.addEventListener("click", () => setMemory(memoryPane.hasAttribute("hidden")));
  memoryBack.addEventListener("click", () => setMemory(false, true));
  memoryButton.setAttribute("aria-pressed", "false");
  // Recent, beside the conversation in a wide window: the saved ones and a new one.
  const recent = createRecent({
    list: () => chat.conversations(),
    open: (id) => { chat.openConversation(id); if (!wide.matches) setRecent(false, true); },
    fresh: () => { chat.newConversation(); if (!wide.matches) setRecent(false, true); },
    busy: () => chat.isBusy(),
    close: () => setRecent(false, true),
  });
  /** A sidebar when wide, a separate view on explicit request when narrow. */
  const wide = window.matchMedia("(min-width: 900px)");
  // The title bar, as the prototype draws it: the conversations' column, the conversation's title,
  // the chat's own pickers (moved here from its head) and the Memory panel.
  const chatTitle = el("h2", { class: "chat-title" });
  const recentButton = el("button", {
    class: "chat-ico recent-toggle",
    onclick: () => {
      setRecent(wide.matches ? chatPane.classList.contains("no-recent") : !chatPane.classList.contains("recent-open"));
    },
  }, icon("sideLeft", 15, 1.6));
  recentButton.setAttribute("aria-pressed", String(wide.matches));
  recentButton.setAttribute("aria-controls", "nest-recent");
  recent.element.id = "nest-recent";
  /** The conversation going on, for the title: asked with Recent, whatever the window's width. */
  let titled = 0;
  const refreshTitle = () => {
    const mine = ++titled;
    void chat.conversations().then(
      (rows) => {
        if (mine === titled) chatTitle.textContent = rows.find((c) => c.current)?.title || t("New conversation");
      },
      () => {},
    );
  };
  const refreshRecent = () => {
    if (tab !== "chat") return;
    refreshTitle();
    if (wide.matches && !chatPane.classList.contains("no-recent") || !wide.matches && chatPane.classList.contains("recent-open")) recent.refresh();
  };
  wide.addEventListener("change", () => {
    recentButton.setAttribute("aria-pressed", String(wide.matches ? !chatPane.classList.contains("no-recent") : chatPane.classList.contains("recent-open")));
    refreshRecent();
    if (!wide.matches && !chatPane.classList.contains("recent-open") && recent.element.contains(document.activeElement)) recentButton.focus();
    if (mounted) paintPortrait();
  });
  const chatPane = el(
    "div",
    { class: "nest-chat" },
    el(
      "header",
      { class: "chat-titlebar" },
      recentButton,
      chatTitle,
      // The chat's pickers and buttons, moved once: they keep their listeners and repaint in place.
      chat.element.querySelector(".chat-head") ?? el("span"),
      memoryButton,
    ),
    el(
      "div",
      { class: "nest-chat-body" },
      // After the conversation in the page, so the keyboard meets the composer first; drawn left.
      el("div", { class: "nest-chat-main" }, chat.element),
      recent.element,
      memoryPane,
    ),
  );

  function setRecent(open: boolean, returnFocus = false): void {
    if (open && window.innerWidth <= 1100) setMemory(false);
    if (wide.matches) chatPane.classList.toggle("no-recent", !open);
    else {
      chatPane.classList.toggle("recent-open", open);
      if (open) chat.cancelVoice();
    }
    recentButton.setAttribute("aria-pressed", String(open));
    if (open) {
      recent.refresh();
      recent.element.querySelector<HTMLInputElement>(".nest-recent-search")?.focus();
    } else if (returnFocus) recentButton.focus();
    if (mounted) paintPortrait();
  }

  const subButtons = SUBS.map((id) => {
    const b = el("button", { class: "nest-sub", onclick: () => showSub(id) });
    b.dataset.sub = id;
    return b;
  });
  const part = (id: Sub) => {
    const p = el(
      "div",
      { class: `nest-part nest-${id}-part` },
      parts[id].element,
    );
    p.hidden = true;
    return p;
  };
  const subPanes: Record<Sub, HTMLElement> = {
    week: part("week"),
    projects: part("projects"),
    history: part("history"),
    cost: part("cost"),
    traces: part("traces"),
    audit: part("audit"),
  };
  const activityPane = el(
    "div",
    { class: "nest-activity" },
    el("nav", { class: "nest-subs" }, ...subButtons),
    ...SUBS.map((id) => subPanes[id]),
  );

  const panes: Record<Tab, HTMLElement> = {
    flock: el("div", { class: "nest-pane nest-flock-pane" }, flock.element),
    tasks: el("div", { class: "nest-pane nest-tasks" }, tasks.element),
    chat: chatPane,
    activity: activityPane,
    connections: el("div", { class: "nest-pane nest-connections-pane" }, connections.element),
  };
  for (const id of TABS) {
    panes[id].id = `nest-pane-${id}`;
    panes[id].setAttribute("role", "tabpanel");
    panes[id].setAttribute("aria-labelledby", `nest-tab-${id}`);
  }
  const head = createNestHead({
    tab: (id) => show(id),
    gear: backend.settings,
    size: (s) => backend.size(s, currentPlace()),
    search: () => openPalette(),
    sound: backend.sound,
    media: backend.media,
    endDnd: backend.endDnd,
  });
  let zecaOn = zeca;
  /** The session in front, whose terminal the footer opens. */
  const front = () => {
    const f = last?.focus;
    return f ? last?.sessions.find((s) => s.agent === f.agent && s.id === f.id) : undefined;
  };
  const foot = createNestFoot({
    zeca: (on) => backend.setZeca(on),
    look: backend.look,
    terminal: () => {
      const s = front();
      if (s) backend.flock.jump(s);
    },
    island: backend.island,
  });
  let full = startFull;

  let tab: Tab = "flock";
  let last: ViewModel | null = null;
  let sub: Sub = "week";
  const currentPlace = () => tab === "activity" ? sub : tab;
  const portraitSpace = window.matchMedia("(min-height: 481px)");
  const portraitNarrow = window.matchMedia("(max-width: 600px)");
  function paintPortrait(): void {
    portrait.setActive(zecaOn && tab === "chat" && !document.hidden && !last?.locked
      && root.isConnected && portraitSpace.matches && !(portraitNarrow.matches && !memoryPane.hidden)
      && (wide.matches || !chatPane.classList.contains("recent-open")));
    portrait.update([], null, zecaOn ? { clip: chat.clip(Clock.now()), agent: chat.agent() } : null);
  }
  portraitSpace.addEventListener("change", paintPortrait);
  portraitNarrow.addEventListener("change", paintPortrait);
  let navigationReady = false;
  let placeSaved: Promise<void> = Promise.resolve();
  const rememberPlace = () => {
    if (!navigationReady) return;
    const place = currentPlace();
    placeSaved = placeSaved.catch(() => {}).then(() => backend.savePlace?.(place));
    void placeSaved.catch(() => {});
  };

  function showSub(id: Sub): void {
    sub = id;
    for (const s of SUBS) subPanes[s].hidden = s !== id;
    subButtons.forEach((b) => {
      const selected = b.dataset.sub === id;
      b.classList.toggle("on", selected);
      b.setAttribute("aria-pressed", String(selected));
      if (selected) b.scrollIntoView({ block: "nearest", inline: "nearest" });
    });
    parts[id].refresh();
    rememberPlace();
  }
  function show(id: Tab): void {
    if (id === "chat" && !zecaOn) id = "flock";
    tab = id;
    if (id !== "activity") rememberPlace();
    for (const k of TABS) panes[k].hidden = k !== id;
    head.set(id);
    paintNeeds();
    // The habitat draws only while it is in sight.
    flock.setActive(id === "flock" && !document.hidden && !last?.locked);
    if (id === "tasks") tasks.refresh();
    if (id === "activity") showSub(sub);
    if (id === "connections") connections.refresh();
    if (id === "chat") {
      chat.toggle(true);
      refreshRecent();
    } else chat.toggle(false);
    paintPortrait();
  }
  const relabel = () => {
    portraitButton.setAttribute("aria-label", t("Greet Zeca"));
    portraitButton.title = t("Greet Zeca");
    const subWords: Record<Sub, string> = {
      week: t("Week"),
      projects: t("By project"),
      history: t("History"),
      cost: t("Cost"),
      traces: t("Traces"),
      audit: t("Audit"),
    };
    for (const b of subButtons) b.textContent = subWords[b.dataset.sub as Sub];
    memoryButton.title = t("Memory");
    memoryButton.setAttribute("aria-label", t("Memory"));
    memoryLabel.textContent = t("Memory");
    memoryBack.textContent = t("Back to Chat");
    recentButton.title = t("Recent");
    recentButton.setAttribute("aria-label", t("Recent"));
    foot.relabel();
    head.relabel();
    chat.relabel();
    // What the tabs drew is in the old language too.
    if (last) flock.update(last);
    connections.redraw();
    if (tab === "tasks") tasks.refresh();
    if (tab === "activity") parts[sub].refresh();
    refreshRecent();
  };
  const setZeca = (on: boolean) => {
    zecaOn = on;
    chat.setEnabled(on);
    head.setZeca(on);
    foot.setZeca(on);
    flock.setZeca(on);
    relabel();
    if (!on && tab === "chat") show("flock");
  };

  // Over every tab but the Flock (which shows them on the wire), who needs you: a tag each, as the
  // prototype draws it; a click puts that session in front on the Flock, where its card waits.
  const needStrip = el("div", { class: "need-strip nest-need" });
  const paintNeeds = () => {
    // Drawn on every view: the tag that had the focus gets it back.
    const had = needStrip.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.key : null;
    const wanting = tab === "flock" ? [] : (last?.sessions ?? []).filter((s) => needsYou(s) || s.status === "failed" || s.attention === "failed");
    needStrip.hidden = !wanting.length;
    needStrip.setAttribute("aria-label", wanting.length === 1 ? t("1 session needs you") : t("{n} sessions need you", { n: wanting.length }));
    needStrip.replaceChildren(
      ...wanting.map((s: SessionView) => {
        const failed = s.status === "failed" || s.attention === "failed";
        const b = el(
          "button",
          {
            class: `need-tag${failed ? " failed" : ""}`,
            onclick: () => {
              backend.flock.focus(s);
              show("flock");
            },
          },
          el("i", { text: failed ? "×" : "!" }),
          el("span", { text: s.project || agentName(s) }),
        );
        b.dataset.key = `${s.agent}:${s.id}`;
        b.title = [s.project || agentName(s), statusText(s)].join(" · ");
        return b;
      }),
    );
    if (had) needStrip.querySelector<HTMLElement>(`[data-key="${CSS.escape(had)}"]`)?.focus();
  };
  root.append(
    ...head.elements,
    needStrip,
    ...TABS.map((id) => panes[id]),
    foot.element,
  );
  mounted = true;
  setZeca(zeca);
  show("flock");
  navigationReady = true;

  // Hidden (minimized, covered) or locked, nobody is looking: the habitat rests.
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) chat.cancelVoice();
    flock.setActive(tab === "flock" && !document.hidden && !last?.locked);
    paintPortrait();
  });
  let waiting: number | null = null;
  const view = (v: ViewModel) => {
    last = v;
    if (v.locked) chat.cancelVoice();
    setZecaLook(v.look ?? null);
    paintPortrait();
    head.setDnd(!!v.dnd);
    head.setCounts({
      sessions: v.sessions.length,
      needs: v.sessions.filter(needsYou).length,
      connections: (v.boards ?? []).reduce((n, b) => n + b.rows.filter((r) => r.group !== "releases").length, 0),
    });
    const f = front();
    foot.setTerminal(!!f && f.raise !== false);
    flock.setActive(tab === "flock" && !document.hidden && !v.locked);
    flock.update(v);
    connections.update(v);
    paintNeeds();
    // A moving flock may move By project; it reads the repositories, so not on every view.
    if (tab !== "activity" || sub !== "projects" || waiting !== null) return;
    waiting = window.setTimeout(() => {
      waiting = null;
      if (tab === "activity" && sub === "projects") parts.projects.refresh();
    }, ROOSTS_EVERY_MS);
  };
  const memoryChanged = () => {
    if (!memoryPane.hidden) memory.refresh();
  };
  const tasksChanged = () => {
    if (tab === "tasks") tasks.refresh();
    // A run's spend or answers may have changed what an open record shows.
    if (tab === "activity" && sub !== "week" && sub !== "projects")
      parts[sub].refresh();
  };
  const open = (id: string) => {
    if ((TABS as string[]).includes(id)) return show(id as Tab);
    if ((SUBS as string[]).includes(id)) {
      sub = id as Sub;
      show("activity");
    }
  };
  // Ctrl+K: the palette, over the tab (it never answers a card: a waiting session leads to its card).
  const palette = createPalette(() => (paletteHost.hidden = true));
  const paletteHost = el("div", { class: "palette-host" }, palette.element);
  paletteHost.hidden = true;
  paletteHost.addEventListener("click", (e) => {
    if (e.target === paletteHost) paletteHost.hidden = true;
  });
  root.append(paletteHost);
  function openPalette(): void {
    paletteHost.hidden = false;
    palette.open(
      paletteItems({
        sessions: last?.sessions ?? [],
        focus: backend.flock.focus,
        review: backend.flock.review,
        command: zecaOn
          ? (text) => {
              show("chat");
              chat.draft(text);
            }
          : null,
        tab: open,
      }),
    );
  }
  window.addEventListener("keydown", (e) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.key.toLowerCase() !== "k") return;
    e.preventDefault();
    openPalette();
  });
  // Alt+1 to 5 pick a tab by the key's place, whatever the layout (a hidden Chat is skipped); Esc
  // leaves Full for the size before it. Neither while typing, composing or speaking.
  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !e.isComposing && !e.defaultPrevented && tab === "chat" && chat.cancelVoice()) {
      e.preventDefault();
      return;
    }
    if (e.key === "Escape" && !e.isComposing && !e.defaultPrevented && paletteHost.hidden && tab === "chat" && !wide.matches && chatPane.classList.contains("recent-open")) {
      e.preventDefault();
      setRecent(false, true);
      return;
    }
    const typing = (e.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]");
    if (e.isComposing || e.defaultPrevented || typing) return;
    // Esc closes an open palette first, wherever the focus went.
    if (e.key === "Escape" && !paletteHost.hidden) {
      e.preventDefault();
      paletteHost.hidden = true;
      return;
    }
    if (e.key === "Escape" && tab === "chat" && !memoryPane.hidden && matchMedia("(max-width: 600px)").matches) {
      e.preventDefault();
      setMemory(false, true);
      return;
    }
    if (e.key === "Escape" && full) {
      e.preventDefault();
      backend.size("room", currentPlace());
      return;
    }
    const pick = tabByKey(e, zecaOn);
    if (pick) {
      e.preventDefault();
      show(pick);
    }
  });
  const setFull = (on: boolean) => {
    full = on;
    root.classList.toggle("full", on);
    head.setFull(on);
    flock.setFull(on);
  };
  setFull(startFull);
  return {
    flushState: async () => {
      chat.cancelVoice();
      await chat.flushDraft();
      await connections.flush();
      await flock.flush();
      await Promise.all([parts.projects.flush?.(), parts.history.flush?.(), parts.audit.flush?.()]);
      rememberPlace();
      await placeSaved;
    },
    chat,
    view,
    relabel,
    memoryChanged,
    tasksChanged,
    setZeca,
    open,
    current: currentPlace,
    setFull,
    setSound: head.setSound,
    setMedia: head.setMedia,
    setSpecies: (id) => {
      setZecaSpecies(id);
      foot.setSpecies(id);
      paintPortrait();
    },
    usageChanged: flock.usageChanged,
    setKeys: (keys) => {
      chat.setKeys(keys);
      flock.setKeys(keys);
    },
    shortcut: (id) => {
      if (id === "talk" || id === "talk-up") {
        if (id === "talk" && zecaOn && !document.hidden && !last?.locked && root.isConnected) {
          if (portraitNarrow.matches) setMemory(false);
          show("chat");
        }
        if (tab === "chat" && !document.hidden && !last?.locked && root.isConnected) chat.holdToTalk(id === "talk");
        return;
      }
      if (id !== "allow" && id !== "deny") return;
      const allow = id === "allow";
      if (tab === "chat") chat.answerWaiting(allow);
      else if (tab === "flock") flock.shortcut(allow);
    },
  };
}
