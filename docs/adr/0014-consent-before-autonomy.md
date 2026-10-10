# 0014. A new consent rule before any autonomy

**Status:** Accepted, 2026-10-10 (proposed 2026-10-04). Amends [0004](0004-a-human-answers-permissions.md).
Policies stay off in code until the audit log and the policy engine exist (`docs/dev/plan-zeca.md`,
S2 and W2).

## Context

0.2.0 brings policies (allow this kind of command in this project) and autonomy (let an agent go on
without asking). Both answer permissions without a click, which
[0004](0004-a-human-answers-permissions.md) forbids today, except for exact *Always* rules.

## Decision

A permission may also be answered by a policy when the user wrote it, saw it as a diff and clicked
to accept it, like an agent config ([0005](0005-agent-configs-backup-diff-click.md)). Policies
class actions as read, write or destructive; a deny beats an allow; destructive actions always
ask; every answer, by a human, a rule or a policy, goes to an append-only audit log the user can
read. A budget's "hard stop" only stops new sessions the app would start; it never blocks or
kills an agent the app only watches.

## Consequences

- `CLAUDE.md`'s rule 2 and the core test that pins it change in the same pull request that
  accepts this record.
- The audit log (step P8) has to exist first.
