# 0021. Surfaces: one core, four sizes, one card host

**Status:** Accepted, 2026-10-11. Consolidated on 2026-10-12 from 0008, 0009, 0022, 0023 and 0024.
Its one card host is amended by [0030](0030-a-card-is-answered-where-you-are.md): the Nest answers a
card too, while the user is in it.

## Context

Users want several levels of presence: the island, a tray icon, a corner widget, notifications, a
palette, a full app. Built naively, each would re-implement who needs attention, which session is
in front and how a card ended. The owner's Living Flock prototype
(`design/explorations/08-living-flock.html`) draws the app as one identity at four distances, with
the flock in a shared sky in every size. Some surfaces show agents' text, which may carry words
meant to steer whatever reads them.

## Decision

- **Meaning in core, look in the surface.** `core` says who needs the user (an ordered attention
  level), which session is in focus, and how each card ended (here, in the terminal, expired, by a
  rule). Surfaces choose clips, icons, colors and sounds, from one small view model (a projection
  per surface only when a payload grows), and speak only in `Intent`s through the same commands.
- **Four sizes**: **Mini** is the pill (340 x 38, the flock and no text: news reaches it only as a
  body, a color or a motion). **Quick** is the island (440 px wide) inside the fixed layer surface.
  **Room** is the Nest window, resizable down to 480 x 360 px with no maximum, first opened large
  within the screen. **Full** is the Nest maximized or in full screen; Esc returns to the size
  before it. The layer-shell overlay never grows past its fixed size.
- **Surfaces at rest are layer surfaces** mapped once at a fixed size, one input region each; the
  corner widget sits in a corner the user picks. At rest one webview lives (the island, plus the
  widget if chosen); the Nest is created when opened. The palette is not a surface: it opens inside
  the island's or the Nest's page.
- **The island is the only card host on the desktop.** Permission and question cards and
  Ctrl+Alt+Y/N live there (in panel mode it anchors by the tray, the same window). Every other
  surface, Room, Full and the palette included, can only bring the card up and never shows Allow
  or Deny. A paired phone may also answer ([0004](0004-a-human-answers-permissions.md)).
- **No presence mode leaves an acknowledged card unseen.** In *Island*, *Panel* and *Quiet* a
  pending card opens the island and plays its sound (*Quiet* only quiets what happens at rest). In
  *Paused* `core` does not acknowledge cards and releases them at once, so the agent asks in its
  terminal with no wait, and connectors stop polling.
- **The Nest** has five tabs, Flock, Tasks, Chat, Activity (Week with *By project*, History, Cost,
  Traces, Audit) and Connections, with Memory a side panel of the Chat, Settings behind a gear and
  the agents' capability table in Settings › Agents. Changing size keeps the selection: the session
  is core's `focus`; the tab, filters and chosen connection are look the app hands to the Nest. The
  Nest opens with Zeca off, without the Chat tab and *Ask Zeca*.
- **One surface at a time.** While the Nest is open and in sight (Room or Full), the island is
  hidden, pill and sky and all, with no region that takes the mouse. It comes back when the Nest
  closes or goes back to Quick or Mini (or is minimized, where the system says so); the footer's
  **Back to the island** is Quick. Nothing else wakes it but a card, which opens the island while
  the Nest is open; answered, it folds after its usual delay and rests hidden again.
- **The Nest's window** (`nest-window`) moves, resizes and is in Alt+Tab, without the system's
  title bar: the prototype's rounded frame, a header that moves it, edges that resize it. It holds
  two webviews under one header, the Nest's (`nest`) and Settings' (`settings`), one shown at a
  time. The gear shows Settings; a tab shows the Nest; `open_settings(section)` opens the window on
  Settings, at a page when asked.
- **Only Settings' webview holds the powers**: writing an agent's config
  ([0005](0005-agent-configs-backup-diff-click.md)), saving or clearing a key, making or taking a
  policy, pairing, revoking or reaching a phone ([0004](0004-a-human-answers-permissions.md)), and
  turning on a request on the user's behalf ([0019](0019-requests-on-the-users-behalf.md)).
  Commands check the label of the webview that called, never the window's (`settings_page`,
  `policies::from_settings`). Settings shows no agent's text: its frame holds tab names and counts,
  the size switch, sound, full screen, and a footer that switches Zeca and brings up the terminal of
  the session in front (it keeps only who that session is). Its pages are General, Agents, Chat &
  voice, Approvals, Projects, Activity, Connectors, Phone, Flock & Zeca, About.

## Consequences

- A test checks every preset: an acknowledged card always has the island showing it, and *Paused*
  never acknowledges one. The Nest open behaves as the Panel preset's rest for the island.
- A future page picks its webview by what it shows: agents' text goes in the Nest's, powers in
  Settings'. A surface that needs a power opens Settings at its page.
- Child webviews need Tauri's `unstable` feature. On Linux the webviews of a window share its box,
  so they are switched, never laid over each other, and the hidden one is kept out of GTK's
  show-all. Opening on Settings loads the Nest's page too, hidden and resting. An event the app
  sends reaches every webview's listeners; a page still draws only what it asked for.
- On Wayland the compositor places the Nest's window; it cannot be pinned under the island's spot.

## History

- [0008](0008-one-core-many-surfaces.md), 2026-10-04: one core, many surfaces; the island is the
  only card host.
- [0009](0009-presence-never-hides-a-card.md), 2026-10-04: no presence mode hides an acknowledged
  card.
- [0022](0022-settings-stays-a-window.md), 2026-10-12: only Settings holds the powers (then as a
  window of its own).
- [0023](0023-settings-a-webview-of-the-nest.md), 2026-10-12: Settings becomes a webview of the
  Nest's window; checks go by webview label.
- [0024](0024-one-surface-at-a-time.md), 2026-10-12: the island steps aside while the Nest is open.
