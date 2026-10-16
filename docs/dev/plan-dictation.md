# Plan: Dictation

Draft of 2026-10-16, for review. One plan for the Dictation area: dictation into any app, your words
(dictionary, replacements, snippets), styles, statistics and diagnostics, meetings, talking with Zeca
(K8) and an MCP tool. It joins the single roadmap ([Plan Zeca](plan-zeca.md)) through the rows proposed in
[Merge into plan-zeca.md](#merge-into-plan-zecamd) once the open decisions are settled.

Nothing here is built yet. The UI was reviewed as a separate prototype (nine pages, eleven island states);
putting it into the main prototype is step V0.7.

## References

| Project | Licence | What we take | How |
|---|---|---|---|
| The reference app credited in `NOTICE` | MIT | Key state machine, text pipeline (snippets, replacements, dictionary), X11/Wayland paste, clipboard lease, history and statistics, meeting notes, end-of-turn policy | Ported, credited only in `NOTICE` (`check-brand.sh`) |
| [voicebox](https://github.com/jamiepine/voicebox) | MIT | Clean-up prompts ("a text filter, not an assistant", three rules), whisper loop remover, key chords (Space locks mid-hold), focus-safe overlay, conditional clipboard restore, MCP with an identified client | Rust copied; Python rewritten in Rust; `NOTICE` line when code is translated |
| [Patter](https://github.com/PatterAI/Patter) | MIT | Turn and barge-in state machine, VAD hysteresis, semantic end of turn (smart-turn v3, Apache-2.0 weights), sentence chunker with Portuguese rules, per-stage metrics | Design and constants; code rewritten |
| VoiceStudio | AGPL-3.0 | Ideas only: spoken code in terminals, a fallback model, an honest "Pasted" vs "Copied", pause | No code |

The research notes behind this plan (per-project specs with file:line citations) are kept outside the
repository; ask the maintainer for them. The reference keys used in the steps are `[TLP]` (reference app,
Linux port), `[DD]` (reference app, dictation and dictionary), `[VA]` (reference app, voice assistant),
`[VB]` (voicebox), `[PT]` (Patter) and `[VS]` (VoiceStudio, ideas only).

## Decisions to settle (V0)

Rules that hold as they are: [ADR 0002](../adr/0002-rust-backend-no-sidecars.md) (Rust only, no Python
sidecar), [ADR 0028](../adr/0028-spoken-replies-through-a-realtime-model.md) (local transcription, audio in
memory, no GPL in the binary, voice never answers a card), [ADR 0004](../adr/0004-a-human-answers-permissions.md)
and [ADR 0019](../adr/0019-requests-on-the-users-behalf.md) (model clean-up is a request on the user's behalf).

| # | Today | Proposal | Why |
|---|---|---|---|
| R1 | X5 voice dictionary dropped (2026-10-12) | **Reopen, local only**: post-processing plus whisper vocabulary; cloud transcription stays dropped | Names of projects and tools come out right; cheap and fully local |
| R2 | Dictation only into Vults' chat | **Dictation into any app** and straight into flock sessions (new ADR 0031) | Clear gain; the pipeline already exists |
| R3 | Wake word: no | **Still no.** Hands-free is a double tap on the key | Keeps ADR 0028; no always-open mic |
| R4 | X3 a voice per vult dropped | Stays dropped; reconsider after V6 | Speaking depends on the opt-in realtime model |
| R5 | No meetings in the plan | **Meetings in V5**, opt-in per recording, mic and the PipeWire monitor | Useful, but the heaviest part (diarization) |
| R6 | `DESIGN.md`: Settings is where configuration lives | Dictation settings inside the area, linked from Settings › Chat & voice, one owner of the values | The prototype puts them in the area |

## Architecture

```
               ┌────────────────────── app (Tauri) ──────────────────────┐
 key / portal ▶│ dictation::KeyLogic (pure FSM)                          │──▶ island states
               │   focused app read at key-down                          │
 microphone ───┼▶ audio::Capture ─▶ voice::Vad ─▶ voice::Stt (whisper.cpp)│
 (cpal)        │        │ levels, 9 bands ─▶ island                       │
               │        ▼                                                │
               │  dictation::text: snippets → replacements → dictionary  │
               │     → spoken code → loop remover → clean-up (optional)  │
               │        ▼                                                │
               │  platform::insert: flock session │ XTest │ RemoteDesktop portal │ clipboard
               │        ▼                                                │
               │  store: history and statistics (vults.sqlite)           │
               └─────────────────────────────────────────────────────────┘
 Talk with Zeca (K8): audio::Capture ─▶ voice::turn (VAD + smart-turn) ─▶ realtime model (opt-in)
                      ◀─ playback ◀─ barge-in, echo cancellation (PipeWire echo-cancel)
```

- **`vults-audio`** (new Connect crate, out of `voice::record`): cpal capture by device name, mono 16 kHz
  through `rubato`, 9 FFT bands (`rustfft`) for the island, high-pass and light AGC, the PipeWire/Pulse
  monitor for meetings, and echo cancellation for talking with Zeca through PipeWire's `module-echo-cancel`
  (WebRTC AEC3) or `webrtc-audio-processing`.
- **`vults-voice`** (exists): Silero with hysteresis (in 0.50, out 0.35, α 0.35, speech ≥ 250 ms, silence
  ≥ 400 ms, reset per turn); whisper.cpp with the live preview it has; a vocabulary prompt (dictionary and
  the focused project's names, where measured to help); a loop and silence-hallucination remover; plan B
  (speech heard but no text: retry with Base and say so); end of turn for talking (silence policy plus
  smart-turn v3 through `ort`, hold ≤ 1.2 s).
- **`vults-dictation`** (new, pure): the key state machine (hold to talk; a tap under 0.30 s and a second
  within 0.40 s lock hands-free; Space during a hold locks; 600 ms mic tail; Esc and other keys cancel on
  X11) and the text pipeline (snippets as marks `⟦n⟧`, replacements, spelling, spoken code, marks back).
- **`platform`**: focus at key-down (EWMH on X11, a KWin script on Wayland), XInput2 raw keys on X11, the
  clipboard lease (give back after 0.7 s only if unchanged, hidden from Klipper), XTest and RemoteDesktop
  paste, delivery into flock sessions through their multiplexer, never into a session waiting on a
  permission.
- **Clean-up**: a filter, never an assistant; styles As spoken, Natural, Polished and Prompt; "Write in"
  another language; per app; the Chat & voice provider or a local one (Ollama, LM Studio) through the gate;
  4 s timeout, then the raw text; the heard text always kept.
- **Meetings**: the only audio on disk (temporary WAV, deleted after the transcript); diarization with
  pyannote segmentation-3.0 and CAM++ through `ort`; summary and to-dos through the gate; Markdown, SRT, VTT.
- **Talk with Zeca (K8)**: the opt-in realtime model of ADR 0028; a turn machine (a 0.5 s lock counted
  from the first audio played, about 260 ms replayed on barge-in, history rewritten to what was heard plus
  "[interrupted]"); never answers a card.
- **MCP**: `vults_dictate(question)`: an agent's question answered by voice and sent with a click.

### Beyond the reference app

1. Dictation straight into agent sessions, the same on X11 and Wayland.
2. A Prompt style and "agent sessions in English": speak Portuguese, the agent gets an English instruction.
3. The focused project's names in whisper's vocabulary.
4. Spoken code in terminals.
5. GPU through Vulkan, a choice of model and language.
6. Silero with hysteresis, plan B and the loop remover: less invented text.
7. Clean-up through a local model as well as the cloud.
8. The key shown as the desktop bound it.
9. Diagnostics with per-stage timings.
10. Talking with real interruption (echo cancellation and the turn machine).

## Working agreement

- One step = one PR, from its own worktree and branch. Commit each adjustment the user validates. Before
  the next module, the user signs off on the one in the running app.
- Each PR carries:
  - its tests, written first;
  - docs in the same PR (`docs/guide/dictation.md` and its `docs/pt-br/` twin, `docs/reference/settings.md`,
    `docs/architecture.md`, CHANGELOG);
  - every UI sentence through `t()`/`tk()`, with pt-BR, es and zh in `ui/src/i18n/*.ts`;
  - core sentences in `crates/core/src/i18n.rs`.
- Run CLAUDE.md's *Before every commit*. Run `cargo deny check` on any dependency change.
- **No Python, no GPL**:
  - each model gets a licence line (code + weights) in `docs/reference/models.md` (new) before it can be
    downloaded;
  - each new crate passes `deny.toml`.
- **Brand**:
  - nothing in the repo names the project this one derives from (`check-brand.sh`); `NOTICE` already credits
    it under MIT;
  - ports cite only *behavior*, never "ported from X" with its name;
  - the other references (MIT ones) get a `NOTICE` line when code is translated from them; AGPL ones are
    ideas only;
  - the prototype comments naming references are cleaned at V0.7.
- Layers (`check-layers.sh`):
  - new Connect crates are `vults-audio` and `vults-dictation` (pure: FSM + text pipeline, no IO);
  - anything touching X11, portals, KWin or the clipboard lives in `platform` (Experience);
  - orchestration lives in `app`.
- Settings keys: each new key gets a default and an H4 fixture (`settings` read with `version`).
- Linux only. Every platform step lists X11 vs Wayland/KWin behavior and gets a smoke check on both, plus
  the layer-surface checklist line ("never resized or unmapped").

Effort: S = up to 2 days, M = up to a week, L = more (split if it grows).

## What already exists (do not redo)

| Piece | Where | Reuse |
|---|---|---|
| Mic into memory (cpal 0.18), levels every 50 ms, auto-stop | `crates/voice/src/record.rs` (`MAX_SECONDS = 60`, too short for hands-free) | Extract into `vults-audio` (V1.1) |
| whisper.cpp via whisper-rs, Vulkan, live preview, `WHISPER` mutex, PT coding vocabulary `initial_prompt` | `crates/voice/src/lib.rs`, `preview.rs` | STT as is; vocabulary grows in V2.3 |
| Silero VAD, end-of-speech 1.5 s, silence trimming | `crates/voice/src/vad.rs` | Add hysteresis (V1.2) |
| Model catalogue, checked downloads (SHA-256) | `crates/voice/src/models.rs`, `app/src/voice.rs` | Same flow for new models (smart-turn, diarizer) |
| GlobalShortcuts portal, id `talk` (Ctrl+Alt+V), bound keys shown | `crates/platform/src/shortcuts.rs`, `app/src/lib.rs:707-760` | Becomes the dictation key (V1.5) |
| KWin script over D-Bus, EWMH via x11rb, multiplexer jump (tmux/kitty/wezterm/herdr) | `crates/platform/src/jump.rs`, `jump/kwin.rs`, `jump/x11.rs` | Focus read (V1.7), session delivery (V1.11) |
| Network gate with switch/budget/trace per feature | `app/src/gate.rs` | New features `dictation-cleanup`, `meeting-summary`, `voice-realtime` |
| Chat providers incl. local Ollama/LM Studio | `crates/chat/src/providers.rs:94`, `api.rs`, `openai.rs` | Clean-up provider (V3.2) |
| `vults.sqlite` with numbered migrations | `crates/store/src/lib.rs:73` | New migrations (V1.13, V2.1, V3.3, V4.1, V5.7) |
| Island chat voice states, `sound.ts`, `clipboard.ts` | `ui/src/island/chat.ts`, `fsm.ts` | Island dictation states (V1.12) |
| Hand-written MCP server | `app/src/mcp.rs` (`vults --zeca-mcp`) | `vults_dictate` (V7) |

Reference keys (`[TLP]`, `[DD]`, `[VA]`, `[VB]`, `[PT]`, `[VS]`) are listed under [References](#references);
line numbers are lines of those research notes, and the source file:line they cite is given where the port
needs it.

---

## V0 Decisions and the prototype

**Goal.** Record R1–R5 (and R6, surfaced here) as ADR edits. Put Dictation into the single UI prototype,
translated and covered.

**User-visible outcome.** The main prototype has a *Dictation* Lab area with its nine pages and the island
states, in four languages, each page carrying its phase badge. Nothing in the app changes yet.

**V0.1 ADR 0028 edit: voice, dictation, meetings** (S)
- Build: edit `docs/adr/0028-spoken-replies-through-a-realtime-model.md` with dated *History* lines.
  - R1: a local dictionary and whisper `initial_prompt` are allowed; cloud transcription stays out.
  - R3: no wake word, mic opens only on a key or a click. Hands-free is a double tap. Decide whether
    "reopen 8 s after Zeca asks" (prototype, Conversation page) counts as "opened by the user"; the
    proposal is yes, and it is bounded and shown on the island.
  - R5: Meetings is the one place where audio touches disk, as a temporary WAV deleted after
    transcription. It is opt-in per recording, with a consent line.
  - Licence rule extended to the new models: smart-turn v3, pyannote segmentation-3.0, CAM++.
  - Also add `docs/pt-br/adr/...`.
- Needs: none. Done when: the user accepts the edit.

**V0.2 ADR 0031: dictation into other apps** (M)
- Build: new topic `docs/adr/0031-dictation-into-other-apps.md` (next free number, README table). It covers:
  - synthetic input: XTest on X11; the RemoteDesktop portal on Wayland, keyboard only, with a restore
    token kept in the keyring (crate `secrets`);
  - a clipboard lease: give back only if unchanged; hidden from Klipper;
  - reading the focused app: EWMH, or a KWin script;
  - delivery into flock sessions through their multiplexer;
  - **dictated text is never sent with Enter into a session that is waiting on a permission or a question,
    and never while its card is open** (rule 2 by proximity: a TUI prompt answered by stray text);
  - dictation works with Zeca off (D7 of plan-zeca);
  - the history keeps text only, with retention.
- Needs: V0.1. Done when: accepted.

**V0.3 ADR 0021 edit: a Dictation area and island states** (S)
- Build:
  - Dictation becomes a sixth Lab area (Flock, Chat, Tasks, Connections, Activity, Dictation).
  - The island gains dictation states, which never hide or delay a card (D5's test extended).
  - **R6**: where dictation settings live. `DESIGN.md` says "Settings remains the configuration
    destination", but the prototype puts *Settings* inside the area. Proposal: the area keeps its page
    and Settings › Chat & voice links to it, with one owner of the values.
- Needs: none. Done when: accepted; `DESIGN.md` gets a line if R6 changes the rule.

**V0.4 ADR 0019 consequences** (S)
- Build: name the new gate features.
  - `dictation-cleanup`: per dictation, budget about 2000 a day.
  - `meeting-summary`: about 20 a day.
  - `voice-realtime`: money a day.
  - State that loopback-only providers (Ollama, LM Studio on 127.0.0.1) skip the gate's budget but still
    write a local trace, as `crates/chat/src/embed.rs` does today (open question Q3).
- Needs: none. Done when: accepted.

**V0.5 Plan merge** (S)
- Build: apply the *Merge into plan-zeca.md* section below to `plan-zeca.md`; this file keeps the step detail.
- Needs: V0.1–V0.4. Done when: merged.

**V0.6 Prototype strings to English** (S)
- Build:
  - the separate proposal (`design/ui-motion-proposal/dictation/`, not committed yet: it is hardcoded pt-BR
    and would fail `check-english.sh`) has `dictation.js` and `page.html`; rewrite every
    sentence in English through the prototype's `t()`;
  - add `[pt-BR, es, zh]` triples to `copy.json` (pt-BR = today's text);
  - remove reference-project names from comments ("Patter's design" → "turn-taking design").
- Tests: `P.missing()` empty.
- Done when: the separate proposal still builds (`dictation/build.mjs`) and reads the same in pt-BR.

**V0.7 Prototype: Dictation as a Lab area** (M)
- Build:
  - add `dictation.js` to `MODULES` in `build.mjs`, adapted to the workspace scope (`el`, `button`, `toast`
    already exist there, so drop the copies);
  - add the Lab nav key `nav-dictation` with icon `mic`, and the area `dictation` with sub-pages `history`,
    `talk`, `notes`, `stats`, `dictionary`, `snippets`, `styles`, `settings`, `diagnostics`;
  - move `dx-*` CSS into `template.html`;
  - draw the island states in `island.js` at the Island/Mini size (production Mini is 340 × 38; long state
    lines such as "In session api · Codex · in English" must truncate with the full text in a tooltip);
  - delete `dictation/` once merged (or keep `build.mjs` as a pointer).
- Rebase on the current uncommitted prototype work first (`git status` shows `build.mjs`, `copy.json`,
  `verify.mjs`… modified).
- Done when: `node design/ui-motion-proposal/build.mjs` builds under the 1 MB limit.

**V0.8 Prototype coverage** (M)
- Build:
  - `features.json`: one entry per page, per island state (11) and per key behavior (hold, double tap,
    Space lock, Esc, paste again, terminal paste, session delivery, Copied fallback). Each entry has
    `status: "planned"`, a `phase` (`V1`…`V7`), `at: {mode:'room', area:'dictation', page}` and an `expect`
    selector.
  - `scope.json`: add sources and an approved line `P12 Dictation` after review.
  - `verify.mjs`:
    - each page shows its phase badge (`.dx-phase` text equals the manifest's phase);
    - "Simulate a dictation" walks idle→listening→working→done;
    - the text pass marks snippet/replacement/word segments;
    - adding a word, replacement and snippet works; choosing a style persists;
    - retention "Do not keep" hides the history;
    - every island state renders without overflow at Mini width;
    - zero untranslated sentences.
- Done when: `node design/ui-motion-proposal/verify.mjs` passes. (It already shells out to `python3` for
  HTML parsing; do not add more Python, and consider replacing it later.)

**V0.9 Prototype review** (S)
- Build: user review of the integrated prototype; the findings are fixed in place; `scope.json` records
  `P12` approved with the date.
- Done when: signed off. This gates every production UI step below.

---

## V1 Dictation in any app

**Goal.** Hold the key in any app, speak, release: the text lands where the cursor is, or in the focused
flock session. Local only, no model clean-up.

**User-visible outcome:**
- Ctrl+Alt+V works in Firefox, Kate, Konsole and a Claude Code session, on KDE Wayland and X11, with Zeca on
  or off.
- The island shows listening, working and then the honest result: Pasted, In session or Copied.
- Lab › Dictation has History and Settings.

**V1.1 `vults-audio` crate** (M)
- Build:
  - new Connect crate `crates/audio`, extracted from `voice::record`: capture thread, device by name with
    fallback to default, monitors skipped;
  - mono 16 kHz through `rubato` (MIT), replacing the hand-written sinc;
  - a 9-band meter with `rustfft` at about 30 Hz;
  - buffer cap set by the caller: 60 s for chat, 10 min for dictation;
  - `voice` depends on it; `check-layers.sh` gets `audio` in layer 2.
- Port:
  - device list and fallback: [TLP] 124-125 (`speech/audio.rs:198-233`);
  - bands: 512-point Hann, 90–7000 Hz log, −78..−30 dB, attack 0.6 / release 0.25 ([TLP] 132; [DD] 97-111
    adds the +2.2 dB per band tilt).
- UI: none (chat mic keeps working).
- Tests:
  - resampler fixture: 1 kHz at 48 kHz → 16 kHz, peak bin and RMS within tolerance;
  - meter: a tone lights its band only;
  - device fallback with a missing name;
  - the chat's existing voice tests still pass.
- Done when: chat push-to-talk behaves as before; `cargo deny` is clean.
- Needs: V0.1.
- X11/Wayland: same (cpal ALSA host today; pulseaudio/pipewire host only in V5.1).

**V1.2 VAD hysteresis** (S)
- Build: `voice::vad` gains activation 0.50 and deactivation 0.35, exp filter α = 0.35, minimum speech
  250 ms, minimum silence 400 ms, queued transitions, and `reset()` per turn.
- Port: [PT] 41-51 (`silero_vad.py:121-176, 345-414`).
- Tests: WAV fixtures (speech, pauses, breath noise, silence) under `crates/voice/tests/fixtures/`, with
  expected segment edges ±40 ms.
- Done when: no regression in the chat's end-of-speech.
- Needs: V1.1.

**V1.3 Transcript gate: loop remover and hallucinations** (S)
- Build: `voice::gate` (pure) collapses repeated word/char runs (threshold 6, unit ≤ 60 chars) and drops the
  whisper silence set ("Thank you.", "Obrigado.", "[BLANK_AUDIO]", "♪"…) when it is the whole result.
- Port: [VB] 37, 113 (`refinement.py:25-111`, rewrite with a manual loop: `regex` has no backrefs); [PT] 242,
  313 (hallucination set).
- Tests: table tests in pt/en/es; real looped outputs saved as fixtures.
- Done when: used by the chat path too.
- Needs: none.

**V1.4 `vults-dictation` crate: key state machine** (S)
- Build: new Connect crate, pure, no IO.
  - `KeyLogic { Idle, Holding(t), TapWait(until), Locked }` with actions `Start | Lock | Finish | Cancel |
    Restart`.
  - Hold ≥ 0.30 s finishes on release; a double tap within 0.40 s locks; the next tap finishes; Esc cancels;
    another key during a hold cancels.
  - **Space during a hold locks** (chord).
  - 600 ms mic tail after release.
- Port:
  - [DD] 53-72 (transitions, constants);
  - [TLP] 222, 351 (`flow/logic.rs:1-137`, 0.30/0.40);
  - the Space chord from [VB] 40, 105 (`hotkey_monitor.rs:160-237`, the same-instant coalescing becomes a
    `Lock` transition).
- Tests: port the 12 cases of [DD] 83-86 plus Space-lock cases (Space before 0.30 s, Space while locked
  ignored, Space then release keeps listening).
- Needs: none.

**V1.5 The talk key becomes the dictation key** (S)
- Build:
  - `platform::shortcuts`: `talk` no longer depends on Zeca (`island_event(…, chat)` gate removed for talk;
    with Zeca off the key dictates);
  - add `talk-again` (paste again, Ctrl+Alt+B) and `talk-cancel` (a non-Esc default, e.g. Ctrl+Alt+X) — the
    desktop asks once for new ids;
  - `app/src/lib.rs:707-760` routes `talk` to `app::dictation` unless a Vults surface holds focus (then the
    chat composer path stays, as today).
- Tests: update `a_press_reaches_the_island…` and `without_the_chat…`; `shortcut_target` cases.
- Done when: the key shown in the UI is the one the desktop bound.
- X11/Wayland:
  - KDE's portal works on both;
  - on Wayland Esc cannot be bound (binding it through the portal takes Esc from every app), so cancel is
    `talk-cancel`, the island ✕, or Esc on X11 (V1.8).
- Needs: V0.2.

**V1.6 Orchestrator, first usable slice: "Copied"** (M)
- Build: `app/src/dictation.rs`:
  - a session thread: FSM events → `audio` capture (10 min cap) → VAD trim → whisper (shared `Transcriber`,
    `WHISPER` mutex) → `voice::gate` → `dictation::text` (marks only) → clipboard → island event
    `dictation` `{phase, text, outcome}`;
  - cancel checked after the tail, after STT and before insert;
  - nothing under 0.25 s or with a peak under 0.004;
  - **plan B**: the VAD heard speech and the text is empty → retry with Base and say so.
- Port: [TLP] 129-131, [DD] 357-363 (cancel races, overlap); [TLP] 260-263 outcomes.
- UI: a temporary island line until V1.12.
- Tests: an orchestrator with a fake recorder, a fake STT and a fake clipboard (Patter rule: only the
  external is fake): cancel at each stage, plan B, empty result records nothing.
- Done when: in any app, hold → speak → release → "Ctrl+V to paste" works on X11 and Wayland. **First thing
  the user can use.**
- Needs: V1.1–V1.5.

**V1.7 Focused app at key-down** (M)
- Build: `platform::focus` returns `{class, title, pid}`.
  - X11: `_NET_ACTIVE_WINDOW` → `WM_CLASS`, `_NET_WM_PID` (x11rb, as in `jump/x11.rs`).
  - Wayland/KWin: a read-only KWin script (pattern of `jump/kwin.rs`) replying `activeWindow.resourceClass`
    and `pid`; it runs async while the user speaks.
  - Map the pid to a flock session through the ancestors helper in `jump.rs`.
  - A terminal list: konsole, kitty, alacritty, wezterm, foot, ghostty, gnome-terminal, ptyxis, xterm, VS
    Code's terminal by title.
- Port: [TLP] 258, 265 (`os_linux.rs:600-655`).
- Tests: script text is built from numbers and UTF-16 codes only (as the jump script); pid → session
  mapping with a fake process table.
- Done when: History shows the app; the Settings note covers GNOME/wlroots (no KWin → app unknown, default
  style, Ctrl+V).
- Needs: V1.6.

**V1.8 X11 raw keys: Esc and other-key cancel** (M)
- Build:
  - an XInput2 raw key listener on the root window, no grab (x11rb `xinput` feature), only while a dictation
    is live;
  - Esc → cancel; any non-modifier key during Holding/TapWait → cancel; Space → lock.
- Port: [TLP] 212-221 (`flow/os_linux.rs:490-558`).
- Tests: the keysym mapping table; the listener's start/stop lifecycle with a fake connection.
- X11 only; on Wayland the Space-lock needs the key events we do not get. **Space-lock is X11-only** unless
  Q1's spike finds a way.
- Needs: V1.4, V1.6.

**V1.9 Clipboard lease and X11 paste** (M)
- Build: `platform::insert`.
  - `arboard` (features `wayland-data-control`), one long-lived instance, `exclude_from_history()` (Klipper
    skips it).
  - Save the text, set it, wait 40 ms, send the XTest Ctrl+V (Ctrl+Shift+V for terminals), and after 700 ms
    restore **only if the clipboard still holds our text**.
  - Outcome `Pasted | Copied | Failed`.
- Port: [TLP] 249-263 (`flow/paste.rs:1-73`, `os_linux.rs:567-594`); [VB] 44, 108 (`main.rs:1188-1255`
  conditional restore).
- Tests: the lease logic with a fake clipboard (someone copied during the lease → no restore); terminal
  detection picks Ctrl+Shift+V.
- Done when: in an X11 session or an XWayland app it pastes and gives back the old clipboard.
- Needs: V1.6, V1.7.

**V1.10 Wayland paste through the RemoteDesktop portal** (M)
- Build:
  - ashpd `remote_desktop`: CreateSession → SelectDevices(keyboard, `persist_mode = 2`, restore token) →
    Start, kept open;
  - `NotifyKeyboardKeysym` Ctrl(+Shift)+V;
  - the restore token in the keyring (`crates/secrets`, Q2);
  - one refusal is not asked again until the user retries in Settings.
- Port: [TLP] 254-257 (`wayland.rs:61-203`).
- Tests: the session state machine with a fake portal; the token round trip.
- Done when: on Plasma 6 Wayland KDE asks once; later dictations paste with no dialog.
- Wayland only; without the portal the outcome is Copied.
- Needs: V1.9.

**V1.11 Delivery into flock sessions** (M)
- Build: when the focused window maps to a flock session (V1.7), deliver without the clipboard:
  - tmux `send-keys -l` on the session's `TMUX_PANE`;
  - kitty `kitten @ send-text`;
  - wezterm `cli send-text`;
  - Vults-driven runs: the Lab/Quick composer of that run, never a new turn;
  - otherwise fall back to paste.
- **Guard (V0.2):**
  - if core says the session waits on a permission or a question, deliver nothing into it: the outcome is
    Copied and the island points to the card;
  - "Send automatically" (Enter) is off by default and never fires on a waiting session.
- Tests:
  - multiplexer command builders (as `jump.rs` tests, no shell injection: text passed as an argument, never
    interpolated);
  - a core test that a waiting session yields Copied.
- Done when: the island says "In session api".
- X11 and Wayland behave the same (that is the point).
- Needs: V1.9, V0.2.

**V1.12 Island dictation states** (M)
- Build: in `ui/src/island/` (`fsm.ts`, `render.ts`, new `dictation.ts`):
  - the states idle hint (bound key), listening/locked (bands + partial text from the existing preview),
    working, done (Pasted / In session · app · words · fixes), clipboard ("Ctrl+V to paste"), retry
    (plan B), failed (no sound → Dictation › Settings);
  - an optional start sound through `sound.ts`;
  - cards always win: a card arriving mid-dictation shows (D5).
- UI: prototype island states → production.
- Tests: `scripts/check-island-geometry.mjs` stays green; a new `scripts/check-dictation-island.mjs` steps
  each state at Mini and Quick widths and checks no overflow and a translated text.
- Done when: the user reviews it in the running app on X11 and Wayland.
- Needs: V1.6, V0.9.
- Open question Q5: "Undo" on the Done state needs a synthetic Ctrl+Z; drop it unless wanted.

**V1.13 History store** (S)
- Build: a migration adds
  - `dictations(id, at, app, session, heard, text, style, language, secs, words, fixes, outcome)`;
  - `dictation_days(day, words, secs, count)` and `dictation_apps(app, words)`.
- Rules:
  - 0 words → nothing recorded; aggregates always updated;
  - retention not kept / 7 / 30 / 365 / always (default 30), pruned on open and on record;
  - clearing the history keeps the aggregates;
  - "Not kept" keeps the last text in memory only, for paste again.
- Port: [TLP] 363-403 (`store.rs:262-316`); [DD] 278-303.
- Tests: migration on an old db fixture, pruning by date, counts.
- Needs: V1.6.

**V1.14 Lab › Dictation area and History page** (M)
- Build:
  - `ui/src/surfaces/nest/dictation/` with `area.ts` (sub-navigation as in the prototype) and `history.ts`;
  - grouped by day, search, heard vs written, copy, paste again (`talk-again`), delete;
  - "Not kept" empty state;
  - the `nav-dictation` tab in the Nest head; Tauri commands `dictation_history`, `dictation_delete`,
    `dictation_clear`, `dictation_paste_again`.
- UI: prototype History → production.
- Tests: a renderer check script on production components (empty, populated, long values, filtered empty,
  read error); i18n check.
- Needs: V1.13, V0.3, V0.9.

**V1.15 Dictation settings page** (M)
- Build: settings keys and the page:
  - key: bound key shown, "Change in KDE…" opens System Settings › Shortcuts;
  - mode (hold + double tap / hold only), Space lock, Esc cancel, paste again key;
  - agents: deliver into session, send automatically, spoken code (V1.16);
  - voice: mic, model (shared with chat), language, start sound, cut repeats, plan B;
  - paste: terminals with Ctrl+Shift+V, give back the clipboard;
  - privacy: retention, erase.
- R6 decides the owner of these values; Settings › Chat & voice links here.
- Tests: an H4 fixture for the new keys; renderer check.
- Needs: V1.14.

**V1.16 Spoken code in terminals and sessions** (S)
- Build: `dictation::text::spoken_code`, applied only when the target is a terminal or a session:
  - "ponto/dot", "barra/slash", "underline/underscore", "traço/dash", "dois pontos/colon" → `. / _ - :` <!-- check-english:allow -->
    inside identifiers;
  - no forced final period, no auto-capital.
- Port: idea from [VS] 227-236 (AGPL, rewrite) and [VB] 51.
- Tests: pt/en/es tables; prose apps unchanged.
- Needs: V1.7.

**V1.17 V1 docs and smoke** (S)
- Build: `docs/guide/dictation.md` (+ pt-BR), a settings reference, the architecture diagram, CHANGELOG.
- Smoke matrix: {X11, Wayland} × {Firefox, Kate, Konsole, tmux session, Vults chat} × {hold, double tap,
  cancel}, with Zeca on and off.
- Done when: the user signs off V1.

---

## V2 Your words (R1, reopens X5 locally)

**Goal.** Names come out right.

**Outcome.**
- Dictionary, replacements and text snippets apply before paste.
- Whisper is primed with your words and the focused project's names.
- "Try a sentence" shows what changed.

**V2.1 Store and commands** (S)
- Build: tables `dictionary_words(word)`, `dictionary_replacements(id, from, to)`, `snippets(id, trigger,
  text)`; Tauri commands to list, add and delete.
- Duplicate rules:
  - a word that already exists in any case is rejected;
  - a replacement with the same `from` (any case) overwrites;
  - snippet triggers are de-duplicated (the reference allows duplicates, see [DD] 393).
- Port: [DD] 212-224.
- Tests: duplicate rules.

**V2.2 Text pipeline: snippets, replacements, spelling** (S)
- Build: in `dictation::text`:
  - snippets → marks `⟦n⟧`;
  - replacements in list order;
  - dictionary spelling for plain words only (≥ 2 alphanumerics; letters, digits, space, `-`, `'`);
  - whole-word, any case, any separator; the replacement inserted literally;
  - marks → snippet text last;
  - the result returned as segments (for "Try a sentence" and history highlights).
- Port: [DD] 226-241 (`FlowText.swift:31-107`); [TLP] 285-297 (`text.rs:48-89, 144-184`).
- Tests: port [DD] 259-261 (13 cases) and the `text.rs:283-314` cases, plus pt cases ("cloud code" → Claude
  Code, "códex" → Codex). <!-- check-english:allow -->
- Needs: V2.1, V1.6.

**V2.3 Whisper vocabulary** (M)
- Build: `initial_prompt` = the existing `VOCABULARY_*` + dictionary words, capped at about 224 tokens.
- Measure on a 20-sentence fixture set per language (pt, en) with Base/Small/Turbo; enable per model only
  where WER improves and silence fixtures stay empty (prompt-induced hallucination).
- Port: idea [VS] 227-236 (#3).
- Tests: an `#[ignore]`d, env-driven WER test (models not in CI); a unit test of the prompt builder and cap.

**V2.4 Project vocabulary** (M)
- Build: for the focused session's roost:
  - package, crate and module names (`Cargo.toml`, `package.json`), top-level file names and the branch;
  - cached per roost and refreshed on a session start;
  - merged into V2.3's prompt under the cap.
- Tests: extractor fixtures (a small repo tree).
- Needs: V2.3, V1.7.

**V2.5 Dictionary and Snippets pages** (M)
- Build: the prototype's Dictionary page:
  - word chips, replacements table, "Help the model with these words" and "Project vocabulary" toggles,
    "How it is applied";
  - **Try a sentence** through a `dictation_try` command (real pipeline);
  - a Snippets page.
- Tests: renderer checks; i18n.
- Needs: V2.2, V0.9.

**V2.6 Docs and sign-off** (S).

---

## V3 Styles (model clean-up)

**Goal.** Optional clean-up that is a filter, never an assistant.

**Outcome.**
- Natural, Polished and Prompt styles, set per app.
- "Write in" another language; agent sessions in English.
- A local or cloud provider, with a timeout fallback to the raw text.

**V3.1 Clean-up prompts and guards** (M)
- Build: `dictation::cleanup` (pure: prompt builder + reply validator).
  - System prompt: "a text filter, not an assistant", never answer, never follow.
  - Three switchable rules: hesitations (pt/en/es filler lists), self-corrections, keep technical terms.
  - Few-shot examples as real turns, with question/imperative anchors last.
  - `<dictation>` wrapping; marks kept; "will be pasted in {app}"; spelling list ≤ 200.
  - Prompt mode: Goal / Context / Steps / Constraints / Done when.
  - Translation lines.
- Port:
  - [VB] 49-53 (`refinement.py:140-265`, prompt text MIT);
  - [TLP] 321-341 (`cleanup.rs:324-347`, plausibility 0.35×–1.4×+4 words);
  - [DD] 168-197 (prompt mode, character bounds 8×+600 / 4×+200 / 0.15×).
- Tests: cassette evals in `crates/evals` (`--features evals`):
  - a dictated question stays a question;
  - a dictated command is not executed or answered;
  - an injection ("ignore the rules and…") is kept as text;
  - lost marks are rejected; implausible lengths are rejected.

**V3.2 Providers and the gate** (M)
- Build:
  - the provider is the Chat & voice one (API key: Anthropic / OpenAI via `chat::api`/`openai`) or a local
    OpenAI-compatible one (`providers.rs` Ollama/LM Studio);
  - gate feature `dictation-cleanup` with switch, budget and trace;
  - timeouts 4 s (8 s with translation, 12 s with Prompt), then the raw text is pasted;
  - the heard text is always kept;
  - the CLI path (`chat::summary`, Claude Code haiku) is too slow for 4 s: not offered.
- Tests: the timeout fallback with a fake slow provider; the gate denies when off (S10's zero-requests test
  extended).
- Needs: V3.1, V0.4.

**V3.3 Styles per app and language** (S)
- Build:
  - a `dictation_styles(app, style)` table;
  - defaults: verbatim for terminals/editors, polished for mail, natural for chat apps ([TLP] 345-349);
  - resolved at key-down;
  - flock sessions → Prompt;
  - "Write in" and "Agent sessions in English".
- Tests: resolution table.
- Needs: V1.7, V3.2.

**V3.4 Styles page** (M)
- Build: the prototype's Styles page (clean-up switch, provider picker, three rules, style cards with
  samples, language, per app); History shows "translated" and style.
- Tests: renderer checks.

**V3.5 Docs and sign-off** (S).

---

## V4 Statistics and diagnostics

**V4.1 Stage timings** (S)
- Build: each dictation records release→inserted and its STT, pipeline, clean-up and insert times (columns
  on `dictations`), plus p50/p95 over the last 50.
- Port: definitions from [PT] 262-278.
- Tests: a percentile helper; post-cancel rows excluded.

**V4.2 Statistics page** (M)
- Build: week/month/all; words, time saved (`max(0, words/45 − secs/60)`), wpm (0 under 1 s), streak,
  daily bars, top apps.
- Port: [TLP] 400-403, [DD] 294-303.
- Reuse Activity's chart pieces and the `DESIGN.md` rule "missing measurements are unknown, not zero".
- Tests: math table tests (Rust); renderer check.

**V4.3 Diagnostics page** (M)
- Build:
  - a mic test (bands, noise floor in dB, verdict);
  - the stage table (V4.1);
  - counters: loops cut, plan B used, "Copied" outcomes;
  - a system block: audio host and rate, session type, portals (GlobalShortcuts, RemoteDesktop) ok, KWin
    script ok, GPU (`voice::gpu()`), VAD ready.
- Fold into `app/src/doctor.rs` where it overlaps.
- Tests: the doctor's probes with fakes.

**V4.4 Docs and sign-off** (S).

---

## V5 Meetings (R5)

**Goal.** Record a call (mic + system audio), get a diarized transcript, a summary and to-dos; the audio is
deleted.

**V5.1 System audio capture** (M, spike first)
- Build:
  - `vults-audio` reads the monitor of the default sink through cpal's `pulseaudio` (or `pipewire`) host,
    instead of ALSA, which has no monitor names;
  - fallback: `<sink>.monitor`, else the first monitor;
  - two streams (mic, system) to temporary WAVs (`hound`, MIT) in a 0700 folder;
  - delete on finish; delete leftovers on start (crash).
- Port: [TLP] 126, 130 (`speech/audio.rs:241-252`, `notes/mod.rs:282-302`); [VB] 90, 110 (why not ALSA).
- Tests: monitor-name selection table; leftover cleanup.
- Licence check of the new host crate.
- Wayland and X11 behave the same (PipeWire).

**V5.2 Recording controls** (M)
- Build:
  - start/stop from Meetings and from the island (red dot, timer, the "tell people you are recording" line);
  - an optional live transcript;
  - dictation stays available, with its own stream.
- Tests: state machine (recording × dictation).

**V5.3 Long transcription** (S)
- Build: whisper over the full WAVs with segment timestamps; chunks ≤ 28 s at the quietest 100 ms window
  ([TLP] 138); runs off the dictation path (Q7: the `WHISPER` mutex starves dictation during a long
  transcription; queue meetings behind dictations).

**V5.4 Diarization spike** (M)
- Build:
  - pyannote segmentation-3.0 + 3D-Speaker CAM++ through `ort` (ADR 0002 allows it), not through the
    `sherpa-onnx` crate (espeak-ng, ADR 0028);
  - agglomerative clustering, threshold 0.7 ([TLP] 146);
  - measure on 3 recorded fixtures;
  - verify the licences (segmentation-3.0 is MIT but gated on Hugging Face: use an ungated, hashed
    re-export; CAM++ Apache-2.0).
- Done when: the go/no-go is written here.

**V5.5 Diarization** (L)
- Build: a production version of V5.4, plus model download entries with SHA-256 and speakers merged into
  segments.

**V5.6 Summary and to-dos** (M)
- Build: the JSON `{title, summary, key_points, todos[{text, owner, due}]}` from the chat's model through
  gate feature `meeting-summary`, 120 s timeout.
- Port: [TLP] 353 (`notes/builder.rs:239-243`).
- A to-do can become a Task with a click (W3).
- Tests: cassette eval.

**V5.7 Meetings store and export** (M)
- Build: tables `meetings`, `meeting_segments`, `meeting_speakers`; rename people; export Markdown, SRT
  and VTT.
- Tests: exporter golden files.

**V5.8 Meetings page** (M)
- Build: the prototype's page (list, transcript/summary tabs, speakers, to-dos, export), docs and sign-off.

---

## V6 Talk with Zeca (K8)

**Goal.** Speak with Zeca and hear him, interruptible.

**Rules that still hold:**
- the realtime model is opt-in, with a key in the keyring, a budget and a trace;
- there is no wake word;
- voice never answers a card.

**V6.1 Turn machine** (M)
- Build: `voice::turn` (pure), with states Listening → Thinking → Speaking(locked) → Armed → interrupt
  pending → draining → tail grace, plus a generation counter.
  - The lock runs 0.5 s (1.0 s with AEC), **counted from the first audio played**.
  - On barge-in, replay about 260 ms of ring buffer.
  - Desktop tail grace is 300–600 ms, not the phone's 1500 ms.
  - The two-phase speaking end.
- Port: the design, not the code: [PT] 77-138, 325 (`stream_handler.py` ranges cited there).
- Tests: an eval session with fake STT/LLM/TTS driving the real machine ([PT] 283-294); the race cases.

**V6.2 End of turn** (M)
- Build:
  - a silence policy of 1.6 s, or 2.6 s when the last word extends (pt/en lists: conjunctions, fillers,
    articles);
  - optional smart-turn v3 via `ort`: the 8 s window, whisper log-mel port, threshold 0.5, re-score every
    200 ms, hold at most 1200 ms, off after one failure.
- Port: [VA] 194-198 (`TurnEndPolicy.swift:18-55`); [PT] 64-73, 309-310 (`smart_turn.py:86-250`).
- Tests: log-mel parity against stored reference arrays; policy table.
- Licence check of smart-turn's weights (the spec says Apache-2.0; confirm on the model card).
- Q8: how well smart-turn handles pt-BR is unknown.

**V6.3 Realtime client** (L)
- Build:
  - an OpenAI Realtime WebSocket client in `chat` (`tokio-tungstenite`), PCM in/out, behind gate feature
    `voice-realtime` (money budget per day; past it Zeca falls back to text and says so);
  - the key in the keyring;
  - a provider trait so Gemini Live can come later.
- Needs: Z6, G9 (K8's needs), V0.4.
- Tests: a cassette of a session; the gate refuses when off.

**V6.4 Playback and what the user heard** (M)
- Build:
  - cpal output with a playback cursor from frames consumed;
  - the heard prefix split by words;
  - history rewritten to what was heard + "[interrupted]";
  - a re-delivery nudge when ≥ 3 words went unheard.
- Port: [PT] 122-129, 312.
- Tests: split math; the history rewrite.

**V6.5 Echo cancellation** (M, spike first)
- Build:
  - preferred: detect PipeWire `module-echo-cancel` (WebRTC AEC3) and use its source;
  - Vults does not load modules silently: Settings explains and offers it with a click, or the user runs it
    (Q6);
  - alternative: the `webrtc-audio-processing` crate (BSD), measured.
- Without AEC, barge-in is off on speakers and on with headphones.
- Port: [PT] 140-172, 326.

**V6.6 Zeca's voice tools and delegation** (M)
- Build:
  - the voice turn uses the existing `flock_tools.rs` (`flock_list`…) — K8's "Done when";
  - "moving-around commands" (C11: next or previous session, open the Nest) map to existing intents that are
    not answers;
  - a consult pattern for Claude Code/Codex questions, with a reassurance line after 1.5 s.
- Tests: the rule-2 test over every `Intent` (H1) covers the voice path: no `Decide`/`DecideAlways`
  reachable.

**V6.7 Conversation page and island speaking states** (M)
- Build: the prototype's Conversation page (opt-in switch saying where audio goes, model, budget, turn
  settings, interrupt, echo, "remember what you heard") and island `speaking`/`interrupted`; docs; sign-off;
  K8 moves to Done.

---

## V7 MCP `vults_dictate`

**V7.1 ADR 0028 / M7 alignment** (S)
- Build: record that a *question* card (M7's `ask_user`, a card answered with text) may be filled by
  dictation and is sent with a click or Enter. Permission cards never take voice. ADR 0027/G1 is
  unaffected.

**V7.2 The tool** (M)
- Build:
  - `vults_dictate(question)` in `app/src/mcp.rs`, building on M7's `ask_user`;
  - the client is identified (the process tree, not a self-asserted header);
  - an audit line;
  - the island shows the question; the mic opens only on the key; the transcript is shown and sent on
    confirmation.
- Tests: an MCP round trip with a fake agent; the audit row.

**V7.3 Offer it to agents** (M)
- Build: add the tool to agents' MCP configs only through `agent-config` (rule 3: backup, diff, click); the
  Settings switch "Agents may ask for a spoken answer" (off).

---

## Merge into plan-zeca.md

Add a section under *Left to do*, after *Zeca proactive (wave 5)*. The ids are `V` (unused today); the step
detail stays in this file as `V<phase>.<n>`, and code comments cite `plan-zeca V1.4`.

### Dictation (proposed)

| # | Step | Needs | Done when |
|---|---|---|---|
| V0 | Decisions: ADR 0028 edit (R1, R3, R5, model licences), ADR 0031 dictation into other apps (R2), ADR 0021 edit (Dictation area, island states, R6), ADR 0019 features; Dictation in the prototype, four languages, covered by `verify.mjs` | G9 | The user accepts the ADRs and the prototype (P12) |
| V1 | Dictation in any app: `vults-audio`, key state machine, the talk key with Zeca off, focus at key-down, clipboard lease, XTest and RemoteDesktop paste, delivery into flock sessions (never into one waiting on a permission), island states, History and Settings | V0 | The R3-style smoke passes on X11 and Wayland with Zeca on and off |
| V2 | Your words (was X5, local only): dictionary, replacements, snippets, whisper vocabulary, project vocabulary, Try a sentence | V1 | Measured WER down on the fixture set with the dictionary on |
| V3 | Styles: clean-up as a filter (evals), Prompt, translation, per app; Chat & voice provider or a local one, through the gate | V1, Z6 | Injection and question evals pass; the timeout pastes raw text |
| V4 | Statistics and Diagnostics, stage timings p50/p95 | V1 | Release→pasted p95 shown and under 2 s with Small on Vulkan |
| V5 | Meetings: mic + system audio, temporary WAV deleted, diarization via `ort`, summary through the gate, MD/SRT/VTT | V1, V0 (R5) | A 30-minute call gives speakers, a summary and no audio left on disk |
| V6 | = K8, delivered with the turn machine, end of turn, realtime client, playback and heard prefix, AEC | K8's needs, V1 | K8's own criterion |
| V7 | `vults_dictate`: a question card filled by voice, sent with a click | M7, V1 | An agent's question answered by voice, audited |

Edits to existing rows and lists:
- **K8**:
  - becomes "Voice on the same brain (V6): an opt-in realtime model (replaces C9), turn-taking and barge-in,
    open mic only as a bounded follow-up after Zeca asks (P9), moving-around commands (C11); never answers a
    card";
  - Needs: Z6, G9, V1;
  - Done when unchanged ("A spoken question uses `flock_list`").
- **UX11** ("moods/voice"): its voice part is V6/V7's UI.
- **Dropped (2026-10-12)**: rewrite as "X5 voice dictionary: reopened as V2 (local only, 2026-10-xx); cloud
  transcription stays dropped".
- **C10** in *Done*: "Moved to X5, reopened as V2".
- **X3** (a voice per vult) stays dropped (R4); add "reconsider after V6".
- **Out of scope**: "Wake word | No" stays; add a "Why" column note: "Hands-free is a double tap (V1)".
- **Decisions table**: a row for "Dictation: a Connect crate with no IO; injection in `platform`; never
  delivered into a session waiting on a permission (ADR 0031)".
- **Guarantees**:
  - "Dictation never answers a permission | core test: a waiting session yields Copied (V1.11); H1 covers
    voice intents (V6.6)";
  - "Meetings leave no audio | a leftover-WAV cleanup test (V5.1)".

---

## Risks

1. **Wayland input.**
   - The GlobalShortcuts portal delivers only the bound key: no Esc, no other-key cancel, no Space-lock.
   - Binding Esc through it takes Esc from every app.
   - Mitigation: `talk-cancel`, the island ✕, and the X11 raw path. Spike Q1.
2. **RemoteDesktop portal.** KDE asks once. GNOME and wlroots may lack it or refuse it, so the outcome is
   Copied. It is also a strong capability (it types anywhere): kept in the keyring, explained in ADR 0031.
3. **Typing into an agent's TUI.**
   - A stray Enter answers its own prompt.
   - Mitigation: the core-state guard; auto-send off by default.
   - Multiplexer delivery only where the pane id is known.
4. **Clipboard.** Only text is restored (images and rich formats are lost during the 700 ms lease). Klipper
   could still record if `exclude_from_history` is ignored on some version. Test on Plasma 6.
5. **The whisper mutex.** Chat preview, dictation, meetings and plan B share one lock, so a long meeting
   transcription would freeze dictation. Queue it, and keep dictation first.
6. **Memory.** Small or Turbo stays resident (hundreds of MB); a 10-minute buffer is about 38 MB. Add an
   unload-after-idle policy.
7. **Clean-up answering the dictation.** Small models do. Mitigation: evals, the reply validator and the
   raw-text fallback.
8. **Licences.**
   - Diarization weights are gated on Hugging Face.
   - smart-turn's licence needs confirming.
   - Each audio host crate needs `cargo deny`.
   - No `sherpa-onnx` crate (espeak-ng).
9. **Brand.** Ported code must not carry the source's name; reviews grep for it (`check-brand.sh` catches
   only the two names).
10. **Surfaces.** Island text at Mini width; a sixth Lab tab vs ADR 0021's fixed set; Settings ownership
    (R6).
11. **Prototype drift.** The main prototype has uncommitted work. V0.7 must rebase, not overwrite.

## Open questions

- Q1: can Wayland get cancel or Space-lock without stealing keys? (Layer-surface keyboard focus only while
  locked, then hand focus back?) Spike before V1.8 closes.
- Q2: does the RemoteDesktop restore token belong in the keyring (rule 4) or in the data folder (0600)?
- Q3: does a loopback provider (Ollama) skip the gate's budget, as embeddings do today?
- Q4: should dictation keep Ctrl+Alt+V shared with the chat (one key, context decides), or get its own id?
  Proposal: one key.
- Q5: keep "Undo" after a paste (needs a synthetic Ctrl+Z) or drop it from the prototype?
- Q6: may Vults offer to load PipeWire `module-echo-cancel` itself (a system audio change, with a click), or
  only guide the user?
- Q7: order of whisper work when a meeting, a dictation and a chat preview collide.
- Q8: how well smart-turn v3 handles pt-BR; is NAMO (text, Apache-2.0) a better fit?
- Q9: auto-learn words from corrections in History (the reference has none)? Proposal: a "Add to
  dictionary" button on a highlighted word, no silent learning.
- Q10: Parakeet v3 through `ort` (CC-BY-4.0 weights) as a second engine: worth an evaluation step after V2?
- Q11: where dictation settings live (R6): in the area, in Settings, or one page linked from both.

### Critical Files for Implementation
- /home/rogeradas/Projects/Vults/crates/voice/src/record.rs (and lib.rs, vad.rs)
- /home/rogeradas/Projects/Vults/crates/platform/src/shortcuts.rs (and jump.rs, jump/kwin.rs, jump/x11.rs)
- /home/rogeradas/Projects/Vults/app/src/lib.rs (listen_shortcuts, 707-760) and app/src/voice.rs, app/src/gate.rs
- /home/rogeradas/Projects/Vults/crates/store/src/lib.rs (MIGRATIONS)
- /home/rogeradas/Projects/Vults/design/ui-motion-proposal/dictation/dictation.js with build.mjs, verify.mjs, features.json, copy.json
