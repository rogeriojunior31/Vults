# 0008. One core, many desktop surfaces; the island is the only card host

**Status:** Accepted, 2026-10-04. Amended by [0017](0017-a-signed-phone-decision-is-a-click.md),
2026-10-10: a paired phone may answer a card with a biometric signature; on the desktop the island
is still the only card host. Amended by [0021](0021-four-sizes-one-nest.md),
[0023](0023-settings-a-webview-of-the-nest.md) and [0024](0024-one-surface-at-a-time.md): the Nest
is a normal window, and the palette lives inside the island's and the Nest's pages. Superseded in
part by [0030](0030-a-card-is-answered-where-you-are.md): the Nest answers a card too, while the
user is in it.

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
- **Surfaces at rest are layer surfaces** mapped once at a fixed size, like the island. The corner
  widget is fixed in a corner the user picks; it is not a window to drag. The Nest (the full app)
  is a normal window the user moves and resizes.
- **One webview lives at rest** (the island, plus the widget if chosen). The Nest is created when
  opened. The palette is not a surface: it opens inside the island's or the Nest's page.

## Consequences

- Moving attention, focus and outcomes into `core` comes before the first new surface, and must
  leave the island looking exactly the same (visual tests with no new baselines).
- The layer-shell code becomes per window (one input region per surface).
- Answering a card stays in one place, which keeps [0004](0004-a-human-answers-permissions.md)
  simple to check.
