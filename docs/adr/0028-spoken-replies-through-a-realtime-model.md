# 0028. Spoken replies through an opt-in realtime model

**Status:** Accepted, 2026-10-12. Supersedes the spoken replies and cloud transcription of
[0013](0013-voice-local-first.md); the rest of 0013 stands.

## Context

[0013](0013-voice-local-first.md) planned spoken replies from a local engine with permissive
licenses, and an opt-in cloud transcription. The local voice that shipped (Kokoro, with
`espeak-ng` phonemes) was removed on 2026-10-07: it sounded too poor to keep, above all in
Portuguese. `crates/voice` has no text to speech today. Cloud transcription was never
built, and the user dropped it from the plan on 2026-10-12.

## Decision

- Speech to text stays local: whisper.cpp, as [0013](0013-voice-local-first.md) says.
- Spoken replies come from a realtime model the user turns on (`docs/dev/plan-zeca.md`, K8). It is
  a request on the user's behalf ([0019](0019-requests-on-the-users-behalf.md)): off by default,
  with its own switch, budget and trace, through the network gate in `app`. Its key lives only in
  the OS keyring.
- Turning it on says that audio leaves the machine and where it goes.
- No cloud transcription.
- Voice never answers a card ([0004](0004-a-human-answers-permissions.md)), whatever the model says.

## Consequences

- With the feature off, Zeca does not speak; nothing about voice leaves the machine.
- No speech engine or voice ships in our binary, so 0013's license checks apply only to what
  still runs locally (whisper.cpp and its models).
