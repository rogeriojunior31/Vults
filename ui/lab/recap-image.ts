// The week as an image to share, for the visual tests: `?hide=1` leaves the project names out.
import { recapImage } from "../src/surfaces/settings/recap-image";

const hideProjects = new URLSearchParams(location.search).get("hide") === "1";
const canvas = recapImage(
  {
    monday: "2026-10-05",
    sunday: "2026-10-11",
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
  },
  { hideProjects },
);
document.getElementById("image")!.append(canvas);
document.body.dataset.ready = "1";
