// The Nest's Roosts tab (plan-zeca M3): the repositories the flock works in, each with its
// branches, its sessions, the GitHub card's rows for it and what Zeca remembers of the project.
import type { RoostView, SharePreview } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";
import { statusWords } from "../../island/views";

const CHECKS = {
  passing: tk("Checks passing"),
  failing: tk("Checks failing"),
  running: tk("Checks running"),
} as const;

/** What the default branch's checks say at the card's top. */
const BRANCH = {
  passing: tk("Passing"),
  failing: tk("Failing"),
  running: tk("Running"),
} as const;

const REVIEW = {
  approved: tk("Approved"),
  changes: tk("Changes requested"),
} as const;

function section(title: string, ...rows: (Node | null)[]): HTMLElement | null {
  return rows.some(Boolean) ? el("div", { class: "roost-part" }, el("h4", { text: title }), ...rows) : null;
}

export interface RoostsBackend {
  load(): Promise<RoostView[]>;
  /** The change that lets Claude Code read the roost's AGENTS.md, to show before writing. */
  preview(key: string): Promise<SharePreview>;
  /** Writes the change to the very file previewed, over the very bytes the diff came from. */
  apply(key: string, file: string, fingerprint: string): Promise<void>;
}

/** Claude Code and Codex reading the same project memory (M9): said, or offered with its diff.
 * `hold` says whether a diff is open, so the list's refreshes leave it alone while it is read. */
function sharing(r: RoostView, backend: RoostsBackend, done: () => void, hold: (on: boolean) => void): HTMLElement | null {
  if (r.agents === "shared") return el("p", { class: "board-empty", text: t("Claude Code and Codex read the same AGENTS.md here.") });
  if (r.agents !== "offer") return null;
  const box = el("div", { class: "roost-share" });
  const failed = (e: unknown) => {
    box.replaceChildren(
      el("p", { class: "memory-note", text: String(e) }),
      el("button", { class: "btn small", text: t("Cancel"), onclick: offer }),
    );
  };
  const offer = () => {
    hold(false);
    box.replaceChildren(
      el("p", { class: "board-empty", text: t("Codex reads this project's AGENTS.md; Claude Code does not.") }),
      el("button", {
        class: "btn small",
        text: t("Share it with Claude…"),
        onclick: () => {
          hold(true);
          void backend.preview(r.key).then(show, failed);
        },
      }),
    );
  };
  // The diff first, then a click: a project's CLAUDE.md is the user's file (rule 3).
  const show = (p: SharePreview) =>
    box.replaceChildren(
      el("p", { class: "board-empty", text: t("This change to {path}, with a dated backup kept by Vults:", { path: p.path }) }),
      el("pre", { class: "roost-diff", text: p.diff }),
      el(
        "div",
        { class: "proposal-actions" },
        el("button", {
          class: "btn primary",
          text: t("Apply"),
          onclick: () => {
            void backend.apply(r.key, p.file, p.fingerprint).then(() => {
              hold(false);
              done();
            }, failed);
          },
        }),
        el("button", { class: "btn", text: t("Cancel"), onclick: offer }),
      ),
    );
  offer();
  return box;
}

function roost(r: RoostView, backend: RoostsBackend, done: () => void, hold: (on: boolean) => void): HTMLElement {
  const checks = r.checks
    ? el("span", { class: `row-tag ${r.checks}`, text: t("Default branch: {checks}", { checks: t(BRANCH[r.checks]) }) })
    : null;
  return el(
    "article",
    { class: "roost" },
    el("header", { class: "roost-head" }, el("span", { class: "roost-name", text: r.name }), checks),
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
    sharing(r, backend, done, hold),
    section(
      t("What Zeca remembers of it"),
      r.memory.length
        ? el("ul", { class: "roost-memory" }, ...r.memory.map((m) => el("li", { text: m })))
        : el("p", { class: "board-empty", text: t("Nothing yet.") }),
    ),
  );
}

/** The tab's body; `load` asks for the roosts again (on opening the tab, and as the flock moves). */
export function createRoosts(backend: RoostsBackend): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "roosts" });
  let asked = 0;
  // One ask at a time: each reads the sessions' repositories, and a slow disk must not pile them up.
  let busy = false;
  let again = false;
  // A diff open in a card: the list is not rebuilt under the user's eyes until it is closed.
  let holding = false;
  const hold = (on: boolean) => {
    holding = on;
  };
  const refresh = () => {
    if (holding) return;
    if (busy) {
      again = true;
      return;
    }
    busy = true;
    const mine = ++asked;
    const done = () => {
      busy = false;
      if (again) {
        again = false;
        refresh();
      }
    };
    void backend.load().then(
      (list) => {
        done();
        // An older answer that came late never replaces a newer one.
        if (mine !== asked) return;
        element.replaceChildren(
          ...(list.length
            ? list.map((r) => roost(r, backend, refresh, hold))
            : [el("p", { class: "board-empty", text: t("No roosts yet: a roost is a repository an agent session works in.") })]),
        );
      },
      done,
    );
  };
  return { element, refresh };
}
