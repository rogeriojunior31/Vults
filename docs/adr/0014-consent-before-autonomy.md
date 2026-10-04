# 0014. A new consent rule before any autonomy

**Status:** Proposed, 2026-10-04. Must be accepted before any 0.5 policy or autonomy work starts.

## Context

0.5 brings policies (allow this kind of command in this project) and autonomy (let an agent go on
without asking). Both answer permissions without a click, which
[0004](0004-a-human-answers-permissions.md) forbids today, except for exact *Always* rules.

## Decision (proposed)

A permission may also be answered by a policy when the user wrote it, saw it as a diff and clicked
to accept it, like an agent config ([0005](0005-agent-configs-backup-diff-click.md)). Policies
class actions as read, write or destructive; a deny beats an allow; destructive actions always
ask; every answer, by a human, a rule or a policy, goes to an append-only audit log the user can
read. A budget's "hard stop" only stops new sessions the app would start; it never blocks or
kills an agent the app only watches.

## Consequences

- `CLAUDE.md`'s rule 2 and the core test that pins it change in the same pull request that
  accepts this record.
- The audit log (0.4) has to exist first.
