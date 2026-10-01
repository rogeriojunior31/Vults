// The Zeca lab: every clip looping at real pixel size, and a sky where he flies a full sortie.
import type { SessionView, ViewModel } from "../src/bridge";
import { Bird } from "../src/character/director";
import { drawFrame, frameAt } from "../src/character/sprites";
import { createIsland } from "../src/island/render";
import { PERCH_HEIGHT, ZECA } from "../src/character/zeca";

// `?still=1` turns motion off, for screenshots taken at load. Before anything renders.
if (new URLSearchParams(location.search).get("still")) document.body.classList.add("still");

const NOTES: Record<string, string> = {
  idle: "Watching: long holds, a blink, a look back over the shoulder.",
  think: "Head drawn up and still, slow blinks.",
  read: "Head down, scanning along a line, dropping to the next.",
  search: "Neck out, quick turns, a tilt to look closer.",
  edit: "The feeding motion: lean in, strike, tear back.",
  run: "Quick tugs at the wire, feet shuffling for grip.",
  approval: "Sunning pose facing you, head bobbing: needs a human.",
  question: "The curious head tilt, held.",
  done: "A wing stretch, a hop, settle.",
  fail: "Feathers up, a hiss, a shake.",
  sleep: "Fluffed up, head sunk into the shoulders.",
  fly: "Three quick stiff flaps, then a short flat glide.",
};

const speedSel = document.getElementById("speed") as HTMLSelectElement;
const scaleSel = document.getElementById("scale") as HTMLSelectElement;
let speed = Number(speedSel.value);
let scale = Number(scaleSel.value);
speedSel.onchange = () => (speed = Number(speedSel.value));
scaleSel.onchange = () => {
  scale = Number(scaleSel.value);
  sizeCanvases();
};

// ── Clip cards ─────────────────────────────────────────────────────────────────
const CARD_W = 44;
const CARD_H = 32;
const cards: { name: string; canvas: HTMLCanvasElement }[] = [];
const grid = document.getElementById("clips")!;
for (const name of Object.keys(ZECA.clips)) {
  const card = document.createElement("div");
  card.className = "card";
  const canvas = document.createElement("canvas");
  const title = document.createElement("h3");
  title.textContent = name;
  const note = document.createElement("p");
  note.textContent = NOTES[name] ?? "";
  card.append(canvas, title, note);
  grid.append(card);
  cards.push({ name, canvas });
}

const sky = document.getElementById("sky") as HTMLCanvasElement;
const SKY_W = 220;
const SKY_H = 70;

function sizeCanvases(): void {
  for (const { canvas } of cards) {
    canvas.width = CARD_W * scale;
    canvas.height = CARD_H * scale;
    canvas.style.width = `${CARD_W * scale}px`;
  }
  sky.width = SKY_W * scale;
  sky.height = SKY_H * scale;
  sky.style.height = `${SKY_H * scale}px`;
}
sizeCanvases();

function wire(ctx: CanvasRenderingContext2D, y: number, from: number, to: number): void {
  ctx.fillStyle = "#3a3a40";
  ctx.fillRect(from * scale, y * scale, (to - from) * scale, scale);
}

// ── The sortie, from the same director the island uses ──────────────────────────
const WIRE_Y = 60;
const skyBird = new Bird(ZECA, { x: 30, wireY: WIRE_Y, height: PERCH_HEIGHT, skyRight: SKY_W - 10, skyTop: 8, thermal: true });
// Perch a moment, then fly; the director lands it before every new sortie.
let skyWant = "idle";
window.setInterval(() => {
  skyWant = skyWant === "fly" ? "idle" : "fly";
  skyBird.want(skyWant, performance.now());
}, 3000);

// ── The real island, fed made-up views ─────────────────────────────────────────
const islandRoot = document.getElementById("island")!;
// A fake chat backend: streams a canned reply word by word.
let island: ReturnType<typeof createIsland>;
const lab = {
  send: async (text: string) => {
    const reply = `Urubus can smell carrion from more than a kilometre away. You asked: ${text}`;
    for (const word of reply.split(" ")) {
      await new Promise((r) => setTimeout(r, 90));
      island.chat.receive({ kind: "text", text: `${word} ` });
    }
    island.chat.receive({ kind: "done" });
  },
  reset: async (provider: "claude" | "codex" | null) => provider ?? "claude",
  decide: () => {},
  keyboard: () => {},
};
island = createIsland(islandRoot, {
  decide: () => {},
  layout: () => {},
  openAlert: () => {},
  jump: () => {},
  openSettings: () => {},
  setSounds: () => {},
  dismissAlert: () => {},
  chat: lab,
});
const renderIsland = island.render;
const demo = (status: SessionView["status"], activity: SessionView["activity"], step: string | null): SessionView => ({
  id: "lab",
  agent: "claude",
  project: "vultures-ai",
  cwd: "/home/me/vultures-ai",
  status,
  activity,
  step,
  steps: step ? ["Reading README.md", "Searching Bird", step] : [],
  step_count: step ? 12 : 0,
  subagents: 0,
});
const others: SessionView[] = [
  { id: "b", agent: "codex", project: "site", cwd: "/home/me/site", status: "working", activity: "read", step: "Reading README.md", steps: ["Reading README.md"], step_count: 3, subagents: 0 },
  { id: "c", agent: "claude", project: "lazyagents", cwd: "/home/me/lazyagents", status: "thinking", activity: "think", step: null, steps: [], step_count: 0, subagents: 0 },
];
const STATES: [string, ViewModel][] = [
  ["Editing", { sessions: [demo("working", "edit", "Editing scene.ts"), ...others], approval: null, alerts: [] }],
  ["Searching", { sessions: [demo("working", "search", "Searching Bird"), ...others], approval: null, alerts: [] }],
  ["On the web", { sessions: [demo("working", "web", "Browsing docs.rs"), ...others], approval: null, alerts: [] }],
  [
    "Approval",
    {
      sessions: [demo("approval", null, "Running cargo test"), ...others],
      approval: { request: "r", agent: "claude", project: "vultures-ai", tool: "Bash", target: "Bash · cargo test --workspace" },
      alerts: [],
    },
  ],
  [
    "Done",
    {
      sessions: [demo("finished", null, null), ...others],
      approval: null,
      alerts: [
        { key: "a", connector: "github", level: "error", title: "Checks failed on main · me/dog_stack", detail: "fix(rules): align common rules", link: true },
        { key: "b", connector: "github", level: "ok", title: "Approved · me/app#12", detail: "Add the flock", link: true },
      ],
    },
  ],
  ["Chat", { sessions: others, approval: null, alerts: [] }],
];
const stateLabel = document.getElementById("island-state")!;
// `?island=N` pins one state, for screenshots.
const pinned = new URLSearchParams(location.search).get("island");
let stateIndex = pinned === null ? 0 : Number(pinned);
function nextState(): void {
  const [label, view] = STATES[stateIndex % STATES.length];
  stateLabel.textContent = label;
  if (label === "Chat") {
    island.chat.toggle(true);
    island.chat.attach(["/inbox/1790000000000-notes.txt"]);
  } else {
    island.chat.toggle(false);
  }
  renderIsland(view);
  stateIndex++;
}
nextState();
if (pinned === null) window.setInterval(nextState, 6000);
// `?open=1` holds the island open, as if hovered.
if (new URLSearchParams(location.search).get("open")) island.hold(true);


// ── Loop ───────────────────────────────────────────────────────────────────────
let clock = 0;
let last = performance.now();
function tick(now: number): void {
  clock += (now - last) * speed;
  last = now;

  for (const { name, canvas } of cards) {
    const ctx = canvas.getContext("2d")!;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const flying = name === "fly";
    if (!flying) wire(ctx, 27, 0, CARD_W);
    drawFrame(ctx, ZECA, frameAt(ZECA.clips[name], clock), flying ? 3 : 10, flying ? 6 : 27 - PERCH_HEIGHT, scale);
  }

  const ctx = sky.getContext("2d")!;
  ctx.clearRect(0, 0, sky.width, sky.height);
  wire(ctx, WIRE_Y, 0, SKY_W);
  const s = skyBird.shot(performance.now());
  drawFrame(ctx, ZECA, s.frame, s.x, s.y, scale, s.flip);

  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);
