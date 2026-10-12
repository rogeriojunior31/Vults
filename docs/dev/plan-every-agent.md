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
| Copilot CLI | Yes | Yes | ACP |
| Cursor | Yes | No: asks in its prompt | No |
| Factory Droid | Yes | No: asks in its prompt | No (ACP candidate) |
| Kimi Code | Yes | No: asks in its prompt | No (ACP candidate) |
| Amp, Hermes, Crush, Pi, Goose, Grok Build, Cline, Auggie, Kiro | Recipe in `other-agents.md`, or nothing | No | No |

## 3. What the code says today

| Fact | Where | What it means |
|---|---|---|
| A new built-in agent touches about 30 files: protocol, hook, parser, caps, dirs, installer, paths, runs, tasks, notify, view and its TS twin, history, settings, policies, Nest, island CSS and scene, themes, three i18n catalogs | `grep -rli qwen` | At 12 more agents this is the real cost. Step F1 turns the per-agent facts into one registry before the batch. |
| `AgentKind` is `Claude \| Codex \| OpenCode \| Qwen \| Other`; protocol `VERSION` 7 (Gemini CLI retired in #310) | `crates/protocol/src/lib.rs` | Each answerable agent still needs its own kind (*Always* rules and policies are keyed by it); observe-only agents can stay `Other` with an `agent_name`. |
| `Other` never waits for a reply | `crates/hook/src/main.rs` (`wants_reply`) | No third-party tool can get a card today, however it speaks. Step G1. |
| "No opinion" is empty stdout for every agent | ADR 0003, `crates/hook/src/output.rs` | Holds for Copilot CLI too: its spike showed an empty reply leaves the call to its own prompt, not denied. |
| ACP agents are a table of name, program, args and env | `crates/chat/src/acp.rs` | A new driven agent is one row plus a spike, if its ACP mode works. Copilot is driven but recorded as `Other` in `runs.rs`. |
| `caps::ALL` is the per-agent capability table the doctor, router and policies read | `crates/agents/src/caps.rs` | Every row is what the code does; a new agent gets a row before it ships. |
| JSON, TOML (comments kept) and whole plugin files can be written | `crates/agent-config` | Kimi (TOML) can be installed from Settings (B4); YAML (Hermes, Goose) still needs a writer with the same preview / fingerprint / backup / atomic contract. |
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
| F1 | **Done.** **Agent registry** (`crates/agents/src/registry.rs`): id, name, CLI, wire (its own kind, or `other` with its name), setup (hooks, plugin, remove-only, none), how Vults runs it (native, ACP with its command line) and what it can do. The installer, the doctor, the chat's ACP engines, the tasks' owners and the engines' wire names read it; `ui/src/agents.gen.ts` is its twin (a test keeps it fresh), read by Settings, the Nest and the island's names. Apart on purpose: the hook's reply table (`vults-hook` is std and serde_json only), the protocol's kinds and their TS twin (a wire change), each kind's color in CSS, and `core`'s names for a session without a folder (`core` can't see `agents`). An observe-only agent is now a row and its parser | M | | Done: tests and visual baselines unchanged |
| F2 | **Done.** **Spike kit**: `crates/spike` (dev only) is the fake model (Anthropic Messages, OpenAI Chat and Responses, scripted per step, Qwen's `tool_call` included), a fake app on the hook's socket answering allow, deny or nothing, a pty for TUIs that answers the terminal's queries and can type keys, the installer into a scratch home, replay through `parse` and `reduce`, the fixture writer and Codex's hook trust for a scratch home; `scripts/spike.sh <agent> [answer] [script]` runs Claude Code (`-p` or its TUI), Codex (`exec`), OpenCode (its server) and Qwen Code (its TUI) end to end. Qwen 0.25.1 re-recorded with it (`qwen-0.25.1-session.jsonl`): the same cards, diff and failure as 0.25.0's fixture | S | | Done |
| F3 | **TOML done** (`agent-config::toml`, on `toml_edit`, already in the tree): the document is edited in place, so comments, order and layout stay; `[[hooks]]` tables of other tools kept, a `hooks` of another shape left alone, a file that does not parse refused, the same fingerprint, dated backup and atomic write. **YAML waits for its first agent** (C6, Goose), whose spike says what it needs: a lossless editor (`yaml-edit`) is the candidate, since a parse-and-render writer would drop the user's comments | S | | TOML: writer tests as for JSON, comments kept. YAML: with C6 |
| F4 | **Checklist done**: [Adding an agent](../contributing/agents.md) (+ pt-BR), linked from `CONTRIBUTING.md`: watch or answer (from a real run), the spike recipe, the agent file and its writers, the registry row (and what an answerable agent adds), Settings and docs. **Issues per agent** (`good first issue` where observe-only) wait for the maintainer's go: they are public | S | F1, F2 | A contributor can add an observe-only agent in one PR |

### Phase A: agents whose permissions we can answer

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| A0 | **Not needed.** B1's spike showed Copilot CLI is not fail-closed: with an empty reply it asks in its own prompt, so ADR 0003 stands | S | | Done |
| B1 | **Done.** **GitHub Copilot CLI**, with approvals (1.0.96, recorded with F2 offline over BYOK: `copilot-session.jsonl`). `AgentKind::Copilot` (protocol `VERSION` 8), the parser (camelCase events, `toolArgs` as a JSON string, `path`/`old_str`/`new_str` as Claude Code's, an edit's or create's `diff` counted, a failed shell read from its `exit code`), the reply `{"behavior":"allow"\|"deny"}`, our own `~/.copilot/hooks/vults.json`, registry row; the tool's output never leaves the hook | M | F1 | Done |
| B3 | **Done.** **Factory Droid**, observe-only: recorded with F2 (droid 0.239.0 `exec`, BYOK, no account: `droid-session.jsonl`). It has no `PermissionRequest`, and `PreToolUse` runs before every tool, asked or not (D1); `exec` never asks (a step it may not take ends the run) and its TUI needs a Factory login, so no prompt was recorded. Claude Code's hooks in `~/.factory/settings.json` (`FACTORY_HOME_OVERRIDE` moves the home), `Execute`/`Create`/`LS` and `old_str`/`new_str` read as Claude Code's; a failed command sends no `PostToolUse`. Its `exec --output-format acp` needs a Factory login (R2) | S | F1, F2 | Done |
| B4 | **Kimi Code done** (2.1.1, `@moonshot-ai/kimi-code`, recorded with F2 over its own Anthropic provider, no account: `kimi-session.jsonl`): observe-only, since it fires `PermissionRequest` without waiting (read from its code; `-p` never asks, so no prompt was recorded); `[[hooks]]` in `~/.kimi-code/config.toml` through `Setup::Toml` and F3's writer; `path` read as `file_path`. Its `kimi acp` takes tasks (R2). **Claude-style hook clones**, one PR each after its spike: Cline CLI, Kiro CLI. Each is answerable or observe-only by what the spike shows | S each | F1–F3 | Fixture; answer round trips where answerable; "waiting in the terminal" otherwise |

### Phase C: observe-only installers

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| C1 | **Done**, from its docs and others' captures (Cursor's CLI and site are out of reach here, and it needs Cursor's own servers: no spike). **Cursor**: `~/.cursor/hooks.json` written as a flat list shared with other tools (`agent-config::flat`), `sessionStart`, `sessionEnd`, `postToolUse`, `postToolUseFailure`, `subagentStop`, `stop`, each `\|\| exit 0`; nothing that Cursor reads as a decision. The project from `workspace_roots` (its hooks run in `~/.cursor`), `Shell` read as `Bash`, `tool_output` and `user_email` dropped in the hook. The relay drops a Claude Code hook call that carries `cursor_version`, so a session is never doubled. To check on a real Cursor: a session's steps and stop | S | F1 | Installed from Settings; no permission ever answered; a recorded session still owed |
| C5 | **Amp** (TS plugin, never `tool.call`), **Pi** (TS extension), **Hermes** (Python plugin; `hermes plugins enable` stays the user's), with the C2 writer | S each | F1, C2 | Installed and seen working with the real CLI |
| C6 | **Crush**, **Goose** (YAML, F3), **Grok Build**, **Muse Code** | S each | F1, F3 | Same |

### Phase R: run over ACP

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| R1 | **Done** with B1. Copilot's run recorded as `Copilot`, not `Other` | S | B1 | Done |
| R2 | **ACP rows** for every agent whose ACP mode passes a spike (candidates: Goose, Auggie, Cline, Hermes): one row in the registry, its args forcing the agent's own prompts (no yolo). **Kimi Code done** (2.1.1, `kimi acp`, run through the real driver against the fake model): its session says `default` while the user's `default_permission_mode = "yolo"` runs every tool unasked, so the row sets the mode (`Acp.mode`, `session/set_mode`); its permission names only its tool, so the driver completes it from what the call streamed before it asked (`Said` in `acp.rs`). Allow runs, deny stops both the command and the edit. **Factory Droid** (`droid exec --output-format acp`) refuses `session/new` without a Factory login, even with BYOK models: not recorded, left for a real account | S each | F1 | A task runs on it from the Nest; a permission reaches a card; deny stops the tool |

### Phase G: any tool, by the open protocol

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| G1 | **Proposed**: [ADR 0027](../adr/0027-an-open-reply-for-any-tool.md), *An open reply for any tool that asks for one*. A named tool opts in with `--reply vults`, sends Claude Code's hook JSON and reads `{"decision":"allow"\|"deny"}`; anything else is "ask yourself"; rules keyed by its name, policies only deny or ask it when they name it; destructive calls always ask | S | | ADR accepted by the user |
| G2 | Build G1: protocol `VERSION` 7, the hook, core keyed by name, Settings lists named tools; `protocol.md` (+ pt-BR) documents it as a spec with an example in shell and in TypeScript | M | G1, F1 | A test tool in shell gets a card, an allow and a deny; an old hook stays silent |

**Order:** F1 → F2 → A0 → B1 → R1 → C1 → B3 → F3 → F4 → G1 → G2, then B4, C5, C6 and R2 in any
order, one agent per PR (the place for contributors).

Docs in the same PR as each step: `other-agents.md` (the agent leaves the recipes), `approvals.md`,
README agent list (+ `README.pt-br.md`), `protocol.md`, CHANGELOG, and the pt-BR twins with their
source marks.

## Notes

**Audit of the agents we have (2026-10-12).** Each CLI ran live against a fake model server
(Anthropic Messages, OpenAI Chat and Responses, scripted tool calls), our real `vults-hook` and
installer, and a fake app on the socket answering allow, deny or nothing; the recorded events went
through `parse` and `reduce`. The scripts are the seed of F2.

| Agent | Version | Checked live | Found |
|---|---|---|---|
| Claude Code | 2.1.296 | session, steps, diffs, card allow / deny / no answer, a question answered from the island (`--ask`) | Nothing |
| Codex | 0.162.1 | session and steps through `codex exec` (which forces `approval: never`) | The card's round trip is unchecked: the TUI hangs while loading in a headless pty. Hooks need trust: `hooks/list` on its app-server gives each key and hash |
| OpenCode | 1.19.0 | through `opencode serve` (`run` rejects at once): steps, card allow / deny, edit and write | Edit and write cards counted nothing and the edit had no diff: the plugin kept only the file's name. Fixed with this note |
| Qwen Code | 0.25.1 | through its TUI (`-i`; `-p` hides the shell and edit tools in default mode): command, edit and write cards, allow / deny | Nothing new. 0.25 hides tools behind `tool_call`; hooks still name the real tool. `read_file` takes `file_path` now |
| Antigravity | not on npm | fixtures only | Unchecked live |

A written file's diff stays out for OpenCode and Qwen on purpose (`diff::from_input`): an
overwrite rebuilt from its content alone would count every line as added.

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
