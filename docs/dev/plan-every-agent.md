# Every agent: watch, answer and drive

Internal board, rewritten 2026-10-12 at `592f6a6`. Deleted when its last step merges. We copy the
reference app's mechanics (MIT, `NOTICE`), not its code. The rest of the roadmap is
[plan-zeca.md](plan-zeca.md).

An agent meets Vults three ways: **watch** (its hooks or plugin run `vults-hook`: a bird, steps,
diffs), **answer** (the same hook waits for our reply in the agent's own shape), **drive** (Vults
starts it over ACP or a native driver, so every permission comes to us).

## Where we stand

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

`AgentKind` is `Claude | Codex | OpenCode | Qwen | Copilot | Other`, protocol `VERSION` 8. An
answerable agent needs its own kind (*Always* rules and policies are keyed by it); an observe-only
one is `Other` with an `agent_name`, a registry row and its parser. `Other` never waits for a reply
(G1 changes that).

## Decisions

- **D1. Approvals only where the agent's own hook can take them**, verified on a real CLI with a
  recorded fixture. Antigravity and Cursor looked answerable on paper and are not.
- **D2. A silent timeout is never an approval.** No click → the agent's own "ask" (or nothing).
- **D3. One table per agent in the hook**: the event that waits, the allow / deny shape, the "no
  opinion" output.
- **D4. Observe-only agents still get installers**; their permissions stay in the terminal unless
  Vults drives them.
- **D5. Plugins are files we own whole**, with a marker line; a file without it is never touched.
- **D6. One registry, many readers** (`crates/agents/src/registry.rs`, F1).
- **D7. Drive before Answer**: when only an agent's ACP mode can answer, ship the ACP row first.

## Open steps

One agent per PR. Done: A1, A2, B1 (Copilot CLI), B2 (Qwen Code), B3 (Factory Droid), B4's Kimi
Code, C1's Cursor installer, C2, C3, C4 (OpenCode), F1 (registry), F2 (spike kit), F3's TOML,
F4's checklist, R1; A0 not needed.

| # | Step | Done when |
|---|---|---|
| F3 | YAML writer, with the first YAML agent (Goose, C6): a lossless editor (`yaml-edit`), since parse-and-render drops comments | Writer tests as for JSON, comments kept |
| F4 | Public issues per agent (`good first issue` where observe-only), on the maintainer's go. The checklist is [Adding an agent](../contributing/agents.md) | Issues open |
| B4 | Cline CLI, Kiro CLI: Claude-style hook clones, one PR each after its spike | Fixture; answers where answerable, "waiting in the terminal" otherwise |
| C1 | A recorded session from a real Cursor (installed from its docs; no spike possible here) | A session's steps and stop seen live |
| C5 | Amp (TS plugin), Pi (TS extension), Hermes (Python plugin; enabling stays the user's), with the C2 writer | Installed and seen working with the real CLI |
| C6 | Crush, Goose (YAML, F3), Grok Build, Muse Code | Same |
| R2 | ACP rows for agents whose ACP mode passes a spike (Goose, Kimi `kimi acp`, Auggie, Cline, Droid `exec --output-format acp`, Hermes): args forcing the agent's own prompts, `caps.starts` true | A task runs from the Nest; a permission reaches a card; deny stops the tool |
| G1 | [ADR 0027](../adr/0027-an-open-reply-for-any-tool.md) (proposed): a named tool opts in with `--reply vults`, sends Claude Code's hook JSON, reads `{"decision":"allow"\|"deny"}`; rules keyed by its name; destructive calls always ask | ADR accepted by the user |
| G2 | Build G1: protocol `VERSION` 9, the hook, core keyed by name, Settings lists named tools; `protocol.md` (+ pt-BR) as a spec with shell and TypeScript examples | A test tool in shell gets a card, an allow and a deny; an old hook stays silent |

Docs in each step's PR: `other-agents.md`, `approvals.md`, the README agent list (+ pt-BR),
`protocol.md`, CHANGELOG.

## Notes

- **Spike kit** (F2): `scripts/spike.sh <agent> [answer] [script]` records an agent against a fake
  model server with no account; most agents can be recorded this way.
- **Qwen Code** runs `PermissionRequest` before its prompt and `PreToolUse` after the approval;
  `UserPromptSubmit` fires again with `"prompt": ""` per tool result (dropped); its questions arrive
  as a `PermissionRequest` whose `allow` it ignores, so they wait in the terminal; a fresh install
  starts in *auto* mode. `write_file` has no diff yet (plan-zeca, *Follow-ups*).
- **OpenCode** answers through the plugin's v1 SDK client
  (`postSessionIdPermissionsPermissionId`); our *Always* maps to `once` plus our own rule, never
  OpenCode's `always`. The race with its TUI prompt is still to watch by hand.
- **Codex**: `codex exec` forces `approval: never`, and its TUI hangs in a headless pty, so the card
  round trip is unchecked live. Hooks need trust (`hooks/list` on the app-server gives key and hash).
- **A written file's diff** stays out for OpenCode and Qwen on purpose: an overwrite rebuilt from
  its content would count every line as added.
- **Unverified hook shapes** (research 2026-10-02): Crush has `hooks.PreToolUse` only in
  `crush.json`; Pi and Amp take TS extensions; Grok CLI, Auggie and Cline hooks may only deny.

## Risks

- **Reply shapes drift between versions.** Each answerable agent records its version in its fixture;
  Settings shows "Update hooks" when our entries change.
- **Claude clones read `.claude/settings.json`**: a session could arrive twice. Cursor's is told
  apart by its payload and dropped; Copilot, Droid, Auggie and the others need the same check.
- **Fail-closed agents**: the neutral output is printed before any IO that can fail.
- **An open reply (G) is a door**: G1 must show a named tool gains nothing a built-in agent's card
  would not show in full, and that a reply it ignores leaves its own prompt in charge.
- **Many CLIs to keep working**: observe-only parsers fail soft, and the doctor says which hook is
  out of date.

## Out of scope

- Answering Antigravity or Cursor permissions through their hooks (D1); driving them over ACP is R2.
- Orchestration recipes, per-agent permission profiles and an orchestration tab: the next board.
