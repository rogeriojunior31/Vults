# 0006. Secrets only in the OS keyring; no telemetry

**Status:** Accepted, 2026-10-04 (the rule dates from the start of the project).

## Context

The app sees commands, file names, diffs and API keys. Users run it next to private work.

## Decision

A secret is written only to the OS keyring, never to a file or a log. The app sends nothing about
its use anywhere. Anything that leaves the machine (an API chat, a cloud transcription) is the
user's explicit choice and says so where it is turned on.

## Consequences

- No usage numbers to steer the roadmap: we learn from issues and from using it ourselves.
- Anything that would learn from users' data (a trained classifier, ranking) has to learn on the
  user's machine or not at all.
