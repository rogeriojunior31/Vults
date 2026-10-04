// A species' sprite set, built once from Zeca's on first use. Zeca's data stays the one source:
// design/mascots/zeca/zeca.py draws him, and every species is a set of transforms over his rig.
import type { Frame, SpriteSet } from "../sprites";
import { ZECA } from "../zeca";
import {
  TALL,
  neck,
  resize,
  tallBody,
  tallLegs,
  vParts,
  type Rig,
} from "./rig";
import { SPECIES, sizeOf, type Species } from "./species";

export { SPECIES } from "./species";
export type { Rig } from "./rig";

const built = new Map<string, Rig>();

export function species(id: string): Species {
  return SPECIES.find((s) => s.id === id) ?? SPECIES[0];
}

/** The sprite set of a species (Zeca's black vulture for an unknown id). */
export function speciesSet(id: string): Rig {
  const s = species(id);
  let set = built.get(s.id);
  if (set) return set;
  set = { ...(JSON.parse(JSON.stringify(ZECA)) as typeof ZECA), perch: 20 };
  Object.assign(set.palette, s.palette);
  if (s.palette.b && !s.palette.i)
    set.palette.i = toward(s.palette.b, 255, 0.3);
  if (s.palette.b && !s.palette.d) set.palette.d = toward(s.palette.b, 0, 0.42);
  if (s.tall) tallBody(set);
  s.build?.(set);
  set.clips.signature = s.signature.clip(set);
  let size = sizeOf(s);
  // The tall body already carries TALL rows of the height.
  if (s.tall) {
    tallLegs(set);
    size = { ...size, rows: (size.rows ?? 0) - TALL };
  }
  if (s.neck) neck(set, ...s.neck);
  resize(set, size);
  if (s.v) vParts(set, ...s.v);
  // A finished bird celebrates, then does its own thing before it rests: the signature follows
  // the done clip. Signatures that happen in the air stay out of a perched clip.
  if (perchedSignature(set)) {
    const done = set.clips.done.frames;
    set.clips.done = {
      loop: false,
      frames: [...done, ...set.clips.signature.frames, done[done.length - 1]],
    };
  }
  built.set(s.id, set);
  return set;
}

// The lit edge and the wing's shadow come from the body's color: a species without them would
// draw Zeca's grey on a brown bird.
function toward(hex: string, target: number, t: number): string {
  const channel = (i: number) =>
    Math.round(parseInt(hex.slice(i, i + 2), 16) * (1 - t) + target * t);
  return (
    "#" +
    [1, 3, 5].map((i) => channel(i).toString(16).padStart(2, "0")).join("")
  );
}

const onPerch = new WeakMap<SpriteSet, boolean>();
/** Whether a set's signature stays on the perch from start to end, so it can play there. */
export function perchedSignature(set: SpriteSet): boolean {
  let on = onPerch.get(set);
  if (on === undefined) {
    const frames = set.clips.signature?.frames ?? [];
    // On the perch: the folded body, or the sunning pose (wings spread, facing you).
    const perched = (f: Frame) =>
      f.layers.some(([p]) => p.startsWith("body") || p === "sunning");
    onPerch.set(set, (on = frames.length > 0 && frames.every(perched)));
  }
  return on;
}
