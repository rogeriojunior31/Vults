// The size switch (ADR 0021), in the island's header and the Nest's: Mini, Quick, Room, Full. The
// same work at the distance the user needs; changing size keeps the place.
import type { Size } from "./bridge";
import { el } from "./dom";
import { t } from "./i18n";

const SIZES: Size[] = ["mini", "quick", "room", "full"];

const label = (s: Size): string => ({ mini: t("Mini"), quick: t("Quick"), room: t("Room"), full: t("Full") })[s];

const about = (s: Size): string =>
  ({
    mini: t("Mini: the flock alone, in the pill"),
    quick: t("Quick: the island, to see and answer"),
    room: t("Room: the Nest, in a window"),
    full: t("Full: the Nest on the whole screen; Esc goes back"),
  })[s];

/** The switch with `now` pressed; `go` changes size. `off` lists sizes that can't be taken now. */
export function sizeSwitch(now: Size, go: (s: Size) => void, off: Size[] = []): HTMLElement {
  const group = el("div", { class: "size-switch" });
  group.setAttribute("role", "group");
  group.setAttribute("aria-label", t("Size"));
  for (const s of SIZES) {
    const b = el("button", { class: `size${s === now ? " on" : ""}`, text: label(s), onclick: () => s !== now && go(s) });
    b.dataset.size = s;
    b.title = about(s);
    b.setAttribute("aria-pressed", String(s === now));
    if (off.includes(s)) b.disabled = true;
    group.append(b);
  }
  return group;
}
