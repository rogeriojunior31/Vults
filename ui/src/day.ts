import { locale, t } from "./i18n";

/** A day as a heading: Today, Yesterday, or its date. */
export function day(ms: number): string {
  const date = new Date(ms);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return t("Today");
  if (date.toDateString() === yesterday.toDateString()) return t("Yesterday");
  return date.toLocaleDateString(locale(), date.getFullYear() === today.getFullYear()
    ? { month: "long", day: "numeric" } : { year: "numeric", month: "long", day: "numeric" });
}
