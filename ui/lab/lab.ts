// The Zeca lab: every clip looping at real pixel size, and a sky where he flies a full sortie.
import type { Diff, NowPlaying, Outcome, SessionView, ViewModel } from "../src/bridge";
import { Bird } from "../src/character/director";
import { drawFrame, frameAt } from "../src/character/sprites";
import { Clock } from "../src/clock";
import { createIsland } from "../src/island/render";
import { PERCH_HEIGHT, perchOf } from "../src/character/zeca";
import { SPECIES, speciesSet } from "../src/character/flock";
import { setZecaLook, setZecaSpecies } from "../src/island/flock";
import { setPlace } from "../src/island/dock";
import { LOOK_IDS, dress } from "../src/character/looks";

// `?still=1` turns motion off, for screenshots taken at load; `?t=<ms>` freezes every animation
// at that instant (visual tests). Both before anything renders.
const query = new URLSearchParams(location.search);
if (query.get("still")) document.body.classList.add("still");
if (query.get("t") !== null) Clock.freeze(Number(query.get("t")));

// `?zeca=<id>` makes the island's Zeca that species, as the Flock settings do.
if (query.get("zeca")) setZecaSpecies(query.get("zeca")!);

// `?look=<id>` dresses Zeca (the clips, the sky and the island's Zeca) in a seasonal look.
const LOOK = query.get("look");
setZecaLook(LOOK);
const lookSel = document.getElementById("look") as HTMLSelectElement;
for (const id of ["", ...LOOK_IDS]) lookSel.add(new Option(id || "none", id, false, id === (LOOK ?? "")));
lookSel.onchange = () => {
  if (lookSel.value) query.set("look", lookSel.value);
  else query.delete("look");
  location.search = query.toString();
};

// `?species=<id>` shows another vulture on Zeca's rig; without it, Zeca.
const SET = dress(speciesSet(query.get("species") ?? "atratus"), LOOK);
const PERCH = perchOf(SET);
const speciesSel = document.getElementById("species") as HTMLSelectElement;
for (const s of SPECIES) speciesSel.add(new Option(`${s.name} (${s.latin})`, s.id, false, s.id === (query.get("species") ?? "atratus")));
speciesSel.onchange = () => {
  query.set("species", speciesSel.value);
  location.search = query.toString();
};

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
// Big species need a bigger card: room for the taller body, the raised neck and the wider wings.
const CARD_WIRE = 27 + (PERCH - PERCH_HEIGHT) + (SPECIES.find((s) => s.id === query.get("species"))?.neck ? 4 : 0);
const CARD_W = Math.max(44, Math.max(...SET.parts.glide.map((r) => r.length)) + 6);
const CARD_H = CARD_WIRE + 5;
const cards: { name: string; canvas: HTMLCanvasElement }[] = [];
const grid = document.getElementById("clips")!;
for (const name of Object.keys(SET.clips)) {
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
const skyBird = new Bird(SET, { x: 30, wireY: WIRE_Y, height: PERCH, skyRight: SKY_W - 10, skyTop: 8, thermal: true });
// Perch a moment, then fly; the director lands it before every new sortie.
let skyWant = "idle";
window.setInterval(() => {
  skyWant = skyWant === "fly" ? "idle" : "fly";
  skyBird.want(skyWant, Clock.now());
}, 3000);

// ── The real island, fed made-up views ─────────────────────────────────────────
const islandRoot = document.getElementById("island")!;
// `?presence=quiet|paused|panel`: a presence preset, as the app's Settings or tray set it.
const presence = query.get("presence");
if (presence === "quiet" || presence === "paused" || presence === "panel") setPlace({ presence, dock: "top" }, () => false);
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
  // A fake microphone: a made-up level every 60 ms, the words so far every 800 ms (as on a GPU),
  // and a canned transcript.
  voice: {
    start: async () => {
      labVoice = window.setInterval(() => island.chat.voiceLevel(Math.abs(Math.sin(Date.now() / 180)) * Math.random()), 60);
      const words = LAB_SAID.split(" ");
      let heard = 0;
      labPartial = window.setInterval(() => {
        heard = Math.min(words.length, heard + 2);
        island.chat.voicePartial(words.slice(0, heard).join(" "));
      }, 800);
    },
    stop: async () => {
      window.clearInterval(labVoice);
      window.clearInterval(labPartial);
      await new Promise((r) => setTimeout(r, 900));
      return LAB_SAID;
    },
    cancel: () => {
      window.clearInterval(labVoice);
      window.clearInterval(labPartial);
    },
  },
};
const LAB_SAID = "Why is the build failing on the release branch?";
let labVoice = 0;
let labPartial = 0;
/** As core answers a click on the card: it leaves the line, its session works again, and the view
 *  says how it ended. */
function endCard(request: string, outcome: Outcome): void {
  const v = island.last();
  if (v.approval?.request !== request) return;
  const { agent, session } = v.approval;
  island.render({
    ...v,
    approval: null,
    sessions: v.sessions.map((s) => (s.card ? { ...s, status: "working", attention: "quiet", card: false } : s)),
    ended: [{ request, agent, session, outcome }, ...(v.ended ?? [])],
    // As core: with the card gone, the user's focus is in front again.
    front: v.focus ?? null,
  });
}
island = createIsland(islandRoot, {
  decide: (request, decision) => endCard(request, decision === "allow" ? "allowed" : "denied"),
  decideAlways: (request) => endCard(request, "allowed"),
  answer: (request) => endCard(request, "answered"),
  release: (request) => endCard(request, "released"),
  layout: () => {},
  openAlert: () => {},
  // The tests read what a quick action asked for.
  jump: (agent, id) => {
    document.body.dataset.opened = `jump ${agent}:${id}`;
  },
  openFolder: (agent, id) => {
    document.body.dataset.opened = `folder ${agent}:${id}`;
  },
  openFile: (agent, id, step, file) => {
    document.body.dataset.opened = `file ${agent}:${id} ${step} ${file}`;
  },
  // As core keeps a project's choices: every session in that folder follows; hidden ones leave
  // unless their card waits.
  projectPref: (agent, id, pref, on) => {
    const v = island.last();
    const cwd = v.sessions.find((s) => s.agent === agent && s.id === id)?.cwd;
    if (!cwd) return;
    const same = (s: SessionView) => s.cwd === cwd;
    let sessions = v.sessions.map((s) => (same(s) && pref !== "hide" ? { ...s, [pref === "mute" ? "muted" : "pinned"]: on } : s));
    if (pref === "hide" && on) sessions = sessions.filter((s) => !same(s) || s.card);
    // Pinned first, as core orders them.
    sessions = [...sessions].sort((a, b) => Number(!!b.pinned) - Number(!!a.pinned));
    island.render({ ...v, sessions });
  },
  // As core: do not disturb ends, and the next view says so.
  endDnd: () => {
    document.body.dataset.dndEnded = "1";
    island.render({ ...island.last(), dnd: false });
  },
  // As core: the flag goes; the tests read which answer it got.
  hush: (agent, id, hush) => {
    document.body.dataset.opened = `hush ${agent}:${id} ${hush}`;
    const v = island.last();
    island.render({ ...v, sessions: v.sessions.map((s) => (s.agent === agent && s.id === id ? { ...s, silent: null } : s)) });
  },
  unfocus: () => {
    const v = island.last();
    island.render({ ...v, focus: null });
  },
  // As core keeps a focus: the session goes in front unless a card waits.
  focus: (agent, id) => {
    const v = island.last();
    const card = v.sessions.find((s) => s.card);
    island.render({ ...v, focus: { agent, id }, front: card ? { agent: card.agent, id: card.id } : { agent, id } });
  },
  stepDiff: async () => LAB_DIFF,
  openSettings: () => {},
  opened: () => {},
  // A fake player: play/pause toggles the song, skipping changes it.
  media: (action) => {
    if (!labSong) return;
    labSong = action === "playpause" ? { ...labSong, playing: !labSong.playing } : { ...labSong, title: action === "next" ? "Lucky" : "Airbag" };
    island.setMedia(labSong);
  },
  setSounds: () => {},
  dismissAlert: () => {},
  // The tests read what a click on a card's row asked to open.
  openRow: (connector, item) => {
    document.body.dataset.opened = `${connector} ${item}`;
  },
  // `?stale=1`: the connector's last poll failed, twelve minutes after its last good one.
  connectorStatus: async () => (query.get("stale") ? { lastOk: Date.now() / 1000 - 720, error: "GitHub did not answer (HTTP 502)" } : null),
  chat: lab,
});
// `?flock=world` gives the sessions the world's tallest vultures, as the core would with that pool.
const TALL_WORLD = ["gyps-himalayensis", "vultur", "aegypius", "torgos", "gyps-fulvus", "gymnogyps"];
// `?visitor=1` brings a rare visitor across the sky once the island is up.
if (query.get("visitor")) window.setTimeout(() => island.visitNow(), 500);
// `?scouts=N` gives the first session N running subagents: their scouts circle near its bird.
const SCOUTS = Number(query.get("scouts") ?? 0);
const renderIsland = (view: ViewModel) => {
  let sessions = view.sessions;
  if (query.get("flock") === "world") sessions = sessions.map((s, i) => ({ ...s, species: TALL_WORLD[i % TALL_WORLD.length] }));
  if (SCOUTS) sessions = sessions.map((s, i) => (i === 0 ? { ...s, subagents: SCOUTS } : s));
  // `?dnd=1`: do not disturb is on (the moon in the header).
  if (query.get("dnd") && !document.body.dataset.dndEnded) view = { ...view, dnd: true };
  // `?raise=1`: on KDE, where Open terminal brings the window forward.
  if (query.get("raise")) sessions = sessions.map((s) => ({ ...s, raise: true }));
  island.render({ ...view, sessions });
};
// For the tests: shortcuts and states driven from Playwright.
Object.assign(window, { island });
const demo = (status: SessionView["status"], activity: SessionView["activity"], step: string | null, note: string | null = null): SessionView => ({
  id: "lab",
  species: "atratus",
  agent: "claude",
  project: "vultures-ai",
  cwd: "/home/me/vultures-ai",
  status,
  // Fixtures whose status is not quiet override these, as core would.
  attention: "quiet",
  card: false,
  activity,
  step,
  steps: step ? ["Reading README.md", "Searching Bird", step] : [],
  step_count: step ? 12 : 0,
  subagents: 0,
  note,
  editor: "Cursor",
});
const others: SessionView[] = [
  { id: "b", species: "aura", agent: "codex", project: "site", cwd: "/home/me/site", status: "working", attention: "quiet", card: false, activity: "read", step: "Reading README.md", steps: ["Reading README.md"], step_count: 3, subagents: 0, note: null, editor: "VS Code" },
  { id: "c", species: "burrovianus", agent: "claude", project: "lazyagents", cwd: "/home/me/lazyagents", status: "thinking", attention: "quiet", card: false, activity: "think", step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null },
];
/** What the "Live diff" state's edit changed. */
const LAB_DIFF: Diff = {
  cut: false,
  files: [
    {
      path: "/home/me/vultures-ai/ui/src/island/ticker.ts",
      added: 4,
      removed: 2,
      hunks: [
        {
          old_start: 8,
          new_start: 8,
          lines: [" const SLIDE_MS = 380;", "-const MAX_QUEUE = 3;", "+const MAX_QUEUE = 4;", " ", " export class Ticker {"],
        },
        {
          old_start: 41,
          new_start: 41,
          lines: ["   private next(): void {", "-    if (this.sliding) return;", "+    if (this.sliding || this.queue.length === 0) return;", "+    const step = this.queue.shift()!;", "+    this.sliding = true;", "   }"],
        },
      ],
    },
  ],
};
const STATES: [string, ViewModel][] = [
  ["Editing", { sessions: [demo("working", "edit", "Editing scene.ts"), ...others], approval: null, alerts: [] }],
  ["Searching", { sessions: [demo("working", "search", "Searching Bird"), ...others], approval: null, alerts: [] }],
  ["On the web", { sessions: [demo("working", "web", "Browsing docs.rs"), ...others], approval: null, alerts: [] }],
  [
    "Approval",
    {
      sessions: [{ ...demo("approval", null, "Running cargo test"), attention: "needs-you", card: true }, ...others],
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
        questions: [],
        queue: 1,
      },
      alerts: [],
    },
  ],
  [
    "Done",
    {
      sessions: [{ ...demo("finished", null, "Running cargo test", "All 42 tests pass. I also fixed the flaky timeout in the ipc tests."), attention: "done" }, ...others],
      approval: null,
      alerts: [
        { key: "a", seq: 1, connector: "github", level: "error", title: "Checks failed on main · me/dog_stack", detail: "fix(rules): align common rules", link: true },
        { key: "b", seq: 2, connector: "github", level: "ok", title: "Approved · me/app#12", detail: "Add the flock", link: true },
      ],
    },
  ],
  ["Chat", { sessions: others, approval: null, alerts: [] }],
  [
    "Busy flock",
    {
      sessions: [
        demo("working", "run", "Running cargo test"),
        { ...others[0], status: "finished", attention: "done", activity: null, note: "Done." },
        { ...others[1], status: "failed", attention: "failed", activity: null, note: "API Error: 529 overloaded" },
        { id: "d", species: "melambrotus", agent: "codex", project: "docs", cwd: "/home/me/docs", status: "question", attention: "needs-you", card: false, activity: null, step: null, steps: [], step_count: 1, subagents: 0, note: null, editor: null },
        { id: "e", species: "atratus", agent: "claude", project: "api", cwd: "/home/me/api", status: "working", attention: "quiet", card: false, activity: "edit", step: "Editing main.rs", steps: ["Editing main.rs"], step_count: 9, subagents: 0, note: null, editor: null },
      ],
      approval: null,
      alerts: [],
    },
  ],
  [
    "Approval queue",
    {
      sessions: [{ ...demo("approval", null, "Editing views.ts"), attention: "needs-you", card: true }, { ...others[0], status: "approval", attention: "needs-you", activity: null }, ...others.slice(1)],
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
        questions: [],
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
  ["Question", { sessions: [{ ...demo("question", null, "Reading settings.ts", "Which theme should the settings use by default, black or noite?"), attention: "needs-you" }, ...others], approval: null, alerts: [] }],
  ["Failed", { sessions: [{ ...demo("failed", null, "Running cargo test", "API Error: 529 overloaded. The request was not retried."), attention: "failed" }, ...others], approval: null, alerts: [] }],
  [
    "Gemini",
    {
      sessions: [
        { id: "g", species: "burrovianus", agent: "gemini", project: "notes", cwd: "/home/me/notes", status: "working", attention: "quiet", card: false, activity: "run", step: "Running npm test", steps: ["Reading package.json", "Running npm test"], step_count: 4, subagents: 0, note: null, editor: null },
        { id: "h", species: "melambrotus", agent: "gemini", project: "blog", cwd: "/home/me/blog", status: "question", attention: "needs-you", card: false, activity: null, step: null, steps: [], step_count: 2, subagents: 0, note: "Run rm -rf dist? Answer in Gemini's terminal.", editor: null },
        ...others,
      ],
      approval: null,
      alerts: [],
    },
  ],
  [
    "Question card",
    {
      sessions: [{ ...demo("question", null, "Reading settings.ts", "Which theme should the settings use by default?"), attention: "needs-you", card: true }, ...others],
      approval: {
        request: "ask1",
        agent: "claude",
        session: "lab",
        project: "vultures-ai",
        tool: "AskUserQuestion",
        target: "AskUserQuestion",
        description: null,
        full: null,
        added: 0,
        removed: 0,
        questions: [
          {
            question: "Which theme should the settings use by default?",
            header: "Theme",
            options: [
              { label: "Black", description: "Pure black, for OLED screens" },
              { label: "Noite", description: "The dark blue the island uses" },
              { label: "Follow the system", description: null },
            ],
            multi: false,
          },
          {
            question: "Which pages should get the new theme first?",
            header: "Pages",
            options: [
              { label: "Agents", description: null },
              { label: "Chat", description: null },
              { label: "Connectors", description: null },
            ],
            multi: true,
          },
        ],
        queue: 1,
      },
      alerts: [],
    },
  ],
  [
    "Live diff",
    {
      sessions: [
        {
          ...demo("working", "run", "Running npm test"),
          steps: ["Reading README.md", "Editing ticker.ts", "Running npm test"],
          diffs: [null, { step: 11, added: 4, removed: 2, files: 1 }, null],
        },
        ...others,
      ],
      approval: null,
      alerts: [],
    },
  ],
  [
    "GitHub card",
    {
      sessions: [demo("working", "edit", "Editing board.ts"), ...others],
      approval: null,
      alerts: [{ key: "pr:me/vultures-ai#56:ci-failed:p1", seq: 1, connector: "github", level: "error", title: "Checks failed · me/vultures-ai#56", detail: "GitHub card: open PRs, reviews, branch checks", link: true }],
      boards: [
        {
          connector: "github",
          rows: [
            { item: "pr:me/vultures-ai#56", group: "yours", name: "vultures-ai#56", title: "GitHub card: open PRs, reviews, branch checks", checks: "failing", review: null, link: true },
            { item: "pr:me/vultures-ai#55", group: "yours", name: "vultures-ai#55", title: "Seasonal looks for Zeca", checks: "running", review: "changes", link: true },
            { item: "pr:me/site#9", group: "yours", name: "site#9", title: "Dark mode for the docs", checks: "passing", review: "approved", link: true },
            { item: "review:team/lib#7", group: "to-review", name: "lib#7", title: "Bump serde to 1.0.220", checks: null, review: null, link: true },
            { item: "branch:me/vultures-ai", group: "branches", name: "vultures-ai", title: "main · Merge pull request #54", checks: "passing", review: null, link: true },
            { item: "branch:me/site", group: "branches", name: "site", title: "main · Fix the landing page", checks: "failing", review: null, link: true },
          ],
        },
      ],
    },
  ],
  // A run stuck 15 minutes on a command (loud), and another bird quiet for 5.
  [
    "Quiet bird",
    {
      sessions: [
        { ...demo("working", "run", "Running npm run deploy"), silent: "loud" },
        { ...others[0], silent: "quiet" },
        others[1],
      ],
      approval: null,
      alerts: [],
    },
  ],
];
const stateLabel = document.getElementById("island-state")!;
// `?state=<name>` pins one state by its label in kebab case (`busy-flock`), for screenshots: a new
// state moves no other. `?island=N` pins one by index.
const slug = (label: string) => label.toLowerCase().replaceAll(" ", "-");
const named = query.get("state");
const pinned = named ?? query.get("island");
let stateIndex = named !== null ? STATES.findIndex(([label]) => slug(label) === named) : pinned === null ? 0 : Number(pinned);
if (stateIndex < 0) throw new Error(`no lab state "${named}": ${STATES.map(([label]) => slug(label)).join(", ")}`);
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
// `?nozeca=1`: Zeca switched off (Settings → Flock): no chat, the front session keeps its bird.
if (query.get("nozeca")) island.setZeca(false);
// `?menu=<id>`: that session's quick actions, as a right-click on its bird opens them; `&editor=1`
// as if VS Code were installed.
if (query.get("editor")) island.setEditor(true);
if (query.get("menu")) window.setTimeout(() => island.openMenu("claude", query.get("menu")!), 0);
// `?looks=1`: Zeca's looks, as a right-click on him opens them (with `open=1`).
if (query.get("looks")) window.setTimeout(() => island.openLooks(), 0);
// `?empty=1`: nobody on the wire, to see the empty island and how it hides.
if (query.get("empty")) {
  stateLabel.textContent = "Empty";
  renderIsland({ sessions: [], approval: null, alerts: [] });
} else {
  nextState();
  if (pinned === null) window.setInterval(nextState, 6000);
}
// `?churn=1`: the same view again every 40 ms, like a busy agent's events, for click tests.
if (query.get("churn")) window.setInterval(() => renderIsland(island.last()), 40);
// `?open=1` holds the island open, as if hovered.
if (new URLSearchParams(location.search).get("open")) island.hold(true);
// As the desktop does when it binds the global shortcuts.
island.setKeys({ allow: "Control+Alt+Y", deny: "Control+Alt+N" });
// `?greet=1`: the hello the app plays at start-up; `&name=Ana` greets someone by name.
if (query.get("greet")) island.greet(query.get("name"));
// `?dropped=1`: a file was just dropped on the island.
if (query.get("dropped")) island.chat.attach(["/inbox/1700000000000-report.pdf"]);
// `?drag=1`: a file is being dragged over the island.
if (query.get("drag")) island.chat.setDragOver(true);
// `?music=1`: a song is playing (Settings → Now playing).
let labSong: NowPlaying | null = null;
if (query.get("music")) {
  labSong = { title: "Paranoid Android", artist: "Radiohead", playing: true };
  island.setMedia(labSong);
}
// `?usage=1`: the subscriptions' usage, one window close to its limit.
if (query.get("usage")) {
  const later = 4_102_444_800; // 2100: never past its reset, whatever the clock says.
  island.setUsage([
    { agent: "claude", minutes: 300, used_percent: 23, resets_at: later },
    { agent: "claude", minutes: 10080, used_percent: 74, resets_at: later },
    { agent: "codex", minutes: 10080, used_percent: 12, resets_at: later },
  ]);
}
// `?api=1` acts as if an API key were saved, to show that chat choice; `?api=OpenRouter` names
// the provider.
const api = query.get("api");
if (api) island.chat.setApi({ ready: true, label: api === "1" ? "Anthropic" : api });
// The lab has a voice model.
island.chat.setVoiceReady(true);
// `?voice=listening|transcribing`: the chat hearing the user, with a fixed waveform.
const voiceState = query.get("voice");
if (voiceState === "listening" || voiceState === "transcribing") {
  island.chat.toggle(true);
  const levels = Array.from({ length: 32 }, (_, i) => Math.abs(Math.sin(i * 0.7)) * (0.3 + 0.7 * ((i * 37) % 11) / 10));
  // `&partial=1`: the words heard so far, dimmed (a long phrase, so its start slides out). They
  // only come while listening.
  if (query.get("partial")) {
    island.chat.showVoice("listening");
    island.chat.voicePartial("Why is the build failing on the release branch after the merge of the voice partials");
  }
  island.chat.showVoice(voiceState, levels);
}


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
    if (!flying) wire(ctx, CARD_WIRE, 0, CARD_W);
    drawFrame(ctx, SET, frameAt(SET.clips[name], clock), flying ? 3 : 10, flying ? 6 : CARD_WIRE - PERCH, scale);
  }

  const ctx = sky.getContext("2d")!;
  ctx.clearRect(0, 0, sky.width, sky.height);
  wire(ctx, WIRE_Y, 0, SKY_W);
  const s = skyBird.shot(Clock.now());
  drawFrame(ctx, SET, s.frame, s.x, s.y, scale, s.flip);

  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);
