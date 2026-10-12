# 0023. Settings, a webview of the Nest's window

**Status:** Accepted, 2026-10-12. Supersedes [0022](0022-settings-stays-a-window.md).

## Context

[0022](0022-settings-stays-a-window.md) kept Settings a window of its own. Only Settings may write
an agent's config, a key, a policy, a phone or a request on the user's behalf, and that trust holds
because it shows no agent's text. The user asked for one window. The trust never needed a separate
window: it needs a separate webview. A webview is its own page, with its own label, its own
scripts and its own memory. A page in one webview cannot read, script or click another.

## Decision

- The Nest's window (`nest-window`) holds two webviews: the Nest's (`nest`) and Settings'
  (`settings`). One is shown at a time, both under the same header (tabs, size switch, gear).
  The gear shows Settings; a tab shows the Nest at that tab. `open_settings(section)` opens the
  window on Settings, at a page when asked.
- Commands check the label of the webview that called, never the window's. The Settings-only calls
  of 0022 (`settings_page`, `policies::from_settings`) stay refused from the Nest's webview, which
  shares the window and shows agents' text.
- Settings' webview still shows no agent's text. The header it draws holds tab names, the size
  switch and the gear, nothing an agent wrote.
- The page titles of 0022 stay.

## Consequences

- One window for the Nest and Settings, as the user asked; the island and the widget stay surfaces
  of their own.
- Child webviews need Tauri's `unstable` feature. On Linux, the webviews of a window share its box:
  the hidden one takes no room, so they are switched, never laid over each other.
- An event the app sends reaches the listeners of every webview, as it did across windows; a page
  still draws only what it asked for.
- A future page in that window picks its webview by what it shows: agents' text goes in the Nest's,
  powers go in Settings'.
