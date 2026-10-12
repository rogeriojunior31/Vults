// The Nest's page, with no Tauri in it: the backend comes in, so the lab can draw it too. Five tabs
// and the gear (ADR 0021): Flock, Tasks, Chat (with its Memory beside it), Activity (Week, By
// project, History, Cost, Traces, Audit) and Connections. Without Zeca there is no Chat tab.
import type { Size } from "../../bridge";
import { el } from "../../dom";
import { createPalette, paletteItems } from "../../palette";
import { t } from "../../i18n";
import { ChatPanel, type ChatBackend } from "../../island/chat";
import { agentName } from "../../island/views";
import { renderFull } from "../../island/markdown";
import type { ViewModel } from "../../view.gen";
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
  /** Zeca's species, for the footer's mark. */
  setSpecies: (id: string) => void;
}

export interface NestBackend {
  chat: ChatBackend;
  /** Brings the island's chat up for a card. */
  toIsland: () => void;
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
  /** The footer's *Zeca's look*: Settings › Flock & Zeca. */
  look: () => void;
  /** The size switch (ADR 0021), with the tab shown now to keep the place. */
  size: (size: Size, tab: string) => void;
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
  return {
    element: el("div", { class: "nest-cost" }, budget.element, usage.element),
    refresh: () => {
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
  // A change in the conversation (a message, New, another one opened) may change Recent.
  let recentTimer: number | undefined;
  const chatChanged = () => {
    window.clearTimeout(recentTimer);
    recentTimer = window.setTimeout(() => refreshRecent(), 400);
  };
  const chat = new ChatPanel(backend.chat, chatChanged, {
    window: "nest",
    render: renderFull,
    toIsland: backend.toIsland,
  });
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
  const connections = createConnections(backend.connections);
  // Activity's parts, each read again when its sub-tab opens.
  const parts: Record<Sub, { element: HTMLElement; refresh: () => void }> = {
    week: createWeek(backend.week),
    projects: createRoosts(backend.roosts),
    history: createHistory(backend.records.history),
    cost: (() => {
      const usage = createUsage(backend.records.usage);
      return costPart(createBudget(backend.budget, usage.refresh), usage);
    })(),
    traces: createTraces(backend.records.traces),
    audit: createAudit(backend.records.audit),
  };

  // Chat: the conversation, and Zeca's memory beside it when asked for.
  const memoryPane = el("aside", { class: "nest-memory" }, memory.element);
  memoryPane.hidden = true;
  const memoryButton = el("button", {
    class: "btn small nest-memory-toggle",
    onclick: () => {
      memoryPane.hidden = !memoryPane.hidden;
      memoryButton.classList.toggle("on", !memoryPane.hidden);
      if (!memoryPane.hidden) memory.refresh();
    },
  });
  // Recent, beside the conversation in a wide window: the saved ones and a new one.
  const recent = createRecent({
    list: () => chat.conversations(),
    open: (id) => chat.openConversation(id),
    fresh: () => chat.newConversation(),
    busy: () => chat.isBusy(),
  });
  /** Recent is only shown in a wide window: asked for only then. */
  const wide = window.matchMedia("(min-width: 900px)");
  const refreshRecent = () => {
    if (tab === "chat" && wide.matches) recent.refresh();
  };
  wide.addEventListener("change", refreshRecent);
  const chatPane = el(
    "div",
    { class: "nest-chat" },
    el("div", { class: "nest-chat-bar" }, memoryButton),
    el(
      "div",
      { class: "nest-chat-body" },
      // After the conversation in the page, so the keyboard meets the composer first; drawn left.
      el("div", { class: "nest-chat-main" }, chat.element),
      recent.element,
      memoryPane,
    ),
  );

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
    connections: el("div", { class: "nest-pane" }, connections.element),
  };
  const head = createNestHead({
    tab: (id) => show(id),
    gear: backend.settings,
    size: (s) => backend.size(s, tab),
    search: () => openPalette(),
    sound: backend.sound,
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

  function showSub(id: Sub): void {
    sub = id;
    for (const s of SUBS) subPanes[s].hidden = s !== id;
    subButtons.forEach((b) => b.classList.toggle("on", b.dataset.sub === id));
    parts[id].refresh();
  }
  function show(id: Tab): void {
    if (id === "chat" && !zecaOn) id = "flock";
    tab = id;
    for (const k of TABS) panes[k].hidden = k !== id;
    head.set(id);
    // The habitat draws only while it is in sight.
    flock.setActive(id === "flock" && !document.hidden && !last?.locked);
    if (id === "tasks") tasks.refresh();
    if (id === "activity") showSub(sub);
    if (id === "connections") connections.refresh();
    if (id === "chat") {
      chat.toggle(true);
      refreshRecent();
    }
  }
  const relabel = () => {
    const subWords: Record<Sub, string> = {
      week: t("Week"),
      projects: t("By project"),
      history: t("History"),
      cost: t("Cost"),
      traces: t("Traces"),
      audit: t("Audit"),
    };
    for (const b of subButtons) b.textContent = subWords[b.dataset.sub as Sub];
    memoryButton.textContent = t("Memory");
    foot.relabel();
    head.relabel();
    // What the tabs drew is in the old language too.
    if (last) flock.update(last);
    connections.redraw();
    if (tab === "tasks") tasks.refresh();
    if (tab === "activity") parts[sub].refresh();
    refreshRecent();
  };
  const setZeca = (on: boolean) => {
    zecaOn = on;
    head.setZeca(on);
    foot.setZeca(on);
    flock.setZeca(on);
    relabel();
    if (!on && tab === "chat") show("flock");
  };

  root.append(
    ...head.elements,
    ...TABS.map((id) => panes[id]),
    foot.element,
  );
  setZeca(zeca);
  show("flock");

  // Hidden (minimized, covered) or locked, nobody is looking: the habitat rests.
  document.addEventListener("visibilitychange", () => flock.setActive(tab === "flock" && !document.hidden && !last?.locked));
  let waiting: number | null = null;
  const view = (v: ViewModel) => {
    last = v;
    head.setCounts({
      sessions: v.sessions.length,
      needs: v.sessions.filter(needsYou).length,
      connections: (v.boards ?? []).reduce((n, b) => n + b.rows.length, 0),
    });
    const f = front();
    foot.setTerminal(!!f && f.raise !== false);
    flock.setActive(tab === "flock" && !document.hidden && !v.locked);
    flock.update(v);
    connections.update(v);
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
  // Ctrl+K: the palette, over the tab (it never answers a card: a waiting session leads to the island).
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
    const typing = (e.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]");
    if (e.isComposing || e.defaultPrevented || typing) return;
    // Esc closes an open palette first, wherever the focus went.
    if (e.key === "Escape" && !paletteHost.hidden) {
      e.preventDefault();
      paletteHost.hidden = true;
      return;
    }
    if (e.key === "Escape" && full) {
      if (chat.cancelVoice()) return;
      e.preventDefault();
      backend.size("room", tab);
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
    chat,
    view,
    relabel,
    memoryChanged,
    tasksChanged,
    setZeca,
    open,
    current: () => tab,
    setFull,
    setSound: head.setSound,
    setSpecies: foot.setSpecies,
  };
}
