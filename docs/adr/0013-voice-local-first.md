# 0013. Voice is local first; cloud only when chosen

**Status:** Accepted, 2026-10-04 (push-to-talk shipped in #15).

## Context

Voice is how many users want to talk to Zeca, and later hear him. Audio is personal, and the best
engines are either cloud services or large local models.

## Decision

- Speech to text runs on this computer: whisper.cpp through `whisper-rs`, on the GPU through
  Vulkan when there is one; models downloaded from Settings and checked by SHA-256. Audio stays in
  memory and is never saved.
- Cloud transcription is opt-in, with its key in the keyring and a clear note that audio leaves
  the machine.
- Spoken replies are optional and local, with a permissively licensed engine and voices. No GPL
  code in our binary: Piper (GPL-3.0) is out, and so is the `sherpa-onnx` crate's default build,
  which links `espeak-ng` (GPL-3.0). A GPL tool the user installed may run as a separate process.
- No wake word and no always-on microphone: the mic opens only when the user asks.
- Voice never answers a permission card ([0004](0004-a-human-answers-permissions.md)).

## Consequences

- Bigger downloads and a GPU path to keep working.
- Each new engine or voice is checked for its license (code and model) before it goes in.
