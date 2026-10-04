# 0009. No presence mode hides a pending card

**Status:** Accepted, 2026-10-04.

## Context

Presence presets let the user choose how much the app shows at rest: the island, the tray only,
or a quiet mode. The hook is acknowledged as soon as the app queues a card, not when the card is
drawn, so the agent then waits up to 110 s for a human. A mode that hid the card would make
agents wait for nobody.

## Decision

In every preset, a pending card opens the island and plays its sound. *Quiet* only quiets what
happens at rest (sessions coming and going, finished work). There is no mode that hides cards.

## Consequences

- A test checks that no preset leaves a pending card without the island showing it.
- Users who want no interruptions at all release cards to the terminal; they do not hide them.
