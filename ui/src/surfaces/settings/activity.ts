// Settings → Activity (docs/guide/activity.md): a week's recap, the year's grid, and the local
// history's switch. Counts only, read from this computer. No Tauri here: `on` says what to do, so the
// lab can draw it with made-up weeks.
import type { AgentKind, GridDay, WeekView } from "../../view.gen";
import { el } from "../../dom";
import { button, row, toggle } from "./pieces";

export interface Activity {
  history: boolean;
  today: string;
  /** Mondays to page through, newest first. */
  weeks: string[];
  week: WeekView;
  grid: GridDay[];
}

/** The *GitHub* tab: the user's contribution calendar, only with the GitHub connector on. */
export type GithubGrid =
  | { state: "off" }
  | { state: "ready"; total: number; days: { day: string; count: number; level: number }[] }
  | { state: "error"; message: string };

export type GridTab = "agents" | "github";

/** What the page holds besides the history: the grid's tab, and GitHub's calendar once asked. */
export interface ActivityView {
  tab: GridTab;
  /** Null while it is being asked for. */
  github: GithubGrid | null;
  /** The user asked to clear and has not said yes yet. */
  confirming: boolean;
  /** The image leaves the project names out. */
  hideProjects: boolean;
  /** Where the image went (`~/Pictures/…`), or why it did not; null before any. */
  saved: { ok: boolean; text: string } | null;
}

export interface ActivityActions {
  week(monday: string): void;
  tab(tab: GridTab): void;
  /** Settings → Connectors, to turn GitHub on. */
  connectors(): void;
  hideProjects(on: boolean): void;
  /** Draws the week as a PNG and asks where to save it. */
  saveImage(): void;
  history(on: boolean): Promise<void>;
  /** Asks first (`confirming`), then clears. */
  confirmClear(on: boolean): void;
  clear(): Promise<void>;
}

const AGENT: Record<AgentKind, string> = {
  claude: "Claude Code",
  codex: "Codex",
  gemini: "Gemini CLI",
  opencode: "OpenCode",
  qwen: "Qwen Code",
  other: "Another tool",
};

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/** `6 h 20 min`, as the core's `i18n::duration`. */
export function duration(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (!h && !m) return "under a minute";
  if (!h) return `${m} min`;
  return m ? `${h} h ${m} min` : `${h} h`;
}

/** A `2026-10-09` day, as a local date at noon (no time zone can move it to another day). */
const date = (day: string): Date => new Date(`${day}T12:00:00`);
const short = (day: string) => date(day).toLocaleDateString("en", { month: "short", day: "numeric" });
const weekday = (day: string) => date(day).toLocaleDateString("en", { weekday: "long" });

/** `Oct 5 – 11, 2026`, or across a month: `Sep 28 – Oct 4, 2026`. */
export function weekLabel(monday: string, sunday: string): string {
  const [a, b] = [date(monday), date(sunday)];
  const end = a.getMonth() === b.getMonth() ? String(b.getDate()) : short(sunday);
  return `${short(monday)} – ${end}, ${b.getFullYear()}`;
}

function stat(value: string, label: string, sub?: string): HTMLElement {
  return el(
    "div",
    { class: "stat" },
    el("div", { class: "stat-value", text: value }),
    el("div", { class: "stat-label", text: sub ? `${label} · ${sub}` : label }),
  );
}

function weekCard(a: Activity, view: ActivityView, on: ActivityActions): HTMLElement {
  const w = a.week;
  const i = a.weeks.indexOf(w.monday);
  const older = i >= 0 && i < a.weeks.length - 1 ? a.weeks[i + 1] : null;
  const newer = i > 0 ? a.weeks[i - 1] : null;
  const nav = (text: string, label: string, to: string | null) => {
    const b = el("button", { class: "btn week-nav", text, onclick: () => to && on.week(to) });
    b.title = label;
    b.setAttribute("aria-label", label);
    if (!to) b.setAttribute("disabled", "");
    return b;
  };
  const head = el(
    "div",
    { class: "card-head" },
    el("div", { class: "card-title", text: i === 0 ? `This week · ${weekLabel(w.monday, w.sunday)}` : weekLabel(w.monday, w.sunday) }),
    el("div", { class: "week-navs" }, nav("‹", "Older week", older), nav("›", "Newer week", newer)),
  );
  if (!w.turns) return el("section", { class: "card week" }, head, el("p", { class: "note", text: w.headline }));
  const most = [
    w.top_agent ? `Most turns: ${AGENT[w.top_agent]}${w.top_project ? ` on ${w.top_project}` : ""}` : null,
    w.busiest_day ? `Busiest: ${weekday(w.busiest_day)}` : null,
    `Longest turn: ${duration(w.longest_secs)}`,
  ].filter((x): x is string => !!x);
  const top = Math.max(1, ...w.days);
  const bars = el(
    "div",
    { class: "week-bars" },
    ...w.days.map((secs, d) => {
      const fill = el("div", { class: "week-bar-fill" });
      fill.style.height = `${Math.round((secs / top) * 100)}%`;
      const bar = el("div", { class: "week-bar" }, el("div", { class: "week-bar-track" }, fill), el("span", { text: WEEKDAYS[d] }));
      bar.title = `${WEEKDAYS[d]}: ${duration(secs)}`;
      return bar;
    }),
  );
  return el(
    "section",
    { class: "card week" },
    head,
    el("p", { class: "week-headline", text: w.headline }),
    el(
      "div",
      { class: "stats" },
      stat(duration(w.active_secs), "with your agents"),
      stat(String(w.turns), w.turns === 1 ? "turn" : "turns", `${w.steps} steps`),
      stat(`+${w.added} −${w.removed}`, "lines", `${w.files} ${w.files === 1 ? "file" : "files"}`),
      stat(String(w.commands), w.commands === 1 ? "command" : "commands"),
      stat(String(w.allowed + w.denied + w.answered), "answers", `${w.allowed} allowed, ${w.denied} denied`),
      stat(String(w.failed), w.failed === 1 ? "turn failed" : "turns failed"),
    ),
    bars,
    el("p", { class: "row-about week-most", text: most.join(" · ") }),
    el(
      "div",
      { class: "actions week-share" },
      view.saved ? el("span", { class: `note${view.saved.ok ? " ok" : " error"}`, text: view.saved.text }) : null,
      el("label", { class: "project-choice" }, toggle(view.hideProjects, async (hide) => on.hideProjects(hide)), el("span", { text: "Hide project names" })),
      button("Save as image…", () => on.saveImage()),
    ),
  );
}

function cellsOf(days: { day: string; level: number; title: string }[]): HTMLElement {
  return el(
    "div",
    { class: "activity-grid" },
    ...days.map((d) => {
      const c = el("div", { class: `cell l${d.level}` });
      c.title = d.title;
      return c;
    }),
  );
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

function gridCard(a: Activity, view: ActivityView, on: ActivityActions): HTMLElement {
  const tabs = el(
    "div",
    { class: "segmented" },
    ...(["agents", "github"] as const).map((t) =>
      el("button", { class: t === view.tab ? "on" : "", text: t === "agents" ? "Agents" : "GitHub", onclick: () => t !== view.tab && on.tab(t) }),
    ),
  );
  const legend = el(
    "div",
    { class: "grid-legend" },
    el("span", { text: "Less" }),
    ...[0, 1, 2, 3, 4].map((l) => el("div", { class: `cell l${l}` })),
    el("span", { text: "More" }),
  );
  let about: string;
  let body: HTMLElement[];
  if (view.tab === "agents") {
    const total = a.grid.reduce((n, d) => n + d.turns, 0);
    about = `${plural(total, "turn", "turns")} in the last year`;
    // Monday on top: the grid starts on a Monday, so columns are weeks.
    body = [cellsOf(a.grid.map((d) => ({ day: d.day, level: d.level, title: d.turns ? `${short(d.day)}: ${plural(d.turns, "turn", "turns")}, ${duration(d.secs)}` : `${short(d.day)}: nothing` }))), legend];
  } else if (!view.github) {
    about = "";
    body = [el("p", { class: "note", text: "Asking GitHub…" })];
  } else if (view.github.state === "off") {
    about = "";
    body = [
      el("p", { class: "note", text: "Your GitHub contribution calendar shows here once the GitHub connector is on. It is asked through the gh you are logged into, once when this tab opens." }),
      el("div", { class: "actions" }, button("Open Connectors", () => on.connectors())),
    ];
  } else if (view.github.state === "error") {
    about = "";
    body = [el("p", { class: "note error", text: `GitHub didn't answer: ${view.github.message}` })];
  } else {
    about = `${plural(view.github.total, "contribution", "contributions")} in the last year`;
    // As GitHub's profile: its weeks start on Sunday.
    body = [cellsOf(view.github.days.map((d) => ({ day: d.day, level: d.level, title: `${short(d.day)}: ${d.count ? plural(d.count, "contribution", "contributions") : "none"}` }))), legend];
  }
  return el(
    "section",
    { class: "card" },
    el("div", { class: "card-head" }, tabs, el("div", { class: "row-about", text: about })),
    ...body,
  );
}

/** The page. */
export function activityPage(a: Activity | null, view: ActivityView, on: ActivityActions): HTMLElement[] {
  const confirming = view.confirming;
  const clear = confirming
    ? el(
        "div",
        { class: "actions" },
        button("Cancel", () => on.confirmClear(false)),
        button("Clear history", () => void on.clear(), true),
      )
    : button("Clear history…", () => on.confirmClear(true));
  return [
    el("h1", { text: "Activity" }),
    el("p", {
      class: "lede",
      text: "What your agents did, counted on this computer: each finished turn's length, agent, project folder name and counts (steps, commands, lines, your answers). Never a prompt, a command or a path, and nothing leaves this computer.",
    }),
    ...(a ? [weekCard(a, view, on), gridCard(a, view, on)] : [el("section", { class: "card" }, el("p", { class: "note", text: "Reading the history…" }))]),
    el(
      "section",
      { class: "card rows" },
      row(
        "Keep a history",
        "Each finished turn is kept for 12 weeks, and each day's totals for a year, for this page. Off, nothing new is kept and what is there stays.",
        toggle(a?.history ?? true, on.history),
      ),
      row(confirming ? "Clear the history?" : "Clear history", confirming ? "Every kept turn and day is removed. This can't be undone." : "Removes every kept turn and day from this computer.", clear),
    ),
  ];
}
