# 0007. First release 0.1.0; one theme per version up to 0.5

**Status:** Superseded by [0015](0015-everything-by-0.2-in-small-releases.md), 2026-10-04. The
first release is still 0.1.0.

## Context

The internal checklist aimed at a first release called 1.0, but much of the product is still
ahead: more ways to show the flock, control, history and, later, autonomy.

## Decision

- The first release is **0.1.0**. It ships with the GitHub card and Zeca's seasonal looks.
- Then one theme per version:
  - **0.2 Experience**: more desktop surfaces (tray, widget, notifications), focus, presence.
  - **0.3 Control**: quick actions on the birds, a command palette, attention that escalates.
  - **0.4 Platform**: local history, projects with their branch, PR and CI, agent capabilities.
  - **0.5 Operations**: the full app, policies and autonomy, starting agents from the app.
- Linux, KDE Plasma first, through 0.5. Windows and macOS come after.

The steps live in the internal plan, `docs/dev/road-to-0.5.md`.

## Consequences

- 1.0 means "done for Linux", after 0.5. The pt-BR translation still waits for it.
- Autonomy waits until the app shows and controls everything well, and until
  [0014](0014-consent-before-autonomy.md) is accepted.
