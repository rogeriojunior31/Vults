// Which clip a session's bird plays. States first (they need a human or report an outcome),
// then what the agent is doing right now.
import type { SessionView } from "../bridge";
import { dayPart } from "./daypart";

const BY_STATUS: Partial<Record<SessionView["status"], string>> = {
  approval: "approval",
  question: "question",
  finished: "done",
  failed: "fail",
  ratelimited: "sleep",
  idle: "idle",
  thinking: "think",
};

const BY_ACTIVITY: Record<NonNullable<SessionView["activity"]>, string> = {
  read: "read",
  search: "search",
  edit: "edit",
  run: "run",
  web: "fly",
  plan: "read",
  subagent: "search",
  think: "think",
  work: "run",
};

/** Music is playing: Zeca dances, and the flock's idle birds court on the wire instead of soaring. */
let music = false;
export function setMusic(on: boolean): void {
  music = on;
}
export const musicOn = (): boolean => music;
/** At dawn a resting bird opens its wings to the sun: the spread-wing pose, with no mark over it. */
const sunning = () => dayPart() === "dawn";

/** Zeca's, on his own (the empty wire, the pill). */
export function idleClip(): string {
  return music ? "dance" : sunning() ? "sun" : "idle";
}

export function clipFor(s: SessionView): string {
  if (s.status === "idle") return music ? "court" : sunning() ? "sun" : "idle";
  if (s.status !== "working") return BY_STATUS[s.status] ?? "idle";
  return s.activity ? BY_ACTIVITY[s.activity] : "run";
}

/** The mark over a bird's head: the states worth seeing at a glance. Working needs none. */
const EMOTE: Partial<Record<SessionView["status"], string>> = {
  approval: "alert",
  question: "ask",
  finished: "done",
  failed: "fail",
  ratelimited: "sweat",
  thinking: "think",
};

export function emoteFor(s: SessionView): string | null {
  if ((s.status === "idle" || s.status === "finished") && s.rested === false) return "done";
  if (s.status === "finished" && s.rested === true) return null;
  if (s.status === "idle" && music) return "music";
  return EMOTE[s.status] ?? null;
}
