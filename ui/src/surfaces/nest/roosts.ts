// The Nest's Activity → By project (plan-zeca M3): the repositories the flock works in, each with its
// branches, its sessions, the GitHub card's rows for it and what Zeca remembers of the project.
import type { AgentKind, RoostView, SharePreview } from "../../bridge";
import type { WeekView } from "../../view.gen";
import { duration } from "../../duration";
import { el } from "../../dom";
import { t, tk } from "../../i18n";
import { statusWords } from "../../island/views";
import { money } from "./budget";
import { viewState } from "./view-state";

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
  query?: (value?: string) => Promise<string>;
  load(): Promise<RoostView[]>;
  /** The change that lets Claude Code read the roost's AGENTS.md, to show before writing. */
  preview(key: string): Promise<SharePreview>;
  /** Writes the change to the very file previewed, over the very bytes the diff came from. */
  apply(key: string, file: string, fingerprint: string): Promise<void>;
  /** A GitHub card's row, by its item: the core opens its link, if it has a safe one. */
  open(item: string): void;
  /** This week's activity, for the time each project took; null when history is off or unread. */
  week(): Promise<WeekView | null>;
  /** Puts a session in front, on the Flock. */
  focus(kind: AgentKind, id: string): void;
  /** Settings → Projects: each project's preferences. */
  preferences(): void;
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
  const item = r.checksItem;
  const checks = r.checks
    ? el(item ? "button" : "span", {
        class: `row-tag ${r.checks}${item ? " link" : ""}`,
        text: t("Default branch: {checks}", { checks: t(BRANCH[r.checks]) }),
        onclick: item ? () => backend.open(item) : undefined,
      })
    : null;
  return el(
    "article",
    { class: "roost" },
    el("header", { class: "roost-head" }, el("span", { class: "roost-name", text: r.name }), checks),
    r.cost
      ? el("p", {
          class: "roost-cost",
          text: r.cost.subscription
            ? t("Cost today {today}, {total} in all, at API prices", { today: money(r.cost.today), total: money(r.cost.total) })
            : t("Cost today {today}, {total} in all", { today: money(r.cost.today), total: money(r.cost.total) }),
        })
      : null,
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
          "button",
          { class: "board-row link", onclick: () => backend.focus(s.kind, s.id) },
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
          p.link ? "button" : "div",
          { class: `board-row${p.link ? " link" : ""}`, onclick: p.link ? () => backend.open(p.item) : undefined },
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

/** A folder's last part: the name the week counts a project by (`~/Projects/Vults-w2` → `Vults-w2`). */
const base = (path: string) => path.replace(/\/+$/, "").split("/").pop() ?? path;

/**
 * This week's time per project, as the prototype's bars draw it, the most first. The week counts a
 * project by its folder's name, so a repository's worktrees add up under it (with how many it
 * has); a folder no roost holds keeps a bar of its own.
 */
function bars(week: WeekView | null, list: RoostView[]): HTMLElement | null {
  const counted = (week?.projects ?? []).filter((p) => p.secs > 0);
  if (!counted.length) return null;
  const rows: { name: string; secs: number }[] = [];
  const taken = new Set<string>();
  for (const r of list) {
    const folders = new Set(r.trees.map((x) => base(x.path).toLowerCase()));
    const mine = counted.filter((p) => folders.has(p.name.toLowerCase()));
    if (!mine.length) continue;
    for (const p of mine) taken.add(p.name);
    const name = r.name.split("/").pop() ?? r.name;
    rows.push({ name: r.trees.length > 1 ? t("{name} · {n} wt", { name, n: r.trees.length }) : name, secs: mine.reduce((n, p) => n + p.secs, 0) });
  }
  for (const p of counted) if (!taken.has(p.name)) rows.push({ name: p.name, secs: p.secs });
  rows.sort((a, b) => b.secs - a.secs);
  const most = rows[0].secs;
  return el(
    "div",
    { class: "activity-card act-bars" },
    el("h3", { text: t("Time with the flock") }),
    ...rows.map((row) => {
      const fill = el("b");
      fill.style.width = `${Math.round((row.secs / Math.max(1, most)) * 100)}%`;
      const line = el("div", { class: "act-bar" }, el("span", { text: row.name }), el("div", {}, fill), el("span", { text: duration(row.secs) }));
      line.title = row.name;
      return line;
    }),
  );
}

/** What the search reads in a roost: its name, folders and branches, and its sessions' agents. */
const said = (r: RoostView) =>
  [r.name, ...r.trees.flatMap((x) => [x.path, x.branch ?? ""]), ...r.sessions.map((s) => s.agent)].join(" ").toLowerCase();

/** The tab's body; `load` asks for the roosts again (on opening the tab, and as the flock moves). */
export function createRoosts(backend: RoostsBackend): { element: HTMLElement; refresh: () => void; flush: () => Promise<void> } {
  const element = el("div", { class: "roosts" });
  const cards = el("div", { class: "roost-list" });
  const feedback = el("div", { class: "roost-feedback" });
  let roosts: RoostView[] = [];
  let loaded = false;
  let failed = false;
  let failure: string | null = null;
  let lastWeek: WeekView | null = null;
  let weekAt = 0;
  // The prototype's search: a project, a branch or an agent. Built once and never moved, so the
  // flock's refreshes (every second or so while agents work) never take the focus from it.
  const search = el("input", { class: "project-search" });
  search.type = "search";
  search.maxLength = 4096;
  let edited = false;
  let saved: Promise<void> = Promise.resolve();
  search.placeholder = t("Find a project, branch or agent…");
  search.setAttribute("aria-label", t("Find a project, branch or agent…"));
  const count = el("span");
  const title = el("h2");
  const about = el("p");
  const note = el("span");
  const preferences = el("button", { class: "link-btn", onclick: () => backend.preferences() });
  const retry = el("button", { onclick: () => { if (!busy) refresh(); } });
  title.tabIndex = -1;
  const barsHost = el("div");
  element.append(
    el("header", { class: "activity-heading" }, el("div", {}, title, about, count)),
    barsHost,
    search,
    feedback,
    cards,
    el(
      "p",
      { class: "small-note" },
      note,
      preferences,
    ),
  );
  const paint = () => {
    const returning = document.activeElement === retry;
    title.textContent = t("By project");
    about.textContent = t("Repositories, worktrees and the sessions working in each one.");
    note.textContent = t("Each project's bird, and whether it is muted, pinned or hidden, are its preferences.");
    preferences.textContent = t("Preferences ↗");
    search.placeholder = t("Find a project, branch or agent…");
    search.setAttribute("aria-label", search.placeholder);
    element.setAttribute("aria-busy", String(busy));
    feedback.replaceChildren();
    if (failed) {
      retry.textContent = busy ? t("Loading") : t("Try again");
      retry.setAttribute("aria-busy", String(busy));
      feedback.append(viewState("error", t("Could not load {name}.", { name: t("By project") }),
        loaded ? t("The last loaded view is kept and may be out of date.") : t("Try again to read the data saved on this computer."), retry, failure));
    } else if (!loaded && busy) feedback.append(viewState("loading", t("Loading {name}…", { name: t("By project") })));
    const q = search.value.trim().toLowerCase();
    const shown = q ? roosts.filter((r) => said(r).includes(q)) : roosts;
    const trees = roosts.reduce((n, r) => n + r.trees.length, 0);
    count.textContent = roosts.length
      ? `${roosts.length === 1 ? t("1 project") : t("{n} projects", { n: roosts.length })} · ${trees === 1 ? t("1 worktree") : t("{n} worktrees", { n: trees })}`
      : "";
    barsHost.replaceChildren(...[bars(lastWeek, roosts)].filter((b): b is HTMLElement => b !== null));
    search.hidden = !roosts.length && !search.value;
    cards.replaceChildren(
      ...(!loaded ? [] : roosts.length
        ? shown.length
          ? shown.map((r) => roost(r, backend, refresh, hold))
          : [
              viewState("empty", t("No project matches."), undefined,
                el("button", {
                  class: "outline",
                  text: t("Clear filters"),
                  onclick: () => {
                    search.value = "";
                    change();
                    search.focus();
                  },
                }),
              ),
            ]
        : [viewState("empty", t("No projects yet"), t("No roosts yet: a roost is a repository an agent session works in."))]),
    );
    if (returning) (failed ? retry : search.hidden ? title : search).focus({ preventScroll: true });
  };
  const change = () => {
    edited = true;
    paint();
    const value = search.value;
    if (backend.query) saved = saved.catch(() => {}).then(() => backend.query?.(value)).then(() => {});
    void saved.catch(() => {});
  };
  search.addEventListener("input", change);
  const restored = backend.query?.().then(value => {
    if (!edited) { search.value = value; paint(); }
  }).catch(() => {});
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
    if (!loaded || failed) paint();
    const mine = ++asked;
    const done = () => {
      busy = false;
      if (again) {
        again = false;
        refresh();
      }
    };
    // The week reads the whole history: once a minute is plenty, while the roosts follow the flock.
    const fresh = Date.now() - weekAt < 60_000;
    void Promise.all([backend.load(), fresh ? Promise.resolve(lastWeek) : backend.week().catch(() => null)]).then(
      ([list, week]) => {
        done();
        // An older answer that came late never replaces a newer one.
        if (mine !== asked) return;
        roosts = list;
        loaded = true;
        failed = false;
        if (!fresh) weekAt = Date.now();
        lastWeek = week;
        paint();
      },
      (error) => {
        done();
        if (mine !== asked) return;
        failed = true;
        failure = String(error);
        paint();
      },
    );
  };
  return { element, refresh, flush: async () => {
    await restored;
    saved = saved.catch(async () => { if (backend.query) await backend.query(search.value); });
    await saved;
  } };
}
