// Which clip a session's bird plays. States first (they need a human or report an outcome),
// then what the agent is doing right now.
import type { SessionView } from "../bridge";

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

export function clipFor(s: SessionView): string {
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
  return EMOTE[s.status] ?? null;
}
