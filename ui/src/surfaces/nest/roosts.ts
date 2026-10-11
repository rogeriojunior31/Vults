// The Nest's Roosts tab (plan-zeca M3): the repositories the flock works in, each with its
// branches, its sessions, the GitHub card's rows for it and what Zeca remembers of the project.
import type { RoostView } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";
import { statusWords } from "../../island/views";

const CHECKS = {
  passing: tk("Checks passing"),
  failing: tk("Checks failing"),
  running: tk("Checks running"),
} as const;

const REVIEW = {
  approved: tk("Approved"),
  changes: tk("Changes requested"),
} as const;

function section(title: string, ...rows: (Node | null)[]): HTMLElement | null {
  return rows.some(Boolean) ? el("div", { class: "roost-part" }, el("h4", { text: title }), ...rows) : null;
}

function roost(r: RoostView): HTMLElement {
  const checks = r.checks
    ? el("span", { class: `row-tag ${r.checks}`, text: t("Default branch: {checks}", { checks: t(CHECKS[r.checks]) }) })
    : null;
  return el(
    "article",
    { class: "roost" },
    el("header", { class: "roost-head" }, el("span", { class: "roost-name", text: r.key }), checks),
    ...r.trees.map((tree) =>
      el(
        "div",
        { class: "roost-tree" },
        el("span", { class: "roost-path", text: tree.path }),
        el("span", { class: "roost-branch", text: tree.branch ?? t("detached") }),
      ),
    ),
    section(
      t("Sessions"),
      ...r.sessions.map((s) =>
        el(
          "div",
          { class: "board-row" },
          el("span", { class: `dot ${s.status}` }),
          el("span", { class: "row-name", text: s.agent }),
          el("span", { class: "row-title", text: s.status === "working" && s.step ? s.step : statusWords(s.status) }),
        ),
      ),
    ),
    section(
      t("Your pull requests"),
      ...r.prs.map((p) => {
        const dot = el("span", { class: `dot ${p.checks ?? "none"}` });
        if (p.checks) dot.title = t(CHECKS[p.checks]);
        return el(
          "div",
          { class: "board-row" },
          dot,
          el("span", { class: "row-name", text: p.name }),
          el("span", { class: "row-title", text: p.title }),
          p.review ? el("span", { class: `row-tag ${p.review}`, text: t(REVIEW[p.review]) }) : null,
        );
      }),
    ),
    section(
      t("What Zeca remembers of it"),
      r.memory.length
        ? el("ul", { class: "roost-memory" }, ...r.memory.map((m) => el("li", { text: m })))
        : el("p", { class: "board-empty", text: t("Nothing yet.") }),
    ),
  );
}

/** The tab's body; `load` asks for the roosts again (on opening the tab, and as the flock moves). */
export function createRoosts(load: () => Promise<RoostView[]>): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "roosts" });
  let asked = 0;
  const refresh = () => {
    const mine = ++asked;
    void load().then(
      (list) => {
        // An older answer that came late never replaces a newer one.
        if (mine !== asked) return;
        element.replaceChildren(
          ...(list.length
            ? list.map(roost)
            : [el("p", { class: "board-empty", text: t("No roosts yet: a roost is a repository an agent session works in.") })]),
        );
      },
      () => {},
    );
  };
  return { element, refresh };
}
