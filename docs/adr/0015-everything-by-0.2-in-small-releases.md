# 0015. Everything planned lands by 0.2.0, in small 0.1.x releases

**Status:** Superseded by [0020](0020-0.2.0-at-wave-4-then-0.2.x.md), 2026-10-10. Supersedes
[0007](0007-versions-to-0.5.md).

## Context

[0007](0007-versions-to-0.5.md) gave each version up to 0.5 one theme (Experience, Control,
Platform, Operations). That kept users waiting months between releases for work that is ready in
days, and made a version number promise a whole theme.

## Decision

- The first release is still **0.1.0**.
- After it, releases are small and frequent: a **0.1.x** goes out whenever one or two planned
  steps are merged and a short smoke check passes. Urgent fixes are the next 0.1.x too.
- Everything planned (desktop surfaces, control, voice, history, roosts) lands in 0.1.x
  releases, in the order of the waves in the internal plan (`docs/dev/road-to-0.2.md`).
- **0.2.0** is the Operations milestone: the full app, policies and starting agents. It starts
  only after [0014](0014-consent-before-autonomy.md) is accepted.
- Linux, KDE Plasma first, through 0.2.0. Windows and macOS come after.

## Consequences

- Users get each feature when it is ready; the changelog is per 0.1.x.
- A patch number no longer means "fixes only" here: 0.1.x releases carry features. Patch
  numbers may pass 9.
- 1.0 still means "done for Linux"; the pt-BR translation still waits for it.
