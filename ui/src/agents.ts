// The agents' names, from the registry's twin (`agents.gen.ts`): a new agent is a row there.
import type { AgentKind } from "./view.gen";
import { AGENTS } from "./agents.gen";

export { AGENTS };
export type { AgentEntry, Engine, InstallAgent } from "./agents.gen";

/** Each agent's name by its id, the retired ones too: old records still name them. */
export const NAME_BY_ID: Readonly<Record<string, string>> = Object.fromEntries(AGENTS.map((a) => [a.id, a.name]));

/** The name of each kind that travels under its own name on the wire. */
export const NAME_BY_KIND = Object.fromEntries(
  AGENTS.filter((a) => a.kind !== "other").map((a) => [a.kind, a.name]),
) as Readonly<Record<Exclude<AgentKind, "other">, string>>;
