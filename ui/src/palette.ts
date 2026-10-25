// The command palette (ADR 0021, plan-zeca K9): Ctrl+K in the island and the Nest. It jumps to a
// session, puts `/continue` or `/remember` in the chat to finish, and opens a
// tab. It never answers a card (D1): a session that waits only leads to its card.
import { el } from "./dom";
import { t } from "./i18n";
import type { SessionView } from "./view.gen";
import { agentName } from "./island/views";

export interface PaletteItem {
  label: string;
  hint: string;
  run: () => void;
}

/** What the palette can do where it opens. */
export interface PaletteContext {
  sessions: SessionView[];
  /** Puts a session in front. */
  focus: (s: SessionView) => void;
  /** Brings up the card this session waits on (the island's, or the Nest's Flock). */
  review: (s: SessionView) => void;
  /** The chat, with a command started for the user to finish; only with Zeca on. */
  command: ((text: string) => void) | null;
  /** Opens a tab of the Nest by its id. */
  tab: (id: string) => void;
}

/** Each palette's list gets its own id, for its input's `aria-controls`. */
let palettes = 0;

const waits = (s: SessionView) => s.status === "approval" || s.status === "question" || s.waiting === true;

/** Everything the palette offers, sessions first. */
export function paletteItems(c: PaletteContext): PaletteItem[] {
  const sessions = c.sessions.map((s) => ({
    label: s.project || agentName(s),
    hint: waits(s) ? t("Review its card") : [s.branch, s.step].filter(Boolean).join(" · ") || t("Put this session in front"),
    run: () => (waits(s) ? c.review(s) : c.focus(s)),
  }));
  const run = c.command;
  const commands = run
    ? [
        { label: "/continue", hint: t("Hand off to a session that stopped"), run: () => run("/continue ") },
        { label: "/remember", hint: t("Keep a fact for Zeca"), run: () => run("/remember ") },
      ]
    : [];
  const tabs: [string, string][] = [
    ["flock", t("Open the Flock")],
    ...(run ? ([["chat", t("Open the Chat")]] as [string, string][]) : []),
    ["dictation", t("Open Dictation")],
    ["connections", t("Open Connections")],
    ["activity", t("Open Activity")],
  ];
  return [...sessions, ...commands, ...tabs.map(([id, label]) => ({ label, hint: t("In the Nest"), run: () => c.tab(id) }))];
}

/** `close` runs on a choice or Esc; `resized` after the list is drawn again (the island measures). */
export function createPalette(close: () => void, resized: () => void = () => {}): {
  element: HTMLElement;
  open: (items: PaletteItem[]) => void;
} {
  const input = el("input", { class: "palette-input" });
  input.type = "text";
  input.autocomplete = "off";
  input.spellcheck = false;
  const list = el("ul", { class: "palette-list" });
  list.setAttribute("role", "listbox");
  list.id = `palette-list-${++palettes}`;
  input.setAttribute("role", "combobox");
  input.setAttribute("aria-expanded", "true");
  input.setAttribute("aria-controls", list.id);
  const foot = el("p", { class: "palette-foot" });
  const element = el("div", { class: "palette" }, input, list, foot);
  element.setAttribute("role", "dialog");
  let items: PaletteItem[] = [];
  let shown: PaletteItem[] = [];
  let at = 0;
  const draw = () => {
    const q = input.value.trim().toLowerCase();
    shown = items.filter((i) => !q || `${i.label} ${i.hint}`.toLowerCase().includes(q));
    at = Math.min(at, Math.max(0, shown.length - 1));
    list.replaceChildren(
      ...shown.map((item, i) => {
        const li = el("li", { class: `palette-item${i === at ? " on" : ""}` }, el("span", { class: "palette-label", text: item.label }), el("span", { class: "palette-hint", text: item.hint }));
        li.setAttribute("role", "option");
        li.setAttribute("aria-selected", String(i === at));
        li.id = `${list.id}-${i}`;
        li.addEventListener("click", () => choose(item));
        return li;
      }),
      ...(shown.length ? [] : [el("li", { class: "palette-none", text: t("Nothing matches.") })]),
    );
    if (shown.length) input.setAttribute("aria-activedescendant", `${list.id}-${at}`);
    else input.removeAttribute("aria-activedescendant");
    resized();
  };
  const choose = (item: PaletteItem) => {
    close();
    item.run();
  };
  input.addEventListener("input", () => {
    at = 0;
    draw();
  });
  input.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = Math.max(1, shown.length);
      at = (at + (e.key === "ArrowDown" ? 1 : n - 1)) % n;
      draw();
    } else if (e.key === "Enter") {
      e.preventDefault();
      const item = shown[at];
      if (item) choose(item);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
    }
  });
  return {
    element,
    open: (next) => {
      items = next;
      at = 0;
      input.value = "";
      input.placeholder = t("Jump to a session, or type /");
      input.setAttribute("aria-label", t("Search sessions and commands"));
      foot.textContent = t("Cards are answered on the card itself or on the phone, never from here.");
      draw();
      input.focus();
    },
  };
}
