// Which species each bird is. The core draws every session's (`SessionView.species`: Brazil's
// vultures by the session id and the app's season, the king vulture by role); Zeca keeps his own.
// His is a build-time choice for now (VITE_ZECA_SPECIES), until the settings own it.
import type { SessionView } from "../bridge";
import { speciesSet, type Rig } from "../character/flock";

const zecaSpecies = (): string => import.meta.env.VITE_ZECA_SPECIES ?? "atratus";

const key = (s: SessionView) => `${s.agent}:${s.id}`;
/** FNV-1a: a session id to a stable number (its place in the thermal). */
export function hash(text: string): number {
  let h = 2166136261;
  for (const c of text) h = Math.imul(h ^ c.charCodeAt(0), 16777619) >>> 0;
  return h;
}

let assigned = new Map<string, string>();

/** Every session's species for this render; `front` is Zeca. */
export function assignSpecies(shown: SessionView[], front: SessionView | null): void {
  assigned = new Map(shown.map((s) => [key(s), s === front ? zecaSpecies() : s.species]));
}

/** The sprite set of a session's bird, by its key ("agent:id"). */
export const speciesOf = (k: string): Rig => speciesSet(assigned.get(k) ?? zecaSpecies());
/** Zeca's own sprite set, with or without a session in front. */
export const zecaSet = (): Rig => speciesSet(zecaSpecies());
