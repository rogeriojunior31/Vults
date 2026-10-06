// Which species each bird is. The core draws every session's (`SessionView.species`: from the
// pool the user chose, by the session id and the app's season; the king vulture by role). Zeca
// keeps the species the user picked for him in the settings, and wears the look of the day.
import type { SessionView } from "../bridge";
import { speciesSet, type Rig } from "../character/flock";
import { dress } from "../character/looks";

let zeca = "atratus";
export const zecaSpecies = (): string => zeca;
/** The species picked for Zeca (the settings, at start and on every change). */
export function setZecaSpecies(id: string): void {
  zeca = id;
}

let on = true;
/** Zeca on or off (Settings → Flock; ADR 0010). Off, the session in front keeps its own bird and
 *  an empty wire stays empty. */
export function setZeca(shown: boolean): void {
  on = shown;
}
export const zecaShown = (): boolean => on;

let look: string | null = null;
/** What Zeca wears (the view's `look`: the core picks it by the date and the settings). */
export function setZecaLook(id: string | null): void {
  look = id;
}
export const zecaLook = (): string | null => look;

const key = (s: SessionView) => `${s.agent}:${s.id}`;
/** FNV-1a: a session id to a stable number (its place in the thermal). */
export function hash(text: string): number {
  let h = 2166136261;
  for (const c of text) h = Math.imul(h ^ c.charCodeAt(0), 16777619) >>> 0;
  return h;
}

let assigned = new Map<string, string>();
/** The key of the session in front: its bird is Zeca. */
let frontKey: string | null = null;

/** Every session's species for this render; `front` is Zeca. */
export function assignSpecies(shown: SessionView[], front: SessionView | null): void {
  assigned = new Map(shown.map((s) => [key(s), s.species]));
  frontKey = front && key(front);
}

/** Zeca's own sprite set, with or without a session in front. */
export const zecaSet = (): Rig => dress(speciesSet(zecaSpecies()), look);
/** The sprite set of a session's bird, by its key ("agent:id"). Only Zeca wears a look. */
export const speciesOf = (k: string): Rig =>
  k === frontKey && on ? zecaSet() : speciesSet(assigned.get(k) ?? zecaSpecies());
