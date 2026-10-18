import type { DictationPeriod, DictationStatistics } from "../../../bridge";
import { el } from "../../../dom";
import { locale, t } from "../../../i18n";
import activityCss from "../activity.css?inline";
import stateCss from "../view-state.css?inline";
import { viewState } from "../view-state";
import { settingsPage } from "../week";
import { WEEK_CSS } from "../week-view";

export function createDictationStatistics(load: (period: DictationPeriod) => Promise<DictationStatistics>, settings: () => void) {
  const element = el("div", { class: "nest-dictation-statistics" });
  const page = settingsPage(element, activityCss + WEEK_CSS + stateCss + `
    .stats-periods { display: flex; flex-wrap: wrap; gap: 6px; }
    .stats-periods button, .stats-actions button { min-height: var(--control); padding: 4px 12px; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--raised); color: var(--fg); cursor: pointer; font: inherit; }
    .stats-periods button[aria-pressed="true"] { background: var(--mark); color: var(--on-accent); }
    .stats-actions { display: flex; flex-wrap: wrap; gap: 8px; }
    .chart-scroll { overflow-x: auto; }
    .chart-scroll .week-chart { margin-top: 8px; }
    .stats-apps { display: grid; gap: 12px; }
    .stats-apps .agent-turn { margin: 0; }
    .stats-apps bdi { overflow-wrap: anywhere; }
    .stats-periods button:focus-visible, .stats-actions button:focus-visible, .chart-scroll:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  `);
  page.tabIndex = -1;
  const title = el("h2"), about = el("p"), feedback = el("div");
  const figures = el("div", { class: "activity-summary" });
  const chart = el("section", { class: "card" });
  const apps = el("section", { class: "card" });
  const note = el("p", { class: "note" });
  const refresh = el("button", { onclick: () => { if (!loading) void read(); } });
  const configure = el("button", { onclick: settings });
  const retry = el("button", { onclick: () => { if (!loading) void read(); } });
  let period: DictationPeriod = "week", data: DictationStatistics | null = null;
  try {
    const saved = localStorage.getItem("vults.dictation.statistics.period");
    if (saved === "week" || saved === "month" || saved === "all") period = saved;
  } catch { /* The current filter remains usable without saved presentation state. */ }
  let asked = 0, loading = false, failed = false;
  let selected: string | null = null;
  const controls = (["week", "month", "all"] as const).map(id => {
    const button = el("button", { onclick: () => { if (period !== id) {
      period = id; selected = null;
      try { localStorage.setItem("vults.dictation.statistics.period", id); } catch { /* Keep the current filter. */ }
      void read();
    } } });
    button.dataset.statsPeriod = id;
    return button;
  });
  const periods = el("div", { class: "stats-periods" }, ...controls);
  periods.setAttribute("role", "group");
  page.append(el("header", { class: "activity-heading" }, el("div", {}, title, about),
    el("div", { class: "stats-actions" }, refresh, configure)), periods, feedback, figures, chart, apps, note);
  const number = (value: number) => value.toLocaleString(locale(), { maximumFractionDigits: 1 });
  const date = (day: string, short = false) => new Date(`${day}T12:00:00`).toLocaleDateString(locale(),
    short ? { month: "short", day: "numeric" } : { year: "numeric", month: "long", day: "numeric" });
  const draw = () => {
    const focused = element.shadowRoot?.activeElement as HTMLElement | null;
    const focusedDay = focused?.dataset.statsDay;
    const returning = focused === retry;
    const scroll = element.parentElement?.scrollTop ?? 0;
    const chartScroll = chart.querySelector(".chart-scroll")?.scrollLeft ?? 0;
    const value = data?.period === period ? data : null;
    title.textContent = t("Dictation statistics");
    about.textContent = t("Measured dictation usage on this computer. Usage totals survive transcript erasure.");
    periods.setAttribute("aria-label", t("Dictation period"));
    const labels = { week: t("Last 7 days"), month: t("Last 30 days"), all: t("All time") };
    for (const control of controls) {
      const id = control.dataset.statsPeriod as DictationPeriod;
      control.textContent = labels[id]; control.setAttribute("aria-pressed", String(period === id));
    }
    refresh.textContent = loading ? t("Loading") : t("Refresh"); refresh.disabled = loading;
    configure.textContent = t("Dictation settings");
    retry.textContent = t("Try again"); retry.disabled = loading;
    page.setAttribute("aria-busy", String(loading));
    feedback.replaceChildren(...(failed ? [viewState("error", t("Could not load dictation statistics."),
      value ? t("The last loaded view is kept and may be out of date.") : t("Try again to read the data saved on this computer."), retry)]
      : loading ? [viewState("loading", t("Reading dictation statistics…"))] : []));
    figures.replaceChildren(); chart.replaceChildren(); apps.replaceChildren();
    chart.hidden = apps.hidden = !value || !value.usage.count;
    note.textContent = t("Time saved estimates typing at 45 words per minute. Speed is zero below one second of measured audio. Replay does not add usage.");
    if (value) {
      const known = value.usage.count > 0;
      const metric = (value: string, label: string) => el("div", { class: "activity-stat" }, el("strong", { text: value }), el("span", { text: label }));
      figures.append(metric(known ? number(value.usage.words) : t("Unknown"), t("Dictated words")),
        metric(value.usage.saved_minutes === null ? t("Unknown") : t("{n} min", { n: number(value.usage.saved_minutes) }), t("Estimated time saved")),
        metric(value.usage.wpm === null ? t("Unknown") : number(value.usage.wpm), t("Words per minute")),
        metric(known ? number(value.usage.count) : t("Unknown"), t("Dictations")));
      if (!known && !failed && !loading) feedback.append(viewState("empty", t("No dictation recorded in this period."), t("Use the dictation key outside Vults to add your first entry.")));
      if (known) {
        chart.append(el("h3", { text: value.chart_limited ? t("Daily words · latest 90 days") : t("Daily dictated words") }));
        const bars = el("div", { class: "week-chart" });
        bars.style.gridTemplateColumns = `repeat(${value.days.length}, minmax(24px, 1fr))`;
        const top = Math.max(1, ...value.days.map(d => d.words));
        const detail = el("p", { class: "chart-detail" }); detail.setAttribute("role", "status");
        const described = (day: typeof value.days[number]) => !day.count
          ? t("{day}: no dictation recorded", { day: date(day.day) })
          : day.words === 1 ? t("{day}: 1 dictated word", { day: date(day.day) })
            : t("{day}: {count} dictated words", { day: date(day.day), count: number(day.words) });
        for (const day of value.days) {
          const fill = el("div", { class: "bar-fill" }); fill.style.height = `${day.words * 100 / top}%`;
          const button = el("button", { class: "day-column", onclick: () => { selected = day.day; draw(); } },
            el("span", { class: "day-total", text: day.count ? number(day.words) : "—" }),
            el("div", { class: "bar-track" }, fill), el("span", { text: date(day.day, true) }));
          button.dataset.statsDay = day.day; button.title = described(day);
          button.setAttribute("aria-label", described(day)); button.setAttribute("aria-pressed", String(selected === day.day));
          bars.append(button);
        }
        const chosen = value.days.find(d => d.day === selected) ?? value.days.at(-1);
        if (chosen) detail.textContent = described(chosen);
        const scroller = el("div", { class: "chart-scroll" }, bars); scroller.tabIndex = 0;
        scroller.setAttribute("role", "region"); scroller.setAttribute("aria-label", t("Daily dictated words"));
        chart.append(scroller, detail); scroller.scrollLeft = chartScroll;
        apps.append(el("h3", { text: t("Top apps") }));
        if (!value.apps_complete) apps.append(el("p", { class: "note", text: t("Application totals are partial: older dictations have no daily app breakdown.") }));
        const rows = el("div", { class: "stats-apps" });
        for (const app of value.apps) {
          const name = app.app ?? t("Unknown app");
          const progress = el("progress"); progress.max = Math.max(1, value.usage.words); progress.value = app.words;
          progress.setAttribute("aria-label", t("Words dictated in {app}", { app: name }));
          rows.append(el("div", { class: "agent-turn" }, el("span", {}, el("bdi", { text: name })), progress, el("strong", { text: number(app.words) })));
        }
        apps.append(rows);
      }
    }
    if (element.parentElement) element.parentElement.scrollTop = scroll;
    if (focusedDay) page.querySelector<HTMLElement>(`[data-stats-day="${focusedDay}"]`)?.focus({ preventScroll: true });
    if (returning && !loading) (failed ? retry : page).focus({ preventScroll: true });
  };
  const read = async () => {
    const mine = ++asked, wanted = period;
    loading = true; failed = false; draw();
    try { const next = await load(wanted); if (mine === asked) data = next; }
    catch { if (mine === asked) failed = true; }
    finally { if (mine === asked) { loading = false; draw(); } }
  };
  draw();
  return { element, refresh: () => void read(), relabel: draw };
}
