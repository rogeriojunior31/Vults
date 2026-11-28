// Shared cost figures and usage thresholds, independent of the surface.
import type { SpendToday, UsageWindow } from "./bridge";
import { t } from "./i18n";

/** A cost as a card says it: $1.25, or under a cent. */
export function money(usd: number): string {
  if (usd === 0) return "$0.00";
  return usd < 0.01 ? "< $0.01" : `$${usd.toFixed(2)}`;
}

export type UsageLevel = "normal" | "warning" | "critical";

/** The same usage warning thresholds everywhere Vults shows a plan. */
export function usageLevel(percent: number): UsageLevel {
  if (percent >= 90) return "critical";
  if (percent >= 70) return "warning";
  return "normal";
}

export function usageLabel(percent: number): string {
  const level = usageLevel(percent);
  return level === "critical"
    ? t("Near limit")
    : level === "warning"
      ? t("Warning")
      : t("Normal");
}

/** Today's recorded spend; subscriptions are priced at API rates, not charged again. */
export function spentLine(today: SpendToday, plan = true): string {
  const spent = money(today.usd);
  let line =
    today.budget !== null
      ? t("Today the agents cost {spent} of your {budget}.", { spent, budget: money(today.budget) })
      : t("Today the agents cost {spent}.", { spent });
  if (plan && today.subscription) line += " " + t("On a subscription this is at API prices: your plan does not charge it.");
  return line;
}

/** A plan window as its row names it: its length, or what it counts when it is not the whole plan. */
export function usageWindowLabel(w: Pick<UsageWindow, "minutes" | "scope">): string {
  if (w.scope === "spend") return t("Monthly spend");
  if (w.scope === "credits") return t("Monthly credit limit");
  if (w.scope) return t("{model}, weekly", { model: w.scope });
  return usageWindowName(w.minutes);
}

/** A plan window's length, in words. */
export function usageWindowName(minutes: number): string {
  if (minutes === 300) return t("5 hours");
  if (minutes === 10080) return t("A week");
  if (minutes === 1440) return t("1 day");
  if (minutes === 60) return t("1 hour");
  if (minutes % 1440 === 0) return t("{n} days", { n: minutes / 1440 });
  if (minutes % 60 === 0) return t("{n} hours", { n: minutes / 60 });
  return t("{n} minutes", { n: minutes });
}

/** Tokens as t3code writes them: 950, 1.25K, 12.5K, 125K, 1.2M; the unit is chosen after rounding,
 *  so 999,999 reads 1M, never 1000K. */
export function tokens(n: number): string {
  const whole = Math.round(n);
  if (Math.abs(whole) < 1000) return String(whole);
  const trim = (v: number) => v.toFixed(Math.abs(v) >= 100 ? 0 : Math.abs(v) >= 10 ? 1 : 2).replace(/(\.\d*?)0+$/, "$1").replace(/\.$/, "");
  const units = ["K", "M", "B", "T"];
  let scaled = n / 1000, unit = 0;
  while (Math.abs(Number(trim(scaled))) >= 1000 && unit < units.length - 1) { scaled /= 1000; unit += 1; }
  return `${trim(scaled)}${units[unit]}`;
}

export type Pace = "ahead" | "on" | "under";

/** How a plan window is being spent against the time it has run: more than 5 points over the time
 *  gone is ahead, under it by 5 is under. None without a reset time. */
export function pace(w: Pick<UsageWindow, "minutes" | "used_percent" | "resets_at">, nowSec = Date.now() / 1000): Pace | null {
  if (w.resets_at === null || w.minutes <= 0) return null;
  const length = w.minutes * 60;
  const elapsed = Math.max(0, Math.min(1, (length - (w.resets_at - nowSec)) / length));
  const gap = w.used_percent - elapsed * 100;
  return gap > 5 ? "ahead" : gap < -5 ? "under" : "on";
}

export function paceLabel(p: Pace): string {
  return p === "ahead" ? t("Ahead of pace") : p === "under" ? t("Under pace") : t("On pace");
}
