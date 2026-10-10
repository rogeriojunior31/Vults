// A project's bird, picked on the island: a session's quick action opens this in place of its card.
// Every species but the king's (a role, not a breed), and Automatic for the pool's draw; a click
// picks it for every session of the project, now and later. No Tauri here: `on` says what to do.
import type { SessionView } from "../bridge";
import { speciesSet } from "../character/flock";
import { SPECIES } from "../character/flock/species";
import { drawFrame, frameAt } from "../character/sprites";
import { perchOf } from "../character/zeca";
import { el } from "../dom";
import { icon } from "./icons";

export interface BirdActions {
  /** A species for the project, or null to go back to the pool's draw. */
  pick(s: SessionView, species: string | null): void;
  close(): void;
}

/** The king vulture comes by role (crates/core `flock::KING`): never a project's breed. */
const KING = "papa";
/** Each tile perches its bird on the same line, so the sizes compare at a glance. */
const SCALE = 2;
const W = 44;
const WIRE = 36;

const GROUPS = [
  { family: "new-world", title: "The Americas" },
  { family: "old-world", title: "Africa, Europe and Asia" },
] as const;

/** The picker for `s`'s project, its current bird marked when the user chose it. `perch` is the
 *  session's canvas, moved here while it is open: it shows the pick at once. */
export function birdCard(s: SessionView, perch: HTMLElement, on: BirdActions): HTMLElement {
  perch.className = `perch ${s.status} ${s.agent}`;
  const tile = (id: string, name: string): HTMLElement => {
    const canvas = document.createElement("canvas");
    canvas.width = W * SCALE;
    canvas.height = (WIRE + 4) * SCALE;
    const set = speciesSet(id);
    drawFrame(canvas.getContext("2d")!, set, frameAt(set.clips.idle, 0), 10, WIRE - perchOf(set), SCALE);
    const marked = s.bird_chosen && s.species === id;
    const b = el("button", { class: `look bird${marked ? " on" : ""}`, onclick: () => on.pick(s, id) }, canvas);
    b.title = name;
    b.setAttribute("aria-label", name);
    b.setAttribute("aria-pressed", String(marked));
    return b;
  };
  const auto = el("button", { class: `look-chip${s.bird_chosen ? "" : " on"}`, text: "Automatic", onclick: () => on.pick(s, null) });
  auto.title = "The flock's own draw, the same every time the app starts";
  auto.setAttribute("aria-pressed", String(!s.bird_chosen));
  const close = el("button", { class: "icon-btn", onclick: () => on.close() }, icon("close", 11));
  close.title = "Close (Esc)";
  const groups = GROUPS.flatMap((g) => [
    el("div", { class: "looks-label", text: g.title }),
    el("div", { class: "looks-row" }, ...SPECIES.filter((x) => x.family === g.family && x.id !== KING).map((x) => tile(x.id, x.name))),
  ]);
  return el(
    "section",
    { class: "card looks birds" },
    perch,
    el(
      "div",
      { class: "looks-body" },
      el(
        "div",
        { class: "looks-head" },
        el("span", { text: `${s.project || "This project"}'s bird` }),
        el("div", { class: "looks-chips" }, auto, close),
      ),
      el("div", { class: "looks-groups" }, ...groups),
    ),
  );
}
