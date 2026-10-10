# 0004. A permission is answered by a human, or by an exact rule a human made

**Status:** Accepted, 2026-10-04. Amended by [0014](0014-consent-before-autonomy.md), 2026-10-10: a
policy the user wrote, saw as a diff and accepted with a click may also answer.

## Context

Allowing a command is the one thing the app does that can cause harm. It must never come from a
timer, a guess, a model or another program.

## Decision

`core` turns a pending permission into an answer from three inputs only:

- `Intent::Decide`: a click on Allow or Deny (or Ctrl+Alt+Y/N with the card visible).
- `Intent::DecideAlways`: a click on Always, which allows this request and saves a rule.
- A saved rule that matches **exactly**: the same agent, folder, tool and target. `cargo test`
  does not allow `cargo test && rm -rf build`.

A test feeds every other input and checks that none answers. Every new `Intent` joins that test.

## Consequences

- No notification, tray item, palette or assistant can allow anything; they can only bring the
  card up ([0008](0008-one-core-many-surfaces.md)).
- Broader rules (patterns, policies) need a new consent rule first ([0014](0014-consent-before-autonomy.md)).
