# 0001. Record decisions as ADRs

**Status:** Accepted, 2026-10-04. Amended on 2026-10-12: records on one topic may be consolidated.

## Context

Choices were spread over `CLAUDE.md`, the internal plans in `docs/dev/` and pull request threads.
The plans are deleted once done, so their reasons went with them. Later, records that amended one
another piled up on the same topic, and finding the rule in force meant reading four of them.

## Decision

Every choice that shapes the project gets a short record in `docs/adr/`, numbered, with its
status, context, decision and consequences. Plans in `docs/dev/` point to the records; the
records outlive the plans.

Records on the same topic may be consolidated into one **living record**: the current rule, whole,
with a short *History* of the records folded into it. A consolidation decides nothing new and drops
only what a later record had already superseded. Each folded record stays as a stub that says where
it was merged, so links keep working; the git history keeps its old text.

## Consequences

- A contributor can learn why the app works this way without the history.
- A change to a rule starts with a new record that supersedes the old one, not with an edit to
  `CLAUDE.md` alone. A later consolidation may fold that new record into its topic's living record.
- One topic, one place to read the rule in force.
