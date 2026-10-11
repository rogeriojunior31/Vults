// The Nest's lab: its Chat tab with a reply in full Markdown and a card it can only send to the
// island. Development only (`npm run dev`, then /lab/nest.html).
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
const chat = createNest(
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
