// Which species each bird is. Zeca is the species the user picked; every other session draws one
// from the flock's pool by a hash of its id, so it keeps its bird for as long as it lives. A
// project with three or more sessions crowns its oldest vult king (the king vulture is never
// drawn at random: it stays rare).
// For now the choices come from the build (VITE_ZECA_SPECIES, VITE_FLOCK=all); the settings
// will own them, and the assignment moves to core with a season seed.
import type { SessionView } from "../bridge";
import { BRAZIL, SPECIES, speciesSet, type Rig } from "../character/flock";

const KING = "papa";
const zecaSpecies = (): string =>
  import.meta.env.VITE_ZECA_SPECIES ?? "atratus";
const pool = (): string[] =>
  (import.meta.env.VITE_FLOCK === "all"
    ? SPECIES.map((s) => s.id)
    : BRAZIL
  ).filter((id) => id !== KING);

const key = (s: SessionView) => `${s.agent}:${s.id}`;
function hash(text: string): number {
  let h = 2166136261;
  for (const c of text) h = Math.imul(h ^ c.charCodeAt(0), 16777619) >>> 0;
  return h;
}

let assigned = new Map<string, string>();

/** Decides every session's species; `shown` in arrival order, `front` is Zeca. */
export function assignSpecies(
  shown: SessionView[],
  front: SessionView | null,
): void {
  const common = pool();
  const next = new Map<string, string>();
  for (const s of shown)
    next.set(
      key(s),
      s === front ? zecaSpecies() : common[hash(s.id) % common.length],
    );
  const projects = new Map<string, SessionView[]>();
  for (const s of shown)
    projects.set(s.project, [...(projects.get(s.project) ?? []), s]);
  for (const group of projects.values()) {
    const king = group.length >= 3 ? group.find((s) => s !== front) : undefined;
    if (king) next.set(key(king), KING);
  }
  assigned = next;
}

/** The sprite set of a session's bird, by its key ("agent:id"). */
export const speciesOf = (k: string): Rig =>
  speciesSet(assigned.get(k) ?? zecaSpecies());
/** Zeca's own sprite set, with or without a session in front. */
export const zecaSet = (): Rig => speciesSet(zecaSpecies());
