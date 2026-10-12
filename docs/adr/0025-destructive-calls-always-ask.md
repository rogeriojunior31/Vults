# 0025. A destructive call always asks, an Always rule included

**Status:** Accepted, 2026-10-12. Amends [0004](0004-a-human-answers-permissions.md) and
[0014](0014-consent-before-autonomy.md).

## Context

[0014](0014-consent-before-autonomy.md) says destructive actions always ask, and the card says so:
*"Destructive: no policy ever allows it"*. But the same card offered **Always allow**, and an exact
*Always* rule ([0004](0004-a-human-answers-permissions.md)) answered every later identical call with
no card. One click on `rm -rf target` allowed that command in that folder forever, against what the
card had just said.

## Decision

- A destructive call always asks: no policy and no *Always* rule answers it.
- Its card offers no **Always allow**. If the core gets one anyway (an older UI, the palette), it is
  a plain Allow for that card, audited as such, and no rule is saved.
- A rule saved before this record never answers a destructive call either.

## Consequences

- `CLAUDE.md`'s rule 2 and [Safety](../safety.md) say the exception holds for rules too.
- One test pins each half: Always on a destructive card saves no rule, and an old rule does not
  answer one.
