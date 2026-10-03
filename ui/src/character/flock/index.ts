// A species' sprite set, built once from Zeca's on first use. Zeca's data stays the one source:
// design/mascots/zeca/zeca.py draws him, and every species is a set of transforms over his rig.
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

export { BRAZIL, SPECIES, type Species } from "./species";
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
  built.set(s.id, set);
  return set;
}
