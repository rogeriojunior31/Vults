// The Nest's lab: its Chat tab with a reply in full Markdown and a card it can only send to the
// island, and its Roosts tab (`?tab=roosts`). Development only (`npm run dev`, then /lab/nest.html).
import type { AgentRow, MemoryView, RoostView, TaskRow } from "../src/bridge";
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
      { name: "Vults#243", title: "M3: roosts", checks: "running", review: null, url: null },
      { name: "Vults#242", title: "M2: the frozen and volatile context", checks: "passing", review: "approved", url: null },
    ],
    checks: "passing",
    memory: ["Merges with --merge, never squash.", "Every UI sentence goes through t() in four languages."],
    agents: "offer",
  },
  {
    key: "local:/home/me/Projects/site/.git",
    name: "site",
    github: null,
    trees: [{ path: "~/Projects/site", branch: null }],
    sessions: [{ agent: "Gemini CLI", status: "idle", step: null }],
    prs: [],
    checks: null,
    memory: [],
    agents: "",
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

const AGENTS: AgentRow[] = [
  { id: "claude", name: "Claude Code", command: "claude", permissions: "card", questions: "card", diffs: "numbered", stop: "interrupt", sandbox: "bubblewrap", starts: true, path: "~/.local/bin/claude", version: "2.1.295 (Claude Code)", hooks: "ok" },
  { id: "codex", name: "Codex", command: "codex", permissions: "card", questions: "terminal", diffs: "lines", stop: "interrupt", sandbox: "codex", starts: true, path: "/usr/bin/codex", version: "codex-cli 0.160.1", hooks: "untrusted" },
  { id: "gemini", name: "Gemini CLI", command: "gemini", permissions: "terminal", questions: "terminal", diffs: "lines", stop: "no", sandbox: "container", starts: false, path: null, version: null, hooks: "missing" },
];

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
    folder: "~/Projects/site",
    owner: "claude",
    status: "doing",
    costUsd: 0.42,
    issue: "https://github.com/me/site/issues/12",
    canOpenIssue: false,
    running: true,
    canStart: false,
    hasWorktree: true,
    reply: "Ran the login test 20 times: all green. Removed the sleep in auth.spec.ts and waited on the redirect instead. Committed on vults/task-3.",
    updatedMs: 3,
  },
  {
    id: 2,
    goal: "Translate the policies page",
    criteria: [{ text: "pt-BR and its source mark", done: false }],
    roost: "Vults",
    folder: "~/Projects/Vults",
    owner: "you",
    status: "open",
    costUsd: null,
    issue: null,
    canOpenIssue: true,
    running: false,
    canStart: true,
    hasWorktree: false,
    reply: null,
    updatedMs: 2,
  },
  {
    id: 1,
    goal: "Ask the designer about the new wings",
    criteria: [],
    roost: "",
    folder: "",
    owner: "you",
    status: "blocked",
    costUsd: null,
    issue: null,
    canOpenIssue: false,
    running: false,
    canStart: false,
    hasWorktree: false,
    reply: null,
    updatedMs: 1,
  },
];

const { chat } = createNest(
  document.getElementById("nest")!,
  {
    send: nop,
    decide: () => {},
    stop: () => {},
    reset: async (p) => p ?? "claude",
    keyboard: () => {},
    models: async () => [],
    pick: async () => ({}),
    setPick: nop,
    conversations: async () => [],
    open: async () => {
      throw new Error("none in the lab");
    },
    rename: nop,
    remove: async () => false,
  },
  () => document.body.setAttribute("data-to-island", "1"),
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
  },
  {
    load: async () => (new URLSearchParams(location.search).has("empty") ? { ...MEMORY, files: [], pending: [], history: [], accepted: 0, rejected: 0, night: null } : MEMORY),
    decide: async (id) => {
      document.body.setAttribute("data-decided", String(id));
    },
    undo: async () => {},
  },
  async () => AGENTS,
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
    start: async (id) => {
      document.body.setAttribute("data-started", String(id));
    },
    stop: async () => {},
    removeWorktree: async () => {},
  },
);
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
// `?tab=roosts` or `?tab=memory`: opened on that tab.
const tab = new URLSearchParams(location.search).get("tab");
if (tab) (document.querySelector(`.nest-tab[data-tab="${tab}"]`) as HTMLButtonElement | null)?.click();
