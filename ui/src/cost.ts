// Shared cost figures and usage thresholds, independent of the surface.
import type { SpendToday } from "./bridge";
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
export function spentLine(today: SpendToday): string {
  const spent = money(today.usd);
  let line =
    today.budget !== null
      ? t("Today the agents cost {spent} of your {budget}.", { spent, budget: money(today.budget) })
      : t("Today the agents cost {spent}.", { spent });
  if (today.subscription) line += " " + t("On a subscription this is at API prices: your plan does not charge it.");
  return line;
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
