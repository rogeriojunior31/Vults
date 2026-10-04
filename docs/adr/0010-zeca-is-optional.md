# 0010. Zeca is optional, and kept apart from the flock

**Status:** Accepted, 2026-10-04.

## Context

The app does two jobs: it shows what the user's agents are doing (the flock), and it offers a
companion who chats, listens and may later act (Zeca). Some users want only the first. The
second will grow more power over time, and power must not leak into the first.

## Decision

- Everything about the flock works with Zeca off: sessions, cards, notifications, connectors.
- With Zeca off: no chat, no microphone, no talk shortcut, no *Chat…* in the tray, and the island
  has its own idle look.
- When Zeca becomes an agent, he gets a handle that cannot build `Decide` or `DecideAlways`, by
  type. His commands and edits ask the user like any chat does today.

## Consequences

- The island needs an idle look without Zeca.
- The companion's growth never changes the guarantees of [0004](0004-a-human-answers-permissions.md).
