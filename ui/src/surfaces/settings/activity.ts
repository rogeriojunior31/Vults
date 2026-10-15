// Shared Activity calendar and history controls. Room owns the recap; Settings owns retention.
import type { AgentKind, GridDay, WeekView } from "../../view.gen";
import { NAME_BY_KIND } from "../../agents";
import { el } from "../../dom";
import { button, row, toggle } from "./pieces";
import { currentLang, locale, t, tk } from "../../i18n";
import { duration } from "../../duration";

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

export const AGENT: Record<AgentKind, string> = { ...NAME_BY_KIND, other: tk("Another tool") };

/** Monday first, in the user's language: Mon … Sun. 2026-10-05 was a Monday. */
export function weekdays(): string[] {
  return Array.from({ length: 7 }, (_, d) => new Date(2026, 9, 5 + d, 12).toLocaleDateString(locale(), { weekday: "short" }));
}

export { duration } from "../../duration";

/** A `2026-10-09` day, as a local date at noon (no time zone can move it to another day). */
const date = (day: string): Date => new Date(`${day}T12:00:00`);
const short = (day: string) => date(day).toLocaleDateString(locale(), { month: "short", day: "numeric" });
export const weekday = (day: string) => date(day).toLocaleDateString(locale(), { weekday: "long" });

/** `Oct 5 – 11, 2026`, or across a month: `Sep 28 – Oct 4, 2026`. */
export function weekLabel(monday: string, sunday: string): string {
  const [a, b] = [date(monday), date(sunday)];
  // Other languages say a range their own way (`5–11 de out. de 2026`, `2026年10月5日至11日`).
  if (currentLang() !== "en") return new Intl.DateTimeFormat(locale(), { month: "short", day: "numeric", year: "numeric" }).formatRange(a, b);
  const end = a.getMonth() === b.getMonth() ? String(b.getDate()) : short(sunday);
  return `${short(monday)} – ${end}, ${b.getFullYear()}`;
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


export function gridCard(a: Activity, view: ActivityView, on: ActivityActions): HTMLElement {
  const tabs = el(
    "div",
    { class: "segmented" },
    ...(["agents", "github"] as const).map((tab) =>
      el("button", { class: tab === view.tab ? "on" : "", text: tab === "agents" ? t("Agents") : "GitHub", onclick: () => tab !== view.tab && on.tab(tab) }),
    ),
  );
  const legend = el(
    "div",
    { class: "grid-legend" },
    el("span", { text: t("Less") }),
    ...[0, 1, 2, 3, 4].map((l) => el("div", { class: `cell l${l}` })),
    el("span", { text: t("More") }),
  );
  let about: string;
  let body: HTMLElement[];
  if (view.tab === "agents") {
    const total = a.grid.reduce((n, d) => n + d.turns, 0);
    about = total === 1 ? t("1 turn in the last year") : t("{n} turns in the last year", { n: total });
    // Monday on top: the grid starts on a Monday, so columns are weeks.
    body = [cellsOf(a.grid.map((d) => ({ day: d.day, level: d.level, title: !d.turns ? t("{day}: nothing", { day: short(d.day) }) : d.turns === 1 ? t("{day}: 1 turn, {time}", { day: short(d.day), time: duration(d.secs) }) : t("{day}: {n} turns, {time}", { day: short(d.day), n: d.turns, time: duration(d.secs) }) }))), legend];
  } else if (!view.github) {
    about = "";
    body = [el("p", { class: "note", text: t("Asking GitHub…") })];
  } else if (view.github.state === "off") {
    about = "";
    body = [
      el("p", { class: "note", text: t("Your GitHub contribution calendar shows here once the GitHub connector is on. It is asked through the gh you are logged into, once when this tab opens.") }),
      el("div", { class: "actions" }, button(t("Open Connectors"), () => on.connectors())),
    ];
  } else if (view.github.state === "error") {
    about = "";
    body = [el("p", { class: "note error", text: t("GitHub didn't answer: {error}", { error: view.github.message }) })];
  } else {
    about = view.github.total === 1 ? t("1 contribution in the last year") : t("{n} contributions in the last year", { n: view.github.total });
    // As GitHub's profile: its weeks start on Sunday.
    body = [cellsOf(view.github.days.map((d) => ({ day: d.day, level: d.level, title: !d.count ? t("{day}: none", { day: short(d.day) }) : d.count === 1 ? t("{day}: 1 contribution", { day: short(d.day) }) : t("{day}: {n} contributions", { day: short(d.day), n: d.count }) }))), legend];
  }
  return el(
    "section",
    { class: "card" },
    el("div", { class: "card-head" }, tabs, el("div", { class: "row-about", text: about })),
    ...body,
  );
}

/** Keep a history, and clear it (asking first). */
export function historyCard(a: Activity | null, view: Pick<ActivityView, "confirming">, on: Pick<ActivityActions, "history" | "confirmClear" | "clear">): HTMLElement {
  const confirming = view.confirming;
  const clear = confirming
    ? el(
        "div",
        { class: "actions" },
        button(t("Cancel"), () => on.confirmClear(false)),
        button(t("Clear history"), () => void on.clear(), true),
      )
    : button(t("Clear history…"), () => on.confirmClear(true));
  return el(
    "section",
    { class: "card rows" },
    row(
      t("Keep a history"),
      t("Each finished turn is kept for 12 weeks, and each day's totals for a year, for this page. Off, nothing new is kept and what is there stays."),
      toggle(a?.history ?? true, on.history),
    ),
    row(
      confirming ? t("Clear the history?") : t("Clear history"),
      confirming ? t("Every kept turn and day, the audit log of your answers and Zeca's traces are removed. This can't be undone.") : t("Removes every kept turn and day, the audit log of your answers and Zeca's traces from this computer."),
      clear,
    ),
  );
}
