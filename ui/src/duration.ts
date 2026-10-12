// A length of time as people read it, for every surface (the island's Activity, Settings, the Nest).
import { t } from "./i18n";

/** `6 h 20 min`, as the core's `i18n::duration`. */
export function duration(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (!h && !m) return t("under a minute");
  if (!h) return t("{m} min", { m });
  return m ? t("{h} h {m} min", { h, m }) : t("{h} h", { h });
}
