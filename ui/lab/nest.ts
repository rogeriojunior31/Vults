// The Nest's lab: its Chat tab with a reply in full Markdown and a card it can only send to the
// island, and its Roosts tab (`?tab=roosts`). Development only (`npm run dev`, then /lab/nest.html).
import type { RoostView } from "../src/bridge";
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
    key: "rogeriojunior31/Vults",
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
  },
  {
    key: "site",
    github: null,
    trees: [{ path: "~/Projects/site", branch: null }],
    sessions: [{ agent: "Gemini CLI", status: "idle", step: null }],
    prs: [],
    checks: null,
    memory: [],
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
  async () => (new URLSearchParams(location.search).has("empty") ? [] : ROOSTS),
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
// `?tab=roosts`: opened on the Roosts tab.
if (new URLSearchParams(location.search).get("tab") === "roosts") {
  (document.querySelector('.nest-tab[data-tab="roosts"]') as HTMLButtonElement).click();
}
