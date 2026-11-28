// One sentence and one dot for how each engine and API provider stands, so every list says it the
// same way (t3code's provider summary). Each headline is a whole sentence of its own.
import { el } from "../../dom";
import { t } from "../../i18n";
import type { ApiProvider, ChatEngines } from "../../bridge";

export type Dot = "ok" | "warn" | "off" | "unknown";

export interface Summary {
  dot: Dot;
  headline: string;
}

/** A CLI engine (Claude Code, Codex) as `chat_engines` reports it. */
export function engineSummary(state: ChatEngines["claude"]): Summary {
  if (state === "signed-in") return { dot: "ok", headline: t("Signed in and ready.") };
  if (state === "installed") return { dot: "warn", headline: t("Installed, not signed in.") };
  return { dot: "off", headline: t("Not installed on this computer.") };
}

/** An installed ACP agent: it says nothing of its login before a turn. */
export function acpSummary(): Summary {
  return { dot: "unknown", headline: t("Installed. Sign-in is checked in its own app.") };
}

/** An API provider: its key, or the local model it needs. */
export function apiSummary(p: ApiProvider): Summary {
  if (p.local) return { dot: "ok", headline: t("Runs on this computer; no key needed.") };
  return p.hasKey ? { dot: "ok", headline: t("Key saved in the system keyring.") } : { dot: "off", headline: t("No key yet.") };
}

export function dot(kind: Dot): HTMLElement {
  const d = el("span", { class: `status-dot ${kind}` });
  d.setAttribute("aria-hidden", "true");
  return d;
}
