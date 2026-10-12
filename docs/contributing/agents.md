# Adding an agent

Vults shows a coding agent's sessions through the hooks the agent runs on its own events. Most
agents are a few hundred lines: one file in `crates/agents`, a row in the registry, a recorded
session, and the docs. This page is the checklist, in the order the work goes.

## Watch or answer

Decide this first, from a real run of the agent, never from its docs alone: Gemini CLI, Antigravity
and Cursor all looked answerable on paper and are not.

- **Observe-only** (Antigravity, Cursor, Factory Droid, Kimi Code): the island shows the steps; the agent asks
  its permissions in its own prompt. This is the default, and the right first PR.
- **Answerable** (Claude Code, Codex, Qwen Code, Copilot CLI): the agent has an event sent only when
  it would ask the user (a `PermissionRequest`), takes an allow or a deny from the hook, and on an
  empty answer asks in its own prompt. A hook that runs before *every* tool (`PreToolUse`) is not
  enough: it says nothing of whether the agent would have asked.

Silence is never an approval: no click means the agent's own prompt, never allow, never deny.

## 1. Record a session

The spike kit runs the real CLI against a fake model and a fake app, with no account and nothing
outside a scratch folder:

```sh
scripts/spike.sh <agent> [allow|deny|none] [script.json]
```

Add a recipe for the agent in `scripts/spike.sh`: point its own model settings (most take a custom
OpenAI or Anthropic base URL) at `$base`. If the fake model does not know the agent's tool names,
teach `crates/spike/src/model.rs` (`plain()`), with a test. The run leaves `events.jsonl` (what the
hook sent), `replay.txt` (what the island makes of it) and `fixture.jsonl`. An agent that needs its
vendor's servers cannot be recorded this way: say so in the PR and in the plan.

## 2. Write the agent

One file, `crates/agents/src/<agent>.rs`, implementing the `Agent` trait:

- `parse`: the agent's events in Claude Code's words. Most agents copied Claude Code's hook JSON;
  map what differs (event names, tool names such as `Execute` → `Bash`, argument names such as
  `old_str` → `old_string`), then hand it to `other::as_claude`. Unknown events and tools fail soft:
  a generic step, never a crash.
- `config_file`, `hook_entries`, and `install` / `uninstall` / `installed` / `up_to_date` when the
  file is not Claude Code's shape. `vults-agent-config` has the writers: Claude-style JSON hooks,
  named keys (`named`), flat lists per event (`flat`), TOML (`toml`, set up with `Setup::Toml`; Kimi Code is the example) and whole plugin files
  (`plugin`). Other tools' hooks in a shared file are always kept.
- A folder variable that moves the agent's files goes in `dirs.rs` (`DIR_VARS`), read the way the
  agent reads it.
- If a failing hook stops the agent, end every command in `|| exit 0`. If the agent reads any
  output of a hook as a decision, do not register that hook.
- Fields that carry a tool's output or an account (`tool_output`, `user_email`) are dropped in the
  hook (`DROPPED_FIELDS` in `crates/hook/src/lib.rs`); only an error or an exit code is kept.

Test it against the recorded fixture (`crates/agents/tests/fixtures/<agent>-session.jsonl`, paths
rewritten to `/home/me`), plus an install and a removal that keep the rest of the file.

## 3. Add the registry row

`crates/agents/src/registry.rs` holds one `Entry` per agent: its id, name, command, how it goes on
the wire (`Wire::Named` for an observe-only agent, sent as `other` with its name), its setup, how
Vults runs it, and its `Caps` (what **Settings → Agents** shows). Update the registry's tests, then
regenerate the UI's twin:

```sh
VULTS_REGEN=1 cargo test -p vults-agents agents_ts
```

An answerable agent also needs its own `AgentKind` (a protocol `VERSION` bump), its reply shape in
`crates/hook/src/output.rs`, its name in `crates/core/src/notify.rs`, the view's twin
(`VULTS_REGEN=1 cargo test -p vults-core view_ts`), its color in the UI, and the fuzz targets'
agent lists. Copilot CLI's PR is the example.

## 4. Settings and docs

- A note under its card in **Settings → Agents** (`ui/src/surfaces/settings/main.ts`), with its
  translations in `ui/src/i18n/{pt-BR,es,zh}.ts`.
- In English and Brazilian Portuguese: its section in [Other agents](../guide/other-agents.md) (and
  its row in *What each agent can do*; it leaves the recipes), [Approving](../guide/approvals.md),
  the table in [Getting started](../getting-started.md), the agent lists in the READMEs,
  `docs/README.md` and `architecture.md`.
- A line in `CHANGELOG.md`, and its step marked done in `docs/dev/plan-every-agent.md` with what
  the spike showed.

Then the checks in [CONTRIBUTING.md](https://github.com/rogeriojunior31/Vults/blob/main/CONTRIBUTING.md#before-opening-a-pr); a new agent
widens a select or two, so the visual tests may need new baselines on purpose.
