// The Zeca lab: every clip looping at real pixel size, and a sky where he flies a full sortie.
import type { SessionView, ViewModel } from "../src/bridge";
import { Bird } from "../src/character/director";
import { drawFrame, frameAt } from "../src/character/sprites";
import { Clock } from "../src/clock";
import { createIsland } from "../src/island/render";
import { PERCH_HEIGHT, ZECA } from "../src/character/zeca";

// `?still=1` turns motion off, for screenshots taken at load; `?t=<ms>` freezes every animation
// at that instant (visual tests). Both before anything renders.
const query = new URLSearchParams(location.search);
if (query.get("still")) document.body.classList.add("still");
if (query.get("t") !== null) Clock.freeze(Number(query.get("t")));

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
  swallow: "Down to the wire, pick it up, toss the head back and gulp.",
  gape: "Head up, bill open: something is about to be dropped in.",
  preen: "A bout of grooming, head into the wing: the pointer resting on him.",
  startle: "A jump, wings flung up: a click on him.",
  hello: "He turns to you and waves a wing: the app starting.",
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
  card.className = "clip-card";
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
  skyBird.want(skyWant, Clock.now());
}, 3000);

// ── The real island, fed made-up views ─────────────────────────────────────────
const islandRoot = document.getElementById("island")!;
// A fake chat backend: streams a canned reply word by word.
let labStopped = false;
let island: ReturnType<typeof createIsland>;
const lab = {
  send: async (text: string) => {
    labStopped = false;
    const reply = `Urubus can smell carrion from more than a kilometre away. You asked: ${text}`;
    for (const word of reply.split(" ")) {
      await new Promise((r) => setTimeout(r, 90));
      if (labStopped) return;
      island.chat.receive({ kind: "text", text: `${word} ` });
    }
    island.chat.receive({ kind: "done" });
  },
  reset: async (provider: "claude" | "codex" | "api" | null) => provider ?? "claude",
  decide: () => {},
  stop: () => {
    labStopped = true;
    island.chat.receive({ kind: "stopped" });
  },
  keyboard: () => {},
};
island = createIsland(islandRoot, {
  decide: () => {},
  decideAlways: () => {},
  layout: () => {},
  openAlert: () => {},
  jump: () => {},
  openSettings: () => {},
  setSounds: () => {},
  dismissAlert: () => {},
  chat: lab,
});
const renderIsland = island.render;
const demo = (status: SessionView["status"], activity: SessionView["activity"], step: string | null, note: string | null = null): SessionView => ({
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
  note,
  editor: "Cursor",
});
const others: SessionView[] = [
  { id: "b", agent: "codex", project: "site", cwd: "/home/me/site", status: "working", activity: "read", step: "Reading README.md", steps: ["Reading README.md"], step_count: 3, subagents: 0, note: null, editor: "VS Code" },
  { id: "c", agent: "claude", project: "lazyagents", cwd: "/home/me/lazyagents", status: "thinking", activity: "think", step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null },
];
const STATES: [string, ViewModel][] = [
  ["Editing", { sessions: [demo("working", "edit", "Editing scene.ts"), ...others], approval: null, alerts: [] }],
  ["Searching", { sessions: [demo("working", "search", "Searching Bird"), ...others], approval: null, alerts: [] }],
  ["On the web", { sessions: [demo("working", "web", "Browsing docs.rs"), ...others], approval: null, alerts: [] }],
  [
    "Approval",
    {
      sessions: [demo("approval", null, "Running cargo test"), ...others],
      approval: {
        request: "r",
        agent: "claude",
        session: "lab",
        project: "vultures-ai",
        tool: "Bash",
        target: "Bash · cargo test --workspace && cargo clippy --workspace…",
        description: "Run the test suite, then the linter",
        full: "cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings",
        added: 0,
        removed: 0,
        queue: 1,
      },
      alerts: [],
    },
  ],
  [
    "Done",
    {
      sessions: [demo("finished", null, "Running cargo test", "All 42 tests pass. I also fixed the flaky timeout in the ipc tests."), ...others],
      approval: null,
      alerts: [
        { key: "a", connector: "github", level: "error", title: "Checks failed on main · me/dog_stack", detail: "fix(rules): align common rules", link: true },
        { key: "b", connector: "github", level: "ok", title: "Approved · me/app#12", detail: "Add the flock", link: true },
      ],
    },
  ],
  ["Chat", { sessions: others, approval: null, alerts: [] }],
  [
    "Busy flock",
    {
      sessions: [
        demo("working", "run", "Running cargo test"),
        { ...others[0], status: "finished", activity: null, note: "Done." },
        { ...others[1], status: "failed", activity: null, note: "API Error: 529 overloaded" },
        { id: "d", agent: "codex", project: "docs", cwd: "/home/me/docs", status: "question", activity: null, step: null, steps: [], step_count: 1, subagents: 0, note: null, editor: null },
        { id: "e", agent: "claude", project: "api", cwd: "/home/me/api", status: "working", activity: "edit", step: "Editing main.rs", steps: ["Editing main.rs"], step_count: 9, subagents: 0, note: null, editor: null },
      ],
      approval: null,
      alerts: [],
    },
  ],
  [
    "Approval queue",
    {
      sessions: [demo("approval", null, "Editing views.ts"), { ...others[0], status: "approval", activity: null }, ...others.slice(1)],
      approval: {
        request: "q1",
        agent: "claude",
        session: "lab",
        project: "vultures-ai",
        tool: "Edit",
        target: "Edit · /home/me/vultures-ai/ui/src/island/views.ts",
        description: null,
        full: null,
        added: 12,
        removed: 3,
        queue: 3,
      },
      alerts: [],
    },
  ],
  [
    "Idle flock",
    {
      sessions: [demo("idle", null, "Running cargo test"), { ...others[0], status: "idle", activity: null }, { ...others[1], status: "idle", activity: null }],
      approval: null,
      alerts: [],
    },
  ],
  ["Chat permission", { sessions: others, approval: null, alerts: [] }],
  ["Question", { sessions: [demo("question", null, "Reading settings.ts", "Which theme should the settings use by default, black or noite?"), ...others], approval: null, alerts: [] }],
  ["Failed", { sessions: [demo("failed", null, "Running cargo test", "API Error: 529 overloaded. The request was not retried."), ...others], approval: null, alerts: [] }],
];
const stateLabel = document.getElementById("island-state")!;
// `?island=N` pins one state, for screenshots.
const pinned = new URLSearchParams(location.search).get("island");
let stateIndex = pinned === null ? 0 : Number(pinned);
function nextState(): void {
  const [label, view] = STATES[stateIndex % STATES.length];
  stateLabel.textContent = label;
  if (label === "Chat permission") {
    island.chat.toggle(true);
    island.chat.attach([], [{ name: "photos", reason: "folder" }]);
    island.chat.receive({ kind: "text", text: "I'll run the tests first." });
    island.chat.receive({
      kind: "permission",
      id: "c1",
      tool: "Bash",
      target: "Bash · cargo test --workspace",
      description: "Run the test suite",
      full: null,
      added: 0,
      removed: 0,
    });
  } else if (label === "Chat") {
    island.chat.toggle(true);
    island.chat.receive({
      kind: "text",
      text: "The island folds when you leave it. Two things keep it open:\n\n- a **permission card** that is still waiting\n- the `chat` itself\n\nTo run the lab:\n\n```\nnpm run dev\n```\n",
    });
    island.chat.receive({ kind: "done" });
  } else {
    island.chat.toggle(false);
  }
  renderIsland(view);
  stateIndex++;
}
// `?empty=1`: nobody on the wire, to see the empty island and how it hides.
if (query.get("empty")) {
  stateLabel.textContent = "Empty";
  renderIsland({ sessions: [], approval: null, alerts: [] });
} else {
  nextState();
  if (pinned === null) window.setInterval(nextState, 6000);
}
// `?open=1` holds the island open, as if hovered.
if (new URLSearchParams(location.search).get("open")) island.hold(true);
// As the desktop does when it binds the global shortcuts.
island.setKeys({ allow: "Control+Alt+Y", deny: "Control+Alt+N" });
// `?greet=1`: the hello the app plays at start-up.
if (query.get("greet")) island.greet();
// `?dropped=1`: a file was just dropped on the island.
if (query.get("dropped")) island.chat.attach(["/inbox/1700000000000-report.pdf"]);
// `?drag=1`: a file is being dragged over the island.
if (query.get("drag")) island.chat.setDragOver(true);
// `?api=1` acts as if an API key were saved, to show that chat choice.
if (query.get("api")) island.chat.setApiKey(true);


// ── Loop ───────────────────────────────────────────────────────────────────────
let clock = 0;
let last = performance.now();
function tick(now: number): void {
  clock = query.get("t") !== null ? Clock.now() : clock + (now - last) * speed;
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
  const s = skyBird.shot(Clock.now());
  drawFrame(ctx, ZECA, s.frame, s.x, s.y, scale, s.flip);

  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);
