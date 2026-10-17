// Weekly figures use only recorded activity on this computer.
import { el } from "../../dom";
import { locale, t } from "../../i18n";
import { AGENT, duration, gridCard, weekday, weekdays, weekLabel, type Activity, type ActivityActions, type ActivityView } from "../settings/activity";
import { button, toggle } from "../settings/pieces";
import { viewState } from "./view-state";

/** The prototype's look for this page, over the settings pages' own (both in the Week's shadow root). */
export const WEEK_CSS = `
.activity-heading, .activity-summary { margin-bottom: 0; }
.screen-actions { display: flex; flex: none; gap: 4px; padding: 4px; border-radius: 999px; background: var(--raised); }
.screen-actions button { display: grid; place-items: center; width: 32px; height: 32px; padding: 0; border: 0; border-radius: 999px; background: none; color: var(--fg); font-size: 20px; cursor: pointer; }
.screen-actions button:hover:not(:disabled) { background: var(--float); }
.screen-actions button:disabled { opacity: 0.35; cursor: default; }
.activity-summary { grid-template-columns: repeat(4, minmax(0, 1fr)); }
.screen-columns { display: grid; grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr); gap: 16px; }
.card { min-width: 0; padding: 20px; border-color: var(--border); border-radius: var(--radius-card); background: var(--panel); }
.week-chart { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: 8px; height: 180px; margin-top: 24px; }
.day-column { display: flex; flex-direction: column; align-items: center; justify-content: flex-end; gap: 8px; min-width: 0; padding: 6px 4px; border: 0; border-radius: var(--radius-control); background: none; color: var(--fg-dim); cursor: pointer; font: inherit; transition: background-color var(--motion-fast) var(--ease-out); }
.day-column:hover, .day-column[aria-pressed="true"] { background: var(--raised); }
.bar-track { display: flex; align-items: flex-end; flex: 1; width: 100%; max-width: 28px; min-height: 0; border-radius: 6px; background: var(--raised); }
.bar-fill { width: 100%; border-radius: 6px; background: var(--fg-muted); transition: height var(--motion-enter) var(--ease-out), background-color var(--motion-fast) var(--ease-out); }
.day-column:hover .bar-fill, .day-column[aria-pressed="true"] .bar-fill { background: var(--mark); }
.day-column span { font-size: 11px; font-variant-numeric: tabular-nums; }
.day-column .day-total { font: 11px var(--mono); white-space: nowrap; }
.day-column[aria-pressed="true"] span { color: var(--fg); }
.chart-detail { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 8px; margin-top: 16px; padding: 12px; border-radius: var(--radius-control); background: var(--raised); color: var(--fg-dim); font-size: 12px; font-variant-numeric: tabular-nums; }
.chart-detail strong { color: var(--fg); font-weight: 600; }
.week-most { margin: 12px 0 0; }
.recap-card .share-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 16px; padding: 16px 0; border-top: 1px solid var(--border); }
.recap-card .share-row strong { display: block; font-size: 12px; font-weight: 550; }
.recap-card .share-row small { color: var(--fg-dim); font-size: 11px; }
.recap-card > .btn { width: 100%; min-height: 36px; margin-top: 8px; border: 0; border-radius: var(--radius-control); background: var(--accent); color: var(--on-accent); font-size: 12px; font-weight: 600; }
.recap-card > .btn:hover { background: var(--mark); }
.recap-card .note { margin-top: 12px; }
.card-head .card-title { font-size: 14px; font-weight: 600; }
@media (max-width: 760px) {
  .activity-summary { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .screen-columns { grid-template-columns: 1fr; }
}
@media (max-width: 600px) { .card { padding: 16px; } .week-chart { gap: 4px; } }
`;

/** The page; `day` is the day shown under the bars (Monday is 0), `pick` changes it. */
export function nestWeek(a: Activity | null, view: ActivityView, on: Pick<ActivityActions, "week" | "tab" | "connectors" | "hideProjects" | "saveImage"> & { settings: () => void }, day: number | null, pick: (d: number) => void): HTMLElement[] {
  if (!a) return [viewState("loading", t("Reading the history…"))];
  const w = a.week;
  const i = a.weeks.indexOf(w.monday);
  const older = i >= 0 && i < a.weeks.length - 1 ? a.weeks[i + 1] : null;
  const newer = i > 0 ? a.weeks[i - 1] : null;
  const nav = (text: string, label: string, to: string | null) => {
    const b = el("button", { text, onclick: () => to && on.week(to) });
    b.title = label;
    b.setAttribute("aria-label", label);
    if (!to) b.setAttribute("disabled", "");
    return b;
  };
  const heading = el(
    "header",
    { class: "activity-heading" },
    el(
      "div",
      {},
      el("div", { class: "activity-eyebrow", text: t("Activity / on this computer") }),
      el("h2", { text: i === 0 ? t("This week") : weekLabel(w.monday, w.sunday) }),
      i === 0 ? el("p", { text: weekLabel(w.monday, w.sunday) }) : null,
    ),
    el("div", { class: "week-actions" },
      el("button", { class: "week-settings", text: t("History settings"), onclick: on.settings }),
      el("div", { class: "screen-actions" }, nav("‹", t("Older week"), older), nav("›", t("Newer week"), newer))),
  );
  // What is counted, and that nothing leaves: said once, under the heading.
  const lede = el("details", { class: "activity-privacy" }, el("summary", { text: t("How activity is counted") }), el("p", { text: t("What your agents did, counted on this computer: each finished turn's length, agent, project folder name and counts (steps, commands, lines, your answers). Never a prompt, a command or a path, and nothing leaves this computer.") }));
  if (!w.turns) return [heading, lede, viewState("empty", w.headline), gridCard(a, view, on)];
  const previous = a.previous;
  const delta = (now: number, before: number | undefined) => !previous?.turns || before === undefined
    ? t("No recorded baseline for comparison.")
    : before === 0 ? t("The previous week's value was zero.") : t("{change}% compared with the previous week", { change: `${now >= before ? "+" : ""}${Math.round((now - before) / before * 100)}` });
  const metric = (value: string, label: string, small: string, change: string) =>
    el("div", { class: "activity-stat" }, el("strong", { text: value }), el("span", { text: label }), el("small", { text: small }), el("small", { class: "stat-delta", text: change }));
  const names = weekdays();
  const top = Math.max(1, ...w.days);
  // A week with turns but no time counted has no busiest day: nothing is picked until asked.
  const busiest = Math.max(...w.days) > 0 ? w.days.indexOf(Math.max(...w.days)) : null;
  const shown = day ?? busiest;
  const chart = el(
    "div",
    { class: "week-chart" },
    ...w.days.map((secs, d) => {
      const fill = el("div", { class: "bar-fill" });
      fill.style.height = `${Math.round((secs / top) * 100)}%`;
      const total = !secs ? "—" : secs < 60 ? t("<1 min") : secs < 3600
        ? t("{m} min", { m: Math.floor(secs / 60) })
        : t("{h} h", { h: (secs / 3600).toLocaleString(locale(), { maximumFractionDigits: 1 }) });
      const b = el("button", { class: "day-column", onclick: () => pick(d) }, el("span", { class: "day-total", text: total }), el("div", { class: "bar-track" }, fill), el("span", { text: names[d] }));
      b.dataset.day = String(d);
      b.title = `${names[d]}: ${duration(secs)}`;
      b.setAttribute("aria-label", b.title);
      b.setAttribute("aria-pressed", String(d === shown));
      return b;
    }),
  );
  chart.setAttribute("role", "group");
  chart.setAttribute("aria-label", t("Time with the flock"));
  const most = [
    w.top_agent
      ? w.top_project
        ? t("Most turns: {agent} on {project}", { agent: t(AGENT[w.top_agent]), project: w.top_project })
        : t("Most turns: {agent}", { agent: t(AGENT[w.top_agent]) })
      : null,
    w.busiest_day ? t("Busiest: {day}", { day: weekday(w.busiest_day) }) : null,
    t("Longest turn: {time}", { time: duration(w.longest_secs) }),
  ].filter((x): x is string => !!x);
  return [
    heading,
    el("p", { class: "week-headline", text: w.headline }),
    el(
      "div",
      { class: "activity-summary" },
      metric(duration(w.active_secs), t("with your agents"), t("Longest turn: {time}", { time: duration(w.longest_secs) }), delta(w.active_secs, previous?.active_secs)),
      metric(String(w.turns), w.turns === 1 ? t("turn") : t("turns"), t("{n} steps", { n: w.steps }), delta(w.turns, previous?.turns)),
      metric(`+${w.added} −${w.removed}`, t("lines changed"), w.files === 1 ? t("1 file") : t("{n} files", { n: w.files }), delta(w.added + w.removed, previous ? previous.added + previous.removed : undefined)),
      metric(
        String(w.allowed + w.denied + w.answered),
        t("answers"),
        // The parts add up to the figure: a question answered is an answer too.
        w.answered
          ? t("{allowed} allowed · {denied} denied · {answered} answered", { allowed: w.allowed, denied: w.denied, answered: w.answered })
          : t("{allowed} allowed · {denied} denied", { allowed: w.allowed, denied: w.denied }),
        delta(w.allowed + w.denied + w.answered, previous ? previous.allowed + previous.denied + previous.answered : undefined),
      ),
    ),
    // The period and its totals answer first; the counting policy can stay nearby without
    // interrupting the scan.
    el(
      "div",
      { class: "screen-columns" },
      el(
        "section",
        { class: "activity-card" },
        el("h3", { text: t("Time with the flock") }),
        chart,
        el("div", { class: "chart-detail" }, shown === null ? null : el("strong", { text: `${names[shown]} · ${duration(w.days[shown])}` }), el("span", { text: t("Select a day to inspect") })),
        el("p", { class: "week-most", text: most.join(" · ") }),
      ),
      el(
        "section",
        { class: "activity-card recap-card" },
        el("h3", { text: t("Keep a small recap") }),
        el("p", { text: t("Time, turns and decisions from this computer. Choose how much to share.") }),
        // A label around the switch: its words are its name.
        el(
          "label",
          { class: "share-row" },
          el("div", {}, el("strong", { text: t("Hide project names") }), el("small", { text: t("For the saved image.") })),
          toggle(view.hideProjects, async (hide) => on.hideProjects(hide)),
        ),
        button(t("Save as image…"), () => on.saveImage()),
        view.saved ? el("p", { class: `note${view.saved.ok ? " ok" : " error"}`, text: view.saved.text }) : null,
      ),
    ),
    el("section", { class: "activity-card agent-turns" },
      el("h3", { text: t("Turns by agent") }),
      ...(w.agents ?? []).map(([agent, count]) => {
        const progress = el("progress"); progress.max = w.turns; progress.value = count;
        progress.setAttribute("aria-label", t(AGENT[agent]));
        return el("div", { class: "agent-turn" }, el("span", { text: t(AGENT[agent]) }), progress, el("strong", { text: String(count) }));
      })),
    gridCard(a, view, on),
    lede,
  ];
}
