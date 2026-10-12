// The Nest window's header (ADR 0021, 0023): the five tabs, the size switch and the gear. The Nest's
// page and Settings' draw the same one: they are two webviews of one window, and the gear or a tab
// moves between them. It holds no agent's text, so Settings may show it.
import type { Size } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { sizeSwitch } from "../../sizes";

export type NestTab = "flock" | "tasks" | "chat" | "activity" | "connections";
export const NEST_TABS: NestTab[] = ["flock", "tasks", "chat", "activity", "connections"];

export interface NestHeadActions {
  tab: (id: NestTab) => void;
  /** The gear: Settings. */
  gear: () => void;
  size: (size: Size) => void;
}

export interface NestHead {
  element: HTMLElement;
  /** The tab shown, or the gear's Settings. */
  set: (on: NestTab | "settings") => void;
  /** Without Zeca there is no Chat tab. */
  setZeca: (on: boolean) => void;
  setFull: (full: boolean) => void;
  /** Draws its words again: the language changed. */
  relabel: () => void;
}

export function createNestHead(on: NestHeadActions): NestHead {
  const tabs = NEST_TABS.map((id) => {
    const b = el("button", { class: "nest-tab", onclick: () => on.tab(id) });
    b.dataset.tab = id;
    return b;
  });
  const gear = el("button", { class: "nest-gear", text: "⚙", onclick: on.gear });
  const sizes = el("span", { class: "nest-sizes" });
  let full = false;
  const drawSizes = () => sizes.replaceChildren(sizeSwitch(full ? "full" : "room", on.size));
  const relabel = () => {
    const words: Record<NestTab, string> = {
      flock: t("Flock"),
      tasks: t("Tasks"),
      chat: t("Chat"),
      activity: t("Activity"),
      connections: t("Connections"),
    };
    for (const b of tabs) b.textContent = words[b.dataset.tab as NestTab];
    gear.title = t("Settings");
    gear.setAttribute("aria-label", t("Settings"));
    drawSizes();
  };
  relabel();
  return {
    element: el("header", { class: "nest-head" }, el("nav", { class: "nest-tabs" }, ...tabs), sizes, gear),
    set: (id) => {
      tabs.forEach((b) => b.classList.toggle("on", b.dataset.tab === id));
      gear.classList.toggle("on", id === "settings");
      gear.setAttribute("aria-pressed", String(id === "settings"));
    },
    setZeca: (zeca) => {
      tabs.find((b) => b.dataset.tab === "chat")!.hidden = !zeca;
    },
    setFull: (f) => {
      full = f;
      drawSizes();
    },
    relabel,
  };
}

/** Alt+1 to 5: the tab at that place, whatever the layout (a hidden Chat is skipped). Not while
 * typing or composing. */
export function tabByKey(e: KeyboardEvent, zeca: boolean): NestTab | undefined {
  const typing = (e.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]");
  if (e.isComposing || e.defaultPrevented || typing || !e.altKey || e.ctrlKey || e.metaKey) return undefined;
  const n = /^Digit([1-5])$/.exec(e.code);
  return n ? NEST_TABS.filter((id) => id !== "chat" || zeca)[Number(n[1]) - 1] : undefined;
}
