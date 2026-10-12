# 0021. Four sizes of attention, one Nest with five tabs

**Status:** Accepted, 2026-10-11. Amends [0020](0020-0.2.0-at-wave-4-then-0.2.x.md): 0.2.0 ships
with this layout. Amended by [0023](0023-settings-a-webview-of-the-nest.md) (Settings is a webview
of the Nest's window; it superseded [0022](0022-settings-stays-a-window.md)) and
[0024](0024-one-surface-at-a-time.md) (the island steps aside while the Nest is open); the text
below includes both. Superseded in part by [0030](0030-a-card-is-answered-where-you-are.md): the
Nest answers cards.

## Context

Wave 4 grew the Nest to nine tabs (Chat, Tasks, Memory, Roosts, History, Usage, Traces, Audit,
Agents). Settings was a window of its own, the week recap one of its pages, and the GitHub card
opened from a tab of the island. Nine tabs did not fit a narrow window, the same records sat in
three places, and the Nest only opened with Zeca on, so the flock's records and the tasks vanished
with him.

The owner's Living Flock prototype (`design/explorations/08-living-flock.html`) redraws the app as
one identity at four distances, with the flock in a shared sky in every size.

## Decision

- **Four sizes**:
  - **Mini** is the pill, 340 x 38, with the flock and no text. Anything new reaches it only as a body, a color or a motion.
  - **Quick** is the island, 440 px wide, inside the fixed layer surface.
  - **Room** is the Nest window, resizable down to 480 x 360 px with no maximum; it first opens large, within the screen.
  - **Full** is the Nest maximized or in full screen; Esc returns to the size before it.
  - The layer-shell overlay never grows past its fixed size for Room or Full.
- **The Nest has five tabs**, Flock, Tasks, Chat, Activity and Connections, and Settings behind a gear:
  - Memory is a side panel of the Chat.
  - Activity holds Week (with *By project*, which replaces Roosts), History, Cost (which replaces Usage), Traces and Audit.
  - The agents' capability table moves into Settings › Agents.
  - Settings is a webview in the Nest's window, not a window of its own (0023).
- **Changing size keeps the selection.** The selected session is core's `focus`. The tab, the filters and the chosen connection are look, so the app keeps them and hands them to the Nest when it opens.
- **The Nest opens with Zeca off.** Without him there is no Chat tab and no *Ask Zeca*. The flock, tasks, records, connections and settings all stay.
- **Cards stay where they are.** The island is still the only card host on the desktop ([0008](0008-one-core-many-surfaces.md)). Room and Full lead to it and never show Allow or Deny. The command palette never answers a card either.

## Consequences

- The Nest gains a reason to exist besides Zeca, so [0010](0010-zeca-is-optional.md) holds in both directions: the flock's own surfaces never depend on him.
- At rest there is one live webview, the island (plus the corner widget while it is chosen). The Nest's window holds two, the Nest's and Settings', one shown at a time (0023).
- Almost every island screenshot changes, so the visual baselines are refreshed step by step (`docs/dev/plan-zeca.md`, U0 to U13).
- The 0.2.0 gate is the R3 smoke on this layout: once after U1, then again after U13. The tag comes after the second smoke.
