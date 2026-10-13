# 0018. Zeca: optional, apart from the flock, rules before models

**Status:** Accepted, 2026-10-10. Consolidated on 2026-10-12 from 0010 and 0012.

## Context

The app does two jobs: it shows what the user's agents are doing (the flock), and it offers a
companion who chats, listens, reads the flock, remembers, creates tasks and hands them to the
user's agents (Zeca). Some users want only the first, and the second's power must not leak into
it. A small local decision model was also proposed for the app's own choices, but the events it
receives are already typed, and such models without fine-tuning score below a majority-class
guess, with confidence we cannot calibrate without collecting data
([0019](0019-requests-on-the-users-behalf.md)).

## Decision

- **Optional.** Everything about the flock works with Zeca off: sessions, cards, notifications,
  connectors, the Nest with its tasks and records ([0021](0021-four-sizes-one-nest.md)). With him
  off: no chat, no microphone, no talk shortcut, no *Chat…* in the tray, no Chat tab or *Ask Zeca*,
  the island has its own idle look, and `zeca` starts no process, writes no file and makes no
  request.
- **Where:** his brain is a `zeca` crate in the Connect layer, beside `chat`; his body (sprites,
  the island, the Nest) stays in Experience. No crate below `zeca` knows it exists; `app` wires it in.
- **What he reads:** the flock through a `FlockReader` built from the view, with no path to an
  `Intent`. Flock content is untrusted input: it may carry an injection.
- **What he does:** typed actions of his own; `zeca` does not depend on `vults-core`, so `app` maps
  each to `core`'s policy classes (read, external read, write, destructive, spend, external) and
  ledger, like any agent's. He never builds `Decide` or `DecideAlways`, by type, and never answers a
  permission, his own or another agent's ([0004](0004-a-human-answers-permissions.md)).
- **What he remembers:** a git repository of his own (through `gix`): every write is a commit,
  undone by a revert.
- **Code is the agents' work:** on the harness (an API key or a local model) he never runs code
  himself; he makes it a task for an agent, which asks the user as it always does. His commands and
  edits in the chat ask the user like any chat.
- **His work is never a session:** his CLI processes carry `VULTS_ZECA=1`; the hook marks their
  events and `core` sends them to the chat's state (taint and activity), never to the flock. Any
  process could set that variable, so a marked event never hides or drops a permission request: it
  is still asked of the user, through the chat's approver.
- **Rules before models** in the app's own decisions:
  - What can be decided from typed state is decided by plain rules in `core`, with tests:
    notifications, attention, Zeca's look, which agent is installed and has quota. Routing a task
    between agents is a user preference plus availability, not a guess.
  - Free text from the user (what they ask Zeca) goes to the chat model.
  - A learned classifier may come later only behind the same interface, only for free text, only
    local, and only if an evaluation set on this repository shows the rules fall short.
  - No model ever answers a permission.

## Consequences

- The companion's growth never changes who answers a permission; decisions stay exact, testable
  and explainable.
- `scripts/check-layers.sh` places `zeca` in Connect; a compile-fail test pins that `FlockReader`
  cannot reach an `Intent`.
- The protocol gained a field for the marked event (version 6).

## History

- 0010, 2026-10-04: Zeca is optional and kept apart from the flock;
  his handle cannot build a decision.
- 0012, 2026-10-04: rules before models in the app's own decisions.
