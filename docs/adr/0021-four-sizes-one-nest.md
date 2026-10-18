# 0021. Surfaces: one core, four sizes, a card answered where you are

**Status:** Accepted, 2026-10-11. Consolidated on 2026-10-12 from 0008, 0009, 0022, 0023, 0024
and 0030. Amended on 2026-10-16 by [0031](0031-tasks-leave-vults.md): the Nest has no Tasks tab.
Amended on 2026-10-17 for N3 V0.3: the Dictation area and its feedback contract. These additions
land with their feature phases; the decision does not claim production UI or native validation.

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
- **Four sizes**: **Mini** is the pill (340 x 38, the flock and no text at rest: news reaches it
  only as a body, a color or a motion; active dictation has the transient feedback below).
  **Quick** is the island inside the fixed 720 x 640 logical-pixel
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
- **The Nest** has Flock, Chat, Connections and Activity (Week with *By project*,
  History, Cost, Traces, Audit), with Dictation as the fifth tab when N3 delivers it. Memory is a
  side panel of the Chat, with Settings behind a gear and
  the agents' capability table in Settings › Agents. Changing size keeps the selection: the session
  is core's `focus`; the tab, filters and chosen connection are look the app hands to the Nest. The
  Nest opens with Zeca off, without the Chat tab and *Ask Zeca* (D7). Activity keeps the spend
  figures, including historical `task:N` rows; Settings keeps the budget value. `/continue` remains
  in Chat; it does not bring back a Tasks surface. Stop is available only for an execution Vults
  started through its supported driver (D8), never merely because it watches a flock session.
- **Dictation has one owner for each kind of content.** Its Nest area owns History, Voice
  commands, Talk with Zeca, Meetings, Dictionary, Snippets and Styles as those phases ship.
  Its side list links to **Settings > Dictation**, which owns configuration and Diagnostics,
  and **Activity > Dictation**, which owns aggregate figures (V4). The shared transcription
  model and language stay in **Settings > Chat & voice**; links open that existing owner.
  No second Settings page lives in the Dictation area. Model switches, keys, budgets, insertion
  permission and agent-config changes remain in the Settings webview with their existing
  authority checks. Transcripts and meeting content stay in the Nest's webview; Diagnostics
  in Settings shows technical measurements, not transcripts or agent text. Selection, filters,
  drafts and an active recording survive size changes and visits to Settings or Activity.
  With Zeca off, independent dictation stays available; Talk with Zeca follows his opt-in.
- **Dictation feedback stays over the flock.** While active, a neutral grey line can show
  listening, hands-free, transcribing, retry, cancellation, failure or the delivery outcome
  established under [0032](0032-dictation-into-other-apps.md). Later phases add voice-command and
  spoken-reply states. There is no idle hint, second navigation, Undo or other button on this
  line; a normal island click still opens Quick. Color stays on status shapes, and a card's
  amber edge remains visible. An optional caption below carries heard text, clipped words or
  the result only when it adds information, with at most two rows for the result. It has no
  controls, takes neither focus nor mouse input and clears within two seconds of completion.
  Long text truncates within the surface, with the full line available in its detail or tooltip.
  The overlay stays mapped at its fixed size; caption drawing must not enlarge its input region.
- **Cards always take priority over dictation.** An arriving card opens in its existing owner
  immediately, with its controls and command readable; dictation never hides it, covers it,
  delays its notification or extends its deadline. If space is short, the line and caption
  yield to the card. Feedback does not answer it or grant insertion into a waiting session
  ([0032](0032-dictation-into-other-apps.md)). Dictation does not override the one-surface rule:
  while the Nest is visible, its active page carries feedback without forcing Mini or showing
  the hidden island. Cards keep the existing island/Nest routing. Feedback never steals focus
  from the captured destination.
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
  voice, Approvals, Projects, Activity, Connectors, Phone, Flock & Zeca, About; N3 adds Dictation.

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
- N3 UI checks must exercise each dictation state alongside an arriving permission or question,
  with Zeca on and off, Black and Light, and reduced motion. Native validation covers Mini/Quick
  geometry, focus and input regions, plus Room at 480 x 360 and larger sizes, on X11 and Plasma
  Wayland. The prototype's P12 visual review and the later running-app sign-offs remain separate.

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
- 2026-10-17: N3 V0.3 adds the phased fifth Nest tab, Settings and Activity ownership, transient
  dictation feedback and card priority. It follows R6's revised central Settings decision and
  Tasks removal; neither a sixth tab nor area-owned privileged settings is introduced.
