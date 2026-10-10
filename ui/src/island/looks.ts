// Zeca's looks, picked on the island: a right-click on him opens this in place of the overview.
// Zeca stays on his perch at the left and wears whatever the pointer rests on (the live preview);
// a click picks it, the same setting as Settings → Flock → Look. No Tauri here: `on` says what to do.
import { speciesSet } from "../character/flock";
import { dress, LOOK_GROUPS } from "../character/looks";
import { drawFrame, frameAt } from "../character/sprites";
import { perchOf } from "../character/zeca";
import { el } from "../dom";
import { icon } from "./icons";
import { zecaSpecies } from "./flock";
import { t } from "../i18n";

export interface LookActions {
  /** The pointer rests on a look (null: it left them all): Zeca wears it until it moves. */
  preview(look: string | null): void;
  pick(look: string): void;
  close(): void;
}

/** Each tile perches its Zeca on the same line, so the looks compare at a glance. */
const SCALE = 2;
const W = 30;
const WIRE = 32;

/** The picker, with `current` marked. `perch` is Zeca's canvas, moved here while it is open. */
export function lookPicker(current: string, perch: HTMLElement, on: LookActions): HTMLElement {
  const tile = (value: string, label: string): HTMLElement => {
    const canvas = document.createElement("canvas");
    canvas.width = W * SCALE;
    canvas.height = (WIRE + 2) * SCALE;
    const set = dress(speciesSet(zecaSpecies()), value);
    const ctx = canvas.getContext("2d")!;
    drawFrame(ctx, set, frameAt(set.clips.idle, 0), 4, WIRE - perchOf(set), SCALE);
    const b = el(
      "button",
      {
        class: `look${value === current ? " on" : ""}`,
        onclick: () => on.pick(value),
      },
      canvas,
    );
    const show = () => on.preview(value === "auto" ? null : value);
    b.addEventListener("pointerenter", show);
    b.addEventListener("focus", show);
    b.title = label;
    b.setAttribute("aria-label", label);
    b.setAttribute("aria-pressed", String(value === current));
    return b;
  };
  /** The calendar's choice and none, as words in the head: they have no look of their own. */
  const chip = (value: string, label: string): HTMLElement => {
    const b = el("button", { class: `look-chip${value === current ? " on" : ""}`, text: label, onclick: () => on.pick(value) });
    // Auto previews what he wears now: the calendar's look is the core's to say.
    const show = () => on.preview(value === "auto" ? null : value);
    b.addEventListener("pointerenter", show);
    b.addEventListener("focus", show);
    b.setAttribute("aria-pressed", String(value === current));
    return b;
  };
  const close = el("button", { class: "icon-btn", onclick: () => on.close() }, icon("close", 11));
  close.title = t("Close");
  const [calendar, ...rest] = LOOK_GROUPS;
  const grid = el(
    "div",
    { class: "looks-groups" },
    ...rest.flatMap((g) => [
      el("div", { class: "looks-label", text: t(g.title) }),
      el("div", { class: "looks-row" }, ...g.looks.map((l) => tile(l.value, t(l.label)))),
    ]),
  );
  const body = el(
    "div",
    { class: "looks-body" },
      el(
        "div",
        { class: "looks-head" },
        el("span", { text: t("Zeca's look") }),
        el("div", { class: "looks-chips" }, ...calendar.looks.map((l) => chip(l.value, l.value === "auto" ? t("Auto") : t(l.label))), close),
      ),
      grid,
  );
  body.addEventListener("pointerleave", () => on.preview(null));
  return el("section", { class: "card looks" }, perch, body);
}
