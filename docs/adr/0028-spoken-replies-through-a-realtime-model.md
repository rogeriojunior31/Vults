# 0028. Voice: local listening, opt-in spoken replies

**Status:** Accepted, 2026-10-12. Consolidated on 2026-10-12 from 0013.
Amended on 2026-10-17 for dictation, bounded follow-up listening and temporary meeting audio.
The maintainer accepted V0.1; N3 implementation still lands in separate steps.

## Context

Voice is how many users want to talk to Zeca, and hear him. Audio is personal, and the best
engines are either cloud services or large local models. Push-to-talk shipped in #15. The local
voice that shipped for spoken replies (Kokoro, with `espeak-ng` phonemes) was removed on
2026-10-07: it sounded too poor to keep, above all in Portuguese. Cloud transcription was never
built, and the user dropped it from the plan on 2026-10-12.

## Decision

- **Speech to text runs on this computer**: whisper.cpp through `whisper-rs`, on the GPU through
  Vulkan when there is one; models downloaded from Settings and checked by SHA-256. Dictation and chat audio stay in
  memory; meetings have the temporary exception below. No cloud transcription.
- **No wake word and no always-on microphone**: the mic opens only when the user asks,
  including the explicit, bounded follow-up setting below.
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

## Dictation and meetings

These clauses were accepted in V0.1 of [the Dictation plan](../dev/plan-dictation.md#v0-decisions-and-the-prototype).
They set N3's contract; acceptance does not mean the features are already implemented:

- **Local words, local transcription (R1).** A local dictionary, replacements and snippets may
  correct the transcript; the dictionary and project vocabulary may also guide whisper through
  its `initial_prompt`. These passes make no network request. Cloud transcription stays out.
  Optional model clean-up is a separate, off-by-default request under [0019](0019-requests-on-the-users-behalf.md),
  with the heard text retained and a raw-text fallback; it is not part of local transcription.
- **Explicit, bounded listening (R3).** A key or a click starts capture. A double tap may lock
  hands-free dictation, with a visible listening state and an explicit stop/cancel path. No wake
  word or background listening is added. A separate *Answer without the key* setting is off by
  default: after the user enables it, Zeca may reopen the mic for at most eight seconds after
  his own spoken question in an active voice conversation. The island shows the open mic and
  the user can cancel it. Expiry or cancellation closes it; notifications, tool results and
  permission cards do not open or extend it.
- **Meetings are the only audio-on-disk exception (R5).** Each recording starts with a user
  action and a consent line explaining which sources are captured and that audio is temporarily
  saved for local transcription. Temporary WAVs are private to the user and deleted when
  transcription finishes, fails or is cancelled; startup cleans up leftovers from an interrupted
  run. There is no audio archive or automatic recording. Ordinary dictation and chat capture
  stay in memory. Only the transcript and explicitly requested notes remain, under their
  retention setting. A model summary is a separate, off-by-default request through the gate.
- **Check code and weights before enabling a download.** The existing license rule applies to
  every added model, including smart-turn v3, pyannote segmentation-3.0 and CAM++: record the
  source, code license, weights license, redistribution conditions and download SHA-256 in the
  model reference before offering it. A proposed model is not approved by appearing in the
  plan. Dependencies still pass `cargo deny`; no GPL-linked engine enters the binary.
- **Voice still never answers a card.** None of these clauses changes [0004](0004-a-human-answers-permissions.md).
  Spoken text is never interpreted as an approval or denial. A future question-card composer
  requires its own decision and a separate human confirmation to send; this amendment does
  not enable it.

The eight-second follow-up and temporary meeting files are the explicit exceptions to the
listening and storage rules. Dictation into other apps follows
[0032](0032-dictation-into-other-apps.md).

## History

- 2026-10-17 (accepted by the maintainer): N3 V0.1 defines local vocabulary, bounded opt-in
  follow-up listening, temporary meeting audio and the license evidence required for new models.
- 0013, 2026-10-04: local speech to text, no always-on mic, no GPL in
  the binary, voice never answers a card. Its local spoken replies and opt-in cloud transcription
  were superseded here.
