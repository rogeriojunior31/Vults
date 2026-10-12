# 0024. One surface at a time: the island steps aside for the Nest

**Status:** Accepted, 2026-10-12. Amends [0021](0021-four-sizes-one-nest.md).

## Context

The Living Flock prototype draws Mini, Quick, Room and Full as one surface that grows from the top
of the screen: the pill becomes the island, the island becomes the panel. The app built them as
two: the island, a layer surface at the top, and the Nest, a window (0021, 0023). With the Nest
open the island stayed at the top, its pill and its birds beside the window that already shows the
whole flock. The user's verdict: in the larger size the birds up there are not needed.

## Decision

- While the Nest is open and in sight (Room or Full), the island steps aside: hidden, pill and sky
  and all, with no region that takes the mouse and nothing that wakes it.
- It comes back when the Nest closes or goes back to Quick or Mini (and, where the system says a
  window was minimized, as on X11, while it is). The footer's **Back to the island** is Quick: it
  closes the Nest and opens the island. Nothing else wakes it: no news, digest, recap or hello.
- A card still opens the island while the Nest is open: only the island answers a card (D1), and
  an agent never waits unseen (rule 1). Answered, the island folds after its usual delay and rests
  hidden again.
- The Nest stays a window (moved, resized, in Alt+Tab) without the system's title bar: it wears the
  prototype's rounded frame, its header moves it and its edges resize it. Settings stays its own
  webview in it (0023); on Linux the hidden webview is kept out of GTK's show-all. The island and the Nest stay two surfaces underneath; only one is on screen.

## Consequences

- One thing on screen at a time, as the prototype draws it.
- On Wayland the compositor places the Nest's window; it cannot be pinned under the island's spot.
- The presets (0009) keep their meaning; the Nest open behaves as the Panel preset's rest for the
  island while it lasts.
