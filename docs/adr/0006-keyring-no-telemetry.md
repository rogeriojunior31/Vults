# 0006. Secrets only in the OS keyring; no telemetry

**Status:** Accepted, 2026-10-04 (the rule dates from the start of the project). Amended by
[0016](0016-updates-asked-for-and-signed.md) and by [0019](0019-requests-on-the-users-behalf.md)
(requests on the user's behalf: each turned on, budgeted and traced).

## Context

The app sees commands, file names, diffs and API keys. Users run it next to private work.

## Decision

A secret is written only to the OS keyring, never to a file or a log. The app sends nothing about
its use anywhere. Anything that leaves the machine (an API chat, a cloud transcription) is the
user's explicit choice and says so where it is turned on. The app makes a request on its own only
for a feature the user turned on, each with its own switch, budget and local trace, through one
network gate in `app`: the update check ([0016](0016-updates-asked-for-and-signed.md)) and the
requests on the user's behalf ([0019](0019-requests-on-the-users-behalf.md)).

## Consequences

- No usage numbers to steer the roadmap: we learn from issues and from using it ourselves.
- Anything that would learn from users' data (a trained classifier, ranking) has to learn on the
  user's machine or not at all.
