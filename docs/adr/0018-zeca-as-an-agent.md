# 0018. Zeca as an agent: a brain in Connect that never answers a permission

**Status:** Accepted, 2026-10-10. Builds on [0010](0010-zeca-is-optional.md).

## Context

Zeca chats today. Next he reads the flock, remembers, creates tasks and hands them to the user's
agents. [0010](0010-zeca-is-optional.md) promised that when he becomes an agent he gets a handle
that cannot answer a permission, and that the flock works with him off. Both need a home in the
code before the first step is built: where his brain lives, what he may do, and how his own
model calls stay apart from the sessions he watches.

## Decision

- **Where:** his brain is a `zeca` crate in the Connect layer, beside `chat`. His body (sprites,
  the island, the Nest) stays in Experience. No crate below `zeca` knows it exists; `app` wires
  it in.
- **What he reads:** the flock through a `FlockReader` built from the view, with no path to an
  `Intent`. Flock content is untrusted input: it may carry an injection.
- **What he does:** typed actions, each with a `core::policy::Class` (read, external read, write,
  destructive, spend, external), through `core`'s ledger and policy like any agent's. He never
  builds `Decide` or `DecideAlways` and never answers a permission, his own or another agent's.
- **Code is the agents' work:** on the harness (an API key or a local model) he never runs code
  himself; he makes it a task for an agent, which asks the user as it always does.
- **His work is never a session:** his CLI processes carry `VULTS_ZECA=1`; the hook marks their
  events and `core` sends them to the chat's state (taint and activity), never to the flock.
- **Off means inert:** with Zeca off, `zeca` starts no process, writes no file and makes no
  request.

## Consequences

- `scripts/check-layers.sh` already places `zeca` in Connect; `docs/architecture.md` and
  `CLAUDE.md` say where his brain and body live.
- A compile-fail test pins that `FlockReader` cannot reach an `Intent` (plan-zeca S9).
- The protocol gains a field for the marked event (version 6) when native mode lands (plan-zeca Z1).
