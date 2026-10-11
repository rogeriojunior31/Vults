// The Nest's Agents tab (plan-zeca W1): for each agent Vults knows, what this computer has (its
// CLI and version, its hooks) and what it can do with Vults: whether a card answers it, where its
// questions go, what its diffs show, how a turn could be stopped, its own sandbox, and whether
// Vults can start it. What the router and the policies will lean on, in the open.
import type { AgentRow } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";

const HOOKS: Record<AgentRow["hooks"], string> = {
  ok: tk("Hooks in"),
  outdated: tk("Hooks out of date: update them in Settings, Agents"),
  missing: tk("No hooks: set them up in Settings, Agents"),
  untrusted: tk("Codex won't run the hooks until you trust them (/hooks in Codex)"),
  error: tk("Its config could not be read"),
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
  "before-next-tool": tk("Before its next tool"),
  interrupt: tk("At once, when Vults started it"),
  no: tk("No way yet"),
};

const SANDBOX: Record<AgentRow["sandbox"], string> = {
  bubblewrap: tk("Its own (bubblewrap)"),
  codex: tk("Its own (writes only in its folder)"),
  container: tk("A container, with --sandbox"),
  no: tk("None"),
};

function fact(label: string, value: string): HTMLElement {
  return el("div", { class: "agent-fact" }, el("span", { class: "agent-label", text: label }), el("span", { text: value }));
}

function card(a: AgentRow): HTMLElement {
  const installed = a.path !== null;
  return el(
    "article",
    { class: `agent${installed ? "" : " absent"}` },
    el(
      "header",
      { class: "roost-head" },
      el("span", { class: "roost-name", text: a.name }),
      el("span", {
        class: `row-tag ${installed ? (a.hooks === "ok" ? "passing" : "running") : ""}`,
        text: installed ? (a.version ?? a.path ?? "") : t("Not installed"),
      }),
    ),
    installed ? el("p", { class: "board-empty", text: t(HOOKS[a.hooks]) }) : null,
    fact(t("Permissions"), t(PERMISSIONS[a.permissions])),
    fact(t("Questions"), t(QUESTIONS[a.questions])),
    fact(t("Diffs"), t(DIFFS[a.diffs])),
    fact(t("Stopping a turn"), t(STOP[a.stop])),
    fact(t("Sandbox"), t(SANDBOX[a.sandbox])),
    fact(t("Started from Vults"), a.starts ? t("Yes") : t("Not yet")),
  );
}

/** The tab; `load(fresh)` asks the doctor (kept ten minutes unless `fresh`). */
export function createAgents(load: (fresh: boolean) => Promise<AgentRow[]>): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "agents" });
  let asked = 0;
  const show = (fresh: boolean) => {
    const mine = ++asked;
    void load(fresh).then(
      (rows) => {
        if (mine !== asked) return;
        element.replaceChildren(
          el("div", { class: "proposal-actions" }, el("button", { class: "btn small", text: t("Check again"), onclick: () => show(true) })),
          ...rows.map(card),
        );
      },
      () => {},
    );
  };
  return { element, refresh: () => show(false) };
}
