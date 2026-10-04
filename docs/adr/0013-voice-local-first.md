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
- Spoken replies are optional and local, with a permissively licensed engine and voices. GPL
  engines are out (Piper went GPL-3.0).

## Consequences

- Bigger downloads and a GPU path to keep working.
- Each new engine or voice is checked for its license (code and model) before it goes in.
