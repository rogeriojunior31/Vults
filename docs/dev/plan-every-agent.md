# Every agent: watch, answer and drive

Internal board, rewritten 2026-10-12 against the code at `65e9809` (first written 2026-10-09).
Deleted when its last step merges. Research inputs: `agents-and-keys.md` (2026-10-02) and the
reference app's Windows/Linux build (MIT, attributed in `NOTICE`). We copy mechanics, not code.

This board comes before the bigger work (the new UI, a better Zeca, orchestration with recipes
and per-agent permissions): an orchestration tab only pays off once it sees and drives every agent
the user runs.

## 1. Three ways an agent meets Vults

| Path | What it gives | How | Where it lives |
|---|---|---|---|
| **Watch** | A bird, steps, diffs, finished / failed | The agent's hooks or plugin run `vults-hook` | `crates/agents/<agent>.rs`, the installer |
| **Answer** | Its permission cards on the island | The same hook waits for our reply, in the agent's own shape | `crates/hook` reply table (D3) |
| **Drive** | Vults starts it for a task or the chat; every permission comes to us | ACP (`session/request_permission`), or the native drivers for Claude and Codex | `crates/chat/src/acp.rs` `AGENTS`, `app/src/runs.rs` `Engine` |

Drive is the cheap path to approvals: an agent Vults starts over ACP asks Vults for every
permission, whatever its hooks can do. Watch and Answer stay for the sessions the user starts in a
terminal.

## 2. Where we stand

| Agent | Watch (installer) | Answer | Drive |
|---|---|---|---|
| Claude Code | Yes | Yes (+ questions) | Native |
| Codex | Yes | Yes | Native |
| Antigravity | Yes | No: answers "ask" | No |
| OpenCode | Yes (plugin) | Yes (+ questions) | ACP |
| Qwen Code | Yes | Yes | ACP |
| Copilot CLI | No | No | ACP (as `Other`) |
| Cursor, Amp, Hermes, Kimi, Crush, Droid, Pi, Goose, Grok Build, Cline, Auggie, Kiro | Recipe in `other-agents.md`, or nothing | No | No |

## 3. What the code says today

| Fact | Where | What it means |
|---|---|---|
| A new built-in agent touches about 30 files: protocol, hook, parser, caps, dirs, installer, paths, runs, tasks, notify, view and its TS twin, history, settings, policies, Nest, island CSS and scene, themes, three i18n catalogs | `grep -rli qwen` | At 12 more agents this is the real cost. Step F1 turns the per-agent facts into one registry before the batch. |
| `AgentKind` is `Claude \| Codex \| OpenCode \| Qwen \| Other`; protocol `VERSION` 7 (Gemini CLI retired in #310) | `crates/protocol/src/lib.rs` | Each answerable agent still needs its own kind (*Always* rules and policies are keyed by it); observe-only agents can stay `Other` with an `agent_name`. |
| `Other` never waits for a reply | `crates/hook/src/main.rs` (`wants_reply`) | No third-party tool can get a card today, however it speaks. Step G1. |
| "No opinion" is empty stdout for every agent | ADR 0003, `crates/hook/src/output.rs` | Copilot CLI is fail-closed and needs `{"permissionDecision":"ask"}`: ADR first (A0). |
| ACP agents are a table of name, program, args and env | `crates/chat/src/acp.rs` | A new driven agent is one row plus a spike, if its ACP mode works. Copilot is driven but recorded as `Other` in `runs.rs`. |
| `caps::ALL` is the per-agent capability table the doctor, router and policies read | `crates/agents/src/caps.rs` | Every row is what the code does; a new agent gets a row before it ships. |
| Only JSON configs and whole plugin files can be written | `crates/agent-config` | Kimi (TOML) and YAML (Hermes, Goose) need writers with the same preview / fingerprint / backup / atomic contract. |
| The Qwen spike ran against a fake OpenAI-compatible server, no account | Notes, B2 | The same trick records most agents without a login. Step F2 makes it a tool. |

## 4. Decisions

- **D1. Approvals only where the agent's own hook can take them**, verified on a real CLI, with a
  recorded fixture. "The docs say so" is not enough: Gemini, Antigravity and Cursor all looked
  answerable on paper and are not. Every capability claimed below is a candidate until its spike.
- **D2. A silent timeout is never an approval.** No click → the agent's own "ask" (or nothing),
  so its terminal prompt takes over. Never `allow`, never `deny`.
- **D3. One table per agent in the hook**, not more `match` arms: event that waits, decision
  shape for allow / deny, and the "no opinion" output. Tests iterate over it.
- **D4. Observe-only agents still get installers.** Moving a recipe from the docs into Settings is
  the main win; their permissions stay "answer in the terminal", or come to us when Vults drives
  them.
- **D5. Plugins are files we own whole.** A generated file carries a marker line; a file without it
  is refused for install and uninstall (never overwritten). Same diff, backup, click.
- **D6. One registry, many readers.** An agent's name, color, command, config path, events, reply
  shape, caps and ACP entry are written once; the TS twin, Settings, the Nest and the i18n names
  read it. Adding an observe-only agent should not touch the UI by hand.
- **D7. Drive before Answer.** When an agent's hooks cannot answer but its ACP mode can, ship the
  ACP row first: tasks and the chat get its cards at once; the hook path follows only if the spike
  shows it can.

## 5. Steps

One step, one PR, as in `plan-fourth-review.md` (*How to work a step*). Done earlier on this board:
A1 (hook reply table, #162), A2 (protocol versions for new kinds), B2 (Qwen Code), C2 (plugin file
writer), C3 and C4 (OpenCode installer and approvals).

### Phase F: make the next agent cheap

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| F1 | **Agent registry.** One table in `crates/agents` (D6) that `caps`, `dirs`, the installer list, the hook's reply table and `acp::AGENTS` read; `view.gen.ts` and the Settings / Nest lists generated from it; the i18n catalogs keep only sentences, not agent names. Qwen and OpenCode moved onto it with no behavior change | M | | Adding a fake observe-only agent in a test touches one file; visual tests unchanged |
| F2 | **Spike kit.** A dev-only fake model server (OpenAI Chat, Responses and Anthropic Messages) that scripts tool calls, plus a script that runs an agent against it with our hook in record mode and writes the fixture | S | | The Qwen fixture is re-recorded with it and matches |
| F3 | **Config writers** for TOML and YAML, under the `agent-config` contract (strict read, diff, fingerprint, dated backup, atomic write, third-party entries kept) | S | | Writer tests as for JSON: foreign keys kept, comments kept where the format allows, refused on a parse error |
| F4 | **"New agent" checklist** in `CONTRIBUTING.md` (+ pt-BR): registry row, spike with F2, parser and fixture, caps row, docs. Issues opened per agent, labeled `good first issue` where observe-only | S | F1, F2 | A contributor can add an observe-only agent in one PR |

### Phase A: agents whose permissions we can answer

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| A0 | Only if B1's spike confirms Copilot is fail-closed: the next free ADR, *An agent's "no opinion" may be JSON*, superseding the part of 0003 about empty stdout: exit 0 always, the budget unchanged, the per-agent neutral output | S | | ADR accepted by the user |
| B1 | **GitHub Copilot CLI**, with approvals. Spike: record a session (`preToolUse`, `permissionRequest`, `agentStop`…), confirm the reply shape (`{"permissionDecision":"allow"\|"deny"\|"ask"}` in the reference). Then `AgentKind::Copilot` (also for the ACP run in `runs.rs`), the parser (camelCase events, `toolArgs` maybe a JSON string), our own `~/.copilot/hooks/vults.json`, registry row | M | F1, A0 | Fixture end to end; allow / deny / timeout round trips; a deny from the island stops the tool in the real CLI |
| B3 | **Factory Droid**: `~/.factory/settings.json`, a Claude Code clone. Answer only if the spike shows `PermissionRequest` (or a `PreToolUse` decision) is honored; otherwise observe-only | S | F1, F2 | B2's checklist |
| B4 | **Claude-style hook clones**, one PR each after its spike: Kimi Code (TOML, F3), Auggie, Cline CLI, Kiro CLI. Each is answerable or observe-only by what the spike shows | S each | F1–F3 | Fixture; answer round trips where answerable; "waiting in the terminal" otherwise |

### Phase C: observe-only installers

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| C1 | **Cursor**: `~/.cursor/hooks.json`, events without `preToolUse` and `beforeSubmitPrompt` (Cursor reads their output as a decision), `\|\| exit 0`; no doubled session with Claude Code in Cursor's terminal. Spike: does `cursor-agent` run hooks on Linux? | S | F1 | Installed from Settings; recorded session; no permission ever answered |
| C5 | **Amp** (TS plugin, never `tool.call`), **Pi** (TS extension), **Hermes** (Python plugin; `hermes plugins enable` stays the user's), with the C2 writer | S each | F1, C2 | Installed and seen working with the real CLI |
| C6 | **Crush**, **Goose** (YAML, F3), **Grok Build**, **Muse Code** | S each | F1, F3 | Same |

### Phase R: run over ACP

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| R1 | Copilot's run recorded as `Copilot`, not `Other` | S | B1 | Its task cards and policies name Copilot |
| R2 | **ACP rows** for every agent whose ACP mode passes a spike (candidates: Goose, Kimi, Auggie, Cline, Droid, Hermes): one row in the registry, its args forcing the agent's own prompts (no yolo), `caps.starts` true | S each | F1 | A task runs on it from the Nest; a permission reaches a card; deny stops the tool |

### Phase G: any tool, by the open protocol

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| G1 | ADR *An open reply for any agent*: a `vults` profile of the hook protocol (neutral events, and a fixed reply `allow` / `deny` / `ask` the tool must honor) that a third-party tool declares with `--reply vults`; its *Always* rules and policies are keyed by its `agent_name`; destructive actions always ask; the name stays reserved once used | S | A0 | ADR accepted by the user |
| G2 | Build G1: protocol `VERSION` 7, the hook, core keyed by name, Settings lists named tools; `protocol.md` (+ pt-BR) documents it as a spec with an example in shell and in TypeScript | M | G1, F1 | A test tool in shell gets a card, an allow and a deny; an old hook stays silent |

**Order:** F1 → F2 → A0 → B1 → R1 → C1 → B3 → F3 → F4 → G1 → G2, then B4, C5, C6 and R2 in any
order, one agent per PR (the place for contributors).

Docs in the same PR as each step: `other-agents.md` (the agent leaves the recipes), `approvals.md`,
README agent list (+ `README.pt-br.md`), `protocol.md`, CHANGELOG, and the pt-BR twins with their
source marks.

## Notes

**C4, OpenCode approvals (read in the opencode 1.18.35 bundle, 2026-10-09).** No hook returns a
verdict, but the plugin's `client` (the SDK) can answer: OpenCode's own `run` command and TUI call
`client.permission.reply({ requestID, reply })` on the `permission.asked` event, `reply` being
`"once" | "always" | "reject"` (with an optional `message`). The event carries `PermissionRequest`:
`{ id, sessionID, permission, patterns: string[], metadata, always: string[], tool?: { messageID,
callID } }`. So the plugin can, on `permission.asked`, run the relay as a `PermissionRequest`
(waiting for its answer, within the budget) and reply `once` or `reject` only when the island
answered; OpenCode's own prompt stays up meanwhile, and whichever answers first wins. Nothing
blocks OpenCode. Needs its own `AgentKind` (A1, A2: *Always* rules are per kind), a reply shape
for the plugin to read, and a live TUI check (in `opencode run` the CLI auto-rejects at once, so
it can't show the race). Our *Always* maps to `once` plus our own rule, never to OpenCode's
`always`, which would change OpenCode's config.

**C4, built (2026-10-09).** The plugin's `client` is the **v1** SDK: there is no
`client.permission.reply`; the answer goes through `client.postSessionIdPermissionsPermissionId({
path: { id: sessionID, permissionID }, body: { response } })`. Checked under `opencode serve` (no
prompt, so only the plugin can answer): `once` ran `echo … > b.txt`, `reject` stopped a `write`.
`permission.asked` carries `metadata.command` for `bash` and `metadata.filepath` (+ a unified
`diff`) for `edit`. The plugin names the card after the running call's tool (`tool.execute.before`
by `callID`), so the tool's `PostToolUse` settles it; `permission.replied` stops the relay when
OpenCode answered first. Still to see: the race in the TUI, by hand.

**B2, Qwen Code 0.25.0 (2026-10-09).** Recorded against a fake OpenAI-compatible server
(`OPENAI_BASE_URL`), so no account was needed: the model's tool calls were scripted, Qwen's hooks
and prompts are its own. What we learned:

- Same events and JSON as Claude Code, plus `permission_mode`, `prompt_id`, `tool_use_id`. The
  `PermissionRequest` answer is Claude Code's `hookSpecificOutput.decision.behavior`; empty stdout
  means no decision, and Qwen shows its own prompt.
- Qwen runs `PermissionRequest` **before** its prompt and `PreToolUse` after the approval (Claude
  Code: the other way round). Its prompt shows only when the hook returns, and it ignores
  `statusMessage` there: its screen shows its spinner meanwhile.
- `UserPromptSubmit` fires again with `"prompt": ""` each time a tool's result goes back to the
  model; we drop those.
- Questions (`ask_user_question`) arrive as a `PermissionRequest` whose `allow` Qwen ignores
  (`requiresUserInteraction`), and `PreToolUse` comes after the user answered: the island cannot
  answer them. They show as "waiting in the terminal".
- A fresh install starts in *auto* mode (an LLM classifier decides); cards matter in *default*.
- Follow-up: `write_file` has no diff on the island (the hook keeps only Claude Code's
  `structuredPatch`; Qwen sends a unified diff in `tool_response.returnDisplay.fileDiff`, which the
  hook could turn into hunks). `edit` diffs work, rebuilt from its input.

## Risks

- **Reply shapes drift between CLI versions.** Each answerable agent records its version in the
  fixture's doc comment; Settings shows "Update hooks" when our entries change.
- **The registry (F1) is a refactor across every surface.** It lands with no behavior change and
  the visual baselines untouched before any agent rides on it.
- **Copilot reads `.claude/settings.json` in a repository**: with project-level Claude hooks a
  Copilot session could arrive twice (as `claude` and `copilot`). The spike checks it; if so,
  the Claude entry run by Copilot is told apart by its payload and dropped. Droid, Auggie and the
  other Claude clones get the same check.
- **Fail-closed agents.** A hook crash before printing would deny every tool in Copilot. The
  neutral output is printed before any IO that can fail, and a round-trip test kills the app
  mid-wait.
- **An open reply (G) is a door.** Any process of the user's can already reach the socket; G1 must
  show that a named tool gains nothing a built-in agent's card would not show in full, and that
  a reply it ignores leaves its own prompt in charge.
- **Twelve agents is twelve CLIs to keep working.** Observe-only parsers fail soft (an unknown
  event is a step, never a crash), and the doctor (W1) says which hook is out of date.

## Out of scope

- Answering Antigravity or Cursor permissions through their hooks (D1: they cannot,
  today); driving them over ACP is in scope (R2).
- Orchestration recipes, per-agent permission profiles and the orchestration tab: the next board,
  once this one is done.
