// The Week shares settings styles in a shadow root, without leaking names into the island.
import pagesCss from "../settings/pages.css?inline";
import activityCss from "./activity.css?inline";
import stateCss from "./view-state.css?inline";
import { viewState } from "./view-state";
import { el } from "../../dom";
import { t } from "../../i18n";
import { nestWeek, WEEK_CSS } from "./week-view";
import {
  type Activity,
  type ActivityActions,
  type GithubGrid,
  type GridTab,
} from "../settings/activity";
import { recapPng } from "../settings/recap-image";

export interface WeekBackend {
  activity: (monday: string | null) => Promise<Activity>;
  github: () => Promise<GithubGrid>;
  saveImage: (name: string, png: Uint8Array) => Promise<string | null>;
  settings: () => void;
  /** Settings → Connectors, for the GitHub grid. */
  connectors: () => void;
}

const HOST = `:host { display: block; color: var(--fg); font: 13.5px/1.5 var(--font); }
.page { padding: 20px 24px 32px; display: flex; flex-direction: column; gap: 16px; }
@media (max-width: 600px) { .page { padding: 16px; } }`;

/** A page drawn with the settings pages' look, inside `host`. */
export function settingsPage(host: HTMLElement, extra = ""): HTMLElement {
  const shadow = host.attachShadow({ mode: "open" });
  shadow.append(el("style", { text: pagesCss + HOST + extra }));
  const page = el("div", { class: "page" });
  shadow.append(page);
  return page;
}

export function createWeek(backend: WeekBackend): {
  element: HTMLElement;
  refresh: () => void;
} {
  const element = el("div", { class: "nest-week" });
  const page = settingsPage(element, activityCss + WEEK_CSS + stateCss);
  /** The day shown under the bars; null shows the busiest. */
  let day: number | null = null;
  let activity: Activity | null = null;
  let monday: string | null = null;
  let tab: GridTab = "agents";
  let github: GithubGrid | null = null;
  let confirming = false;
  let hideProjects = false;
  let exporting = false;
  let periodVersion = 0;
  let saved: { ok: boolean; text: string } | null = null;
  let asked = 0;
  let loading = false;
  let failed = false;
  let failure: string | null = null;

  const retry = el("button", { onclick: () => { if (!loading) void read(); } });
  page.tabIndex = -1;
  const render = () => {
    const active = (page.getRootNode?.() as ShadowRoot | undefined)?.activeElement ?? document.activeElement;
    const returning = active === retry;
    // Its pane scrolls, not the page: keep where the user was.
    const scroller = element.parentElement;
    const scroll = scroller?.scrollTop ?? 0;
    retry.textContent = loading ? t("Loading") : t("Try again");
    retry.setAttribute("aria-busy", String(loading));
    page.setAttribute("aria-busy", String(loading));
    page.setAttribute("aria-label", t("Activity"));
    page.replaceChildren(
      ...(failed ? [viewState("error", t("Could not load {name}.", { name: t("This week") }),
        activity ? t("The last loaded view is kept and may be out of date.") : t("Try again to read the data saved on this computer."), retry, failure)] : []),
      ...(!activity ? failed ? [] : [viewState("loading", t("Reading the history…"))] : nestWeek(activity, { tab, github, confirming, hideProjects, saved, exporting }, on, day, (d) => {
        day = d;
        render();
        // The page was drawn again: the keyboard stays on the day picked.
        page.querySelector<HTMLElement>(`.day-column[data-day="${d}"]`)?.focus();
      })),
    );
    if (scroller) scroller.scrollTop = scroll;
    if (returning) (failed ? retry : page).focus({ preventScroll: true });
  };
  const read = async () => {
    const mine = ++asked;
    loading = true;
    render();
    try {
      const next = await backend.activity(monday);
      if (mine !== asked) return;
      if (activity?.week.monday !== next.week.monday) saved = null;
      activity = next;
      failed = false;
    } catch (error) {
      if (mine !== asked) return;
      failed = true;
      failure = String(error);
    } finally {
      if (mine === asked) { loading = false; render(); }
    }
  };
  const on: Pick<ActivityActions, "week" | "tab" | "connectors" | "hideProjects" | "saveImage"> & { settings: () => void } = {
    settings: () => backend.settings(),
    week: (m) => {
      periodVersion++;
      monday = m;
      day = null;
      saved = null;
      void read();
    },
    tab: (next) => {
      tab = next;
      if (next !== "github") return render();
      github = null;
      render();
      backend
        .github()
        .then(
          (g) => (github = g),
          (e) => (github = { state: "error", message: String(e) }),
        )
        .finally(render);
    },
    hideProjects: (hide) => {
      if (exporting) return;
      hideProjects = hide;
    },
    saveImage: () => {
      if (!activity?.week.turns || exporting) return;
      const week = activity.week;
      const owner = periodVersion;
      const options = { hideProjects };
      exporting = true;
      saved = null;
      render();
      const current = () => owner === periodVersion && activity?.week.monday === week.monday;
      void recapPng(week, options)
        .then((png) => backend.saveImage(`vults-week-${week.monday}.png`, png))
        .then(
          (path) => { if (current()) saved = path ? { ok: true, text: t("Saved to {path}", { path }) } : null; },
          (error) => { if (current()) saved = { ok: false, text: t("Not saved: {error}", { error: String(error) }) }; },
        )
        .finally(() => { exporting = false; render(); });
    },
    connectors: () => backend.connectors(),
  };
  render();
  return {
    element,
    refresh: () => {
      confirming = false;
      void read();
    },
  };
}
