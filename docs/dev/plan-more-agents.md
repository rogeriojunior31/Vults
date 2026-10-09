# More agents, and approvals beyond Claude Code and Codex

Internal board (2026-10-09), checked against the code at `52cbc0d`. Deleted when its last step
merges. Research inputs: `agents-and-keys.md` (2026-10-02) and the reference app's Windows/Linux
build (`windows/src-tauri/src/agents.rs`, `windows/hook/src/reply.rs`, MIT, attributed in
`NOTICE`). We copy mechanics, not code.

## 1. Where we stand

| | Built in (Settings → Agents) | Island answers permissions |
|---|---|---|
| Us | Claude Code, Codex, Gemini CLI, Antigravity | Claude Code, Codex |
| Reference | the same + Copilot CLI, Cursor, OpenCode, Amp, Hermes, Muse Code | Claude Code, Codex, **Copilot CLI**, **Muse Code** |

Everything else reaches us only through `--agent <name>` and the recipes in
`docs/guide/other-agents.md`: the user edits the agent's config by hand, and any permission shows as
"waiting in the terminal". The gap is two things: **one-click installers** (backup, diff, click) and
**approvals** for the agents whose hooks can take an answer.

## 2. What the code says today

| Fact | Where | What it means |
|---|---|---|
| Which agents get an answer is decided in two hook matches | `crates/hook/src/main.rs` (`wants_reply`), `crates/hook/src/output.rs` (`decision_json`) | A per-agent table replaces them before a third shape arrives (step A1). |
| "No opinion" is always empty stdout | ADR 0003, `output.rs` | **Copilot CLI is fail-closed**: per the reference, it must get valid JSON, and gets `{"permissionDecision":"ask"}` when nobody clicked. Our rule 1 needs a per-agent "no opinion" output: still exit 0, still never waits past the budget, but not always empty. ADR 0003 gets a superseding record (A0). |
| `AgentKind` is `Claude | Codex | Gemini | Other`; `Other` tools share one kind | `crates/protocol/src/lib.rs` | *Always* rules are keyed by `AgentKind`: an agent we answer needs its own variant. A variant is a wire change: `VERSION` 3 → 4. |
| Reserved names: `claude`, `codex`, `gemini`, `other` | `valid_agent_name` | Each new built-in joins the list, so no tool can pass for it. |
| An agent we cannot answer must never emit `PermissionRequested` | `crates/agents` (`other.rs` turns it into `Question`) | A card nobody can answer. Each new `parse` is tested for it. |
| Only JSON configs can be written | `crates/agent-config` | Kimi (TOML) and plugin files (OpenCode, Amp) need new writers with the same preview / fingerprint / backup / atomic-rename contract. |
| `AgentKind` has a TS twin with an exhaustive match | `crates/core/src/view.rs` (`agent_kind_twin_serializes_the_same`) | The build fails until the twin, `view.gen.ts`, names, colors and CSS classes have the new agent. Visual tests run. |
| Settings lists agents statically; the *Always* row label is `claude ? "Claude Code" : "Codex"` | `ui/src/surfaces/settings/main.ts` | Both become data-driven with a third answerable agent. |
| Installed locally: `opencode`, `crush`. Not installed: `copilot`, `qwen`, `droid`, `cursor-agent`, `amp`, `hermes` | this machine | Each agent step starts by installing the CLI and recording a fixture from a real session. |

## 3. Decisions

- **D1. Approvals only where the agent's own hook can take them**, verified on a real CLI, with a
  recorded fixture. "The docs say so" is not enough: Gemini, Antigravity and Cursor all looked
  answerable on paper and are not.
- **D2. A silent timeout is never an approval.** No click → the agent's own "ask" (or nothing),
  so its terminal prompt takes over. Never `allow`, never `deny`.
- **D3. One table per agent in the hook**, not more `match` arms: event that waits, decision
  shape for allow / deny, and the "no opinion" output. Tests iterate over it.
- **D4. Observe-only agents still get installers.** Moving a recipe from the docs into Settings is
  the main win for Cursor and OpenCode; their permissions stay "answer in the terminal".
- **D5. Plugins are files we own whole.** A generated file carries a marker line; a file without it
  is refused for install and uninstall (never overwritten). Same diff, backup, click.

## 4. Steps

One step, one PR, as in `plan-fourth-review.md` (*How to work a step*).

**Order (decided 2026-10-09):** what can be checked on this machine first, Copilot CLI last (not
installed, needs a GitHub login): C2 + C3 → C4 → C1 → A1 → B2 → B3 → D1 → A0, A2, B1.

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| A0 | Only if B1's spike confirms Copilot is fail-closed (do the spike first): ADR 0017 *An agent's "no opinion" may be JSON* superseding the part of 0003 about empty stdout: exit 0 always, the budget unchanged, the per-agent neutral output (empty for Claude/Codex, `{"permissionDecision":"ask"}` for Copilot) | S | | ADR accepted by the user |
| A1 | **Done (#162).** Hook: per-agent reply table (D3); Claude and Codex moved onto it with no behavior change | S | | Hook unit + round-trip tests pass unchanged |
| A2 | **Done (with C4).** Protocol `VERSION` 4 ready for new kinds: reserved names list grows with each kind; `protocol.md` (+ pt-BR) history line | S | A1 | Protocol tests; an old v3 hook gets `Unsupported` and stays silent |
| B1 | **GitHub Copilot CLI**, with approvals. Spike first: install the CLI, record a session (`preToolUse`, `permissionRequest`, `agentStop`…), confirm the reply shape (the reference prints `{"permissionDecision":"allow"\|"deny"\|"ask"}`; our research note says `behavior`: the CLI decides). Then: `AgentKind::Copilot`, `copilot.rs` (camelCase events passed in argv, `toolName`/`toolArgs` which may be a JSON string, `sessionId`, `workdir`), our own file `~/.copilot/hooks/vults.json` (`{"version":1,"hooks":{…}}`, `timeoutSec` 120 on `permissionRequest`), UI twin, names, color, Settings row | M | A0–A2 | Fixture test end to end; allow / deny / timeout round trips; a deny from the island stops the tool in the real CLI; nothing written without diff, backup and click |
| B2 | **Done (see Notes).** **Qwen Code**, with approvals: `AgentKind::Qwen`, Claude Code's hooks in `~/.qwen/settings.json`, its Gemini-style tool names read as Claude Code's, UI twin, name, color, Settings row | S | A1, A2 | Qwen fixture test; allow / deny / no answer / app closed checked with the real hook in a live Qwen 0.25.0 |
| B3 | **Factory Droid**: `~/.factory/settings.json`, a Claude clone. Only if the spike shows `PermissionRequest` works; otherwise observe-only | S | A1, A2 | Same as B2: B2's checklist (twin, names, colors, CSS, Settings, docs) applies |
| C1 | **Cursor** installer, observe-only: `~/.cursor/hooks.json`, events without `preToolUse` and `beforeSubmitPrompt` (Cursor reads their output as a decision), `\|\| exit 0`; avoid doubled sessions with Claude Code in Cursor's terminal. Spike: does `cursor-agent` run hooks on Linux? | S | | Installed from Settings; recorded session; no permission ever answered |
| C2 | **Done (with C3).** agent-config: plugin file writer (D5): whole-file create / update / remove with marker, diff against the current file, backup, atomic write | S | | Writer tests: foreign file refused, marker kept, uninstall removes only ours |
| C3 | **Done.** Checked with a live opencode 1.18.35 session (SessionStart → UserPromptSubmit → PreToolUse → PostToolUse → Stop reached the relay; the step is recorded in `opencode.rs`). **OpenCode** installer, from the recipe in `other-agents.md` (plugin in `$XDG_CONFIG_HOME/opencode/plugins/`, not a hard-coded `~/.config`): fire-and-forget spawn, never waits | S | C2 | Installed and seen working with the local `opencode`; restart note in Settings |
| C4 | **Done, live TUI check pending (see Notes).** OpenCode approvals through its plugin `permission.ask` hook: the plugin runs the relay and waits for its answer, falling back to OpenCode's own prompt on timeout. Only if the hook can wait without blocking OpenCode's UI | S | C3, A0 | Notes written; a step added only if it works |
| D1 | Later, one each when asked: Amp (TS plugin, observe-only, never `tool.call`), Hermes (Python plugin; `hermes plugins enable` stays the user's), Kimi (needs a TOML writer), Crush (installed here; `PreToolUse` only) | S each | C2 | |

Docs in the same PR as each step: `other-agents.md` (the agent leaves the recipes), `approvals.md`
("Claude Code, Codex and Copilot CLI…"), README agent list, `protocol.md`, CHANGELOG, and the pt-BR
twins with their source marks.

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

## 5. Risks

- **Reply shapes drift between CLI versions.** Each answerable agent records its version in the
  fixture's doc comment; Settings shows "Update hooks" when our entries change.
- **A new kind touches every surface** (twin, colors, CSS, visual baselines). B1 pays it once and
  writes down the checklist; B2/B3 follow it.
- **Copilot reads `.claude/settings.json` in a repository**: with project-level Claude hooks a
  Copilot session could arrive twice (as `claude` and `copilot`). The spike checks it; if so,
  the Claude entry run by Copilot is told apart by its payload and dropped.
- **Fail-closed agents.** A hook crash before printing would deny every tool in Copilot. The
  neutral output is printed before any IO that can fail, and a round-trip test kills the app
  mid-wait.

## 6. Out of scope

- Answering Gemini CLI, Antigravity or Cursor permissions (D1: their hooks cannot, today).
- Starting or stopping agents (0.2.0, Operations).
