# 0005. Agent configs, and the agents we support

**Status:** Accepted, 2026-10-04 (the rule dates from the start of the project). Consolidated on
2026-10-12 from 0029.

## Context

Installing our hook means editing files the user and other tools own: each supported agent's
config. Agents also come and go (Antigravity took Gemini CLI's place), and users who installed our
hooks in an agent we drop must still be able to remove them.

## Decision

- **Before any write**: a strict read, a diff the user sees, a click, then a dated backup and an
  atomic write. Hooks from other tools stay. Codex's `trusted_hash` is never written: the user
  trusts our hooks in Codex.
- Anything that injects our own config into an agent (skills, MCP servers) goes through the same
  screen, or does not happen.
- **The supported agents** are Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI,
  Cursor, Factory Droid and Kimi Code, each a driver in `crates/agents`. Adding one is a driver, an
  installer entry and the docs.
- **Retiring an agent** is a checklist, not a deletion:
  - the protocol version is bumped;
  - a remove-only installer stays: Settings still finds the hooks an earlier Vults wrote and offers
    to remove them (diff, dated backup, click), and never installs them again;
  - its name stays reserved (`valid_agent_name` in `crates/protocol`), so its old hooks send
    nothing and no other tool can take the name;
  - its old turns are left out of the history, and a policy naming it is ignored whole: nothing it
    left behind widens access.

## Consequences

- Installing takes one screen more than a silent setup.
- Gemini CLI is retired this way (protocol version 7): `crates/agents/src/gemini.rs` only removes
  its hooks.

## History

- [0029](0029-the-supported-agents.md), 2026-10-12: the list of supported agents, and how one is
  retired.
