// The Nest's lab: its Chat tab with a reply in full Markdown and a card it can only send to the
// island, and its Roosts tab (`?tab=roosts`). Development only (`npm run dev`, then /lab/nest.html).
import type { MemoryView, RoostView, TaskRow } from "../src/bridge";
import { Clock } from "../src/clock";
import type { Activity } from "../src/surfaces/settings/activity";
import type { SessionView, ViewModel } from "../src/view.gen";
import { createNest } from "../src/surfaces/nest/render";

const REPLY = `### What changed

The island now **folds** when you leave it. Two things keep it open:

1. a permission card still waiting
2. the \`chat\` itself

| Setting | Default |
| --- | --- |
| Fold after | 15 s |
| Open on hover | off |

> A card keeps the island open until you answer it.

See [the guide](https://example.com/guide) for more.

---

\`\`\`sh
npm run dev
\`\`\``;

const nop = async () => {};
const ROOSTS: RoostView[] = [
  {
    key: "github:rogeriojunior31/vults",
    name: "rogeriojunior31/Vults",
    github: "rogeriojunior31/Vults",
    trees: [
      { path: "~/Projects/Vults", branch: "main" },
      { path: "~/Projects/Vults-m3", branch: "feat/m3-roosts" },
    ],
    sessions: [
      { agent: "Claude Code", status: "working", step: "Running cargo test" },
      { agent: "Codex", status: "approval", step: null },
    ],
    prs: [
      { name: "Vults#243", title: "M3: roosts", checks: "running", review: null, item: "pr:rogeriojunior31/Vults#243", link: true },
      { name: "Vults#242", title: "M2: the frozen and volatile context", checks: "passing", review: "approved", item: "pr:rogeriojunior31/Vults#242", link: true },
    ],
    checks: "passing",
    checksItem: "branch:rogeriojunior31/Vults",
    memory: ["Merges with --merge, never squash.", "Every UI sentence goes through t() in four languages."],
    agents: "offer",
    cost: { today: 1.25, total: 8.4, subscription: true },
  },
  {
    key: "local:/home/me/Projects/site/.git",
    name: "site",
    github: null,
    trees: [{ path: "~/Projects/site", branch: null }],
    sessions: [{ agent: "Gemini CLI", status: "idle", step: null }],
    prs: [],
    checks: null,
    checksItem: null,
    memory: [],
    agents: "",
    cost: null,
  },
];

const MEMORY: MemoryView = {
  on: true,
  files: [
    { place: "user", path: "system/USER.md", facts: ["Prefers answers in Brazilian Portuguese.", "Works on Vults, a Tauri app."], used: 74, limit: 1375 },
    { place: "memory", path: "system/MEMORY.md", facts: ["Merges with --merge, never squash."], used: 37, limit: 2200 },
    { place: "persona", path: "system/persona.md", facts: [], used: 0, limit: 2000 },
  ],
  pending: [
    { id: 1, op: "add", place: "roost:rogeriojunior31/Vults", folder: "~/Projects/Vults", old: "", new: "Every UI sentence goes through t() in four languages.", reason: "You said it twice this week.", source: "extraction" },
    { id: 2, op: "replace", place: "user", folder: "", old: "Uses zsh.", new: "Uses fish as the shell.", reason: "You switched shells.", source: "tool" },
  ],
  history: [
    { id: "a1", message: "Remember in system/MEMORY.md: Merges with --merge, never squash.", by: "you", seconds: 0 },
    { id: "a2", message: "Remember in system/USER.md: Works on Vults, a Tauri app.", by: "zeca", seconds: 0 },
  ],
  accepted: 3,
  rejected: 1,
  night: { day: "2026-10-11", debrief: "You finished the recall step and opened the memory tab; the Windows CI fix is still open." },
};

const session = (id: string, over: Partial<SessionView>): SessionView => ({
  id,
  agent: "claude",
  project: "Vults",
  cwd: "/home/me/Projects/Vults",
  status: "working",
  attention: "quiet",
  card: false,
  activity: "edit",
  step: "Editing render.ts",
  steps: [],
  step_count: 12,
  subagents: 0,
  note: null,
  editor: null,
  species: "atratus",
  bird_chosen: false,
  roost_key: "github:rogeriojunior31/vults",
  roost: "rogeriojunior31/Vults",
  branch: "main",
  ...over,
});

const VIEW: ViewModel = {
  sessions: [
    session("a", { status: "approval", attention: "needs-you", card: true, project: "Vults-quick-wins", branch: "fix/quick-wins-2", step: null, activity: null, raise: true }),
    session("b", { steps: ["Reading render.ts", "Searching Bird", "Editing render.ts"], diffs: [null, null, { step: 12, added: 78, removed: 14, files: 2 }], cost_usd: 1.37, editor: "VS Code" }),
    session("c", { agent: "codex", project: "Vults-z6", branch: "feat/z6", status: "finished", attention: "done", activity: null, step: null }),
    session("d", { agent: "gemini", project: "site", cwd: "/home/me/Projects/site", roost_key: "local:/home/me/Projects/site/.git", roost: "site", branch: null, status: "idle", attention: "quiet", activity: null, step: null, species: "aura" }),
  ],
  approval: null,
  alerts: [],
  focus: { agent: "claude", id: "b" },
  boards: [
    {
      connector: "github",
      rows: [
        { item: "pr:me/vults#56", group: "yours", name: "vults#56", title: "GitHub card: open PRs, reviews, branch checks", checks: "failing", review: null, link: true },
        { item: "review:team/lib#7", group: "to-review", name: "lib#7", title: "Bump serde to 1.0.220", checks: null, review: null, link: true },
        { item: "branch:me/vults", group: "branches", name: "vults", title: "main · Merge pull request #54", checks: "passing", review: null, link: true },
      ],
    },
  ],
};

const WEEK: Activity = {
  history: true,
  today: "2026-10-11",
  weeks: ["2026-10-05", "2026-09-28"],
  week: {
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
    top_project: "Vults",
    busiest_day: "2026-10-07",
    longest_secs: 3 * 3600 + 5 * 60,
    days: [3000, 5400, 7800, 2400, 4200, 0, 1200],
    headline: "41 turns, 6 h 20 min with your agents, most on Vults.",
  },
  grid: [],
};

const TASKS: TaskRow[] = [
  {
    id: 3,
    goal: "Fix the flaky login test",
    criteria: [
      { text: "passes 20 runs in a row", done: true },
      { text: "no sleep added", done: false },
      { text: "CI green on main", done: false },
    ],
    roost: "site",
    roostKey: "local:/home/me/Projects/site/.git",
    folder: "~/Projects/site",
    owner: "claude",
    status: "doing",
    costUsd: 0.42,
    tokens: 184000,
    issue: "https://github.com/me/site/issues/12",
    canOpenIssue: false,
    running: true,
    canStart: false,
    hasWorktree: true,
    hasSession: true,
    sessionEngine: "claude",
    pack: null,
    route: null,
    review: null,
    reviewVerdict: null,
    reviewRound: null,
    reply: "Ran the login test 20 times: all green. Removed the sleep in auth.spec.ts and waited on the redirect instead. Committed on vults/task-3.",
    updatedMs: 3,
  },
  {
    id: 2,
    goal: "Translate the policies page",
    criteria: [{ text: "pt-BR and its source mark", done: false }],
    roost: "Vults",
    roostKey: "github:rogeriojunior31/vults",
    folder: "~/Projects/Vults",
    owner: "you",
    status: "open",
    costUsd: null,
    tokens: null,
    issue: null,
    canOpenIssue: true,
    running: false,
    canStart: true,
    hasWorktree: true,
    reply: null,
    hasSession: true,
    sessionEngine: "claude",
    route: "quota:codex:claude:91",
    review: "- stats.py:6: average divides by len + 1\n\nVERDICT: changes",
    reviewVerdict: "changes",
    reviewRound: 1,
    pack: "## Objective\n\nTranslate the policies page\n\n## Next\n\n- Update the pt-BR source mark",
    updatedMs: 2,
  },
  {
    id: 1,
    goal: "Ask the designer about the new wings",
    criteria: [],
    roost: "",
    roostKey: "",
    folder: "",
    owner: "you",
    status: "blocked",
    costUsd: null,
    tokens: null,
    issue: null,
    canOpenIssue: false,
    running: false,
    canStart: false,
    hasWorktree: false,
    reply: null,
    hasSession: false,
    sessionEngine: null,
    pack: null,
    route: null,
    review: null,
    reviewVerdict: null,
    reviewRound: null,
    updatedMs: 1,
  },
];

const query = new URLSearchParams(location.search);
// `?still=1` turns motion off and freezes every clip at one instant (`?t=<ms>`, 1500 by default),
// as the island's lab does: birds sit on the wire at once, in the same frame each time.
if (query.get("still")) {
  document.body.classList.add("still");
  Clock.freeze(Number(query.get("t") ?? 1500));
}
const zeca = query.get("zeca") !== "0";
const nest = createNest(
  document.getElementById("nest")!,
  {
    chat:
  {
    send: nop,
    decide: () => {},
    stop: () => {},
    reset: async (p) => p ?? "claude",
    keyboard: () => {},
    models: async () => [],
    pick: async () => ({}),
    setPick: nop,
    conversations: async () => [
      { id: 3, title: "A quieter island", provider: "claude", label: "Claude", folder: "/home/me/vults", updatedMs: 1791730800000, current: true },
      { id: 2, title: "Understanding the tool loop", provider: "codex", label: "Codex", folder: "/home/me/vults-z6", updatedMs: 1791640800000, current: false },
    ],
    open: async () => {
      throw new Error("none in the lab");
    },
    rename: nop,
    remove: async () => false,
  },
    toIsland: () => document.body.setAttribute("data-to-island", "1"),
    roosts:
  {
    load: async () => (new URLSearchParams(location.search).has("empty") ? [] : ROOSTS),
    preview: async () => ({
      path: "~/Projects/Vults/CLAUDE.md",
      file: "/home/me/Projects/Vults/CLAUDE.md",
      diff: "--- ~/Projects/Vults/CLAUDE.md\n+++ ~/Projects/Vults/CLAUDE.md\n@@ -1,2 +1,4 @@\n # Vults\n Desktop app.\n+\n+@AGENTS.md\n",
      fingerprint: "f",
    }),
    apply: async () => {
      document.body.setAttribute("data-shared", "1");
    },
    open: (item) => document.body.setAttribute("data-opened", item),
  },
    memory:
  {
    load: async () => (new URLSearchParams(location.search).has("empty") ? { ...MEMORY, files: [], pending: [], history: [], accepted: 0, rejected: 0, night: null } : MEMORY),
    decide: async (id) => {
      document.body.setAttribute("data-decided", String(id));
    },
    undo: async () => {},
  },
    tasks:
  {
    load: async () => (new URLSearchParams(location.search).has("empty") ? [] : TASKS),
    create: async () => 4,
    edit: async (id, edit) => {
      document.body.setAttribute("data-edited", `${id}:${edit.kind}`);
    },
    remove: async () => {},
    openIssue: async () => "https://github.com/me/Vults/issues/7",
    visit: async () => {},
    folders: () => [{ cwd: "/home/me/vults", project: "vults" }],
    startFresh: async (id, engine) => {
      document.body.setAttribute("data-handoff", `${id}:${engine}`);
    },
    start: async (id, engine) => {
      document.body.setAttribute("data-started", `${id}:${engine}`);
    },
    stop: async () => {},
    review: async (id) => {
      document.body.setAttribute("data-review", String(id));
    },
    removeWorktree: async () => {},
    agents: () => [
      { name: "gemini", title: "Gemini CLI" },
      { name: "opencode", title: "OpenCode" },
    ],
    today: async () => ({ usd: 4.2, tokens: 184000, budget: 5, subscription: true }),
  },
    budget: {
      today: async () => ({ usd: 4.2, tokens: 184000, budget: 5, subscription: true }),
      setBudget: async (cents) => {
        document.body.setAttribute("data-budget", String(cents));
      },
    },
    records:
  {
    // A fixed afternoon, 2026-10-11 15:00 UTC, and the hours before it.
    history: async () => ({
      on: true,
      turns: [
        { end: 1791730800, agent: "claude", project: "Vults", secs: 1260, steps: 34, commands: 9, files: 6, added: 182, removed: 41, failed: false },
        { end: 1791727200, agent: "codex", project: "Vults", secs: 300, steps: 8, commands: 3, files: 0, added: 0, removed: 0, failed: true },
        { end: 1791720000, agent: "opencode", project: "site", secs: 45, steps: 1, commands: 0, files: 1, added: 2, removed: 0, failed: false },
      ],
    }),
    usage: async () => ({
      windows: [
        { agent: "claude", minutes: 300, used_percent: 62, resets_at: null },
        { agent: "claude", minutes: 10080, used_percent: 23, resets_at: null },
        { agent: "codex", minutes: 300, used_percent: 91, resets_at: null },
      ],
      days: [
        { day: "2026-10-09", usd: 1.8, tokens: 410000 },
        { day: "2026-10-10", usd: 5.4, tokens: 1250000 },
        { day: "2026-10-11", usd: 4.2, tokens: 184000 },
      ],
      roosts: [
        { name: "rogeriojunior31/Vults", today: 3.95, total: 11.1 },
        { name: "site", today: 0.25, total: 0.3 },
      ],
      budget: 5,
      subscription: true,
    }),
    traces: async () => [
      { startMs: 1791730700000, ms: 18400, name: "chat claude-opus-5-5", kind: "chat", status: "ok", child: false, input: 24310, output: 812, costUsd: 0.0412 },
      { startMs: 1791730705000, ms: 2100, name: "execute_tool flock_list", kind: "execute_tool", status: "ok", child: true, input: null, output: null, costUsd: null },
      { startMs: 1791730000000, ms: 640, name: "execute_tool WebFetch", kind: "execute_tool", status: "denied", child: false, input: null, output: null, costUsd: null },
    ],
    audit: async () => [
      { at: 1791730600, actor: "human", act: "allow", agent: "claude", project: "Vults", tool: "Bash", target: "cargo test -p vults-store", consent: null },
      { at: 1791730500, actor: "policy", act: "allow", agent: "claude", project: "Vults", tool: "Bash", target: "git status", consent: 3 },
      { at: 1791730400, actor: "rule", act: "allow", agent: "codex", project: "Vults", tool: "Bash", target: "npm run build", consent: null },
      { at: 1791730300, actor: "phone", act: "deny", agent: "opencode", project: "site", tool: "Edit", target: "/home/me/site/.env", consent: null },
    ],
  },
    week: {
      activity: async () => WEEK,
      github: async () => ({ state: "off" }),
      saveImage: async () => null,
      setHistory: nop,
      clear: nop,
      connectors: () => document.body.setAttribute("data-settings", "connectors"),
    },
    flock: {
      // As core keeps a focus: the next view says who is in front.
      focus: (s) => {
        document.body.setAttribute("data-focus", s.id);
        nest.view({ ...shownView, focus: { agent: s.agent, id: s.id } });
      },
      jump: (s) => document.body.setAttribute("data-jump", s.id),
      review: (s) => document.body.setAttribute("data-to-island", s.id),
    },
    connections: {
      open: (_, item) => document.body.setAttribute("data-opened", item),
      // ?gh=on or ?gh=off: what Settings last said of GitHub, whatever the card shows.
      status: async () => {
        const gh = new URLSearchParams(location.search).get("gh");
        return gh ? [{ id: "github", enabled: gh === "on", lastOk: null, error: null, watching: 0, overBudget: false }] : [];
      },
      settings: () => document.body.setAttribute("data-settings", "connectors"),
    },
    settings: () => document.body.setAttribute("data-settings", "general"),
    setZeca: (on) => nest.setZeca(on),
    island: () => document.body.setAttribute("data-island", "1"),
    size: (size, tab) => {
      document.body.setAttribute("data-size", `${size}:${tab}`);
      if (size === "full" || size === "room") nest.setFull(size === "full");
    },
  },
  zeca,
  query.has("full"),
);
const shownView: ViewModel = query.has("empty") ? { sessions: [], approval: null, alerts: [] } : VIEW;
nest.view(shownView);
const chat = nest.chat;
// `?busy=1`: opened mid-turn, with the reply so far and a card waiting.
const busy = new URLSearchParams(location.search).has("busy");
chat.load({
  busy,
  partial: busy ? "Running the tests first" : "",
  waiting: busy ? [{ id: "w1", tool: "Bash", target: "Bash · cargo test", outside: null }] : [],
  provider: "claude",
  label: "Claude",
  folder: "/home/me/vults",
  started: true,
  model: null,
  effort: null,
  lines: [
    { who: "you", text: "What changed in the island?", files: [] },
    { who: "zeca", text: REPLY, files: [] },
  ],
});
if (!busy) chat.receive({
  kind: "permission",
  id: "n1",
  tool: "Bash",
  target: "Bash · cargo test --workspace",
  description: "Run the test suite",
  full: null,
  added: 0,
  removed: 0,
  always: true,
  outside: null,
});
// `?tab=<id>`: opened on that tab or Activity sub-tab (`projects`, `cost`…); `?memory=1` opens the
// Chat's Memory beside it; `?zeca=0` without Zeca.
const tab = query.get("tab");
if (tab) nest.open(tab);
if (query.has("memory")) document.querySelector<HTMLButtonElement>(".nest-memory-toggle")?.click();
