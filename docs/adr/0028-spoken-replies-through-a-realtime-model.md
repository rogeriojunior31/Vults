# 0028. Voice: local listening, opt-in spoken replies

**Status:** Accepted, 2026-10-12. Consolidated on 2026-10-12 from 0013.

## Context

Voice is how many users want to talk to Zeca, and hear him. Audio is personal, and the best
engines are either cloud services or large local models. Push-to-talk shipped in #15. The local
voice that shipped for spoken replies (Kokoro, with `espeak-ng` phonemes) was removed on
2026-10-07: it sounded too poor to keep, above all in Portuguese. Cloud transcription was never
built, and the user dropped it from the plan on 2026-10-12.

## Decision

- **Speech to text runs on this computer**: whisper.cpp through `whisper-rs`, on the GPU through
  Vulkan when there is one; models downloaded from Settings and checked by SHA-256. Audio stays in
  memory and is never saved. No cloud transcription.
- **No wake word and no always-on microphone**: the mic opens only when the user asks.
- **Spoken replies come from a realtime model the user turns on.** It is a request on the user's
  behalf ([0019](0019-requests-on-the-users-behalf.md)): off by default, with its own switch, budget
  and trace, through the network gate in `app`. Its key lives only in the OS keyring. Turning it on
  says that audio leaves the machine and where it goes.
- **No GPL code in our binary**: Piper (GPL-3.0) is out, and so is the `sherpa-onnx` crate's
  default build, which links `espeak-ng` (GPL-3.0). A GPL tool the user installed may run as a
  separate process. Each engine or model that runs locally is checked for its license (code and
  model) before it goes in.
- **Voice never answers a card** ([0004](0004-a-human-answers-permissions.md)), whatever the model
  says.

## Consequences

- With the feature off, Zeca does not speak; nothing about voice leaves the machine.
- No speech engine or voice ships in our binary, so the license checks apply to whisper.cpp and
  its models.
- Bigger downloads and a GPU path to keep working.

## History

- [0013](0013-voice-local-first.md), 2026-10-04: local speech to text, no always-on mic, no GPL in
  the binary, voice never answers a card. Its local spoken replies and opt-in cloud transcription
  were superseded here.
