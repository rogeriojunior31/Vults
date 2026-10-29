// Settings → Agents, what each agent can do (plan-zeca W1, moved here by ADR 0021): for each agent Vults knows, what this computer has (its
// CLI and version, its hooks) and what it can do with Vults: whether a card answers it, where its
// questions go, what its diffs show, how a turn could be stopped, its own sandbox, and whether
// Vults can start it. What the router and the policies will lean on, in the open.
import type { AgentRow } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";
import { AGENTS } from "../../agents.gen";

const HOOKS: Record<AgentRow["hooks"], string> = {
  ok: tk("Hooks in"),
  outdated: tk("Hooks out of date: update them in Settings, Agents"),
  missing: tk("No hooks: set them up in Settings, Agents"),
  untrusted: tk("Codex won't run the hooks until you trust them (/hooks in Codex)"),
  error: tk("Its config could not be read"),
  broken: tk("Vults' hook program is missing: reinstall Vults"),
};

const PERMISSIONS: Record<AgentRow["permissions"], string> = {
  card: tk("A card on the island"),
  terminal: tk("Its own terminal"),
};

const QUESTIONS: Record<AgentRow["questions"], string> = {
  card: tk("A card on the island"),
  terminal: tk("Its own terminal"),
};

const DIFFS: Record<AgentRow["diffs"], string> = {
  numbered: tk("The lines changed, with line numbers"),
  lines: tk("The lines changed, without line numbers"),
  none: tk("Not shown"),
};

const STOP: Record<AgentRow["stop"], string> = {
  interrupt: tk("At once, when Vults started it"),
  no: tk("Not from Vults yet"),
};

const SANDBOX: Record<AgentRow["sandbox"], string> = {
  bubblewrap: tk("Its own (bubblewrap), off until you turn it on"),
  codex: tk("Its own (bubblewrap), as its sandbox mode says"),
  container: tk("A container with --sandbox, off until you turn it on"),
  no: tk("None"),
};

function fact(label: string, value: string): HTMLElement {
  return el("div", { class: "row" }, el("div", { class: "row-title", text: label }), el("span", { class: "muted", text: value }));
}

function card(a: AgentRow): HTMLElement {
  const installed = a.path !== null;
  const entry = AGENTS.find(e => e.id === a.id);
  return el(
    "section",
    { class: "card rows" },
    el(
      "div",
      { class: "card-head doctor-head" },
      el("div", { class: "card-title", text: a.name }),
      el("span", {
        class: `badge ${installed ? (a.hooks === "ok" ? "ok" : "warn") : "off"}`,
        text: installed ? (a.version ?? a.path ?? "") : t("Not installed"),
      }),
    ),
    installed ? el("p", { class: "note", text: t(HOOKS[a.hooks]) }) : null,
    fact(t("Watch sessions"), entry?.watch ? t("Yes") : t("Not supported here")),
    fact(t("Permissions"), t(PERMISSIONS[a.permissions])),
    fact(t("Questions"), t(QUESTIONS[a.questions])),
    fact(t("Diffs"), t(DIFFS[a.diffs])),
    fact(t("Stopping a turn"), t(STOP[a.stop])),
    fact(t("Sandbox"), t(SANDBOX[a.sandbox])),
    fact(t("Started from Vults"), a.starts ? t("Yes") : entry?.drive === "planned" ? t("Planned; not enabled here") : t("Not supported here")),
  );
}

/** The list; `load(fresh)` asks the doctor (kept ten minutes unless `fresh`). */
export function createDoctor(load: (fresh: boolean) => Promise<AgentRow[]>): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "doctor" });
  let asked = 0;
  const show = (fresh: boolean) => {
    const mine = ++asked;
    const again = () => el("div", { class: "actions" }, el("button", { class: "btn", text: t("Check again"), onclick: () => show(true) }));
    void load(fresh).then(
      (rows) => {
        if (mine !== asked) return;
        element.replaceChildren(again(), ...rows.map(card));
      },
      () => {
        if (mine !== asked) return;
        element.replaceChildren(again(), el("p", { class: "note", text: t("The check failed") }));
      },
    );
  };
  return { element, refresh: () => show(false) };
}
