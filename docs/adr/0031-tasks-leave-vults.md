# 0031. Tasks leave Vults; releases follow N0 to N4

**Status:** Accepted, 2026-10-16. Implements the owner's order of work in
[Plan Zeca](../dev/plan-zeca.md#order-of-work-2026-10-16).

## Context

Tasks grew their own execution system: goals, isolated clones, context packs, review loops and
agent routing. The owner chose to remove that feature and focus Vults on the flock and Chat.
The old wave 4 release gate required Tasks, so removing the feature also changes when 0.2.0 ships.

This record sets the contract for N0. Its implementation lands in separate slices; accepting the
decision does not mean the code has already been removed or the desktop smoke has passed.

## Decision

- **Remove Tasks and everything only it uses.** No Tasks tab, Open Tasks action, `/task`,
  `/delegate`, task commands or events remain. Remove task execution in isolated clones,
  context-pack handoff, review loops, routing, their readers and writers, and their dedicated
  checks and evals. Remove `VULTS_TASK` and the take-off state it drives, with the protocol note
  needed for that change. The GitHub issue template named Task remains: it describes issues.
- **Keep `/continue` (W7).** Its driver moves out of task execution into its own module. It keeps
  its own channel; `VULTS_ZECA` still separates Zeca's Chat events from flock sessions. D8 still
  holds: Vults stops only an execution it started, now through `/continue`; watching a session
  never grants control over its process. No Stop action is offered without a supported driver.
- **Keep spend and its budget (W9).** Activity > Cost owns the figures; Settings owns the budget
  value. Existing spend rows whose source is `task:N` stay readable and count toward the same
  totals. Migrations 14 to 20 are shipped history: do not edit them. Their task tables remain
  without readers or writers; a later migration may drop them after a release.
- **Surfaces and D7:** the Nest has Flock, Chat, Connections and Activity, with Settings in its
  own webview. With Zeca off, everything except Chat and Zeca's own features keeps working,
  including records, costs and the budget. Memory stays beside Chat. Removing Tasks grants no
  new permission authority to Zeca, an agent or a surface
  ([0004](0004-a-human-answers-permissions.md)).
- **One PR per N0 slice**, each passing the full CI: the decision, surfaces, app, crates,
  protocol/core, then guides and references. Preserve the kept behavior at every slice. Removing
  an entry point before its implementation does not authorize that implementation through a
  different surface.
- **Release gates follow the stages.** N0 and N1 ship together as **0.1.9**, N2 (Flock and Chat)
  as **0.2.0**, N3 (dictation) as **0.2.1**, and N4 as **0.2.2**. An intervening fix takes the next
  patch and shifts the later stage numbers by one. The wave 4 gate is superseded; small, frequent
  releases and Linux first remain the rule ([0020](0020-0.2.0-at-wave-4-then-0.2.x.md)).
- **R3 remains a human smoke in the real desktop app before each release tag.** It covers the
  island, a pending card, Chat, voice, GitHub and the Nest on the UI refined by N1. Check the
  remaining Nest areas, `/continue` and its owned Stop, spend and budget, and operation with Zeca
  off. Tasks, isolated task execution, review and `/delegate` are no longer smoke requirements.
  Native acceptance and automated checks remain separate evidence; neither replaces R3.

## Consequences

- Vults no longer owns an internal task lifecycle or chooses an agent for `/delegate`. Code
  remains the agents' work. The API/local harness gains no code execution capability as a
  substitute; CLI-backed Chat and `/continue` keep their existing permission boundaries.
- Existing databases and spend history survive the removal. Cleanup of unused task tables is
  deferred beyond this release, rather than rewriting shipped migrations.
- [0018](0018-zeca-as-an-agent.md), [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) and
  [0021](0021-four-sizes-one-nest.md) carry the amended rules in place. Plan Zeca retains completed
  step ids as history and tracks N0's removal separately from release acceptance.
