// The Nest's Activity → Week (ADR 0021): the week's recap, the year's grid and the history's switches,
// the same page Settings drew, in a shadow root with the settings pages' own look so their names
// never meet the island's.
import pagesCss from "../settings/pages.css?inline";
import { el } from "../../dom";
import { t } from "../../i18n";
import { activityPage, type Activity, type ActivityActions, type GithubGrid, type GridTab } from "../settings/activity";
import { recapPng } from "../settings/recap-image";

export interface WeekBackend {
  activity: (monday: string | null) => Promise<Activity>;
  github: () => Promise<GithubGrid>;
  saveImage: (name: string, png: Uint8Array) => Promise<string | null>;
  setHistory: (on: boolean) => Promise<void>;
  clear: () => Promise<void>;
  /** Settings → Connectors, for the GitHub grid. */
  connectors: () => void;
}

const HOST = `:host { display: block; color: var(--fg); font: 13.5px/1.5 var(--font); }
.page { padding: 16px 20px 32px; display: flex; flex-direction: column; gap: 14px; }`;

/** A page drawn with the settings pages' look, inside `host`. */
export function settingsPage(host: HTMLElement): HTMLElement {
  const shadow = host.attachShadow({ mode: "open" });
  shadow.append(el("style", { text: pagesCss + HOST }));
  const page = el("div", { class: "page" });
  shadow.append(page);
  return page;
}

export function createWeek(backend: WeekBackend): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "nest-week" });
  const page = settingsPage(element);
  let activity: Activity | null = null;
  let monday: string | null = null;
  let tab: GridTab = "agents";
  let github: GithubGrid | null = null;
  let confirming = false;
  let hideProjects = false;
  let saved: { ok: boolean; text: string } | null = null;

  const render = () => {
    const scroll = element.scrollTop;
    // The tab says Week: the page's own title would say it twice.
    page.replaceChildren(...activityPage(activity, { tab, github, confirming, hideProjects, saved }, on).slice(1));
    element.scrollTop = scroll;
  };
  const read = async () => {
    try {
      activity = await backend.activity(monday);
    } catch {
      activity = null;
    }
    render();
  };
  const on: ActivityActions = {
    week: (m) => {
      monday = m;
      saved = null;
      void read();
    },
    tab: (next) => {
      tab = next;
      if (next !== "github") return render();
      github = null;
      render();
      backend.github().then(
        (g) => (github = g),
        (e) => (github = { state: "error", message: String(e) }),
      ).finally(render);
    },
    hideProjects: (hide) => {
      hideProjects = hide;
    },
    saveImage: () => {
      if (!activity) return;
      const week = activity.week;
      void recapPng(week, { hideProjects })
        .then((png) => backend.saveImage(`vults-week-${week.monday}.png`, png))
        .then(
          (path) => (saved = path ? { ok: true, text: t("Saved to {path}", { path }) } : null),
          (e) => (saved = { ok: false, text: t("Not saved: {error}", { error: String(e) }) }),
        )
        .finally(render);
    },
    connectors: () => backend.connectors(),
    history: async (keep) => {
      await backend.setHistory(keep);
      if (activity) activity = { ...activity, history: keep };
    },
    confirmClear: (yes) => {
      confirming = yes;
      render();
    },
    clear: async () => {
      await backend.clear();
      confirming = false;
      monday = null;
      await read();
    },
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
