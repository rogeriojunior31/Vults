// Settings → Activity with made-up weeks, for the visual tests. `?state=` full (default), empty, off,
// confirm; `?week=` an older Monday. The year's grid is a fixed pattern, never random.
import { el } from "../src/dom";
import { activityPage, type Activity } from "../src/surfaces/settings/activity";
import type { WeekView } from "../src/view.gen";

const query = new URLSearchParams(location.search);
const state = query.get("state") ?? "full";

const week = (monday: string, sunday: string, full: boolean): WeekView =>
  full
    ? {
        monday,
        sunday,
        active_secs: 6 * 3600 + 20 * 60,
        turns: 41,
        steps: 512,
        commands: 87,
        files: 34,
        added: 1217,
        removed: 312,
        allowed: 14,
        denied: 2,
        answered: 5,
        questions: 6,
        failed: 1,
        top_agent: "claude",
        top_project: "site",
        busiest_day: "2026-10-07",
        longest_secs: 3 * 3600 + 5 * 60,
        days: [3000, 5400, 7800, 2400, 4200, 0, 1200],
        headline: "41 turns, 6 h 20 min with your agents, most on site.",
      }
    : {
        monday,
        sunday,
        active_secs: 0,
        turns: 0,
        steps: 0,
        commands: 0,
        files: 0,
        added: 0,
        removed: 0,
        allowed: 0,
        denied: 0,
        answered: 0,
        questions: 0,
        failed: 0,
        top_agent: null,
        top_project: null,
        busiest_day: null,
        longest_secs: 0,
        days: [0, 0, 0, 0, 0, 0, 0],
        headline: "No agent turns that week.",
      };

/** 53 weeks to Friday 2026-10-09, from 2025-10-06: a pattern busier on weekdays and lately. */
function grid(empty: boolean) {
  const first = new Date("2025-10-06T12:00:00");
  return Array.from({ length: 52 * 7 + 5 }, (_, i) => {
    const d = new Date(first);
    d.setDate(first.getDate() + i);
    const day = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const weekend = i % 7 >= 5;
    const level = empty ? 0 : weekend ? (i % 3 === 0 ? 1 : 0) : Math.min(4, ((i * 7) % 5) + (i > 300 ? 1 : 0));
    return { day, turns: level * 3, secs: level * 1500, level };
  });
}

const full = state !== "empty";
const data: Activity = {
  history: state !== "off",
  today: "2026-10-09",
  weeks: full ? ["2026-10-05", "2026-09-28", "2026-09-14"] : ["2026-10-05"],
  week: query.get("week") === "2026-09-28" ? week("2026-09-28", "2026-10-04", true) : week("2026-10-05", "2026-10-11", full),
  grid: grid(!full),
};

const root = document.getElementById("settings")!;
let confirming = state === "confirm";
const draw = () => {
  const page = el("main", { class: "page" }, ...activityPage(data, {
    week: (monday) => {
      document.body.dataset.week = monday;
    },
    history: async (on) => {
      document.body.dataset.history = String(on);
    },
    confirmClear: (on) => {
      confirming = on;
      draw();
    },
    clear: async () => {
      document.body.dataset.cleared = "1";
      confirming = false;
      draw();
    },
  }, confirming));
  root.replaceChildren(page);
};
draw();
