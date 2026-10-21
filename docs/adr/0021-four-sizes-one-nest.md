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
  **Quick** is the island inside the fixed 880 x 640 logical-pixel
  layer surface: 440 CSS px wide normally, 492 CSS px in Chat, before the UI's 1.14 zoom
  (502 and 561 logical px respectively). Height stays within 556 CSS px (634 after zoom),
  clamped further if the viewport is smaller. Mini and Quick may share a user-selected width of
  560, 680 or 760 logical pixels, saved as local presentation state. Default restores the original
  per-size/per-tab widths and Quick height; Room and Full are unaffected. Mini stays 38 logical
  pixels high for every width. A selected width caps Quick at 600 logical pixels, including inline
  dictation feedback. General > Appearance and Quick > General edit the same presentation
  preference; shared storage notifications and refocus keep both controls current.
  Selected dimensions are clamped to both the viewport and available screen space, with Quick
  zoom applied exactly once. Selection never
  resizes the mapped layer surface. Only the measured visible rectangle takes input.
  Mini's overflow is an icon-only list button with a localized hidden-session count for assistive
  technology. Reserve a separate hit target before placing birds. It opens the Nest's Flock through
  the existing surface transition; it never decides, releases or changes an offer.
  **Room** is the Nest window, resizable down to 480 x 360 px with no maximum, first opened large
  within the screen. **Full** is the Nest maximized or in full screen; Esc returns to the size
  before it. The layer-shell overlay never grows past its fixed size.
- **Surfaces at rest are layer surfaces** mapped once at a fixed size, one input region each; the
  corner widget sits in a corner the user picks and is not a window to drag. At rest the island and passive Linux dictation webview live, plus the
  widget if chosen; the Nest is created when opened. The dictation surface stays transparent when idle. The palette is not a surface: it opens inside
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
  commands, Talk with Zeca, Meetings, Dictionary and Snippets as those phases ship.
  The Dictation tab also presents its configuration in separate Voice, Keyboard shortcuts,
  Command settings, Vocabulary, Styles, Paste, Privacy and Diagnostics pages, with the same
  navigation as its work pages. Voice owns the shared transcription model and spoken language;
  Chat & voice links there. Statistics links to **Activity > Dictation**, the aggregate owner.
  Configuration still runs in the Settings webview, with existing label checks; Nest gains no
  model, key, budget, insertion-permission or agent-config authority. Each setting has one editor.
  Transcripts and meeting content stay in the Nest's webview; Diagnostics
  in Settings shows technical measurements, not transcripts or agent text. Selection, filters,
  drafts and an active recording survive size changes and visits to Settings or Activity.
  With Zeca off, independent dictation stays available; Talk with Zeca follows his opt-in.
- **Linux dictation feedback has its own passive capsule.** A fixed bottom-centered surface
  shows a microphone and measured bands while listening, an amber border hands-free, and existing
  transcription/result states. It uses the production theme, never takes keyboard or pointer
  input, and stays transparent when idle. An optional caption above holds heard/result text and
  retains its existing two-second result expiry. There are no buttons or independent audio
  owners. Island/Nest feedback is suppressed while this surface exists; creation failure retains
  inline feedback. This is a passive status exception to the main island/Nest surface rule.
- **Cards always take priority over dictation.** Pending cards, screen lock and Paused presence
  hide the capsule; expired results do not return afterward. Feedback never covers, answers or
  delays a card, grants insertion, or steals focus. Cards retain their existing island/Nest
  routing. The fixed surface remains mapped with an empty input region, even while visible.
- **One surface at a time** (the island and the Nest stay two surfaces underneath). While the Nest is open and in sight (Room or Full), the island is
  hidden, pill and sky and all, with no region that takes the mouse. It comes back when the Nest
  closes or goes back to Quick or Mini (or is minimized, where the system says so); the footer's
  **Back to the island** is Quick. Nothing else wakes it but a card while the user is outside the
  Nest; answered, it folds after its usual delay and rests hidden again.
- **The Nest's window** (`nest-window`) moves, resizes and is in Alt+Tab, without the system's
  title bar: the prototype's rounded frame, a header that moves it, edges that resize it. It holds
  two webviews under one header, the Nest's (`nest`) and Settings' (`settings`), one shown at a
  time. The gear shows Settings; a tab shows the Nest; `open_settings(section)` opens the window on
  Settings, at a page when asked. Dictation configuration uses that same privileged webview
  while keeping the Dictation tab selected.
- **Only Settings' webview holds the powers**: writing an agent's config
  ([0005](0005-agent-configs-backup-diff-click.md)), saving or clearing a key, making or taking a
  policy, pairing, revoking or reaching a phone ([0004](0004-a-human-answers-permissions.md)), and
  turning on a request on the user's behalf ([0019](0019-requests-on-the-users-behalf.md)).
  Commands check the label of the webview that called, never the window's (`settings_page`,
  `policies::from_settings`). The two webviews share the origin's storage, which Settings never reads. Settings shows no agent's text: its frame holds tab names and counts,
  the size switch, sound, full screen, and a footer that switches Zeca and brings up the terminal of
  the session in front (it keeps only who that session is). Its pages are General, Agents, Chat &
  voice, Approvals, Projects, Activity, Connectors, Phone, Flock & Zeca and About. N3 Dictation
  configuration uses this webview under its own tab, outside the global Settings sidebar.

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

- 2026-10-18: consolidate dictation configuration under the Dictation tab at the maintainer's
  request, with one editor per setting and shared navigation. Move the shared transcription
  controls there and link from Chat. Keep configuration in the Settings webview and all
  transcript work pages in Nest; approval, recording and privileged-command authority is unchanged.

- 2026-10-19: move Linux dictation status to a passive bottom capsule, with existing inline
  feedback as a creation-failure fallback; preserve card/lock priority and the captured focus.

- 2026-10-21: UX7 adds a shared Mini/Quick width preference and a fixed 880 x 640 overlay
  reserved at startup. Keep the original default widths, Mini height, Room/Full behavior and
  authority; selection changes only content geometry and its synchronous input rectangle.

- 2026-10-21: UX7.2 applies the paired 56/600 logical-pixel height references only to selected
  widths. Preserve original defaults and sprite scale; bound the whole Quick box, including
  inline feedback, before synchronous input-region measurement.

- 2026-10-21: UX7.3 reserves a Mini overflow action for sessions beyond the available perches.
  Keep the resting pill free of visible text, with a separate keyboard target and the existing
  Nest/Flock navigation. Cards retain their priority and original action owner.

- 2026-10-21: maintainer feedback restores Mini's 38 px height for every width, avoiding extra
  resting-bar height over application tabs. Expose the same layout preference in General and
  Quick, with shared storage and no additional privileged setting or permission authority.

- 2026-10-21: UX8 starts with Agent profiles inside Flock, preserving the five main tabs. The Nest reads the built-in catalog and retains the bounded `flock-profiles` destination. The catalog is independent of Zeca and never owns a permission or agent-config write.
- 2026-10-21: Agent profiles in Flock can create local copies and edit their name/instructions. The Nest alone owns these saves; drafts survive in-window navigation, and failed saves retain them. No new main tab, agent-config write or permission owner is introduced.
- 2026-10-21: The profile editor adds native species/accessory selectors and a static full-frame preview. Appearance edits share the same local Save/Cancel and revision boundary; neither Zeca settings nor watched sessions are modified.
- 2026-10-21: Profile-bound conversation records retain their own immutable snapshot across profile edits, rename, restart and reopen. The current Zeca Chat does not expose these records or reinterpret them as companion conversations; no profile Chat button or recipient selector is enabled in this storage slice.
- 2026-10-21: The first specialist runtime is Claude Code only, with a fixed brief and no automatic companion context or MCP. No recipient selector, profile Chat action or specialist conversation reopening is enabled by this runtime-only slice; the existing Nest and Quick remain on their current conversation owner.
- 2026-10-21: Native profile conversation restoration preserves the saved model, effort, session and profile snapshot rather than current profile edits or companion settings. Turn preflight rejects mismatched recipients, sources and unsupported voice/mention/source overrides. The existing Chat UI still refuses profile records until it can present and preserve these owners across surfaces.
