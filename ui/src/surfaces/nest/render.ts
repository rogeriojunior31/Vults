// The Nest's page, with no Tauri in it: the backend comes in, so the lab can draw it too.
import { el } from "../../dom";
import { t } from "../../i18n";
import { ChatPanel, type ChatBackend } from "../../island/chat";
import { renderFull } from "../../island/markdown";
import { createMemory, type MemoryBackend } from "./memory";
import { createRoosts, type RoostsBackend } from "./roosts";

export interface Nest {
  chat: ChatPanel;
  /** Asks for the roosts again, when their tab is open; at most once every second and a half. */
  flockMoved: () => void;
  /** The header's words again, in the language just set. */
  relabel: () => void;
  /** The memory changed: its tab reads it again, when open. */
  memoryChanged: () => void;
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
): Nest {
  const chat = new ChatPanel(backend, () => {}, { window: "nest", render: renderFull, toIsland });
  const roosts = createRoosts(roostsBackend);
  const chatPane = el("div", { class: "nest-chat" }, chat.element);
  const roostsPane = el("div", { class: "nest-roosts" }, roosts.element);
  roostsPane.hidden = true;
  const memory = createMemory(memoryBackend);
  const memoryPane = el("div", { class: "nest-memory" }, memory.element);
  memoryPane.hidden = true;
  const tabs: HTMLButtonElement[] = [];
  const show = (which: "chat" | "roosts" | "memory") => {
    chatPane.hidden = which !== "chat";
    roostsPane.hidden = which !== "roosts";
    memoryPane.hidden = which !== "memory";
    tabs.forEach((b) => b.classList.toggle("on", b.dataset.tab === which));
    if (which === "roosts") roosts.refresh();
    if (which === "memory") memory.refresh();
  };
  const labels = { chat: () => t("Chat"), memory: () => t("Memory"), roosts: () => t("Roosts") } as const;
  for (const id of ["chat", "memory", "roosts"] as const) {
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
    memoryPane,
    roostsPane,
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
  return { chat, flockMoved, relabel, memoryChanged };
}
