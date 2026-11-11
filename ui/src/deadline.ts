// A waiting request's clock, the same on the island and in the Nest: both count to the core's
// `expires_at`, so the two never disagree by a second.
import { t } from "./i18n";

/** The clock turns amber in the last half minute and red in the last ten seconds. */
export const DEADLINE_SOON_S = 30;
export const DEADLINE_URGENT_S = 10;

/** Whole seconds left until `expiresAt` (Unix seconds), never negative. */
export function secondsLeft(expiresAt: number, nowMs = Date.now()): number {
  return Math.max(0, Math.ceil(expiresAt - nowMs / 1000));
}

/** "Goes back to the terminal in 1:39". */
export function deadlineText(left: number): string {
  return t("Goes back to the terminal in {time}", { time: `${Math.floor(left / 60)}:${String(left % 60).padStart(2, "0")}` });
}

/** Paints a clock node: its text, and `soon` / `urgent` for the colour. */
export function paintClock(node: HTMLElement, left: number): void {
  const text = deadlineText(left);
  if (node.textContent !== text) node.textContent = text;
  node.classList.toggle("soon", left <= DEADLINE_SOON_S);
  node.classList.toggle("urgent", left <= DEADLINE_URGENT_S);
}
