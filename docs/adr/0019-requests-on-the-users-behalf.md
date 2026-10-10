# 0019. Requests on the user's behalf: each one turned on, budgeted and traced

**Status:** Accepted, 2026-10-10. Amends [0006](0006-keyring-no-telemetry.md).

## Context

[0006](0006-keyring-no-telemetry.md) and [0016](0016-updates-asked-for-and-signed.md) allow one
request the app makes on its own: the update check. The plan for Zeca and the phone
(`docs/dev/plan-zeca.md`) adds more that happen while the user is not asking at that moment: web
search and fetch during a chat turn, Zeca's sleep (memory consolidation by a model), routines
(a morning brief, nightly CI), the phone's relay and push. Each one sends something the user
cares about off the machine, costs money or quota, and could become telemetry by accident.

## Decision

- Every request that leaves the machine without a click at that moment is a **feature with its own
  switch** in Settings, off by default. Turning one on says what it sends, where, and what it costs.
- Each one has a **budget** (requests, tokens or money per day) and stops when it is spent.
- Each request writes a **local trace** the user can read (what, where, when, cost). Nothing about
  it is sent anywhere else.
- All of them pass **one network gate owned by `app`**, which checks the feature's switch and
  budget. No crate opens a connection for such a feature around it.
- **Never telemetry:** no request carries usage, an id or anything about the user beyond what the
  feature needs to do its job.
- A tool call the model makes on its own during a turn (a web search or fetch) is a request on the
  user's behalf, even though the user started the turn.
- Connectors (GitHub through `gh` today) are such features: each is off until the user connects it,
  and its polling moves behind the gate, with a budget and a trace, in its own step.
- A request the user starts with a click or a message (sending a chat turn, an API chat, a cloud
  transcription they chose) stays as [0006](0006-keyring-no-telemetry.md) says: the user's choice,
  said where it is turned on.

## Consequences

- `CLAUDE.md`'s rule 4 names this record beside the update check.
- A test runs the app with every feature off and counts zero outgoing requests from the gate.
- Web tools (plan-zeca Z7), sleep (M8), routines (K4) and the relay (L2) each add their switch,
  budget and trace in the step that builds them; the connectors' poll moves behind the gate in S10.
