# 0021. Surfaces: one core, four sizes, a card answered where you are

**Status:** Accepted, 2026-10-11. Consolidated on 2026-10-12 from 0008, 0009, 0022, 0023, 0024
and 0030. Amended on 2026-10-16 by [0031](0031-tasks-leave-vults.md): the Nest has no Tasks tab.

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
- **Quick's action inbox**: Flock, Chat and Now, with Settings in the gear. Now presents one
  `ViewModel.inbox` from core/app, including deferred offers, session problems, actionable
  connector rows, Chat requests and memory proposals. A row opens its existing interaction
  owner; no inbox operation answers a permission or executes the destination's action.
  Activity and Connections keep their full views in the Nest.
- **Four sizes**: **Mini** is the pill (340 x 38, the flock and no text: news reaches it only as a
  body, a color or a motion). **Quick** is the island inside the fixed 720 x 640 logical-pixel
  layer surface: 440 CSS px wide normally, 492 CSS px in Chat, before the UI's 1.14 zoom
  (502 and 561 logical px respectively). Height stays within 556 CSS px (634 after zoom),
  clamped further if the viewport is smaller. Only the measured visible rectangle takes input.
  **Room** is the Nest window, resizable down to 480 x 360 px with no maximum, first opened large
  within the screen. **Full** is the Nest maximized or in full screen; Esc returns to the size
  before it. The layer-shell overlay never grows past its fixed size.
- **Surfaces at rest are layer surfaces** mapped once at a fixed size, one input region each; the
  corner widget sits in a corner the user picks and is not a window to drag. At rest one webview lives (the island, plus the
  widget if chosen); the Nest is created when opened. The palette is not a surface: it opens inside
  the island's or the Nest's page.
- **A card is answered where the user is: on the island or in the Nest.** The island holds
  permission and question cards and Ctrl+Alt+Y/N (in panel mode it anchors by the tray, the same
  window). The Nest answers the flock's cards (Allow, Deny, Always allow, a question's choices,
  Reply in the terminal) on the Flock's card of the session in front, and its Chat's cards in the
  Chat, with the island's buttons and rules: destructive keeps Deny the bright one and offers no
  Always ([0004](0004-a-human-answers-permissions.md)). The user is in the Nest when its window has
  the focus and shows the Nest, not Settings; then a card does not open the island, the Nest's
  need strip and sound say it waits, and Ctrl+Alt+Y/N answer the card on screen there. Anywhere
  else a card opens the island, so an agent never waits unseen. Going into the Nest folds an island
  that holds a card; leaving it brings the island back. Only these two answer: the app checks the
  webview behind each answer, and Settings, the widget and the palette never do. Every answer is
  still a click (or its key) by the user, and is audited. A paired phone
  may also answer ([0004](0004-a-human-answers-permissions.md)).
- **No presence mode leaves an acknowledged card unseen.** In *Island*, *Panel* and *Quiet* a
  pending card opens the island and plays its sound, or, in the Nest the user is in, shows in
  its need strip with its sound (*Quiet* only quiets what happens at rest). In
  *Paused* `core` does not acknowledge cards and releases them at once, so the agent asks in its
  terminal with no wait, and connectors stop polling.
- **The Nest** has four tabs, Flock, Chat, Connections and Activity (Week with *By project*,
  History, Cost, Traces, Audit), with Memory a side panel of the Chat, Settings behind a gear and
  the agents' capability table in Settings › Agents. Changing size keeps the selection: the session
  is core's `focus`; the tab, filters and chosen connection are look the app hands to the Nest. The
  Nest opens with Zeca off, without the Chat tab and *Ask Zeca* (D7). Activity keeps the spend
  figures, including historical `task:N` rows; Settings keeps the budget value. `/continue` remains
  in Chat; it does not bring back a Tasks surface. Stop is available only for an execution Vults
  started through its supported driver (D8), never merely because it watches a flock session.
- **One surface at a time** (the island and the Nest stay two surfaces underneath). While the Nest is open and in sight (Room or Full), the island is
  hidden, pill and sky and all, with no region that takes the mouse. It comes back when the Nest
  closes or goes back to Quick or Mini (or is minimized, where the system says so); the footer's
  **Back to the island** is Quick. Nothing else wakes it but a card while the user is outside the
  Nest; answered, it folds after its usual delay and rests hidden again.
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
  `policies::from_settings`). The two webviews share the origin's storage, which Settings never reads. Settings shows no agent's text: its frame holds tab names and counts,
  the size switch, sound, full screen, and a footer that switches Zeca and brings up the terminal of
  the session in front (it keeps only who that session is). Its pages are General, Agents, Chat &
  voice, Approvals, Projects, Activity, Connectors, Phone, Flock & Zeca, About.

## Consequences

- A test checks every preset: an acknowledged card is always shown, on the island or in the Nest
  the user is in, and *Paused* never acknowledges one. While a card waits, the agent's terminal
  says *Waiting for your answer in Vults*; the palette's *Review its card* opens it in the Nest or
  on the island. The Nest open behaves as the Panel preset's rest for the island.
- A future page picks its webview by what it shows: agents' text goes in the Nest's, powers in
  Settings'. A surface that needs a power opens Settings at its page.
- Child webviews need Tauri's `unstable` feature. On Linux the webviews of a window share its box,
  so they are switched, never laid over each other, and the hidden one is kept out of GTK's
  show-all. Opening on Settings loads the Nest's page too, hidden and resting. An event the app
  sends reaches every webview's listeners; a page still draws only what it asked for.
- On Wayland the compositor places the Nest's window; it cannot be pinned under the island's spot.

## History

- 0008, 2026-10-04: one core, many surfaces; the island is the
  only card host.
- 0009, 2026-10-04: no presence mode hides an acknowledged
  card.
- 0022, 2026-10-12: only Settings holds the powers (then as a
  window of its own).
- 0023, 2026-10-12: Settings becomes a webview of the
  Nest's window; checks go by webview label.
- 0024, 2026-10-12: the island steps aside while the Nest is open.
- 0030, 2026-10-12: the Nest answers a card too, while the user is in it.

- 2026-10-15: Chat uses a wider Quick composition within the existing fixed overlay.
  Geometry accounts for zoom once and clamps to the viewport before measurement.
  Initial native configuration matches the runtime's 720 x 640 size; Mini and the
  other Quick tabs retain their widths. User-selectable paired widths remain future work.
- 2026-10-16, [0031](0031-tasks-leave-vults.md): remove Tasks from the Nest; preserve D7, spend and
  budget, and the D8 boundary for `/continue`.

- 2026-10-17: N2.2 adds Quick's Now inbox, exact-request review and explicit links to the existing
  Nest details; cards keep their original authority, ledger and deadline.
