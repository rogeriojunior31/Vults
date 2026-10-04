# 0009. No presence mode leaves an acknowledged card unseen

**Status:** Accepted, 2026-10-04.

## Context

Presence presets let the user choose how much the app shows at rest: the island, the tray only,
a quiet mode, or a pause. The hook is acknowledged as soon as the app queues a card, not when the
card is drawn, so the agent then waits up to 110 s for a human. A mode that hid an acknowledged
card would make agents wait for nobody.

## Decision

- In *Island*, *Panel* and *Quiet*, a pending card opens the island and plays its sound. *Quiet*
  only quiets what happens at rest (sessions coming and going, finished work).
- In *Paused*, `core` does not acknowledge cards and releases them at once: the agent asks in its
  terminal with no wait. Connectors stop polling while paused.
- No mode hides an acknowledged card.

## Consequences

- A test checks every preset: an acknowledged card always has the island showing it, and
  *Paused* never acknowledges one.
- Users who want no interruptions pause the app; agents then behave as if it were closed.
