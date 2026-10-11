// The Nest's page, with no Tauri in it: the backend comes in, so the lab can draw it too.
import type { RoostView } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { ChatPanel, type ChatBackend } from "../../island/chat";
import { renderFull } from "../../island/markdown";
import { createRoosts } from "./roosts";

export interface Nest {
  chat: ChatPanel;
  /** Asks for the roosts again, when their tab is open; at most once every second and a half. */
  flockMoved: () => void;
}

/** How often the Roosts tab may ask again while the flock moves: each ask reads the repositories. */
const ROOSTS_EVERY_MS = 1500;

/** The Nest's header and its tabs; `toIsland` brings the island's chat up for a card. */
export function createNest(
  root: HTMLElement,
  backend: ChatBackend,
  toIsland: () => void,
  roostsOf: () => Promise<RoostView[]>,
): Nest {
  const chat = new ChatPanel(backend, () => {}, { window: "nest", render: renderFull, toIsland });
  const roosts = createRoosts(roostsOf);
  const chatPane = el("div", { class: "nest-chat" }, chat.element);
  const roostsPane = el("div", { class: "nest-roosts" }, roosts.element);
  roostsPane.hidden = true;
  const tabs: HTMLButtonElement[] = [];
  const show = (which: "chat" | "roosts") => {
    chatPane.hidden = which !== "chat";
    roostsPane.hidden = which !== "roosts";
    tabs.forEach((b) => b.classList.toggle("on", b.dataset.tab === which));
    if (which === "roosts") roosts.refresh();
  };
  for (const [id, label] of [
    ["chat", t("Chat")],
    ["roosts", t("Roosts")],
  ] as const) {
    const b = el("button", { class: "nest-tab", text: label, onclick: () => show(id) });
    b.dataset.tab = id;
    tabs.push(b);
  }
  root.append(
    el(
      "header",
      { class: "nest-head" },
      el("span", { class: "nest-title", text: t("Nest") }),
      el("nav", { class: "nest-tabs" }, ...tabs),
    ),
    chatPane,
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
  return { chat, flockMoved };
}
