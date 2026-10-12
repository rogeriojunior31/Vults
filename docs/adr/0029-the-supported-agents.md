# 0029. The supported agents, and retiring one

**Status:** Accepted, 2026-10-12.

## Context

Vults installs a hook in each agent it supports and edits that agent's config
([0005](0005-agent-configs-backup-diff-click.md)). Agents come and go: Antigravity took Gemini
CLI's place, and Gemini CLI was retired in #310. Users who installed our hooks in an agent we drop
must still be able to remove them, and an old hook must not pass for something else.

## Decision

- The supported agents are Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI,
  Cursor, Factory Droid and Kimi Code, each a driver in `crates/agents`.
- Retiring an agent bumps the protocol version and keeps a remove-only installer: Settings still
  finds the hooks an earlier Vults wrote and offers to remove them (diff, dated backup, click),
  and never installs them again.
- Its name stays reserved (`valid_agent_name` in `crates/protocol`), so its old hooks send nothing
  and no other tool can take the name.
- Its old turns are left out of the history, and nothing it left behind widens access: a policy
  naming it is ignored whole.

## Consequences

- Gemini CLI is retired this way (protocol version 7): Antigravity replaces it, and
  `crates/agents/src/gemini.rs` only removes its hooks.
- Adding an agent is a driver, an installer entry and the docs; retiring one is this record's
  checklist, not a deletion.
