# 0001. Record decisions as ADRs

**Status:** Accepted, 2026-10-04.

## Context

Choices were spread over `CLAUDE.md`, the internal plans in `docs/dev/` and pull request threads.
The plans are deleted once done, so their reasons went with them.

## Decision

Every choice that shapes the project gets a short record in `docs/adr/`, numbered, with its
status, context, decision and consequences. Plans in `docs/dev/` point to the records; the
records outlive the plans.

## Consequences

- A contributor can learn why the app works this way without the history.
- A change to a rule starts with a new record that supersedes the old one, not with an edit to
  `CLAUDE.md` alone.
