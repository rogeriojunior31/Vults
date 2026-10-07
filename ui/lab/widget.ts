// The corner widget's lab: every state side by side, fed made-up views. Development only
// (`npm run dev`, then /lab/widget.html). `?still=1&t=<ms>` freezes time for the visual tests;
// `?nozeca=1` turns Zeca off.
import type { SessionView, ViewModel } from "../src/bridge";
import { Clock } from "../src/clock";
import { setZeca } from "../src/island/flock";
import { createWidget } from "../src/widget/render";

const query = new URLSearchParams(location.search);
if (query.get("still")) document.body.classList.add("still");
if (query.get("t") !== null) Clock.freeze(Number(query.get("t")));
if (query.get("nozeca")) setZeca(false);

const session = (id: string, species: string, status: SessionView["status"], attention: SessionView["attention"], activity: SessionView["activity"] = null): SessionView => ({
  id,
  agent: id === "b" ? "codex" : "claude",
  project: id,
  cwd: `/home/me/${id}`,
  status,
  attention,
  card: false,
  activity,
  step: null,
  steps: [],
  step_count: 0,
  subagents: 0,
  note: null,
  editor: null,
  species,
});
const view = (sessions: SessionView[], front: SessionView | null, card = false): ViewModel => ({
  sessions,
  approval: card && front
    ? { request: "r", agent: front.agent, session: front.id, project: front.project, tool: "Bash", target: "Bash · cargo test", description: null, full: null, added: 0, removed: 0, questions: [], queue: 1 }
    : null,
  alerts: [],
  front: front && { agent: front.agent, id: front.id },
});

const editing = session("vults", "atratus", "working", "quiet", "edit");
const reading = session("b", "aura", "working", "quiet", "read");
const thinking = session("lazyagents", "burrovianus", "thinking", "quiet");
const asking = { ...session("site", "papa", "approval", "needs-you"), card: true };
const done = session("notes", "vultur", "finished", "done");

const STATES: [string, ViewModel, boolean][] = [
  ["empty", view([], null), false],
  ["one", view([editing], editing), false],
  ["busy", view([editing, reading, thinking, done], editing), false],
  // The card's session is in front (core's rule); the rest by how much they want you.
  ["card", view([editing, done, asking, reading], asking, true), false],
  ["resting", view([done], done), false],
  ["paused", view([editing, reading], editing), true],
];

const box = document.getElementById("states")!;
const only = query.get("state");
for (const [name, v, paused] of STATES) {
  if (only && only !== name) continue;
  const root = document.createElement("main");
  root.className = "widget";
  const figure = document.createElement("figure");
  figure.dataset.state = name;
  const caption = document.createElement("figcaption");
  caption.textContent = name;
  figure.append(root, caption);
  box.append(figure);
  const widget = createWidget(root, { open: () => {}, focus: () => {} });
  widget.render(v);
  if (paused) widget.setPaused(true);
}
