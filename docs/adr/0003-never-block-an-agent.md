# 0003. The hook never blocks an agent

**Status:** Accepted, 2026-10-04 (the rule dates from the start of the project).

## Context

The hook runs on every event of every agent. If it hangs or prints the wrong thing, the user's
work stops, and they did not ask us to be in that path.

## Decision

On any failure (app closed, slow, a bad message) the hook exits 0 with empty stdout, and the agent
goes on as if we were not there. A permission request waits for the app's acknowledgement for
800 ms at most; a human for at most the decision budget (110 s), then the agent asks in its
terminal.

## Consequences

- The hook is `std` and `serde_json` only: it starts fast and has little to break.
- Features that would need the agent to wait for us (a stop, a pause) cannot be built on the hook
  alone. See [0011](0011-only-actions-that-work.md).
