# 0005. Agent configs change only after a backup, a diff and a click

**Status:** Accepted, 2026-10-04 (the rule dates from the start of the project).

## Context

Installing our hook means editing files the user and other tools own: Claude Code's settings,
Codex's config, Gemini's settings.

## Decision

Before any write: a strict read, a diff the user sees, a click, then a dated backup and an atomic
write. Hooks from other tools stay. Codex's `trusted_hash` is never written: the user trusts our
hooks in Codex.

## Consequences

- Installing takes one screen more than a silent setup.
- Ideas that inject our own config into agents (skills, MCP servers) go through the same screen,
  or do not happen.
