# 0012. Rules before models in the app's own decisions

**Status:** Accepted, 2026-10-04.

## Context

A small local decision model (a "System 1" classifier) was proposed to choose when to notify, how
Zeca should look, which agent to send a task to, and when to call a large model. The events the
app receives are already typed (a status, a tool, a permission), and the published accuracy of
such models without fine-tuning is below a majority-class guess, with confidence that needs
calibration we cannot do without collecting data ([0006](0006-keyring-no-telemetry.md)).

## Decision

- What can be decided from typed state is decided by plain rules in `core`, with tests:
  notifications, attention, Zeca's look, which agent is installed and has quota.
- Free text from the user (what they ask Zeca) goes to the chat model, which is already there.
- A learned classifier may come later only behind the same interface, only for free text, only
  local, and only if an evaluation set on this repository shows the rules fall short.
- No model ever answers a permission ([0004](0004-a-human-answers-permissions.md)).

## Consequences

- Decisions are exact, testable and explainable.
- Routing a task between agents is a user preference plus availability, not a guess.
