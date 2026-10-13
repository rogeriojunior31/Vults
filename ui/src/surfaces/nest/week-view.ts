// The Nest's Activity → Week, as the Living Flock prototype draws it: the kicker, the title and the
// week's dates with its arrows; four figures in one strip; *Time with the flock* (a bar a day, a day
// to inspect) beside *Keep a small recap*; the year's grid and the history's switches under them.
// Only what this computer counted: no figure the history does not hold.
import { el } from "../../dom";
import { t } from "../../i18n";
import { AGENT, duration, gridCard, historyCard, weekday, weekdays, weekLabel, type Activity, type ActivityActions, type ActivityView } from "../settings/activity";
import { button, toggle } from "../settings/pieces";

/** The prototype's look for this page, over the settings pages' own (both in the Week's shadow root). */
export const WEEK_CSS = `
.screen-heading { display: flex; align-items: center; justify-content: space-between; gap: 15px; margin-bottom: 8px; }
.screen-kicker { margin-bottom: 7px; font: 9px var(--mono); letter-spacing: 1.5px; text-transform: uppercase; color: #9da29b; }
.screen-heading h2 { margin: 0; font-size: 25px; font-weight: 550; line-height: 1.3; letter-spacing: -0.85px; }
.screen-heading p { margin: 7px 0 0; font-size: 12px; line-height: 1.65; color: var(--fg-dim); }
.screen-actions { display: flex; gap: 8px; }
.screen-actions button { display: grid; place-items: center; width: 30px; height: 30px; padding: 0; border: 1px solid #3a3d44; border-radius: 6px; background: none; color: var(--fg); font-size: 13px; cursor: pointer; }
.screen-actions button:disabled { opacity: 0.4; cursor: default; }
.metric-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); overflow: hidden; border: 1px solid #292c32; border-radius: 8px; background: #101114; }
.metric { padding: 20px; border-right: 1px solid #292c32; min-width: 0; }
.metric:last-child { border-right: 0; }
.metric strong { display: block; font-size: 27px; font-weight: 500; letter-spacing: -1px; font-variant-numeric: tabular-nums; }
.metric span { font-size: 11px; color: var(--fg-dim); }
.metric small { display: block; margin-top: 9px; font: 9px var(--mono); color: #a0b495; }
.screen-columns { display: grid; grid-template-columns: minmax(0, 1.5fr) minmax(0, 0.8fr); gap: 16px; }
.surface-card, .card { min-width: 0; padding: 20px; border: 1px solid #292b31; border-radius: 8px; background: #101114; }
.surface-card h3 { margin: 0; font-size: 15px; font-weight: 550; letter-spacing: -0.2px; }
.surface-card p { margin: 6px 0 0; font-size: 12px; line-height: 1.8; color: var(--fg-dim); }
.week-chart { display: grid; grid-template-columns: repeat(7, 1fr); gap: 14px; height: 170px; margin-top: 20px; }
.day-column { display: flex; flex-direction: column; align-items: center; justify-content: flex-end; gap: 8px; min-width: 0; padding: 0 3px; border: 0; border-radius: 5px; background: none; color: var(--fg-dim); cursor: pointer; font: inherit; }
.bar-track { display: flex; align-items: flex-end; width: 100%; max-width: 30px; height: 100%; border-radius: 3px; background: #1d2025; }
.bar-fill { width: 100%; border-radius: 2px; background: #6f8266; transition: height 0.25s, background 0.15s; }
.day-column:hover .bar-fill, .day-column[aria-pressed="true"] .bar-fill { background: #b1c4a1; }
.day-column span { font: 10px var(--mono); }
.day-column[aria-pressed="true"] span { color: #d3dfc9; }
.chart-detail { display: flex; justify-content: space-between; gap: 10px; margin-top: 18px; padding-top: 14px; border-top: 1px solid #23262b; font-size: 11px; color: var(--fg-dim); }
.chart-detail strong { font-weight: 550; color: var(--fg); }
.surface-card .week-most { margin-top: 12px; font-size: 11px; color: var(--fg-dim); }
.lede { margin: 0; font-size: 12px; line-height: 1.6; color: var(--fg-dim); }
.recap-card .share-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 16px; padding-bottom: 15px; border-bottom: 1px solid #23262b; }
.recap-card .share-row strong { display: block; font-size: 12px; font-weight: 550; }
.recap-card .share-row small { font-size: 10px; color: var(--fg-dim); }
.recap-card > .btn { width: 100%; min-height: 36px; margin-top: 18px; border: 1px solid #3a3d44; border-radius: 6px; background: none; color: var(--fg); font-size: 12px; font-weight: 500; }
.recap-card > .btn:hover { border-color: #6b7065; background: #1c1e22; }
.recap-card .note { margin-top: 10px; }
.card-head .card-title { font-size: 15px; font-weight: 550; }
@media (max-width: 760px) {
  .metric-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .metric:nth-child(2) { border-right: 0; }
  .metric:nth-child(-n + 2) { border-bottom: 1px solid #292c32; }
  .screen-columns { grid-template-columns: 1fr; }
}
`;

/** The page; `day` is the day shown under the bars (Monday is 0), `pick` changes it. */
export function nestWeek(a: Activity | null, view: ActivityView, on: ActivityActions, day: number | null, pick: (d: number) => void): HTMLElement[] {
  if (!a) return [el("section", { class: "surface-card" }, el("p", { text: t("Reading the history…") }))];
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
    { class: "screen-heading" },
    el(
      "div",
      {},
      el("div", { class: "screen-kicker", text: t("Activity / on this computer") }),
      el("h2", { text: i === 0 ? t("This week") : weekLabel(w.monday, w.sunday) }),
      i === 0 ? el("p", { text: weekLabel(w.monday, w.sunday) }) : null,
    ),
    el("div", { class: "screen-actions" }, nav("‹", t("Older week"), older), nav("›", t("Newer week"), newer)),
  );
  // What is counted, and that nothing leaves: said once, under the heading.
  const lede = el("p", { class: "lede", text: t("What your agents did, counted on this computer: each finished turn's length, agent, project folder name and counts (steps, commands, lines, your answers). Never a prompt, a command or a path, and nothing leaves this computer.") });
  if (!w.turns) return [heading, lede, el("section", { class: "surface-card" }, el("p", { text: w.headline })), gridCard(a, view, on), historyCard(a, view, on)];
  const metric = (value: string, label: string, small: string) =>
    el("div", { class: "metric" }, el("strong", { text: value }), el("span", { text: label }), el("small", { text: small }));
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
      const b = el("button", { class: "day-column", onclick: () => pick(d) }, el("div", { class: "bar-track" }, fill), el("span", { text: names[d] }));
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
      { class: "metric-grid" },
      metric(duration(w.active_secs), t("with your agents"), t("Longest turn: {time}", { time: duration(w.longest_secs) })),
      metric(String(w.turns), w.turns === 1 ? t("turn") : t("turns"), t("{n} steps", { n: w.steps })),
      metric(`+${w.added} −${w.removed}`, t("lines changed"), w.files === 1 ? t("1 file") : t("{n} files", { n: w.files })),
      metric(
        String(w.allowed + w.denied + w.answered),
        t("answers"),
        // The parts add up to the figure: a question answered is an answer too.
        w.answered
          ? t("{allowed} allowed · {denied} denied · {answered} answered", { allowed: w.allowed, denied: w.denied, answered: w.answered })
          : t("{allowed} allowed · {denied} denied", { allowed: w.allowed, denied: w.denied }),
      ),
    ),
    // The period and its totals answer first; the counting policy can stay nearby without
    // interrupting the scan.
    lede,
    el(
      "div",
      { class: "screen-columns" },
      el(
        "section",
        { class: "surface-card" },
        el("h3", { text: t("Time with the flock") }),
        chart,
        el("div", { class: "chart-detail" }, shown === null ? null : el("strong", { text: `${names[shown]} · ${duration(w.days[shown])}` }), el("span", { text: t("Select a day to inspect") })),
        el("p", { class: "week-most", text: most.join(" · ") }),
      ),
      el(
        "section",
        { class: "surface-card recap-card" },
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
    gridCard(a, view, on),
    historyCard(a, view, on),
  ];
}
