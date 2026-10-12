// The Nest's page, with no Tauri in it: the backend comes in, so the lab can draw it too.
import { el } from "../../dom";
import { t } from "../../i18n";
import { ChatPanel, type ChatBackend } from "../../island/chat";
import { renderFull } from "../../island/markdown";
import type { AgentRow } from "../../bridge";
import { createAgents } from "./agents";
import { createMemory, type MemoryBackend } from "./memory";
import { createAudit, createHistory, createTraces, createUsage, type RecordsBackend } from "./records";
import { createRoosts, type RoostsBackend } from "./roosts";
import { createTasks, type TasksBackend } from "./tasks";

export interface Nest {
  chat: ChatPanel;
  /** Asks for the roosts again, when their tab is open; at most once every second and a half. */
  flockMoved: () => void;
  /** The header's words again, in the language just set. */
  relabel: () => void;
  /** The memory changed: its tab reads it again, when open. */
  memoryChanged: () => void;
  /** The tasks changed (`/task`, another window): their tab reads them again, when open. */
  tasksChanged: () => void;
}

/** How often the Roosts tab may ask again while the flock moves: each ask reads the repositories. */
const ROOSTS_EVERY_MS = 1500;

/** The Nest's header and its tabs; `toIsland` brings the island's chat up for a card. */
export function createNest(
  root: HTMLElement,
  backend: ChatBackend,
  toIsland: () => void,
  roostsBackend: RoostsBackend,
  memoryBackend: MemoryBackend,
  agentsOf: (fresh: boolean) => Promise<AgentRow[]>,
  tasksBackend: TasksBackend,
  recordsBackend: RecordsBackend,
): Nest {
  const chat = new ChatPanel(backend, () => {}, { window: "nest", render: renderFull, toIsland });
  const roosts = createRoosts(roostsBackend);
  const chatPane = el("div", { class: "nest-chat" }, chat.element);
  const roostsPane = el("div", { class: "nest-roosts" }, roosts.element);
  roostsPane.hidden = true;
  const memory = createMemory(memoryBackend);
  const memoryPane = el("div", { class: "nest-memory" }, memory.element);
  memoryPane.hidden = true;
  const agents = createAgents(agentsOf);
  const agentsPane = el("div", { class: "nest-agents" }, agents.element);
  agentsPane.hidden = true;
  const tasks = createTasks(tasksBackend);
  const tasksPane = el("div", { class: "nest-tasks" }, tasks.element);
  tasksPane.hidden = true;
  // The records (W11): each read again when its tab opens.
  const records = {
    history: createHistory(recordsBackend.history),
    usage: createUsage(recordsBackend.usage),
    traces: createTraces(recordsBackend.traces),
    audit: createAudit(recordsBackend.audit),
  };
  const pane = (r: { element: HTMLElement }) => {
    const p = el("div", { class: "nest-records" }, r.element);
    p.hidden = true;
    return p;
  };
  const recordPanes: Record<keyof typeof records, HTMLElement> = {
    history: pane(records.history),
    usage: pane(records.usage),
    traces: pane(records.traces),
    audit: pane(records.audit),
  };
  const tabs: HTMLButtonElement[] = [];
  type Tab = "chat" | "tasks" | "roosts" | "memory" | "agents" | keyof typeof records;
  const show = (which: Tab) => {
    chatPane.hidden = which !== "chat";
    tasksPane.hidden = which !== "tasks";
    if (which === "tasks") tasks.refresh();
    roostsPane.hidden = which !== "roosts";
    memoryPane.hidden = which !== "memory";
    agentsPane.hidden = which !== "agents";
    if (which === "agents") agents.refresh();
    for (const [id, pane] of Object.entries(recordPanes)) {
      pane.hidden = which !== id;
      if (which === id) records[id as keyof typeof records].refresh();
    }
    tabs.forEach((b) => b.classList.toggle("on", b.dataset.tab === which));
    if (which === "roosts") roosts.refresh();
    if (which === "memory") memory.refresh();
  };
  const labels = {
    chat: () => t("Chat"),
    tasks: () => t("Tasks"),
    memory: () => t("Memory"),
    roosts: () => t("Roosts"),
    history: () => t("History"),
    usage: () => t("Usage"),
    traces: () => t("Traces"),
    audit: () => t("Audit"),
    agents: () => t("Agents"),
  } as const;
  for (const id of ["chat", "tasks", "memory", "roosts", "history", "usage", "traces", "audit", "agents"] as const) {
    const b = el("button", { class: "nest-tab", text: labels[id](), onclick: () => show(id) });
    b.dataset.tab = id;
    tabs.push(b);
  }
  const title = el("span", { class: "nest-title", text: t("Nest") });
  root.append(
    el(
      "header",
      { class: "nest-head" },
      title,
      el("nav", { class: "nest-tabs" }, ...tabs),
    ),
    chatPane,
    tasksPane,
    memoryPane,
    roostsPane,
    ...Object.values(recordPanes),
    agentsPane,
  );
  show("chat");
  chat.toggle(true);
  let waiting: number | null = null;
  const flockMoved = () => {
    if (roostsPane.hidden || waiting !== null) return;
    waiting = window.setTimeout(() => {
      waiting = null;
      if (!roostsPane.hidden) roosts.refresh();
    }, ROOSTS_EVERY_MS);
  };
  const relabel = () => {
    title.textContent = t("Nest");
    for (const b of tabs) b.textContent = labels[b.dataset.tab as keyof typeof labels]();
  };
  const memoryChanged = () => {
    if (!memoryPane.hidden) memory.refresh();
  };
  const tasksChanged = () => {
    if (!tasksPane.hidden) tasks.refresh();
    // A run's spend or answers may have changed what an open record shows.
    for (const [id, pane] of Object.entries(recordPanes)) {
      if (!pane.hidden) records[id as keyof typeof records].refresh();
    }
  };
  return { chat, flockMoved, relabel, memoryChanged, tasksChanged };
}
