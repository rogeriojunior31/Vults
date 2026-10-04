# 0008. One core, many desktop surfaces; the island is the only card host

**Status:** Accepted, 2026-10-04.

## Context

The island at the top of the screen is the only view today. Users want other levels of presence:
a tray icon, a corner widget, notifications, a palette, later a side panel and a full app. Built
naively, each would re-implement who needs attention, which session is in front and how a card
ended. Some of that logic lives only in the island's renderer today.

## Decision

- **Meaning in core, look in the surface.** `core` says who needs the user (an ordered attention
  level), which session is in focus, and how each card ended (here, in the terminal, expired, by a
  rule). Surfaces choose clips, icons, colors and sounds.
- **One view model for every surface.** It is small; a projection per surface comes only when a
  payload grows (history).
- **Every surface speaks in `Intent`s** through the same commands. No surface gets its own path
  into the state.
- **The island window is the only card host.** Permission and question cards, and the
  Ctrl+Alt+Y/N shortcuts, live there. In panel mode the island anchors by the tray instead of the
  top, but it is the same window. Every other surface can only bring the card up.
- **New surfaces are layer surfaces** mapped once at a fixed size, like the island. The corner
  widget is fixed in a corner the user picks; it is not a window to drag.
- **One webview lives at rest** (the island, plus the widget if chosen). The palette and the full
  app are created when opened and closed when done.

## Consequences

- Moving attention, focus and outcomes into `core` comes before the first new surface, and must
  leave the island looking exactly the same (visual tests with no new baselines).
- The layer-shell code becomes per window (one input region per surface).
- Answering a card stays in one place, which keeps [0004](0004-a-human-answers-permissions.md)
  simple to check.
