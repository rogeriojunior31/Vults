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
  insertion and history still work. The talk shortcut always uses independent dictation, even
  over Vults windows. Only the Chat microphone button fills its draft directly; it never submits
  a chat message automatically. The island never takes focus from
  the destination to show dictation progress and never hides a pending card.
- **History is text only, with retention.** Keep the heard text and delivered text, outcome and
  timings in the local store, not the audio or previous clipboard. Offer a retention choice and
  *Do not keep*; choosing that still permits the in-memory run without writing its transcript
  into history, logs or network traces. Model clean-up and remote voice remain separate opt-ins
  under [0019](0019-requests-on-the-users-behalf.md) and [0028](0028-spoken-replies-through-a-realtime-model.md).

## Natural voice commands (VC1)

The separate command key requests an action, not text insertion. The 0.1.9 catalog is bounded:

| Scope | Supported requests | Target and execution |
|---|---|---|
| Flock | Existing navigation, session/usage/GitHub reads, project notification mute/pin/hide, repeat and conditional undo | Current live session/project identity; never a permission answer |
| Media | Pause, resume, next, previous and player volume | Explicit live MPRIS player; respect the media switch and clarify multiple targets |
| Computer audio | Mute/unmute default output, set volume, raise/lower by 10 percentage points | Capture and revalidate the output identity; clamp volume to 0..100 percent |
| Applications | Open an installed application | Validated launchable desktop-entry identity; no supplied executable, URI, file or arguments |

- Local media, computer-output and application-launch actions each have their own default-off
  switch in Command settings. They work with Zeca off. Project mute means notification mute,
  never browser or system sound. Bare `Open <name>` resolves across projects and installed apps;
  collisions ask which scope the user intended. An absent app reports not installed.
- Keep the deterministic grammar for supported local requests. Semantic interpretation is a
  separate default-off choice with its own provider/model and the `voice-command-intent` network
  consent in [0019](0019-requests-on-the-users-behalf.md). It is not enabled by turning on Chat,
  speech transcription or the existing unknown-speech-to-Chat option. When semantic mode is on,
  unknown speech has that one destination; a failed interpretation never silently starts Chat.
- The model receives only the bounded heard text and the minimum available action/target labels
  and opaque IDs. It receives no audio, file paths, project files, media metadata, saved chat,
  memory or tools. Reuse qualified tool-free transports: initially Claude Code, with API/local
  transports offered only after the same contract is verified. Unsupported Codex subscription
  consultation stays unavailable; never silently switch account, provider or billing method.
- Model output is an untrusted closed schema: validated typed actions, clarification, unsupported
  or refusal. Enforce the 4096-byte input, 8 KiB output, eight-action and ten-second bounds. Reject
  unknown verbs/fields, invalid numeric values and targets outside the captured catalog. Revalidate
  the live target, switches, lock and cancellation ticket before each effect. A late or duplicate
  reply never executes. No automatic retry, shell execution or model-selected program arguments.
- Negated, quoted, informational or ambiguous speech must not turn into guessed actions. Clarify
  an unclear target or audio scope. Voice never approves, denies, releases or modifies a permission
  card, edits agent configuration, merges a PR, closes an app or runs a destructive operation.
- Validate a batch before effects. If an external effect fails, stop and report the actual completed
  prefix; do not promise rollback or retry actions already performed. Do not restore temporary
  dictation duck/pause state over a deliberate voice media change. Undo is conditional on a known
  restorable prior state; it never kills an application that a command launched.
- Show the heard text, target, clarification and actual outcome in existing command feedback/history,
  respecting retention. Never report Done before native completion. Disabled actions, no target,
  missing CLI/backend, unsupported provider, expired consent, exhausted quota, timeout and native
  failures produce actionable local feedback and no side effect. These are separate outcomes,
  not a generic Not a command result. Transcription and ordinary dictation remain usable.

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

- 2026-10-26: VC1 defines bounded natural-command actions and independent model consent before 0.1.9.

- 2026-10-17: define N3 V0.2's target, clipboard, keyboard capability and permission boundaries.
  Use 0032 because [0031](0031-tasks-leave-vults.md) already records Tasks removal.

- 2026-10-19: reserve the global talk shortcut for independent dictation regardless of focus;
  Chat draft voice input remains an explicit microphone-button action.
