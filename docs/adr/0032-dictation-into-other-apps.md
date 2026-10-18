# 0032. Dictation into other apps

**Status:** Accepted, 2026-10-17, under the maintainer's instruction to continue N3 with the
planned defaults. This is V0.2's contract; capture, insertion and native validation land separately.

## Context

Local transcription already fills Vults' Chat composer. N3 extends it to the application or
flock session the user was using when they pressed the talk key. Text can become an action in
a terminal: an Enter, including one embedded in a transcript, could answer an agent's prompt.
Focus may also change while transcription runs. Permission authority must stay with the human
under [0004](0004-a-human-answers-permissions.md).

## Decision

- **Capture intent at key-down.** Read the focused window using EWMH on X11 or a KWin script on
  Plasma Wayland, reusing the platform integration. Keep the window identity and, when known,
  the exact flock session and multiplexer pane. A window title is context, not proof of identity.
  Recheck the target immediately before delivery; do not paste into a different foreground app
  or raise another app to recover a lost target. Unknown, changed or closed targets yield a
  clipboard fallback. Other Wayland desktops get the same fallback until supported.
- **Insertion belongs in `platform`; orchestration belongs in `app`.** X11 uses XTest for the
  paste chord. Wayland uses the RemoteDesktop portal, requesting keyboard input only after the
  user enables insertion. Explain that the desktop grants the ability to type into other apps.
  A refused, missing or revoked grant falls back to copying; never bypass it with another input
  service. A restore token is a secret: keep it only in the OS keyring through `secrets`, never
  in settings, a database, logs or renderer storage. Without a usable keyring, do not persist it.
  Turning insertion off closes the session and removes the stored token.
- **Lease the text clipboard.** Preserve the previous text in memory, set the transcript and
  request exclusion from clipboard history where supported. After about 700 ms, restore the
  previous text only if the clipboard still belongs to that lease and has not changed; never
  overwrite a newer copy. Overlapping leases must not restore each other's stale text. Do not
  write clipboard contents to disk. Explain that this lease restores text only, not images or
  rich formats, and that history exclusion depends on the desktop. A clipboard-only fallback
  leaves the transcript available to paste manually instead of restoring it away.
- **Agent delivery requires an exact, safe target.** Use a supported multiplexer only when its
  pane identity is known and still belongs to the captured session. Do not send text, a paste
  chord or Enter into a session waiting on a permission or question, or while its card is open.
  Recheck core's current state at delivery, not just at capture. A deferred card still waits.
  When session identity or safety cannot be established for a terminal target, copy instead.
  No input is queued to run automatically after a card is answered.
- **Insert text without submitting it.** Auto-send is off by default. A future explicit
  auto-send setting cannot override the waiting-card guard. Terminal insertion must also handle
  embedded CR/LF and control characters safely; removing the final Enter is insufficient. If a
  supported literal/bracketed paste path cannot establish safe insertion, use the clipboard
  fallback. Dictation never maps spoken words to a permission decision. Voice commands use
  their separate key and never enter this insertion path.
- **Report only what the delivery establishes.** Show *In session* when the supported pane
  transport confirms delivery, *Pasted* when the validated foreground paste dispatch succeeds,
  and *Copied* when the text was only copied. These do not claim that another app submitted,
  executed or saved anything. A failed clipboard write is an error, never *Copied*.
- **Dictation is independent of Zeca.** With Zeca off, the talk key, local transcription,
  insertion and history still work. A focused Vults Chat composer keeps its existing draft
  path; it does not submit a chat message automatically. The island never takes focus from
  the destination to show dictation progress and never hides a pending card.
- **History is text only, with retention.** Keep the heard text and delivered text, outcome and
  timings in the local store, not the audio or previous clipboard. Offer a retention choice and
  *Do not keep*; choosing that still permits the in-memory run without writing its transcript
  into history, logs or network traces. Model clean-up and remote voice remain separate opt-ins
  under [0019](0019-requests-on-the-users-behalf.md) and [0028](0028-spoken-replies-through-a-realtime-model.md).

## Consequences

- *Copied* is a supported result when focus, permissions or a terminal's safety cannot be
  established. Vults never turns a transcription failure into permission to type elsewhere.
- A clipboard lease may temporarily replace non-text content, and the desktop's clipboard
  manager may retain a copy. Settings must explain these limits before insertion is enabled.
- Tests cover focus changes, clipboard ownership changes and overlapping leases, pending and
  deferred cards, multiline terminal text, missing pane identity, revoked portal access and
  operation with Zeca off. Native smoke on X11 and Plasma Wayland checks the target, keyboard
  grant, clipboard behavior and actual insertion. The island remains mapped once at its fixed
  size; only its input region changes. Automated checks do not replace that desktop validation.

## History

- 2026-10-17: define N3 V0.2's target, clipboard, keyboard capability and permission boundaries.
  Use 0032 because [0031](0031-tasks-leave-vults.md) already records Tasks removal.
