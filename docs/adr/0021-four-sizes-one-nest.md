# 0021. Four sizes of attention, one Nest with five tabs

**Status:** Accepted, 2026-10-11. Amends [0020](0020-0.2.0-at-wave-4-then-0.2.x.md): 0.2.0 ships
with this layout.

## Context

Wave 4 grew the Nest to nine tabs (Chat, Tasks, Memory, Roosts, History, Usage, Traces, Audit,
Agents). Settings is a window of its own, the week recap is one of its pages, and the GitHub board is
a tab of the island. Nine tabs do not fit a narrow window, and the same records sit in three places.
The Nest only opens with Zeca on, so the flock's records and the tasks vanish with him.

The owner's Living Flock prototype (`design/explorations/08-living-flock.html`) redraws the app as
one identity at four distances, with the flock in a shared sky in every size.

## Decision

- **Four sizes**:
  - **Mini** is the pill, 340 x 38, with the flock and no text. Anything new reaches it only as a body, a color or a motion.
  - **Quick** is the island, 440 px wide, inside today's fixed layer surface.
  - **Panel** is the Nest window, resizable from 720 to 1280 px.
  - **Full** is the Nest maximized or in full screen; Esc returns to the size before it.
  - The layer-shell overlay never grows past its fixed size for Panel or Full.
- **The Nest has five tabs**, Flock, Tasks, Chat, Activity and Connections, and Settings behind a gear:
  - Memory is a side panel of the Chat.
  - Activity holds Week (with *By project*, which replaces Roosts), History, Cost (which replaces Usage), Traces and Audit.
  - The agents' capability table moves into Settings › Agents.
  - Settings stops being a window of its own.
- **Changing size keeps the selection.** The selected session is core's `focus`, as today. The tab, the filters and the chosen connection are look, so the app keeps them and hands them to the Nest when it opens.
- **The Nest opens with Zeca off.** Without him there is no Chat tab and no *Ask Zeca*. The flock, tasks, records, connections and settings all stay.
- **Cards stay where they are.** The island is still the only card host on the desktop ([0008](0008-one-core-many-surfaces.md)). Panel and Full lead to it and never show Allow or Deny. The command palette never answers a card either.

## Consequences

- The Nest gains a second reason to exist besides Zeca, so [0010](0010-zeca-is-optional.md) holds in both directions: the flock's own surfaces never depend on him.
- At rest there is still one live webview, the island. With Settings inside the Nest, an open app has two webviews at most instead of three.
- Almost every island screenshot changes, so the visual baselines are refreshed step by step (`docs/dev/plan-living-flock.md`, U0 to U13).
- The 0.2.0 gate is the R3 smoke on this layout, after U1 and again after U13.
