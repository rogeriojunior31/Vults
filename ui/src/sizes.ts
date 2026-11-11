// The size switch (ADR 0021), in the island's header and the Nest's: Mini, Quick, Room, Full. The
// same work at the distance the user needs; changing size keeps the place.
import type { Size } from "./bridge";
import { el } from "./dom";
import { t } from "./i18n";
import { icon } from "./island/icons";

const SIZES: Size[] = ["mini", "quick", "room", "full"];

const label = (s: Size): string => ({ mini: t("Mini"), quick: t("Quick"), room: t("Room"), full: t("Full") })[s];

const about = (s: Size): string =>
  ({
    mini: t("Mini: the flock alone, in the pill"),
    quick: t("Quick: the island, to see and answer"),
    room: t("Room: the Nest, in a window"),
    full: t("Full: the Nest on the whole screen; Esc goes back"),
  })[s];

/** What each size is, under its name in the menu. */
const meaning = (s: Size): string =>
  ({
    mini: t("The flock alone, in the pill"),
    quick: t("The island, to see and answer"),
    room: t("The Nest, in a window"),
    full: t("The Nest on the whole screen; Esc goes back"),
  })[s];

export interface SizeMenuOptions {
  /** Sizes that can't be taken now. */
  off?: Size[];
  /** Sounds on or off, as the menu's last item. */
  sound?: { on: boolean; set: (on: boolean) => void };
  /** A class for the Full item (the Nest's tests and styles find it). */
  fullClass?: string;
}

/** One button for how Vults shows itself: it names the size in use and opens the four, each with
 *  what it is, and the sounds. It replaces a row of size buttons, full screen and mute: the header
 *  keeps one control where it had six. The items stay in the tree while closed (hidden), so a
 *  keyboard or a test reaches the same buttons. */
export function sizeMenu(now: Size, go: (s: Size) => void, options: SizeMenuOptions = {}): HTMLElement {
  const wrap = el("div", { class: "size-menu" });
  const button = el("button", { class: "size-menu-button" }, el("span", { class: "size-menu-now", text: label(now) }), icon("chevron", 12, 2));
  button.setAttribute("aria-haspopup", "menu");
  button.setAttribute("aria-expanded", "false");
  button.setAttribute("aria-label", t("Presentation: {size}", { size: label(now) }));
  button.title = t("Presentation: {size}", { size: label(now) });
  const list = el("div", { class: "size-menu-list" });
  list.setAttribute("role", "menu");
  list.setAttribute("aria-label", t("Presentation"));
  list.hidden = true;
  const items: HTMLButtonElement[] = [];
  const outside = (e: PointerEvent) => { if (!wrap.contains(e.target as Node)) close(false); };
  function close(focus: boolean): void {
    if (list.hidden) return;
    list.hidden = true;
    button.setAttribute("aria-expanded", "false");
    document.removeEventListener("pointerdown", outside, true);
    if (focus) button.focus();
  }
  const open = () => {
    list.hidden = false;
    button.setAttribute("aria-expanded", "true");
    document.addEventListener("pointerdown", outside, true);
    (items.find((b) => b.getAttribute("aria-checked") === "true" && !b.disabled) ?? items.find((b) => !b.disabled))?.focus();
  };
  button.addEventListener("click", () => (list.hidden ? open() : close(true)));
  for (const s of SIZES) {
    const item = el("button", { class: `size-item${s === now ? " on" : ""}${s === "full" && options.fullClass ? ` ${options.fullClass}` : ""}`, onclick: () => {
      close(true);
      if (s !== now) go(s);
    } }, el("strong", { text: label(s) }), el("span", { text: meaning(s) })) as HTMLButtonElement;
    item.dataset.size = s;
    item.setAttribute("role", "menuitemradio");
    item.setAttribute("aria-checked", String(s === now));
    item.tabIndex = -1;
    if (options.off?.includes(s)) item.disabled = true;
    items.push(item);
    list.append(item);
  }
  if (options.sound) {
    const sound = options.sound;
    const item = el("button", { class: "size-item size-sound", onclick: () => { close(true); sound.set(!sound.on); } },
      el("strong", { text: t("Sounds") }), el("span", { text: sound.on ? t("On: cards and finished turns chime") : t("Off: Vults stays silent") })) as HTMLButtonElement;
    item.setAttribute("role", "menuitemcheckbox");
    item.setAttribute("aria-checked", String(sound.on));
    item.tabIndex = -1;
    items.push(item);
    list.append(el("div", { class: "size-menu-sep" }), item);
  }
  list.addEventListener("keydown", (e) => {
    const usable = items.filter((b) => !b.disabled);
    const at = usable.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(true); return; }
    if (e.key === "Tab") { close(false); return; }
    const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (step) { e.preventDefault(); usable[(at + step + usable.length) % usable.length]?.focus(); }
    else if (e.key === "Home") { e.preventDefault(); usable[0]?.focus(); }
    else if (e.key === "End") { e.preventDefault(); usable.at(-1)?.focus(); }
  });
  wrap.append(button, list);
  return wrap;
}

/** The switch with `now` pressed; `go` changes size. `off` lists sizes that can't be taken now;
 * `shown` the ones drawn (the Nest draws Full as a button of its own). */
export function sizeSwitch(now: Size, go: (s: Size) => void, off: Size[] = [], shown: Size[] = SIZES): HTMLElement {
  const group = el("div", { class: "size-switch" });
  group.setAttribute("role", "group");
  group.setAttribute("aria-label", t("Size"));
  for (const s of shown) {
    const b = el("button", { class: `size${s === now ? " on" : ""}`, text: label(s), onclick: () => s !== now && go(s) });
    b.dataset.size = s;
    b.title = about(s);
    b.setAttribute("aria-pressed", String(s === now));
    if (off.includes(s)) b.disabled = true;
    group.append(b);
  }
  return group;
}
