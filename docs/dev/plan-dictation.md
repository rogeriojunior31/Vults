# Plan: Dictation

Drafted on 2026-10-16; V0 decisions are being recorded during N3. V0.9 accepted the prototype;
production desktop acceptance remains separate. One plan for the Dictation area: dictation into any app, your words
(dictionary, replacements, snippets), styles, statistics and diagnostics, meetings, talking with Zeca
(K8), local voice commands (V8) and an MCP tool. The phase board lives in [Plan Zeca](plan-zeca.md#dictation-n3); this file keeps substeps,
prerequisites and evidence, with the [V0.5 integration record](#merge-into-plan-zecamd) below.

V1.1-V1.18 are implemented: shared capture/transcription, independent key gestures, foreground
context, guarded X11/Plasma Wayland insertion and exact tmux delivery, measured island feedback,
History, Settings, spoken code/cancellation and music control. The maintainer reported validation
of V1.18 on 2026-10-17. V1.19 reconciles the guide, settings reference and architecture with the
runtime and records regression evidence; unreported cells of its full desktop matrix remain
open rather than inferred from that acceptance. The larger later-phase UI still lives in the
single accepted prototype (`design/ui-motion-proposal/dictation.js`): Voice commands, Talk with
Zeca, Meetings and Styles are not working production pages yet.
V2.1/V2.2 now provide local Your words storage and the final-text pipeline. V2.3 adds optional
dictionary context, qualified only for Turbo/fixed English by synthetic WER and silence fixtures,
off by default. V2.4 adds optional names from the focused known Git project, with bounded local
extraction and per-working-tree caching. V2.5 adds working Dictionary/Snippets pages, the local
sentence preview and Settings-owned vocabulary controls. V2.6 reconciles documentation and
records the evidence/acceptance matrix; human-speech acceptance remains separate. The separate
proposal was folded in and removed.
V4.1-V4.3 now provide retained stage timings, Activity > Dictation usage and Settings > Dictation
Diagnostics, including an explicit memory-only microphone test. V4.4 reconciles their docs and
acceptance checklist. The current evidence and open Small/Vulkan p95 target are recorded in
[V4 Statistics and diagnostics](#v4-statistics-and-diagnostics). The next implementation step is
**V8.1**, the pure local voice-command normalizer and phrase tables. V1/V2/V4 maintainer and real
microphone/desktop acceptance remain separate; N3 is not complete.

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

## Decisions (V0)

Rules that hold as they are: [ADR 0002](../adr/0002-rust-backend-no-sidecars.md) (Rust only, no Python
sidecar), [ADR 0028](../adr/0028-spoken-replies-through-a-realtime-model.md) (local transcription, audio in
memory, no GPL in the binary, voice never answers a card), [ADR 0004](../adr/0004-a-human-answers-permissions.md)
and [ADR 0019](../adr/0019-requests-on-the-users-behalf.md) (model clean-up is a request on the user's behalf).

| # | Today | Proposal | Why |
|---|---|---|---|
| R1 | X5 voice dictionary dropped (2026-10-12) | **Reopen, local only**: post-processing plus whisper vocabulary; cloud transcription stays dropped | Names of projects and tools come out right; cheap and fully local |
| R2 | Dictation only into Vults' chat | **Dictation into any app** and straight into flock sessions (new ADR 0032) | Clear gain; the pipeline already exists |
| R3 | Wake word: no | **Still no.** Hands-free is a double tap on the key. Zeca's 8 s follow-up mic is off by default | Keeps ADR 0028; no always-open mic |
| R4 | X3 a voice per vult dropped | Stays dropped; reconsider after V6 | Speaking depends on the opt-in realtime model |
| R5 | No meetings in the plan | **Meetings in V5**, opt-in per recording, mic and the PipeWire monitor | Useful, but the heaviest part (diarization) |
| R6 | `DESIGN.md`: Settings is where configuration lives | **Settled 2026-10-16 (revised the same day): Settings › Dictation owns every value**, with Diagnostics at its end; Activity › Dictation holds the figures. The area's side list keeps the work (Dictation, Voice commands, Talk with Zeca, Meetings, Dictionary, Snippets, Styles) and links to both. The voice model stays in Chat & voice, shared with the chat | The `DESIGN.md` rule holds unchanged; one owner per value, and figures sit with the other figures |
| R7 | Each request on the user's behalf has its own switch | **Off by default**: model clean-up (and with it styles, "Write in" and agent sessions in English), meeting summaries, Zeca's voice. Off, the clean-up group shows only its switch | Rule 4 and [ADR 0019](../adr/0019-requests-on-the-users-behalf.md) |
| R8 | The island sees: the flock, badges by shape, a click opens Compact | **The island stays the flock.** Dictation draws a grey line over it only while active; no idle hint, no buttons (no Undo); color only on badge shapes; a card's amber edge still shows. A caption under it carries what the line cannot (what was heard, a cut line), with no buttons, and goes with the dictation | The approved island (P2, P9) and D5 |
| R9 | Moving around by voice (C11) waits for K8's realtime model | **Voice commands, local (V8)**: their own key; a parser and a name resolver with no model move around the flock, read what Vults knows and quiet projects, with a 90 s memory ("and docs too", "undo"); never type into an app, never answer a card. Not understood: say so, or (a choice) send it to Zeca as a chat message | Instant, free and offline; K8 then reuses it before the model. The same aliases as Your words |

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
  X11) and the text pipeline (snippets as marks `⟦n⟧`, replacements, spelling, spoken code, marks back);
  `command`: the voice-command parser and name resolver (V8), pure, returning existing non-answer `Intent`s
  or a read of the flock tools.
- **`platform`**: focus at key-down (EWMH on X11, a KWin script on Wayland), XInput2 raw keys on X11, the
  clipboard lease (give back after 0.7 s only if unchanged, hidden from Klipper), XTest and RemoteDesktop
  paste, delivery into flock sessions through their multiplexer, never into a session waiting on a
  permission.
- **Clean-up**: a filter, never an assistant; styles As spoken, Natural, Polished and Prompt; "Write in"
  another language; per app; the Chat & voice provider or a local one (Ollama, LM Studio) through the gate;
  timeouts of 4 s (8 s for translation, 12 s for Prompt), then the raw text; heard text stays
  in memory and enters History only under the chosen retention setting.
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
11. Voice commands for the flock with no model, on their own key, through the same names as Your words.
12. A spoken cancel that works the same on Wayland, where Esc cannot be bound.

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
  - the prototype comments naming references were cleaned at V0.7 (2026-10-16).
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

**Goal.** Record R1–R5 as ADR edits (R6–R8 are settled in the prototype). Put Dictation into the single UI
prototype, translated and covered.

**User-visible outcome.** The main prototype has a *Dictation* Nest/Lab tab with seven work pages, links to
Settings and Activity, and the island states, in four languages, each page carrying its phase badge. Nothing in the app changes yet.

**Status (2026-10-17).** V0.1-V0.8 are recorded and the maintainer accepted the integrated
prototype in V0.9/P12 on 2026-10-17. This accepts the visual direction, not production runtime
behavior or the later native smoke matrix.
The maintainer accepted V0.1 and authorized continuing with the planned defaults, with changes
to be raised as work proceeds. ADR 0028 now carries the accepted local vocabulary, eight-second
opt-in follow-up and temporary meeting audio rules. V0.2 is recorded in
[ADR 0032](../adr/0032-dictation-into-other-apps.md), with its pt-BR translation and index entry.
It requires target revalidation, a conditional clipboard lease, a keyboard-only portal grant
whose token stays in the keyring, and no input into a waiting session. Unknown terminal safety
or lost focus yields Copied; no deferred delivery runs after a card is answered. This also
settles Q2. V0.3 is recorded in ADR 0021 and DESIGN.md: the fifth Nest tab, seven work pages,
central Settings and Activity owners, neutral transient feedback and card priority. With the Nest
visible, feedback stays there instead of waking the island. No runtime behavior has changed.
V0.4 is recorded in ADR 0019 and its pt-BR translation: independent opt-ins, rolling 24-hour
quotas of 2,000 clean-up and 20 summary attempts, a US$1 default realtime limit, and loopback
requests still behind their feature switch, request quota and trace (Q3). The existing count-only
gate does not yet implement monetary reservations. V0.5 integrates the authoritative V0-V8
phase board, dependencies, reopened/dropped ids and required guards into Plan Zeca. This is
roadmap integration, not a claim that this branch is merged or N3 implemented. V0.9/P12 is
accepted and V1.1 implements shared audio capture; later runtime phases and native acceptance
remain separate. The backend follows each substep's prerequisites.
0031 remains allocated to Tasks removal; all dictation references use 0032.

**V0.1 ADR 0028 edit: voice, dictation, meetings** (S) — accepted 2026-10-17
- Build: edit `docs/adr/0028-spoken-replies-through-a-realtime-model.md` with dated *History* lines.
  - R1: a local dictionary and whisper `initial_prompt` are allowed; cloud transcription stays out.
  - R3: no wake word, mic opens only on a key or a click. Hands-free is a double tap. Decide whether
    "reopen 8 s after Zeca asks" (prototype: Dictation › Talk with Zeca, *Answer without the
    key*) counts as "opened by the user"; the proposal is yes, off by default, bounded and shown on the island.
  - R5: Meetings is the one place where audio touches disk, as a temporary WAV deleted after
    transcription. It is opt-in per recording, with a consent line.
  - Licence rule extended to the new models: smart-turn v3, pyannote segmentation-3.0, CAM++.
  - Also add `docs/pt-br/adr/...`.
- Needs: none. Done when: the user accepts the edit.

**V0.2 ADR 0032: dictation into other apps** (M) — recorded 2026-10-17
- Build: new topic `docs/adr/0032-dictation-into-other-apps.md` (next free number, README table). It covers:
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

**V0.3 ADR 0021 edit: a Dictation area and island states** (S) — recorded 2026-10-17
- Build:
  - Dictation becomes the fifth Nest/Lab area (Flock, Chat, Connections, Activity, Dictation),
    delivered with its feature phases. Tasks stays removed under ADR 0031.
  - The island gains dictation states, which never hide or delay a card (D5's test extended).
  - The island stays the flock (R8): a grey line over it while dictating, no idle hint, no buttons.
  - R6, revised: the area has seven work pages and links to Settings › Dictation (configuration
    and Diagnostics) and Activity › Dictation (figures). The shared transcription model and
    language stay in Settings › Chat & voice. `DESIGN.md` records these owners; no settings
    authority moves into the Nest renderer.
  - The optional caption takes no focus or mouse input and clears within 2 s of completion.
    It stays within the fixed overlay, gives way to a card and adds no input region.
  - Feedback respects the one-surface rule: when the Nest is visible, use its active page,
    preserving selection, drafts and the captured destination's focus.
- Needs: none. Done when: the contract is recorded under the maintainer's standing authorization;
  `DESIGN.md` agrees. P12 visual review and production native validation remain separate.

**V0.4 ADR 0019 consequences** (S) — recorded 2026-10-17
- Contract: [ADR 0019](../adr/0019-requests-on-the-users-behalf.md#dictation-model-requests),
  with its pt-BR translation. These features remain unregistered and off until their phases land.
  - `dictation-cleanup`: 2,000 attempted model requests in any rolling 24 hours, retries included.
  - `meeting-summary`: 20 attempts in the same rolling window, retries included.
  - `voice-realtime`: a positive monetary limit, default US$1, zero disables remote voice;
    accounting must reserve in-flight spend and stop audio before the allowance is exhausted.
  - Local Ollama/LM Studio calls retain the switch, request quota and trace through the gate;
    only remote monetary accounting is exempt (Q3). Validate an explicit loopback IP, disable
    proxies and redirects, and never silently fall back to a remote provider.
  - Request traces omit audio and transcript/clipboard contents. Off, refused, spent, failed or
    invalid clean-up uses raw text; a meeting keeps its local transcript; voice falls back to text.
- Needs: none. Recorded under the maintainer's standing authorization. Actual gate registration,
  settings, spend accounting and request tests belong to V3/V5/V6; they are not shipped by V0.4.

**V0.5 Plan merge** (S) — integrated 2026-10-17
- [Plan Zeca > Dictation (N3)](plan-zeca.md#dictation-n3) owns the phase table and delivery order.
  K8 now points to V6 and depends on V1/V8, C11 moves to V8, C10/X5 reopen as local-only V2,
  X3 stays dropped pending V6, and UX11's voice part follows V6/V7.
- Decisions and required guards point to the accepted ADRs; Tasks stays removed. M7's existing
  MCP is distinct from its pending question-card follow-up, and V7.1 must explicitly align the
  voice/card ADR before question dictation can ship.
- Needs: V0.1-V0.4. Roadmap integration is complete; this does not accept P12, complete runtime
  phases, claim a PR merge or move unresolved native spikes into Done.

**V0.6 Prototype strings to English** (S) — done 2026-10-16
- Every sentence is English through the prototype's `L()`/`phrase()`; 245 `[pt-BR, es, zh]` triples went
  into `copy.json`. The sample dictations are translated too, so the text pass shows in each language.
- No reference-project names in the module; sample data has no real names or addresses.
- The separate proposal (`dictation/`, never committed) was folded into V0.7 and removed, as the single
  prototype rule asks.

**V0.7 Prototype: Dictation as a Lab area** (M) — done 2026-10-16
- `dictation.js` is a module in `build.mjs`'s `MODULES`, in the workspace scope.
- The Lab nav gains `nav-dictation` (icon `mic`, Alt+5 after Tasks removal). The area has its own side list, drawn like Settings'
  (grouped, with icons and the sodium bar on the page shown), as the user's original proposal had it: Voice
  (Dictation, Voice commands, Talk with Zeca, Meetings, Statistics), Your words (Dictionary, Snippets, Styles),
  System (Settings, Diagnostics). Dictation has *Try it*, Voice commands *Try a command*. A first pass with
  top tabs and a Settings › Dictation category was reverted the same day after the user's review.
- The final R6 revision keeps the model and language in Settings › Chat & voice; Settings ›
  Dictation owns configuration and Diagnostics, Activity › Dictation owns figures. The area
  has seven work pages and links to those two owners, matching the current prototype and its existing verifier.
  The ten-page side list above describes the earlier pass, not the current contract.
- The island states are drawn over the flock by `paintDictationIsland()` from `island.js`, and the
  prototype controls get an *Island while dictating* row. *Try it* plays listening → transcribing → pasted
  on the island and comes back to History with the new line.
- `dx-*` CSS sits in `template.html`; in a Lab under 560 px the Lab tabs are icons and their counts give
  way, and under 760 px the side list becomes one scrolling row above the page.
- Size: 986,883 of the 1,000,000-byte limit at first. `build.mjs` now inlines `copy.json` unescaped and
  without indentation (the page declares UTF-8): 927,318 before the additions below, 948,702 after, 953,914 with the side list.
- Added the same day, after the comparison with the reference app's voice assistant (R9):
  - *Try a command* next to *Try it*: it plays the next of nine sample commands on the island (open a
    session, who is waiting, approve it → refused, mute web, and docs too, undo, an ambiguous name and its
    answer, not a command) and History keeps each as a *Voice command* line;
  - Dictation › *Voice commands* (V8), a page with the recent commands under it: the switch, the command key (Ctrl+Alt+Z), the names it
    understands, what happens when it is not a command, and *What you can say*;
  - Key: *Cancel by voice*; Voice: *Music while you speak* and *Caption under the island*;
  - Your words: replacements also fix command names; project vocabulary also reads the window title;
  - the island gains *Cancelled* and six command states (a second row in the prototype controls) and the
    caption under it.

**V0.8 Prototype coverage** (M) — done 2026-10-16
- `features.json`: the Dictation entries carry `status: "planned"` and a `phase`; every page of the side list,
  the meeting summary and to-dos, the settings rows, Zeca's voice, each island state and the caption are placed,
  and keys, paste, focus and the command parser are native with a reason (the current post-N0
  manifest has 150 features, 134 placed on screen). `verify.mjs --coverage` opens each place in four languages (new `at` keys:
  `dxTab`, `meetingTab`, `dictation`).
- `scope.json`: `dictation.js` in the sources; `P12 Dictation` under `pending` until V0.9.
- `verify.mjs` flows: the side list has seven work pages and two destination links; History and its phase badge; with the clean-up off nothing is rewritten and
  every line is "As spoken"; a replacement is marked; search filters; adding a word; Try a sentence; *Try it*
  walks the island and adds a line; after it the island is the flock again; the *Held for a card* state; the
  defaults of the clean-up, Send by itself and agents' spoken answers are off; turning the clean-up on shows
  its model and gives the session its English prompt; "Do not keep" hides the history; recording a meeting;
  Zeca's voice is off and shows its model when on; zero untranslated sentences. After R9: the caption stays
  away with nothing to add, shows what was heard over an answer, goes with the dictation and can be turned
  off; *Cancelled*; six commands in a row give commanded, answered, refused (not drawn like a card),
  commanded ×3, and *undo* reverses the remembered one; commands are on by default, and off they ask nothing
  more.
- Not covered: overflow at Mini width (the DOM simulation has no layout; checked by screenshots instead).

**V0.9 Prototype review** (S) — accepted by the maintainer 2026-10-17
- Build: user review of the integrated prototype; the findings are fixed in place; `scope.json` moves `P12`
  from `pending` to `approved` with the date.
- Include a keyboard pass after Tasks removal: the fifth tab uses Alt+5; the prototype
  handler now ignores Alt+6 instead of indexing past the nav, and Ctrl+Alt size keys stay separate.
- Prepared and accepted on 2026-10-17:
  - regression checks first reproduced the retired Alt+6 crash and lost focus after revealing
    settings; fixes preserve tab/size shortcuts, scroll position and keyboard focus;
  - Black is the default and Light reuses production theme roles, including embedded selector
    specificity; no separate light palette was added;
  - model, command and vocabulary switches live in Settings › Dictation; the work pages link
    there and keep only their content and actions. A disabled Zeca cannot start a voice chat;
  - simulated meetings require confirmation; privacy copy distinguishes temporary meeting
    audio from in-memory dictation, and summary copy states that a transcript reaches the model;
  - clipboard caveats, rolling quotas and bounded follow-up copy match the accepted ADRs,
    with pt-BR/es/zh translations. The coverage manifest follows each control's actual owner.
- Local review: run `npm ci`, `node design/ui-motion-proposal/build.mjs`, then
  `node design/ui-motion-proposal/verify.mjs --missing`. Serve the existing prototype with
  `node node_modules/vite/bin/vite.js design/ui-motion-proposal --host 127.0.0.1 --port 4173 --strictPort`
  and open `http://127.0.0.1:4173/vults-ui-proposal.html`. Choose Lab, then Dictation. Prototype
  controls expose Light/Black, narrow/short sizes and island states. This is sample data: no
  microphone, paste, portal or provider is exercised by a simulation.
- Automated evidence: the UI build and existing prototype verifier pass, including all 150
  features, 134 screen placements, new navigation/consent/Settings-owner checks, retained focus
  and translations. The maintainer reviewed the served prototype and approved it. P12 moved
  into `scope.json`'s approved proposals, dated 2026-10-17. No automated browser screenshot or
  production native smoke is claimed: those remain separate evidence for the runtime phases.
- Done: user sign-off received; the prototype gate for later production UI is satisfied.

---

## V1 Dictation in any app

**Goal.** Hold the key in any app, speak, release: the text lands where the cursor is, or in the focused
flock session. Local only, no model clean-up.

**User-visible outcome:**
- Ctrl+Alt+V works in Firefox, Kate, Konsole and a Claude Code session, on KDE Wayland and X11, with Zeca on
  or off.
- The island shows listening, working and then the honest result: Pasted, In session or Copied.
- Lab › Dictation has History and Settings.

**V1.1 `vults-audio` crate** (M) — accepted by the maintainer on 2026-10-17
- Build:
  - new Connect crate `crates/audio`, extracted from `voice::record`: capture thread, device by name with
    fallback to default, monitors skipped;
  - mono 16 kHz through `rubato` (MIT OR Apache-2.0), replacing the existing linear interpolation;
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
- Implementation (2026-10-17): `vults-audio` owns device capture, a bounded callback queue,
  continuous rubato conversion with delay/tail handling, a 16 kHz mono buffer and the nine-band
  meter. Chat's Recorder API is unchanged; its Silero watcher and GPU preview stay in `voice`.
  Chat keeps 60 seconds; other callers may request up to 600 seconds. The main buffer reserves
  only that mono capacity, rather than native-rate multichannel audio. Capture errors, including
  queue overflow, reach `finish` instead of silently returning a partial recording.
- Evidence: seven audio tests cover tone pitch/RMS/tail at 8/44.1/48/96 kHz, anti-alias rejection,
  stereo/invalid input, callback boundaries, meter bands, device fallback and limit validation.
  Sixteen existing voice tests pass with the installed Base/Silero models and the public JFK
  sample, including actual transcription and preview cancellation. Workspace Clippy (all targets) and cargo-deny
  pass. No new model, settings key, network request or renderer behavior was introduced.
- Acceptance: the maintainer reported validation and accepted this step on 2026-10-17.
  This report does not establish a separate X11/Plasma Wayland or disconnected-device matrix;
  those checks remain part of N3 runtime/release acceptance.
- Needs: V0.1.
- X11/Wayland: same (cpal ALSA host today; pulseaudio/pipewire host only in V5.1).

**V1.2 VAD hysteresis** (S) — implemented; automated regression checks pass
- Build: `voice::vad` gains activation 0.50 and deactivation 0.35, exp filter α = 0.35, minimum speech
  250 ms, minimum silence 400 ms, queued transitions, and `reset()` per turn.
- Port: [PT] 41-51 (`silero_vad.py:121-176, 345-414`).
- Tests: WAV fixtures (speech, pauses, breath noise, silence) under `crates/voice/tests/fixtures/`, with
  expected segment edges ±40 ms.
- Done when: no regression in the chat's end-of-speech.
- Implementation (2026-10-17): Silero's 32 ms probabilities feed a small state machine with
  the thresholds, smoothing and dwell times above. Confirmed start/end edges are queued in
  order. Quiet edges remember their onset before the filter finishes decaying, so smoothing
  does not add decay time to Chat's existing 1.5 s end-of-turn pause. Every analysed window
  resets the detector, including empty windows; whisper.cpp also resets its recurrent state
  for each analysis. Short pauses remain inside a speech segment, and the final segment
  stays bounded by the recording length.
- Evidence: 26 audio/voice tests pass with the installed Base and Silero models. Four checked-in
  WAV fixtures cover speech, a 150 ms pause followed by a 750 ms pause, synthetic breath-like
  noise and silence; segment counts and reference edges are checked within 40 ms. Pure tests
  cover threshold flapping, minimum durations, queued transitions and turn reset without a
  model. Existing real-model Chat end-of-speech, transcription and preview-cancel tests pass.
  The Chat stop tolerance is unchanged (200 ms), now compared in samples to include its exact
  boundary without floating-point rounding. No new native microphone validation is claimed
  for V1.2; the full desktop/device matrix remains a release check.
- Needs: V1.1.

**V1.3 Transcript gate: loop remover and hallucinations** (S) — implemented; automated checks pass
- Build: `voice::gate` (pure) collapses repeated word/char runs (threshold 6, unit ≤ 60 chars) and drops the
  whisper silence set ("Thank you.", "Obrigado.", "[BLANK_AUDIO]", "♪"…) when it is the whole result.
- Port: [VB] 37, 113 (`refinement.py:25-111`, rewrite with a manual loop: `regex` has no backrefs); [PT] 242,
  313 (hallucination set).
- Tests: table tests in pt/en/es; real looped outputs saved as fixtures.
- Done when: used by the chat path too.
- Implementation (2026-10-17): pure `voice::gate` filters the common decoder result, covering
  both Chat preview and final transcription. Word/phrase runs of six or more use case and
  surrounding sentence punctuation for comparison; a Unicode character pass also handles
  unspaced loops. Units are capped at 60 characters. Each run retains one occurrence to avoid
  deleting all evidence of the words. Runs below six stay, as do elongated letters and numeric
  or punctuation-only character runs. Known sound annotations replace the previous blanket
  bracket removal, preserving code such as `foo(bar)` and `values[index]`.
- Known silence phrases match only the whole normalized result, including Portuguese, English,
  Spanish and Chinese. A deliberately spoken standalone match is also omitted; larger sentences
  containing those words survive. This limitation is documented in the user guide. No network,
  new model, dependency or permission behavior is introduced.
- Evidence: 28 audio/voice tests pass with installed Base/Silero models, including existing
  transcription, pause timing and preview cancellation regressions. Table fixtures cover loops
  in four languages; tests cover six/five repetition boundaries, 60/61-character units, long
  runs, idempotence, exact silence matching and technical text. A short public, reported Whisper
  music hallucination is retained as a real-output fixture; its subthreshold runs deliberately
  stay. Synthetic fixtures are labelled separately. Workspace Clippy passes with warnings denied.
  This does not claim a new native microphone pass or complete the N3 release matrix.
- Needs: none.

**V1.4 `vults-dictation` crate: key state machine** (S) — implemented; pure transition tests pass
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
- Implementation (2026-10-17): dependency-free Connect crate with caller-supplied monotonic
  `Duration`, four phases and optional Start/Lock/Finish/Cancel/Restart actions. A short single
  tap cancels after the 400 ms window; its second press at the exact deadline still locks.
  A late second press before a timeout tick restarts with fresh audio. Space locks only during
  a hold and is ignored while locked; other keys cancel a hold or the wait between taps.
  Press/release tracking ignores auto-repeat, duplicate releases and repeats after cancellation
  or capture failure until the physical key is released. Reset emits no effect.
- The Finish action carries an absolute `capture_until = now + 600 ms` deadline. It authorizes
  neither insertion nor a permission answer. The future orchestrator must keep capture through
  that deadline and handle cancellation during the tail and processing; this pure step starts
  no microphone, timer or desktop listener. Locked mode finishes on its next genuine press;
  a hold finishes on release. The port's attribution is retained in `NOTICE`.
- Evidence: sixteen deterministic integration tests cover the reference key scenarios plus the
  Space chord, 299/300 ms holds, the inclusive 400 ms boundary, delayed ticks, the 600 ms finish
  deadline, cancellation in every active phase, auto-repeat, stray releases and capture-failure
  reset. All 44 dictation/audio/voice tests pass with the installed Base/Silero models supplied
  to the existing voice regressions. Workspace Clippy (all targets), cargo-deny and the project
  checks pass. Desktop binding (V1.5), orchestration (V1.6) and raw X11 key events (V1.8) remain
  later steps; no new native/runtime acceptance is claimed here.
- Needs: none.

**V1.5 The talk key becomes the dictation key** (S) — implemented; native acceptance pending
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
    the separate `talk-cancel` shortcut; X11 also supports Esc (V1.8). R8 adds no island button.
- Implementation (2026-10-17): Talk no longer gates on Zeca. The app pins release to the
  press owner, preserves focused Chat drafts, ignores Settings, and keeps active hands-free
  dictation on its original owner when focus changes. New `talk-again` and `talk-cancel` ids
  suggest Ctrl+Alt+B/X; until automatic insertion lands, Again honestly copies the last text.
  The desktop consent descriptions for these ids are translated in core's four languages.
  Settings displays the actual reported dictation bindings; ShortcutsChanged updates them.
  Portal signals are checked against this session, and closing it cancels active voice work.
- Evidence: platform dispatch and app route tests cover releases, duplicate events, Zeca off,
  Settings and active-dictation focus changes. Desktop consent/remapping and X11/Plasma Wayland
  microphone checks remain native acceptance, not inferred from these automated checks.
- Needs: V0.2.

**V1.6 Orchestrator, first usable slice: "Copied"** (M) — implemented; native acceptance pending
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
- Implementation (2026-10-17): an app-owned worker drives the real key machine and shared
  capture (600 s maximum), preserving the 600 ms tail. Model/language/VAD are pinned at capture
  start. Transcription uses the existing shared model cache, mutex, VAD trim and transcript gate;
  no snippets produce marks at this stage, so no identity text-pipeline wrapper is added yet.
  Existing installed Base retries only when a different selected model returned no text and
  Silero heard speech. No automatic model download is added.
- Cancellation invalidates a generation immediately, aborts Whisper between tokens and is
  checked after capture/tail, around transcription and under the same lock as the native
  clipboard write and read-back confirmation. A refused or mismatched write never reports Copied.
  Clipboard initialization happens before taking the cancellation guard; no GTK/main-thread
  clipboard callback remains. New turns, voice off, screen lock,
  portal loss and exit invalidate old work; disabling voice, locking and exiting forget Copy
  again's in-memory text. No transcript history, audio file, paste or submission is written.
- A transient, translated status uses the existing theme. It takes no input region or focus,
  and stays in Nest/Settings when the Nest is open. Read-only snapshots restore active feedback
  when a surface mounts; a sequence prevents delayed snapshots rewinding live status, and
  terminal feedback expires. The permanent island presentation remains V1.12.
- Evidence: 199 tests across app/platform/audio/voice/dictation pass, including the real installed
  Base/Silero models and token cancellation with Vulkan enabled. External-only fakes exercise cancellation
  at start, tail, capture finish, decode and clipboard commit; overlap, silence/short audio,
  capture/clipboard failures, the ten-minute cap, Base retry conditions and status snapshots.
  Build and the existing renderer logic checks pass, plus `check-dictation-status.mjs` for
  delayed snapshots, owner handoff, expiry and text safety. A regression first reproduced
  Chat cancellation failing during transcription; per-recording epochs now reject late text,
  errors, focus and state changes after cancellation or a newer recording. The real renderer
  check verifies that race. Workspace Clippy and cargo-deny pass.
  Review fixes (2026-10-18): rebinding clears both the routing and recording key latches even
  when the old shortcut never releases. History replay carries its text in the queued command;
  only confirmed delivery changes Copy again's last successful text. Regressions cover missing
  key-up, Chat-to-dictation routing, failed/cancelled replay and an unavailable worker.
  No available computer-use surface could perform native validation in this session: the real
  microphone, desktop consent, clipboard ownership and X11/Wayland pass remain pending before
  accepting V1.5/V1.6 as usable. The local prototype's earlier acceptance does not close this.
- Native bug fix (2026-10-17): the maintainer observed Copied while the old link remained.
  An independent probe reproduced `gtk_claim=true` with the native Wayland clipboard unchanged:
  GTK's return value acknowledged its local offer, not the compositor accepting a background
  selection. `platform::clipboard` now uses native ext/wlr data-control on Wayland and a retained
  arboard owner on X11. A fresh native read must match before success. Wayland never silently
  falls back to an XWayland-only selection. wl-clipboard-rs >=0.9.4 keeps source bytes in memory;
  no transcript/previous-clipboard file or external runtime helper is added. Arboard is text-only.
  Its Windows-only clipboard-win 5.4.1/error-code 3.4.0 dependencies use the reviewed permissive
  Boost license: deny.toml allows it only for those exact crates/versions, with all other
  dependency checks retained.
- The opt-in `clipboard::tests::background_copy_reaches_an_independent_wayland_client` regression
  runs without activating a surface, confirms replacement/Unicode/newlines through wl-paste and
  rejects NUL without overwriting; its guard restores the previous text only if the test still
  owns it. It passes on the actual Plasma session. An isolated Xvfb probe also confirms the X11
  backend through an independent GTK client. These clipboard checks do not establish microphone,
  shortcut or complete native UI acceptance. Logs contain only phase names; clipboard failure
  messages and the Settings compatibility note are translated into all UI languages.
- Needs: V1.1–V1.5.

**V1.7 Focused app at key-down** (M) — context reader implemented; native/History acceptance pending
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
- Implementation (2026-10-17): `platform::focus` shares the existing X11 readers and private
  KWin script-file safeguards, without calling activation or multiplexer commands. The first
  Start/Restart key event requests context while audio starts; autorepeat and double-tap lock
  do not retarget. One in-flight reader bounds display-server stalls. Failure, an observation
  arriving after 300 ms, an unsupported Wayland desktop, cancellation or a completed turn leave
  context unknown. No XWayland fallback is used on other Wayland desktops.
- KWin uses a read-only, bounded script, accepts replies only from the current KWin bus owner,
  checks its execution deadline and attempts unload even after timeout. Script deletion is
  guarded on every exit. The official [KWin API](https://develop.kde.org/docs/plasma/kwin/api/)
  documents the resource class, caption, PID and internal UUID used here.
- Timing limit: an asynchronous foreground read cannot prove which window was active at the
  exact key-down instant. This slice is context for manual copying only. Exact target capture
  and delivery-time revalidation remain required before V1.9–V1.11 may insert text (ADR 0032);
  a changed or unprovable target must copy instead. Do not promote this metadata into an
  insertion authorization. PID ancestry associates a session only when one candidate matches;
  it never proves a focused tab or pane, even with one known session.
- Metadata stays in the in-memory turn. Status exposes the app class, terminal hint and optional
  session key; titles/window ids do not go to the renderer or logs. A copied result names the
  observed app. General Settings explains GNOME/wlroots's unknown-app/manual-paste fallback.
  History does not exist until V1.13, so its app display is tracked there rather than claimed
  complete here. Native X11/Plasma validation and the exact-key-down guarantee remain open.
- Checks cover WM_CLASS parsing, desktop selection, terminal aliases, bounded/cyclic/foreign
  ancestry with a fake process table, ambiguous sessions, data-only script inputs and replies,
  authenticated one-shot KWin replies, first-press ownership, autorepeat, focus arriving before listening, cancellation, overlapping
  turns, completed turns and renderer text safety. All 207 selected app/platform/audio/voice/dictation
  tests pass (real installed Base/Silero models supplied), as do the UI build, six renderer checks,
  workspace Clippy, formatting, project guards and translation source marks. One existing login-shell
  timeout test failed under the initial concurrent build load, then passed in both subsequent app
  runs; its production code was not changed. Native acceptance is still pending.
- Needs: V1.6.

**V1.8 X11 raw keys: Esc and other-key cancel** (M) — implemented; real X11 desktop acceptance pending
- Build:
  - an XInput2 raw key listener on the root window, no grab (x11rb `xinput` feature), only while a dictation
    is live;
  - Esc → cancel; any non-modifier key during Holding/TapWait → cancel; Space → lock.
- Port: [TLP] 212-221 (`flow/os_linux.rs:490-558`).
- Tests: the keysym mapping table; the listener's start/stop lifecycle with a fake connection.
- X11 only; on Wayland the Space-lock needs the key events we do not get. **Space-lock is X11-only** unless
  Q1's spike finds a way.
- Implementation (2026-10-17): `platform::raw_keys` opens an XI 2.1 connection only for an
  active X11 dictation. It selects RawKeyPress/RawKeyRelease on all master devices at the root,
  without a grab; XI 2.1 is needed to observe keys while the portal shortcut owns its grab
  ([protocol](https://www.x.org/releases/current/doc/inputproto/XI2proto.txt)). The native path
  never starts on Wayland, including sessions that expose DISPLAY for XWayland.
- The server's keymap, modifier map and held-key state classify Escape, Space, modifiers and
  Other; no typed text or key history is retained or logged. Talk and Copy again's actual portal
  bindings are excluded only while their modifiers are held, with no hardcoded talk letter.
  Auto-repeat is ignored. An unknown binding or failed listener leaves portal controls working.
  Human-readable localized binding names remain a compatibility limit until a normalized
  accelerator can be obtained. Map notifications refresh the lookup instead of cancelling:
  the native smoke initially caught an initial map notification causing an immediate false Esc.
- The worker owns the listener through capture, tail and transcription. Generation checks reject
  stale key callbacks, and cancellation clears its active flag immediately, including while
  capture or decoding is blocked. Completion, capture failure and cancellation drop the handle;
  polling exits and closes its XI2 connection. Escape also cancels during the tail/decoder when
  the gesture machine is already Idle. Space locks only Holding, and Other cancels Holding or
  TapWait while preserving Locked. Opening the island on Space is suppressed while Space-lock
  owns it; an Open binding remapped to another key remains available.
- Checks: the mapping table covers modifiers, aliases, repeats, actual/remapped chords and bare
  keys after modifier release. A fake connection proves closure on drop, generation revocation
  and connection failure. Real worker tests exercise lock, other-key cancellation, Esc during tail
  and a blocked decode, stale callbacks, listener failure and cleanup after copy/capture failure.
  The opt-in `raw_keys::tests::private_x11_server_delivers_raw_chords_without_grabbing_the_foreground`
  smoke passes on a private Xvfb server: real XI2 events ignore the bound chords, emit Other/Space/Esc,
  still reach the foreground window and stop after drop. The production listener observes input
  and never injects it; V1.9 separately enables XTest for opt-in insertion. No real X11 desktop/portal/microphone
  acceptance is claimed by that isolated check; the current host is Plasma Wayland.
- Validation: 186 app/platform/dictation tests pass, plus the explicit native XI2 smoke. The UI
  build, six renderer checks, workspace Clippy, cargo-deny, formatting, project guards and
  English/Portuguese translation source marks pass. Audio/voice code was unchanged in this step;
  the full workspace test/eval matrix remains required before a commit.
- Needs: V1.4, V1.6.

**V1.9 Clipboard lease and X11 paste** (M) — implemented; desktop acceptance pending
- `platform::insert` implements native X11 `Target`, clipboard `Lease` and `Pasted | Copied`
  outcomes; a `Result` error is Failed. The Settings-only switch is off by default (schema 24),
  with the typing capability, text-only restoration and clipboard-history limits explained.
  This first scope allows KWrite, GNOME Text Editor, Mousepad and Leafpad. Unknown apps, browsers,
  IDEs, every terminal and Wayland stay copy-only; expanding field/pane safety is V1.11.
- Before queuing capture, the shortcut callback synchronously captures a separate capability:
  EWMH identity, exact keyboard-focus child, class, same-user process and process start time.
  Its X11 connection observes focus loss/root-active changes through delivery, including
  away-and-back. V1.7's asynchronous context cannot authorize input. After the 40 ms clipboard
  settling delay, a short RAII server grab covers target/clipboard checks and Ctrl+V down/up.
  Only an explicitly native X11 environment is eligible; unknown environments copy. Identified
  Caps/Num locks are allowed, while held keys, latched modifiers, alternate XKB groups or unsupported
  modifier mappings yield Copied. It never raises a window, releases
  the user's modifiers or sends Enter. Cancellation and dispatch share the existing ticket lock;
  switching insertion off revokes that ticket before saving. Reading the previous owner happens
  before taking this lock, so a slow clipboard cannot stall cancellation. An abandoned preparation
  never claims the clipboard. Copy again never gets a capability.
- Keep the existing arboard owner and native Wayland clipboard backend. A temporary in-memory
  X11 owner serves UTF-8 text and Klipper's history-exclusion hint. It is necessary because
  arboard's separate connection cannot atomically compare ownership and restore. After a paste,
  the same lease connection checks ownership and restores under a short server grab at about
  700 ms. Unique owner windows prevent stale restores, including a new copy with equal text.
  XFixes also detects a same-owner copy racing the previous-text read. Overlapping leases inherit
  the baseline, never an older transcript. Failed targeting leaves a permanent Copied result;
  its prior text is forgotten. Text over 64 KiB uses ordinary copying; images/rich formats cannot
  be restored. No clipboard contents are written to files, history or logs.
- Checks: the explicit native private-Xvfb child smoke exercises a foreground client's Ctrl+V
  events and independent clipboard requests, Unicode/multiline text, restoration, equal-text
  newer copies, same-owner read races, overlapping leases, focus away-and-back, held modifiers,
  changed clipboard, a closed target, terminal/IDE fallback and size/NUL rejection. Real worker
  tests check Pasted/expiry and existing cancellation/overlap behavior; renderer checks cover
  localized inert Pasted feedback and expiry. These are not real desktop/portal/microphone
  acceptance. Running-app visual validation is pending: the session exposes no controllable UI.
- Validation: 189 app/platform/dictation tests, the explicit private-Xvfb insertion smoke, the UI
  build, six renderer checks and workspace Clippy pass. The full workspace test/eval matrix remains
  required before a commit.
- Outcome labels and the provisional General > Keyboard shortcuts opt-in are translated into
  all four app languages. The full Dictation Settings owner arrives in V1.14.
- Deviation from the original plan: no XWayland keyboard bypass and no automatic terminal
  Ctrl+Shift+V. ADR 0032 requires the Wayland keyboard grant (V1.10) and exact session/pane safety
  (V1.11), regardless of terminal class or title. The native X11 desktop matrix stays open.
- Needs: V1.6, V1.7.

**V1.10 Wayland paste through the RemoteDesktop portal** (M) — implemented; real desktop acceptance pending
- `platform::insert::Portal` owns a keyboard-only RemoteDesktop session with ashpd. Settings
  explicitly enables the ability to type into other apps, with requesting/ready/temporary/unavailable
  feedback and an explicit retry. No pointer, screencast, EIS or XWayland input bypass is used.
  ashpd also needs its screencast feature to compile the response's Stream type; no screen API is called.
  A refusal, missing/revoked grant, input error or timeout yields Copied and never reopens a dialog
  on the next dictation. Separate down/up calls release both keys after an error; a faulty virtual
  keyboard is closed. Expired paste requests cannot execute later from the queue.
- The app owns the single-use restore token through `Secret::DictationKeyboard`: get/delete before
  restore, rotate after the grant, never return it to a webview or write it to settings/logs/files.
  Keyring failure requests a nonpersistent session. Startup only attempts restoration with a token
  it can consume; otherwise Settings must request access. Disabling invalidates the portal epoch
  before cancellation or disk IO and removes the token. Token operations are serialized and epoch
  checked so a late grant cannot recreate a token after disabling. A removal failure keeps insertion
  off, reports the error without rolling the setting back, and offers Retry token removal. Revocation
  also purges the saved token. Clean exit awaits portal/monitor cleanup while retaining a usable token.
- On Plasma Wayland, an already-running read-only KWin monitor publishes bounded app identity and a
  monotonic activation sequence. Key-down captures that snapshot; a nonce-matched compositor
  checkpoint verifies the same sequence/window/process immediately before delivery. Away-and-back
  changes the sequence; a title is never used. Only the actual KWin bus owner may send data or
  consume checkpoint requests. Vults window focus also blocks capture and invalidates older targets,
  including layer surfaces; focus tracking is per window and removes destroyed windows. The same
  plain-editor allowlist applies. Terminals, unknown apps/desktops and sessions remain copy-only.
- A native `ext-data-control` lease serves in-memory UTF-8 and Klipper's exclusion hint. The same
  64 KiB bound applies; multiple clipboard seats fall back to copying. Restoration changes only
  this source's bytes, without reclaiming selection, so a new owner's copy wins even before its
  cancellation event arrives. Nonblocking bounded transfers prevent a stalled reader from blocking
  cancellation/ownership checks. Overlapping leases retain the baseline; Copied keeps its text.
- Tests: the opt-in native smoke creates its own headless KWin, D-Bus session and GTK editor windows,
  never the user's compositor. It checks large Unicode transfers, restoration, equal-text new
  copies, overlapping leases, same-app window changes/return and live focus checkpoints. Only the
  RemoteDesktop boundary is simulated: real ashpd/zbus calls verify keyboard-only/persistence flags,
  token rotation, reuse across dictations, Ctrl+V down/up without Enter, revocation, refusal without
  retry, cancellation during consent, temporary access without persistence and Vults focus invalidation.
  A persistent in-memory credential provider exercises the production secret get/set/delete path.
- Validation: 192 app/platform/dictation/secrets tests pass, plus the explicit Xvfb and headless-KWin
  smokes. The UI build, six renderer checks, workspace Clippy, cargo-deny, formatting, project guards
  and translation source marks pass.
- Native desktop consent, actual editor insertion through the real portal, microphone/Zeca on/off
  and running-app visual acceptance remain open. CUA exposes no app/browser surface in this session.
  The full workspace test/eval matrix is still required before a commit.
- Needs: V1.9. Next: V1.11, exact safe session/pane delivery.

**V1.11 Delivery into flock sessions** (M; supported implementation complete, native acceptance open)
- Implemented: the off-by-default insertion switch captures an exact attached tmux pane/client
  during the start-key callback, tied to same-user socket identity, agent/pane/client start times and
  ancestry. XTerm/UXTerm has a per-window process; Kitty additionally proves its focused OS window,
  tab/pane through an already-enabled local Unix socket query (`kitten` CLI), and its foreground
  tmux client through the pane child ancestry and shared PTY.
- A synchronous X11 target or preexisting KWin/portal capability pins the window at key-down.
  Native pane/focus revalidation precedes a delivery-time request to the core's live loop. A pending
  permission, question or any card (including deferred cards and parallel tool activity) prevents
  delivery and produces Copied/waiting feedback. Missing/stale state fails closed. No dictation waits
  for a card answer to resume delivery; Copy again has no insertion capability.
- The tmux server rechecks attached client, pane process, active window/pane, copy mode and liveness
  immediately before `send-keys -H`. Payloads become UTF-8 hex bytes, not shell or tmux syntax. A synchronous
  server acknowledgement is required for In session. Single control-free lines up to 8 KiB are
  supported; embedded CR/LF, tabs, escapes and larger text use verified ordinary copying.
  Successful session delivery does not touch the clipboard. No Enter or auto-send was added.
- Wayland still requires the existing keyboard grant; refusal/revocation cannot bypass it through
  terminal IPC. Unknown/ambiguous/shared terminal hosts stay copy-only, as required by ADR 0032;
  the original generic-paste fallback below an unknown terminal is intentionally not implemented.
- Direct Kitty remains copy-only: its upstream send-text implementation explicitly disallows
  responses and documents success even when no text was delivered. A local CLI exit cannot prove
  delivery. The 2026-10-17 WezTerm API review also keeps direct delivery copy-only: its copy/search
  overlay returns the underlying pane id, so CLI pane/client data cannot establish input ownership.
  These are the clipboard fallbacks required by ADR 0032, not unverified send commands. Future
  direct transports need an acknowledged write plus an exact input target before they can be enabled.
- WezTerm navigation now captures `WEZTERM_UNIX_SOCKET` alongside `WEZTERM_PANE` in the hook.
  Open terminal pins that same-user local socket with `--skip-config` and `--no-auto-start`, clears
  ambient pane selection and refuses missing, unsafe, symlinked or disappeared sockets. It never
  discovers another instance. Core's availability hint requires socket metadata too; supported
  desktop window activation remains an independent fallback. Navigation grants no typing capability.
- Vults Chat retains its current unsent-draft voice path. Former Lab/Quick task-run composers were
  removed in N0 (ADR 0031); they are not recreated for dictation.
- Checks: pure core waiting/deferred/question/identity/lock guard; terminal payload and Kitty
  focus parsers; worker session/held outcomes, expiry, last-text retention and stale cancellation;
  isolated native tmux/PTY and real Kitty/Xvfb transport, outer/inner pane changes, copy mode,
  client detachment and process identity; existing native X11/Wayland insertion checks and
  translated renderer state checks. Actual desktop/microphone/visual acceptance remains open.
- Validation (2026-10-17): 385 library tests passed (app 148, core 205, platform 32), with
  five opt-in native tests excluded from that normal run. Explicit native smokes passed for tmux
  3.8/PTY, real Kitty on private Xvfb (`VULTS_TEST_KITTY=1`), X11 paste/clipboard/focus and private
  KWin/Wayland, including the new read-only portal checkpoint. The RemoteDesktop grant is still
  simulated in that private test; it does not claim real desktop consent. Build, six renderer
  checks, workspace Clippy, formatting, repository guards and reviewed translation source marks
  pass. CUA still exposes no app/browser surface; the app remains stopped. Full workspace tests
  and evals remain required before committing this accumulated N3 work.
- WezTerm follow-up validation (2026-10-17): 255 library tests passed (core 205, hook 16,
  platform 34; six native tests excluded), plus all 11 hook round-trip checks. An explicit private
  Xvfb/D-Bus test used two real WezTerm instances with colliding pane ids: only the captured
  instance changed, and a missing/disappeared socket never selected or started a replacement.
  The portable official CLI release was 20240203-110809-5046fc22, checked against its published
  SHA-256 (`142cb46de648200b852cc4b1d932e6b8f2a9d3c55f6dc5af44b6d4eeb9385f5c`);
  it was not installed system-wide. Build, workspace Clippy, Flock/dictation renderer checks,
  formatting, repository guards and reviewed translation source marks passed. No Vults UI changed;
  the actual app stayed stopped, and actual desktop/microphone acceptance remains separate.
- The 2026-10-17 API review settles direct Kitty/WezTerm as guarded clipboard outcomes under
  ADR 0032. Implementation is ready to proceed to V1.12's island states; actual native desktop
  consent, microphone and visual acceptance remain open. No full native acceptance is claimed.
- Needs: V1.9, V1.10, V0.2.

**V1.12 Island dictation states** (M)
- In progress (2026-10-17): the first production slice replaces the body overlay on Mini/Quick
  with a reserved neutral row above the flock. Each existing runtime phase has a production icon
  and translated sentence. Waiting cards (including deferred cards), screen lock and Nest
  ownership suppress feedback without extending its expiry. Quick reserves row height before
  zoom; the mapped native overlay is unchanged. Renderer ownership checks exercise every phase
  and pt-BR/es/zh; geometry checks cover viewport limits and both dock anchors. These are DOM
  ownership and computed-geometry checks, not native font/overflow or visual acceptance.
  The next slice connects audio's nine measured bands and the shared GPU-only local Preview
  to the same ordered status/snapshot path. Hands-free preserves the live frame; ending capture
  clears preview text and bands. Cancellation, a new generation and later phases reject old
  callbacks; starting a new generation clears the old snapshot immediately. Meter-only frames
  retain DOM/phase icons and do not remount the row or rewrite unchanged accessible text.
  Reduced motion hides moving bands. Preview remains in memory and never enters logs/history.
  All 174 selected app/audio/voice tests pass with the installed Base/Silero models and speech
  fixture, including preview cancellation; the UI build, renderer checks and workspace Clippy pass.
  Conditional captions now expand clipped lines or display the confirmed delivery's text in two
  rows, outside the pointer region. They follow the existing synchronous frame/anchor measurements,
  yield when the overlay has no room and clear at the original two-second completion deadline.
  Backend snapshots strip expired caption text; settings/ownership/card changes cannot renew it.
  A version-25 setting defaults on (as in the accepted prototype); its only current UI owner is
  General > Keyboard shortcuts until V1.15 moves it into Settings > Dictation. Initial settings
  reads cannot overwrite a newer toggle. The shared Nest/Settings feedback owns its caption there.
  Nest/Settings reserve its measured height before the footer/composer; a Chat card arriving
  before core's next view also suppresses feedback immediately. All 151 app tests, the renderer
  checks, UI build and workspace Clippy pass. Native visual acceptance remains separate.
  At this stage heard and written text are the same local transcript, so the caption shows it
  once. Confirmed delivery states now carry a Unicode word total from their committed text,
  rendered with localized singular/plural labels. The same count survives caption expiry;
  active, failed and cancelled states never reuse an earlier total. This reuses the existing
  locked unicode-segmentation version, with no new transitive packages. Pure fixtures cover
  punctuation, emoji, apostrophes, decimal numbers, combining marks, Unicode spaces and unspaced
  Han text (one unit per ideograph). All 168 app/dictation tests and cargo-deny pass. Correction
  totals await actual replacement/clean-up measurements in V2/V3; there is no fabricated zero.
  The optional start cue now reuses sound.ts and the existing quiet peek sound. The persistent
  island is its only audio owner, including while Nest/Settings show the feedback. A new live
  generation plays once; snapshots, meter frames, surface/setting changes and muted starts
  cannot replay it. Global mute, volume and Do Not Disturb apply. Schema 26 keeps the explicit
  sound choice; General > Keyboard shortcuts owns the switch until V1.15.
  The maintainer accepted the previous UI slices in the running app (2026-10-17). The new
  sound still needs native listening acceptance; the app remains closed as requested. All 169
  selected app/dictation tests, renderer checks, UI build, workspace Clippy and repository
  guards pass. No full-workspace test/eval run or CI result is claimed for this slice.
  Running-app X11/Wayland review remains required; no computer-use surface is available.
- Build: in `ui/src/island/` (`fsm.ts`, `render.ts`, new `dictation.ts`), a grey line over the flock (R8),
  only while a dictation is live:
  - listening/locked (bands + partial text from the existing preview), working, done (Pasted · app · words ·
    fixes), session (In session · app), held (the session waits on a card: Copied, pointing to the card),
    clipboard ("Ctrl+V to paste"), retry (plan B), failed (no sound → Settings › Dictation);
  - no idle hint and no buttons: the key is shown in Settings, and a click still opens Compact;
  - the caption under the island (R8): what was heard, dim, over the result in two rows at most; shown only
    when it adds something (heard text, a cut line, live words that no longer fit); no buttons, clicks pass
    through; it leaves 2 s after the dictation ends; a switch in Settings › Dictation › Voice;
  - *Cancelled* (V1.17);
  - an optional start sound through `sound.ts`;
  - cards always win: a card arriving mid-dictation shows (D5).
- UI: prototype island states → production.
- Tests: `scripts/check-island-geometry.mjs` stays green; a new `scripts/check-dictation-island.mjs` steps
  each state at Mini and Quick widths and checks no overflow and a translated text.
- Done when: the user reviews it in the running app on X11 and Wayland.
- Needs: V1.6, V0.9.
- Q5 settled: no "Undo" on the island (it would need a synthetic Ctrl+Z, and the island has no buttons).

**V1.13 History store** (S)
- Store implementation complete (2026-10-17): migration 21 adds text-only dictations and
  independent daily/app totals. The store accepts Not kept, 7/30/365 days and Always (default
  30 days), pruning on its history-open entry point and before each record. Zero-word results
  add nothing; positive-word totals are kept even when the text is not retained. Reads are
  bounded and deterministic; correction counts remain nullable until measured. Delete, prune,
  clear and the general Clear history remove transcript rows without reducing usage totals.
  Erasure reuses the store's compaction/checkpoint path. A new transcript and its totals commit
  together; ids never return after deletion. Tests cover an upgrade from a schema-20 fixture,
  exact age boundaries, Not kept disk contents, erasure, ordering and failed-write rollback.
  This task supplies the storage API only: the live worker still writes no dictation history.
  V1.14/V1.15 will connect it with the History page and visible privacy controls before text
  persistence is enabled; Copy again continues using the existing in-memory last text.
  Validation: 246 selected app/store/Zeca tests (including a compile-fail doc test), UI build,
  workspace Clippy and repository guards pass. No renderer code changed and the app stays
  closed; no full-workspace test/eval run or CI result is claimed for this task.
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
- Implemented (2026-10-17), native acceptance pending: Dictation is the fifth Nest tab and a
  palette destination, including with Zeca off. Its three-group side list uses the shared
  resizable/foldable panel; only History and its Settings link are active in this phase.
  History has explicit loading, empty, not-kept, filtered-empty, stale/error and retry states,
  local search over the newest 500 retained rows, date grouping, heard/written text and
  clipboard success/failure feedback. Delete/clear require confirmation; erased cached text
  cannot return through a late read. Today's words and audio time are measured; saved time is
  an explicit 45-wpm typing estimate and sub-second speed is unknown. Corrections stay nullable.
  The minimum privacy selector lands early in General > Keyboard shortcuts (schema 27,
  default 30 days), so real worker history is usable now. V1.15 moves it into the dedicated
  category. Successful captures record once; cancelled/failed/empty turns and Copy again do
  not create transcript rows. Erase/retention changes serialize against writes and invalidate
  in-flight older history tickets. No audio or prior clipboard is stored. Use again asks the
  existing copy-only talk-again path to remember/copy the selected retained text; it never types
  into the foreground Nest or uses an old insertion target. Deleted/expired and busy jobs reject it.
  The maintainer also requested a slimmer composer in this task: Nest and Quick now share a
  single writing row with attachment left, microphone/Send right and files above it. Draft,
  IME, key, transcript, voice and permission owners are unchanged.
  Native follow-up (2026-10-17): the maintainer found Dictation leaking below Chat. Its later
  flex rule tied the shared hidden selector and won by source order; the tab owner now has
  stronger scope, protecting every pane. A regression reproduces the CSS contract failure,
  and navigation checks open all five tabs with exactly one pane shown and History loading
  only when selected. The native app receives the CSS correction through Vite HMR.
  Validation: 220 selected app/store tests, the UI build, workspace Clippy, nine renderer/
  continuity checks and repository guards pass. The native development app is launched for
  the maintainer's combined History and composer review; no visual acceptance or full X11/
  Wayland matrix is claimed. No full-workspace test/eval run or CI result is claimed here.
- Build:
  - `ui/src/surfaces/nest/dictation/` with `area.ts` (the side list in three groups, as in the prototype)
    and `history.ts`;
  - figures for today (words, time saved, words per minute), grouped by day, search, heard vs written, copy,
    paste again (`talk-again`), delete;
  - "Not kept" empty state;
  - the `nav-dictation` tab in the Nest head; Tauri commands `dictation_history`, `dictation_delete`,
    `dictation_clear`, `dictation_paste_again`.
- UI: prototype History → production.
- Tests: a renderer check script on production components (empty, populated, long values, filtered empty,
  read error); i18n check.
- Needs: V1.13, V0.3, V0.9.

**V1.15 Settings › Dictation** (M)
- In progress (2026-10-17), category/ownership slice: Settings has a dedicated Dictation page
  with Keyboard shortcuts, Voice, Paste and Privacy groups. The actual bound talk/copy/cancel
  keys, existing caption/start-sound switches, guarded insertion/portal controls and retention
  move here. History and General link here; Chat & voice links here and remains the sole model/
  language editor. Privacy reuses confirmed history erasure, retains pending state across
  redraw/navigation, offers failure/retry, and never resets the independent usage totals.
  Moving the controls adds no persisted key or default change (schema stays 27). An insertion
  setting arriving before the initial snapshot now keeps its newer value. Renderer fixtures
  cover all eleven pages, native permission events, no implicit permission requests, current
  shortcut labels, central routes and erase cancellation/failure/retry/duplicate prevention.
  The UI build, all nine renderer/continuity checks, formatting and repository guards pass.
  No backend behavior, dependency or persisted key changed in this slice; a new Rust test/
  eval or CI run is not claimed. The app remains closed at the maintainer's request; native
  visual acceptance is pending.
  The shortcut-opener slice adds an explicit Settings-only **Change in KDE…** action through
  the installed `kcm_keys.desktop` entry (no shell or binding writes). Missing entries and
  launch failures surface retry feedback; pending launches survive navigation and redraw
  without duplicate requests. The bound-key portal remains the sole source of current keys.
  The UI build, Settings and surface-continuity renderer checks, three platform shortcut tests,
  Settings-only command guard test, targeted app/platform Clippy, formatting, i18n and repository
  guards pass. Tests launch only a harmless fixture executable, never the native settings window.
  Native visual acceptance is pending while the app stays closed.
  The gesture-preference slice adds schema-28 `dictation_double_tap`, `dictation_space_lock`
  and `dictation_escape_cancel`, all defaulting on. Settings owns the mode selector and raw-key
  switches; Wayland shows the latter disabled with their X11 requirement. Hold only discards
  short taps immediately; Space-lock remains independent. Disabling Space treats it as ordinary
  typing and stops reserving Open island; disabling Esc never disables the global Cancel voice
  shortcut. The worker snapshots options at the next fresh press, preserving active recording,
  capture tail and transcription behavior. A failed save cannot publish or apply a new choice.
  Migration/round-trip, malformed fields, Settings-only authority, gesture boundaries, snapshot
  changes, disabled Esc during tail/transcription and renderer save/rollback/retry are covered.
  The UI build, four relevant renderer checks, 177 app/dictation tests, targeted Clippy,
  formatting, translations and repository guards pass. No full workspace/eval or CI run is claimed.
  Native visual acceptance stays pending while the app remains closed.
  The microphone slice adds optional schema-29 `dictation_microphone`; absent keeps the system
  default. Settings-only inventory lists input names without recording, excludes known monitors,
  deduplicates names and supports explicit refresh. Loading, missing/empty, failed and stale lists
  have feedback. A disconnected saved choice stays selected; capture keeps its existing default/
  other-input fallback only when a name is missing, never rewriting the preference. Listed inputs
  that cannot open still fail visibly. Captures keep their starting input, and Chat keeps its own
  system default. Pending saves block inventory refresh, and failed saves keep the old preference.
  Tests cover migration/round-trip, validation, capture snapshots, input filtering/fallback,
  Settings-only commands and renderer loading/failure/stale/empty/retry/save/rollback behavior.
  The UI build, Settings and surface-continuity checks, 166 app/audio tests and targeted Clippy
  pass; actual microphone and native visual acceptance remain pending with the app closed.
  The processing slice adds schema-30 `dictation_cut_repeats` and `dictation_retry_base`, both
  defaulting on. Settings-only, independently saved switches apply to the next recording.
  Preview and final decode receive the captured loop choice per call, never through the shared
  model cache; Chat keeps its existing filtering. Disabling loop removal preserves repetitions
  while sound annotations, standalone silence phrases and audio speech gates remain filtered.
  Disabling Base fallback skips its extra speech detection and second decode; enabled fallback
  still requires empty text, detected speech and a different installed Base, with cancellation
  checks and visible retry feedback. No model is downloaded. Failed saves cannot apply a choice.
  Migration/round-trip, invalid fields/keys, Settings authority, enabled/disabled loop behavior,
  skipped retries/error propagation and renderer late-settings/pending/rollback/retry are covered.
  The UI build, Settings and surface-continuity checks, 183 app/voice tests and targeted Clippy
  pass. These do not establish a real model/microphone or native visual pass while the app is closed.
  Remaining V1.15 work: native acceptance and feature-owned switches as their runtimes land
  (delivery options and meetings; spoken code lands in V1.16 below). No inert toggle is shown as a working feature,
  and this slice does not mark all of V1.15 complete.
- Build: settings keys and a Settings category (R6), linked from the area's side list:
  - key: bound key shown, "Change in KDE…" opens System Settings › Shortcuts;
  - mode (hold + double tap / hold only), Space lock, Esc cancel, paste again key;
  - agents: deliver into session, send automatically, spoken code (V1.16);
  - voice: mic, model (shared with chat), language, start sound, cut repeats, plan B;
  - paste: terminals with Ctrl+Shift+V, give back the clipboard;
  - meetings: summary and to-dos (V5.6);
  - privacy: retention, erase.
- The voice model and its language stay in Settings › Chat & voice, shared with the chat; Chat & voice links
  here.
- Tests: an H4 fixture for the new keys; renderer check.
- Needs: V1.14.

**V1.16 Spoken code in terminals and sessions** (S)
- Implemented (2026-10-17); native acceptance pending. A pure, original `text::spoken_code`
  converter handles identifier neighbors and English/Portuguese/Spanish aliases, with longest
  multiword terms first. It preserves case, existing punctuation and untouched whitespace/control
  bytes; it neither adds sentence punctuation nor joins across tabs or line breaks. Reserved marker
  words cannot be mistaken for identifier chunks. This is literal punctuation syntax, not prose
  interpretation. Unknown targets and prose apps bypass conversion.
  The final dictation path uses its captured terminal/session context and schema-31
  `dictation_spoken_code` choice (on, matching the prototype); Settings > Dictation > Terminals &
  sessions owns the switch. Its saved choice applies to the next recording. Conversion never
  grants insertion, changes waiting-session checks or submits text; clipboard fallback retains
  the converted result. Replay bypasses the conversion path. Retained history keeps pre-conversion
  heard text and written text, with `spoken-code` style only when they differ; erasure, retention
  and stale-generation checks keep their existing owners.
  Tables cover three languages, case, Unicode, boundaries, controls and stable converted text;
  app tests cover terminal/session/prose/unknown/disabled contexts, heard/written history and late
  cancellation. Migration and renderer tests cover defaults, explicit choice, late snapshots and
  save failure/retry. The build, three relevant renderer checks, 184 app/dictation tests, targeted
  Clippy, formatting and repository guards pass. The app remains closed; real speech and native
  visual acceptance are not claimed.
- Build: `dictation::text::spoken_code`, applied only when the target is a terminal or a session:
  - "ponto/dot", "barra/slash", "underline/underscore", "traço/dash", "dois pontos/colon" → `. / _ - :` <!-- check-english:allow -->
    inside identifiers;
  - no forced final period, no auto-capital.
- Port: idea from [VS] 227-236 (AGPL, rewrite) and [VB] 51.
- Tests: pt/en/es tables; prose apps unchanged.
- Needs: V1.7.

**V1.17 Cancel by voice** (S)
- Implemented (2026-10-17); native acceptance pending. Pure whole/trailing phrase matching
  covers English/Portuguese/Spanish, case, extra whitespace, acute accents (including decomposed
  accents) and final sentence punctuation. A suffix needs a whitespace boundary; identifiers,
  quoted literals and `cancel the meeting at noon` stay text. A literal trailing phrase cancels
  even in a spoken-code identifier; the UI explains turning the option off to dictate that text.
  Schema-32 `dictation_cancel_by_voice` defaults on. Settings > Dictation > Keyboard shortcuts
  owns its Settings-only saved switch; recording-start configuration snapshots keep active turns
  stable. The final text pipeline checks the phrase before code conversion or any delivery and
  returns an explicit discard result. The worker reports `cancelled`, releases its listener and
  capabilities, drops history measurement and leaves the last successful dictation intact.
  Copy again/History replay bypass the phrase check, and Chat's draft voice path is unchanged.
  No clipboard write, insertion, submission, history row or aggregate is produced for a match.
  Phrase tables, migration, worker delivery suppression, history removal, resource release,
  mid-recording setting changes and literal replay pass; renderer checks cover the Wayland-enabled
  switch, late snapshots and save rollback/retry. The build, four relevant renderer checks,
  187 app/dictation tests and targeted Clippy pass. No real-speech or native visual acceptance is
  claimed while the app remains closed.
- Build: when the final transcript is only, or ends with, a cancel phrase ("cancela", "deixa pra lá", <!-- check-english:allow -->
  "cancel", "never mind", "déjalo"; normalised, whole phrase) nothing is pasted, the island says <!-- check-english:allow -->
  *Cancelled* and History keeps nothing. A switch in Settings › Dictation › Key, on by default.
- It works the same on X11 and Wayland, so it covers what Q1 cannot: Esc is not bindable on Wayland.
- Port: the cancel phrases from [VA] 3.3 (rewritten, pt/en/es tables).
- Tests: phrase table; "cancel the meeting at noon" is pasted (only a trailing or whole phrase cancels).
- Needs: V1.6.

**V1.18 Music while you speak** (S) — implemented; maintainer validation reported 2026-10-17
- Settings > Dictation > Voice saves Lower to 25% (default), Pause it or Leave it in schema 33.
  Each independent recording owns a platform MPRIS guard: microphone capture never waits for the
  bus, capture end/drop/cancel restores each unchanged player, and rapid recordings serialize
  restoration before taking another snapshot. Quieter and already-paused players stay untouched.
  Unique bus owners prevent writes to restarted players; property signals preserve observed user
  changes even when they return to the temporary value. Reads bypass property caches/GetAll and
  never request track metadata. Leave it opens no connection. This is separate from Now playing;
  Chat and future meeting capture never enter this owner (V5 must preserve that boundary).
- Private D-Bus tests cover per-player restoration, cancellation, quiet/paused players, no metadata
  reads, user changes returning to the temporary value, playback changes, crashes and replacements.
  Settings migration, strict enum input, Settings-only authority and renderer save/repaint/rollback
  checks are covered. The build, 228 affected-package tests (six native opt-in tests ignored),
  targeted Clippy, four renderer checks, repository guards and dependency audit pass.
  The maintainer subsequently reported local validation and authorized V1.19 on 2026-10-17.
  The desktop/player combination was not specified; this does not fill the entire V1 smoke matrix.
- Build: on listening, lower MPRIS players to 25 % (or pause them) and give each back its own volume or
  playback state at the end; not while a meeting records; a player whose volume the user changed meanwhile
  is left alone. A choice in Settings › Dictation › Voice (Lower to 25 %, Pause it, Leave it).
- Port: idea from [VA] 3.4; MPRIS through `zbus` (already a dependency of `platform`), in `platform`.
- Tests: a fake MPRIS player on a private D-Bus; restore after cancel and after a crash of the player.
- Needs: V1.6.

**V1.19 V1 docs and smoke** (S) — documentation/regression checkpoint 2026-10-17; full desktop matrix open
- The English/pt-BR guide now describes implemented History, gestures, microphone/processing,
  music and feedback, and separates working controls from later phases. Its destination table
  makes Firefox, Kate and Konsole manual-paste outcomes explicit; they do not gain insertion
  from application recognition. Settings documents each current group and its update timing.
  Architecture includes the independent capture/preview/transcription/delivery/history flow,
  native capability boundaries and capture-scoped local media control. CHANGELOG records the
  checkpoint without presenting a prototype feature as shipped.
- A complete workspace run exposed the pre-existing media watcher test choosing a real desktop
  player instead of its fixture. It now executes in a private D-Bus child, exercises the actual
  watch/control entry points and no longer skips silently when the desktop has no session bus.
  Production media behavior is unchanged.
- Verification (2026-10-17): UI build, workspace Clippy/tests and evals pass. The workspace run
  supplied installed Base/Silero models and the existing JFK fixture: VAD boundaries, real decode
  and preview cancellation ran instead of silently skipping. Ten available renderer checks pass
  (geometry, record search, Nest close, Chat models, flock placement, project search, Settings,
  surface continuity, dictation History and island/status). Explicit native tests pass for XI2,
  X11 insertion/clipboard, private KWin/Wayland focus and leases, tmux/PTY and real Kitty/Xvfb.
  Repository language/brand/i18n/layer/network/version/README guards, formatting and reviewed
  English/pt-BR source marks pass. The private Wayland portal activation logged a missing-bus
  message during shutdown after both test processes passed; no real desktop consent was tested.
- Maintainer acceptance: V1.18 accepted locally on 2026-10-17 (desktop/player unspecified).
  No desktop combination below is inferred from an automated fixture or from that broad report.

| Smoke dimension | Required observations | Automated evidence | Desktop status |
| --- | --- | --- | --- |
| Native X11 + Firefox/Kate/Konsole | Honest Copied, manual paste, no Enter, no focus activation | Eligibility tables reject these classes; worker delivery/fallback and clipboard checks | Full microphone/app combination unreported |
| Plasma Wayland + Firefox/Kate/Konsole | Same copy path; absent clipboard protocol reports failure | Private KWin clipboard leases/focus; fallback and renderer state checks | Full microphone/app combination unreported |
| X11/Plasma + tracked tmux in XTerm/Kitty | Exact pane only; pending/deferred card, focus or pane change keeps Copied | Core/app readiness tests, private tmux/PTY and real Kitty transport, X11 and private Wayland focus checks | Full microphone/desktop combination unreported; portal grant is simulated in the isolated test |
| Focused Vults Chat, Zeca on | Hold fills unsent draft; independent spoken-cancel/music options do not apply | Shortcut ownership, existing Chat voice path and renderer guards | Full microphone/desktop combination unreported |
| External apps or Nest/island, Zeca off | Independent dictation and History remain usable | Router and renderer Zeca-off cases | Full microphone/desktop combination unreported |
| Hold/double tap, Zeca on/off | Hold/release and locked capture; actual remapped chords; X11 Space/Esc only | Pure key transitions, worker lifetime/races, private XI2 raw input | Desktop shortcut consent/remapping unreported |
| Global cancel and spoken cancel, Zeca on/off | Discard before delivery/history, release resources, keep previous success | Phrase tables, worker races/replay/history tests | Real-speech combinations unreported |
| Music playing, Zeca on/off | Per-player lower/pause/leave; restore on stop/cancel without overriding user changes | Private MPRIS player, cancellation, crash/restart and overlap tests | V1.18 accepted by maintainer; exact desktop/player unreported |

For each applicable external destination, exercise hold, double tap, global cancel, spoken cancel
and music while Zeca is on, then off, on both native X11 and Plasma Wayland. In focused Chat
with Zeca on, verify the draft behavior instead of independent dictation gestures/processing;
with Zeca off, use the Nest/island's independent route. Include insertion off/on, a waiting
session, a focus change away and back, a newer clipboard copy, narrow/short layouts and a failed
or revoked portal grant. Record actual desktop, destination, gesture and outcome before marking
that cell accepted. Isolated D-Bus/Xvfb/KWin tests are not full running-app acceptance.

Reproducible local regression commands (the opt-in tests below create private desktops/PTYs):

```sh
npm run build
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p vults-evals --features evals
cargo test -p vults-platform private_x11_ -- --ignored --nocapture
cargo test -p vults-platform private_wayland_ -- --ignored --nocapture
cargo test -p vults-platform private_tmux_ -- --ignored --nocapture
VULTS_TEST_KITTY=1 cargo test -p vults-platform private_tmux_ -- --ignored --nocapture
```

Set `VOICE_TEST_MODEL`, `VOICE_TEST_VAD` and `VOICE_TEST_WAV` to existing local Base, Silero and
16 kHz mono speech-fixture paths to exercise model-backed tests in the workspace run; they do
not download models. The generic ignored clipboard test uses the user's session and is not
part of the commands above. `scripts/check-task-actions.mjs` is named by the supplied AGENTS
checklist but is absent from this checkout; it cannot be reported as passed or replaced by
an empty check. Existing renderer checks, dictation checks and repository guards are run separately.

- Build: `docs/guide/dictation.md` (+ pt-BR), a settings reference, the architecture diagram, CHANGELOG.
- Smoke matrix: {X11, Wayland} × {Firefox, Kate, Konsole, tmux session, Vults chat} × {hold, double tap,
  cancel, spoken cancel, music playing}, with Zeca on and off.
- Done when: the user signs off V1.

---

## V2 Your words (R1, reopens X5 locally)

**Goal.** Names come out right.

**Outcome.**
- Dictionary, replacements and text snippets apply before paste.
- Whisper is primed with your words and the focused project's names.
- "Try a sentence" shows what changed.

**V2.1 Store and commands** (S) — implemented 2026-10-17; no visible UI or text processing added
- Migration 22 adds dictionary words, ordered replacements and snippets with canonical Unicode
  case keys and database uniqueness. A duplicate word or snippet trigger is rejected; replacement
  upserts keep the existing id/order. Trimming applies only to word/source/trigger boundaries;
  literal bodies retain spaces, newlines and replacement/shell-looking characters. Sources are
  limited to 1024 UTF-8 bytes without controls; bodies to 64 KiB, nonblank and without NUL.
- A transaction reads one coherent Words snapshot. Typed Nest-only Tauri commands list/edit on a
  blocking worker with sanitized database errors; Settings/island/widget callers are refused.
  The shared bridge exposes the typed snapshot/change contract. No renderer page calls it yet.
  Settings remains schema 33; no setting, network, model download or current text flow changes.
- Deletion is idempotent, compacts removed rows and never reuses replacement/snippet ids.
  History retention/clearing preserve these preferences. Tests cover schema-21 upgrade and
  history preservation, Unicode duplicates, ordered upsert, literal round-trip, limits, deletion,
  concurrent duplicates, command serialization, rejected callers and private database errors.
  Validation: 238 app/store tests, targeted Clippy, UI build, three dictation/settings renderer
  checks, dependency audit, formatting, repository guards and reviewed doc source marks pass.
  No renderer or native delivery behavior changed; the app remains stopped.
- Build: tables `dictionary_words(word)`, `dictionary_replacements(id, from, to)`, `snippets(id, trigger,
  text)`; Tauri commands to list, add and delete.
- Duplicate rules:
  - a word that already exists in any case is rejected;
  - a replacement with the same `from` (any case) overwrites;
  - snippet triggers are de-duplicated (the reference allows duplicates, see [DD] 393).
- Port: [DD] 212-224.
- Tests: duplicate rules.

**V2.2 Text pipeline: snippets, replacements, spelling** (S) — implemented 2026-10-18; native acceptance pending
- The pure Rules pipeline protects snippets as opaque typed marks (literal marker text cannot
  collide), applies replacements in saved order, corrects eligible plain dictionary spellings,
  applies existing context-bound spoken-code edits and restores snippet bodies unchanged.
  Replacements insert literal text; later rules can match across previous replacement boundaries.
  No rule is reapplied recursively. Matching uses Unicode case keys, whole-word boundaries and
  interchangeable non-word separators; combining marks stay within words and controls/newlines
  are hard barriers. Dictionary entries need at least two alphanumerics and otherwise only
  alphanumerics, ASCII spaces, hyphens or straight apostrophes. Invalid/empty sources do not match.
- Results retain annotated segments with kind, rule and heard text, including distinct code edits.
  Input/intermediate output is capped at 256 KiB. Exceeding the cap returns an error, not partial
  output. The installed unicode-normalization dependency supplies combining-mark classification;
  no external code or fixture text was copied for this implementation.
- Every independent recording reads one coherent database snapshot before capture. Read failure
  is explicit and sanitized. The final worker checks spoken cancellation on raw text first,
  processes outside its control lock, then rechecks generation before retaining heard/written
  history and the processing style. Replay and Chat do not apply this pipeline. Corrections stay
  unknown until V4's metric contract; no fabricated count is added. Failure never delivers or
  replaces the last successful text. Models, network gates and permission authority are unchanged.
- Original contract tables cover case/separators, partial-word exclusions, Portuguese names,
  literal regex/shell-looking values, dictionary eligibility, Unicode expansion/combining marks,
  opaque/identity snippets, marker collisions, cascading/order, growth limits and code annotations.
  Worker checks cover a mid-recording rules change, replay, cancellation-before-expansion,
  literal expanded cancel text, history styles, staleness, failure and resource release.
  Database-backed tests exercise snapshots before/after saved edits. English/pt-BR docs describe
  current processing; editing pages, Try a sentence and model vocabulary remain later steps.
  Validation: 271 app/dictation/store tests, targeted Clippy, UI build, four renderer checks,
  dependency audit, formatting, repository guards and reviewed translation source marks pass.
  The app remains stopped; actual microphone/destination/visual acceptance is not claimed.
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

**V2.3 Whisper vocabulary** (M) — implemented 2026-10-18; human-speech acceptance pending
- Runtime: Settings schema 34 adds `dictation_vocabulary`, default off, using the existing
  Settings-only processing setter and bridge; its visible control belongs to V2.5. Snapshot the
  switch and dictionary per recording, pass words to preview/final only, and gate a retry by its
  actual model id. Replacement/snippet bodies never reach Whisper. Chat stays on its old path.
- Prompt: use the existing language coding seed plus whole dictionary entries, measured with
  the loaded model's tokenizer and capped at 224 tokens including the seed. Skip empty,
  control-bearing and duplicate entries; stop at the first entry over budget. No cached prompt
  or cross-recording context; empty/invalid dictionaries preserve the previous model prompt.
- Measured offline with all three installed quantized models, Vulkan, 20 original sentences per
  language and one shared 16-name dictionary. eSpeak NG 1.52.0 generated local synthetic speech
  at 155 words/minute; no microphone, model download or app launch. Error counts before/after:
  Base pt 161/121 of 132 words, en 52/39 of 136; Small pt 113/102, en 33/17;
  Turbo pt 63/55, en 19/8. All improved sentence WER, but only Turbo/en kept both digital silence
  and breath-like noise empty through both public and direct-decoder paths. Other pairs remain
  blocked, including automatic language detection. Original coding-seed flags stay unchanged.
  Synthetic Portuguese is particularly poor (WER can exceed 100% with insertions); this is not
  evidence of human-speech accuracy. Exact model/manifest hashes and the reproducible command
  are in `crates/voice/tests/fixtures/README.md`.
- Validation: 228 app/dictation/voice tests passed, plus the ignored real-model WER run across
  all six pairs and a Turbo rerun with the gate assertion enabled. The rerun reproduced the
  measurements and checked matching preview, cancellation, empty-context baseline restoration
  and Chat isolation on the same cached model. Targeted Clippy, renderer build/regressions and
  dependency/language/architecture guards passed. Native microphone acceptance remains open;
  the app stays stopped. V2.4 is next.

Accepted scope:
- Build: `initial_prompt` = the existing `VOCABULARY_*` + dictionary words, capped at about 224 tokens.
- Measure on a 20-sentence fixture set per language (pt, en) with Base/Small/Turbo; enable per model only
  where WER improves and silence fixtures stay empty (prompt-induced hallucination).
- Port: idea [VS] 227-236 (#3).
- Tests: an `#[ignore]`d, env-driven WER test (models not in CI); a unit test of the prompt builder and cap.

**V2.4 Project vocabulary** (M) — implemented 2026-10-18; native acceptance pending
- Runtime: Settings schema 35 adds `dictation_project_vocabulary`, off by default and independent
  of the personal dictionary switch. The Settings-only processing command/public bridge carry
  it; the visible control remains V2.5. With it on, the existing off-loop repository reader
  refreshes the name cache on session start and normal repository refresh (60 seconds, on agent
  events). It does not read project vocabulary while off. No new thread pool, watcher or command.
- Extractor: reuse repository root/branch discovery, parse existing Cargo/npm manifests with
  already-installed JSON/TOML libraries, and collect names rather than code or script bodies.
  Include packages, dependencies, explicit lib/bin names, literal or trailing-`/*` workspace
  members (Cargo/npm), shallow `src` file/directory stems and root file names. Local limits:
  64 KiB per manifest, 64 member directories, 512 entries per directory, 256 deduplicated names
  of at most 128 bytes and 128 cache entries. Linked manifests/member/source directories,
  absolute/parent workspace paths, malformed files and hidden/control-bearing names are skipped.
  More complex workspace globs and recursive module inventories are outside this bounded pass.
- Context: keep working trees separate within a roost; prefer the pinned/focused session cwd.
  Otherwise, a terminal or recognized editor must name a single cached project in its captured
  title. Browser/unknown/ambiguous titles add nothing; missing Git metadata/cache is a no-op.
  Names are context only and never authorize insertion. The generation-bound snapshot freezes
  before the first preview/final pass and rejects late/stale focus updates. Personal dictionary
  entries precede project names, sharing V2.3's 224-token tokenizer cap and Turbo/fixed-English
  gate. Base retries and Chat retain their existing prompts; project names do not become
  dictionary spelling rules or saved preferences.
- Validation: 233 app/dictation/voice tests passed. Small Rust/npm workspace fixtures cover
  package/module/file/branch extraction, no source/script text, deduplication, limits, bad files,
  symlinks, parent/absolute paths, same-name worktree ambiguity and cache refresh/eviction.
  Recording checks cover independent switches, dictionary priority, Base gating, missing context,
  late updates, cancellation and stale generations. Targeted Clippy, renderer build/Settings
  regressions and dependency/language/layer/network guards passed. The TOML parser was already
  in the lockfile; this adds its direct app dependency, with no new package/version. No new WER
  improvement claim is made for project names; native/human acceptance remains pending. The app
  stays stopped. V2.5's editing pages and controls are next.

Accepted scope:
- Build: for the focused session's roost, or for a terminal or editor whose window title names a known
  project (the title read at key-down with the focus, V1.7):
  - package, crate and module names (`Cargo.toml`, `package.json`), top-level file names and the branch;
  - cached per roost and refreshed on a session start;
  - merged into V2.3's prompt under the cap.
- Tests: extractor fixtures (a small repo tree).
- Needs: V2.3, V1.7.

**V2.5 Dictionary and Snippets pages** (M) — implemented 2026-10-18; maintainer acceptance pending
- Runtime: enable the Nest's existing Dictionary/Snippets navigation, retaining selected page,
  drafts, caret and scroll across layout/language changes. Dictionary has word chips and
  ordered replacement rows/forms; saving an existing source updates its stable row. Snippets
  has literal body rows and an add form. Deletion confirms explicitly and supports Escape;
  serialized writes block duplicates, retain failed drafts and reread saved data. Stale reads
  keep their view with explicit retry. User text only reaches text nodes, never HTML.
- Try a sentence: Nest-only `dictation_try`, bounded to 256 KiB, snapshots saved rules on a
  blocking worker and uses the production `Rules::apply` pipeline. It respects saved spoken
  cancellation before rules and, with Terminal context, the saved spoken-code choice. Return
  inert text segments/rule annotations for highlighting. Sentence/rule changes invalidate old
  asynchronous results. No microphone, model, clipboard, history or permission authority.
- Configuration ownership: Help the model with these words and Project vocabulary live only
  in Settings > Dictation > Your words; both remain off by default. Work pages link to Settings.
  Retain V2.3/V2.4's shared 224-token cap, personal-word priority and Turbo/fixed-English gate.
  Explain the synthetic qualification instead of the prototype's obsolete Small-only claim.
- Validation: 233 app/dictation/voice tests passed, including command ownership and real preview
  assertions for replacements/literal snippets/spoken code/cancellation/size limits. The new
  `check-dictation-words.mjs` production-renderer check runs in CI: CRUD, explicit deletion,
  literal bodies, pending/error/retry, draft/scroll retention, stale results, inert text and
  all three translations. Extended Settings checks cover both vocabulary switches, late reads,
  pending redraw and rollback; build, surface continuity, targeted Clippy and guards passed.
  Native Tauri/WebKit was run on Xvfb with isolated config/data/runtime paths: word/replacement/
  snippet saves, four marked rule changes, exact literal whitespace, terminal code, cancellation,
  confirmed deletion/Escape and a persisted Settings switch passed. Black/Light at 1180 x 820
  and 480 x 360 had no horizontal page overflow; lower forms/actions remain reachable by local
  scrolling. Screenshots were inspected. No microphone, real destination or full Wayland matrix
  was validated, and no maintainer acceptance is inferred. The instance is closed and generated
  caches are cleared after checks. V2.6 docs/sign-off is next.

Accepted scope:
- Build: the prototype's two pages under *Your words*:
  - Dictionary: word chips, replacements table, "Help the model with these words" and "Project vocabulary"
    toggles, "How it is applied" and **Try a sentence** through a `dictation_try` command (real pipeline);
  - Snippets: the table and the form for a new one.
- Tests: renderer checks; i18n.
- Needs: V2.2, V0.9.

**V2.6 Docs and sign-off** (S) — documentation/regression checkpoint 2026-10-18; maintainer acceptance pending
- Reconciled the current English/pt-BR dictation and Nest guides, settings reference and
  architecture with V2.1-V2.5. Removed obsolete claims that Dictionary/Snippets and vocabulary
  controls are future features, including Unreleased notes and the current phase summary.
  Historical per-step evidence above retains its original date and scope.
- Added [Check Your words locally](../guide/dictation.md#check-your-words-locally): a concrete
  dictionary/replacement/snippet example, terminal/cancellation preview, explicit deletion and
  microphone/replay checks. Local text rules work with every transcription language and do
  not require either vocabulary switch. Only opt-in Whisper context is restricted to the
  measured Turbo/fixed-English pair; Base/Small, Portuguese and auto keep their old prompts.
- Verification: 303 app/dictation/store/voice tests passed, targeted Clippy and formatting,
  UI build, Your words/Settings/surface-continuity/History/island/status renderer checks,
  brand/English/i18n (1278 sentences)/layers/network/README guards and reviewed translation
  source marks. The ignored WER runner was not rerun at this documentation checkpoint; its
  real-model synthetic results and hashes remain the V2.3 record. No dependency or renderer
  behavior changed. The app stays stopped and generated caches are cleared afterward.

| Acceptance item | Available evidence | Status still requiring maintainer validation |
| --- | --- | --- |
| Storage and clearing | Migration 22, Unicode duplicate rules, ordered upsert, literal round-trip and preference preservation in the current 70 store tests | Review the actual saved entries and history-clearing behavior in the chosen build |
| Words, replacements and snippets | Pure rule tables and worker/database snapshots; V2.5 native Tauri saves with exact whitespace and four marked changes | Dictate through the real microphone and inspect heard/written output; stored rules must survive restarting |
| Sentence preview and deletion | Current production-renderer and Nest-only command tests; V2.5 native terminal/cancel preview, confirmed deletion and Escape | Confirm the example's output and comfortable keyboard use in the user's environment |
| Recording snapshots and replay | Worker tests for rules changed mid-capture, stale generations, cancellation, last success, history styles and literal replay | Real recording plus a mid-capture edit and Copy again in the user's destination |
| Whisper dictionary context | V2.3: 20 synthetic sentences per language/model; only Turbo/en reduced WER and kept silence/noise empty; cached-model/Chat isolation tested | Human speech, pronunciation, microphone/noise and model comparison; no universal accuracy claim |
| Project vocabulary | Rust/npm tree fixtures, bounded name extraction, worktree cache/ambiguity, flags and frozen context checks | Known session/editor/terminal context during real dictation; unsupported/ambiguous targets must add no names |
| Black/Light and narrow UI | V2.5 isolated Tauri/WebKit/Xvfb at 1180 x 820 and 480 x 360, inspected screenshots and no horizontal page overflow | Maintainer's actual compositor/scale, navigation, drafts, scrolling and retry comfort |
| Desktop delivery and permissions | Existing V1 capability guards and regression evidence; V2 adds no insertion/approval authority | V1's still-open X11/Plasma/destination matrix remains separate |

Implementation and the documentation checkpoint are delivered; user sign-off and real-speech
acceptance are not inferred from permission to continue working. Record the actual desktop,
language/model, destination and outcome when acceptance is reported. V2 does not complete N3
or the V1 desktop matrix. The next implementation phase is **V4 Statistics and Diagnostics**,
following the agreed delivery order; V3 Styles follows V8.

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
  - gate feature `dictation-cleanup`, off by default, with 2,000 attempts per rolling 24 hours
    and a content-free trace; local calls obey the same switch and request quota (V0.4);
  - timeouts 4 s (8 s with translation, 12 s with Prompt), then the raw text is pasted;
  - heard text is retained in memory; storing it in History follows its retention setting;
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

**V3.4 Clean-up settings** (M)
- Build: the prototype's *Styles* page: the clean-up switch, off by default (R7), and the four style cards
  (As spoken, Natural, Polished, Prompt; while off they say every line is pasted as spoken). On, it also shows
  the provider, "Write in", agent sessions in English and a collapsed per-app list.
  History shows the style and "in English" when a line was translated.
- Tests: renderer checks.

**V3.5 Docs and sign-off** (S).

---

## V4 Statistics and diagnostics

**V4.1 Stage timings** (S) — implemented 2026-10-18; no new visible controls
- Build: each dictation records release→inserted and its STT, pipeline, clean-up and insert times (columns
  on `dictations`), plus p50/p95 over the last 50.
- Port: definitions from [PT] 262-278.
- Tests: a percentile helper; post-cancel rows excluded.
- Migration 23 adds nullable millisecond timings to retained delivery rows; older rows and the
  not-yet-implemented clean-up stage remain unknown. No content-free timing archive is added:
  erasure/expiry/Not kept remove the samples with their rows; usage totals stay independent.
- The monotonic total begins at the finish request (hold release, hands-free stop or capture
  cap), includes the tail and queue, and ends on confirmed copy/paste/session delivery. STT
  includes model load, waiting and Base retry; pipeline and delivery use their own timers.
  Replay, stale generations, cancellation, failed delivery and empty text create no samples.
- The existing history command exposes row timings and per-stage sample counts/p50/p95, using
  nearest ranks from the newest 50 retained successful deliveries without reading their text.
  Tests cover schema-22 upgrade, unknown/zero values, ranks, the 50-row boundary, invalid input,
  cancelled legacy rows, retention, release-before-capture and cancellation during STT/delivery.
  V4.2 provides the next visible Statistics slice; no diagnostics UI is claimed here.
- Verification: 1,013 workspace tests and six feature-enabled eval tests pass, plus workspace
  Clippy, formatting, UI build and 20 renderer/repository checks. Guide/architecture translations
  and source marks are updated. No renderer behavior or dependency changed; native desktop
  microphone/destination acceptance remains separate. The app stays stopped and generated
  caches are cleared after checks.

**V4.2 Activity › Dictation** (M) — implemented 2026-10-18; maintainer acceptance pending
- Build: a Dictation tab in Activity (R6), linked from the area's side list, and dictated words as a figure
  in Activity's week; week/month/all; words, time saved (`max(0, words/45 − secs/60)`), wpm (0 under 1 s), daily bars,
  top apps. No streaks or records.
- Port: [TLP] 400-403, [DD] 294-303.
- Reuse Activity's chart pieces and the `DESIGN.md` rule "missing measurements are unknown, not zero".
- Tests: math table tests (Rust); renderer check.
- Migration 24 adds daily application-word aggregates in the existing usage transaction. Usage
  survives Not kept and transcript erasure. Legacy lifetime app totals remain exact; missing old
  daily attribution is labeled partial. Statistics use one SQLite read snapshot and no text.
- Activity > Dictation owns 7-day/30-day/all-time figures, daily bars, top eight apps and Settings
  navigation. All-time totals cover every recorded day; the labeled chart is bounded to the latest
  90 calendar days. Missing figures stay unknown; measured sub-second speed follows this step's zero
  convention. A validated presentation-only period preference survives Nest recreation.
- The work area's Statistics button opens that Activity subsection. Its route is retained through
  Room/Full and reopening, including with Zeca off. Week includes dictated words and dictation-only
  weeks even with agent history off; native validation caught and fixed its empty-agent early return.
- Evidence: 1,016 workspace tests and six feature-enabled eval tests pass, workspace Clippy,
  formatting, UI build and 21 renderer/repository checks (including the new statistics check in CI).
  Coverage includes math, period boundaries, retained usage, legacy gaps, generation races, stale
  reads/retry, safe app labels, filter restoration and navigation without side effects.
- Isolated native Tauri/WebKit/Xvfb validation used synthetic totals at 1180 x 820 and 480 x 360,
  Black/Light, actual period reads (450 words/week, 2,325/month), scrollable narrow content and long
  app names without horizontal page overflow. Week showed 450 dictated words with no agent turns.
  User data was not used. The app is stopped and private data/generated caches are cleared afterward.
  The maintainer's compositor/scale and real-microphone acceptance remain open. V4.3 Diagnostics is next.

**V4.3 Diagnostics** (M) — implemented 2026-10-18; maintainer acceptance pending
- Build: a *Diagnostics* section at the end of Settings › Dictation:
  - a mic test (bands, noise floor in dB, verdict);
  - the stage table (V4.1);
  - counters: loops cut, plan B used, "Copied" outcomes;
  - a system block: audio host and rate, session type, portals (GlobalShortcuts, RemoteDesktop) ok, KWin
    script ok, GPU (`voice::gpu()`), VAD ready.
- Fold into `app/src/doctor.rs` where it overlaps.
- Tests: the doctor's probes with fakes.
- Settings > Dictation owns the section at the page's end. The app doctor reads current audio
  format (native device rate), shortcut bindings, keyboard permission/focus-watch owners and local
  GPU/VAD availability. Reading diagnostics never records or asks for a grant; X11 labels the
  Wayland-only keyboard/focus probes not needed, and missing measurements stay unknown.
- The explicit six-second test uses two quiet seconds for RMS noise in dBFS, then four seconds
  for speech bands and a level verdict. It never transcribes, fetches models, changes clipboard
  or writes history. Test ids reject obsolete commands; restarts wait for capture shutdown.
  Departure, native window/webview changes, screen lock, microphone changes, Chat/dictation starts
  and a missing two-second page heartbeat cancel it. Renderer polling retains the Stop action.
- Final dictation decoding reports actual repetitive-run cuts; counters also record installed-Base
  retries and confirmed Copied outcomes, including replay. Preview and Chat are excluded. Counters
  are memory-only since app start; the timings table reads the existing retained-delivery summary
  without loading transcripts. Failures, empty samples and stale readings remain distinct.
- Verification: 1,021 workspace tests and six feature-enabled eval tests pass, workspace Clippy,
  formatting, UI build and 22 renderer/repository checks. Fake probes, noise/level math, missing
  heartbeat, obsolete ids, late results, retained focus, explicit start/stop, stale reads/retry,
  text safety and all three translated catalogs are covered. No dependency or migration is added.
- Native Tauri/WebKit/Xvfb validation uses a separate configuration/database and deliberately
  unavailable audio hardware: microphone failure/retry, Settings-only command rejection, actual
  timing reads from a synthetic row (2,345 ms total, 1,500 ms STT, unknown clean-up), Black/Light,
  1180 x 820 and 480 x 360, and no horizontal page overflow. Successful measurement/verdict paths use
  controlled PCM and the production renderer with a fake backend. Real-microphone, compositor
  grant/revocation and scale acceptance remain with the maintainer. The test app/private data and
  generated caches are cleaned afterward; V4.4 docs/sign-off is next.

**V4.4 Docs and sign-off** (S) — documentation checkpoint 2026-10-18; maintainer acceptance pending
- Reconciled English/pt-BR Dictation, Activity and Nest guides plus the Settings reference. Removed
  obsolete claims that Statistics/Diagnostics were future screens, added the Diagnostics group to
  the settings table and clarified that the talk shortcut does not record from Settings while
  the explicit microphone test does. No runtime, UI, dependency, migration or preference changed.
- Added [Check statistics and Diagnostics locally](../guide/dictation.md#check-statistics-and-diagnostics-locally):
  read-only refresh, quiet/speech microphone test and cancellation, actual dictation delivery,
  usage versus timing versus replay, disposable-entry deletion, retention, restart and keyboard/
  Black/Light/short-window checks. Reports need environment/model/sample counts, not transcript text.
- Explained nearest-rank p50/p95, the mixed newest-50 delivery window and why stage times need not
  sum to the release-to-delivery total. Copied is not evidence of insertion, GPU readiness does
  not prove model speed, and these screens do not establish a universal latency guarantee.
- Current automated/native implementation evidence remains the dated V4.1-V4.3 record, including
  V4.3's 1,021 workspace tests, six eval tests and isolated native UI reads. This documentation
  checkpoint checks the published-docs CI validation rules, reviewed translation source marks,
  repository guards and the Statistics/Diagnostics/Settings/surface-continuity renderer checks;
  it does not rerun model benchmarks or infer a full native sign-off. Results: all 35 required
  public-page translations are current, all 70 public pages start with H1, the offline docs
  link/fragment scan has zero errors (476 checked, 16 excluded links), all four changed pairs
  preserve English section anchors without duplicate ids, and the four renderer checks pass.

| Acceptance item | Available evidence | Status still requiring maintainer validation |
| --- | --- | --- |
| Timings and unknown stages | Migration 23, nearest-rank/50-row/retention tests, cancellation/replay guards and an actual native synthetic-row read | Real retained dictation with the selected model/language and actual delivery outcome |
| Usage figures and periods | Migration 24, math/calendar/app-attribution tests, actual native period reads and Week without agent turns | Check daily/app figures and period restoration against the maintainer's real dictations |
| Lifetimes and replay | Usage survives erasure/Not kept; timings follow retained text; counters are memory-only; replay adds no usage/timing samples | Disposable-entry deletion, chosen retention and restart; no unrelated transcript needs erasing |
| Microphone test | Controlled-PCM noise/level checks, explicit-start/cancellation/obsolete-result renderer tests and native unavailable-input failure/retry | Actual input, quiet/speech bands, useful verdict, explicit stop and departure/lock cancellation |
| System and permissions | Fake current-owner probes, Settings-only native command rejection, actual unavailable-shortcut/X11/local-model states | Plasma grant/refusal/revocation and focus-script state, or the user's other supported desktop |
| Layout and access | Isolated native Black/Light at 1180 x 820 and 480 x 360 with no horizontal page overflow; renderer focus/race/retry checks | Real compositor/scale, keyboard navigation, Room/Full and reopened-page comfort |
| Small/Vulkan latency target | The release-to-delivery p95 is visible; GPU availability and synthetic values are measured separately | Release-to-pasted p95 below 2,000 ms on actual Small/Vulkan deliveries; no qualifying measurement yet |

For the latency criterion, use a fresh isolated test profile so the last-50 summary contains only
this run, without erasing existing user history. Install Small explicitly, confirm Vulkan/GPU
availability and use a fixed spoken language plus one supported editor with insertion enabled.
Record 50 consecutive retained **Pasted** deliveries under the same model/settings/destination;
record the phrase/audio-duration range, microphone, desktop/scale, GPU and app version. Record
cold versus already-loaded behavior, retries and any failed/cancelled attempt separately. Refresh
Diagnostics and record the total sample count and p50/p95, in milliseconds. A Copied/unknown-target
result or mixed prior settings does not demonstrate this target. Do not discard a slow successful
sample to make the percentile pass. The criterion remains open until the actual result is recorded.

The documentation checkpoint is delivered. V4's real-desktop acceptance and latency criterion,
V1's desktop matrix and V2's human-speech checks stay open; permission to continue implementation
is not their sign-off. The next implementation step is **V8.1 Normaliser and phrase tables**,
following the recorded order; V3 Styles follows V8. N3 is not complete. The app remains stopped,
and renderer-check caches are cleared after validation.


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
  - start/stop from the Meetings page, with a grey recording line, a timer and the "tell people you are
    recording" line;
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
  gate feature `meeting-summary`, 120 s timeout and 20 attempts per rolling 24 hours (V0.4).
  Off by default (R7): Settings › Dictation › Meetings; while off, the Summary tab says why and
  links there. Calls to a local model retain the same switch and request quota.
- Port: [TLP] 353 (`notes/builder.rs:239-243`).
- To-dos remain meeting content and can be exported; Tasks/W3 stays removed under N0.
- Tests: cassette eval.

**V5.7 Meetings store and export** (M)
- Build: tables `meetings`, `meeting_segments`, `meeting_speakers`; rename people; export Markdown, SRT
  and VTT.
- Tests: exporter golden files.

**V5.8 Meetings page** (M)
- Build: the prototype's page (list, Transcript/Summary/To-dos, renamed speakers, export), docs and sign-off.

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
    `voice-realtime` (monetary limit per rolling 24 hours, default US$1; zero disables remote
    voice; past it Zeca falls back to text and says so);
  - extend the gate with in-flight cost reservations, usage settlement and stream cutoff before
    the allowance is exhausted; unknown rates/usage cannot grant unlimited audio (V0.4);
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
  - moving-around commands (C11) are V8's: the voice turn tries V8's parser first and calls the model only
    when it is not a command;
  - "acted, not repeated": when the model fails after a tool already ran, the turn ends with "Done" and the
    local parser does not run the action again; Zeca says it is done only after the action ran;
  - a consult pattern for Claude Code/Codex questions, with a reassurance line after 1.5 s.
- Tests: the rule-2 test over every `Intent` (H1) covers the voice path: no `Decide`/`DecideAlways`
  reachable.

**V6.7 Talk with Zeca page and island speaking states** (M)
- Build: the prototype's *Talk with Zeca* page under Voice (an opt-in switch saying where audio
  goes; on, the model, budget, end of turn, interrupting and *Answer without the key*, off by default; "never
  answers a card" always shown) and the island `speaking`/`interrupted` lines; docs; sign-off; K8 moves to Done.

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
  switch "Agents may ask for a spoken answer" in Settings › Dictation › Agents (off).

---

## V8 Voice commands (R9)

**Goal.** Move around the flock and read it by voice, instantly and with no model, on a key of its own.

**User-visible outcome.** Hold Ctrl+Alt+Z, say "open api", "who is waiting?", "mute web… and docs too…
undo", let go: the island and its caption show what was done or the answer. "Approve" is refused with how to
answer a card.

**V8.1 Normaliser and phrase tables** (S)
- Build: in `vults-dictation::command`: lowercase, accents and punctuation folded, politeness and leading
  fillers stripped ("você pode", "por favor", "can you", "please", "uh", "então"), infinitives to imperatives <!-- check-english:allow -->
  (pt/en/es tables), split on "e depois/and then" only when every part parses.
- Port: [VA] 5.2 steps 1–6 and the multi-action rule (rewritten; French tables replaced by pt/en/es).
- Tests: table tests per language.

**V8.2 Name resolver** (S)
- Build: aliases (Your words replacements whose target is a project or agent name), exact name, a single
  whole word, then Levenshtein (1 for names up to 4 letters, 2 above); a runner-up within the same distance
  makes it ambiguous and the island asks *Which one?*; the answer within 90 s resolves it.
- Port: [VA] 5.3.
- Tests: "apy" → ambiguous between api and api-gateway; "cloud code" → Claude Code through the alias.
- Needs: V2.1.

**V8.3 Intents and reads** (M)
- Build: the command table maps only to existing `Intent`s that are not answers: `Focus`, `FocusNext`,
  `FocusPrevious`, `Jump`, `OpenFolder`, `SetProjectPref` (mute, pin, hide and their undo); reads go through
  `flock_tools` (`pending_asks`, `session_detail`, `usage`, `github_snapshot`) and are answered as text
  from templates, with no model. "Approve", "allow", "yes" and the like are matched on purpose and refused
  with the card rule.
- A 90 s context: the last command with its target, for "and X too", "same for X" and "undo" (the inverse:
  mute↔unmute, pin↔unpin, focus↔previous front).
- Tests: the rule-2 test over every `Intent` (H1) gains the command path: no `Decide`, `DecideAlways`,
  `Answer` or `Release` is reachable from any phrase; property test over random transcripts.
- Needs: V8.1, V8.2.

**V8.4 The command key and the island** (M)
- Build: a second portal shortcut `talk-command` (default Ctrl+Alt+Z, shown as the desktop bound it), the
  same capture and whisper path as dictation with no text pass and no paste; island states *Command*,
  *Command done*, *Answer*, *Which one?*, *Not by voice*, *Not a command*, with the caption carrying what was
  heard; History keeps the command line (no paste again).
- When it is not a command: say so (default), or send it to Zeca as a chat message (a request on the user's
  behalf, through the gate like any typed message).
- Tests: `check-dictation-island.mjs` covers the six states; the key never pastes text.
- Needs: V8.3, V1.12, V1.5.

**V8.5 Settings and docs** (S)
- Build: the prototype's *Voice commands* group (switch on by default, key, names, the not-a-command choice,
  *What you can say*); `docs/guide/dictation.md` gains a commands section (+ pt-BR); sign-off.
- Needs: V8.4.

---

## Merge into plan-zeca.md

Integrated on 2026-10-17 by V0.5. The authoritative phase table is now in
[Plan Zeca > Dictation (N3)](plan-zeca.md#dictation-n3), with delivery order V0, V1, V2, V4, V8,
V3, V5, V7, V6 and each phase's prerequisites and acceptance criterion. Keep substeps and
measurements here rather than maintaining a second phase table.

- K8 is delivered through V6, with V1 and V8 added to its dependencies; it is not a second
  realtime implementation. V8 takes C11 and is reused before K8's model.
- C10/X5 reopen locally through V2. Cloud transcription stays dropped; X3 stays dropped and
  may be reconsidered after V6. UX11's voice work lands with V6/V7. Wake word stays out.
- The decisions table points to the pure dictation layer, platform insertion and Settings/Activity
  owners. The required guard rows name the waiting-session and temporary-audio checks; those
  tests must land with production behavior and are not claimed to exist at this documentation step.
- M7's `ask_user` follow-up is still needed for V7. V7.1 must align ADR 0028 before a question
  composer can accept dictation with explicit human confirmation. Permission cards remain excluded.
- V0.9/P12 was accepted on 2026-10-17. That review does not replace production native smoke,
  gate implementation or model-license checks.

---

## Risks

1. **Wayland input.**
   - The GlobalShortcuts portal delivers only the bound key: no Esc, no other-key cancel, no Space-lock.
   - Binding Esc through it takes Esc from every app.
   - Mitigation: the separate `talk-cancel` shortcut and the X11 raw path. No island button
     under R8. Spike Q1 before V1.8 closes.
2. **RemoteDesktop portal.** KDE asks once. GNOME and wlroots may lack it or refuse it, so the outcome is
   Copied. It is also a strong capability (it types anywhere): kept in the keyring, explained in ADR 0032.
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
10. **Surfaces.** Island text at Mini width truncates, with the full line in its detail or tooltip.
    ADR 0021 now defines the fifth Nest tab and its phased delivery; under 560 px the prototype
    uses named icons, and under 760 px its side list becomes one scrolling row. Native checks
    still need to prove card priority, focus and the fixed overlay's input region.
11. **Prototype size.** 948,702 of 1,000,000 bytes after R9's additions (the copy is now inlined unescaped).
12. **Commands misheard.** A wrong name could focus or mute the wrong project. Mitigation: ambiguity asks,
    every command is reversible ("undo", 90 s), and none of them answers a card or sends text anywhere.

## Open questions

- Q1: can Wayland get cancel or Space-lock without stealing keys? (Layer-surface keyboard focus only while
  locked, then hand focus back?) Spike before V1.8 closes.
- Q2: settled 2026-10-17 in ADR 0032: the RemoteDesktop restore token belongs only in the OS
  keyring; if it is unavailable, do not persist the token. Turning insertion off removes it.
- Q3: settled 2026-10-17 in ADR 0019: N3 loopback providers retain the feature switch, request
  quota and local trace through the gate; only remote monetary accounting is exempt. The
  existing embedding path grants no exemption to a new N3 feature.
- Q4: should dictation keep Ctrl+Alt+V shared with the chat (one key, context decides), or get its own id?
  Proposal: one key for dictation and chat; voice commands get their own id (R9), so a command is never
  pasted and dictation never runs one.
- Q5: settled 2026-10-16, no Undo on the island (R8).
- Q6: may Vults offer to load PipeWire `module-echo-cancel` itself (a system audio change, with a click), or
  only guide the user?
- Q7: order of whisper work when a meeting, a dictation and a chat preview collide.
- Q8: how well smart-turn v3 handles pt-BR; is NAMO (text, Apache-2.0) a better fit?
- Q9: auto-learn words from corrections in History (the reference has none)? Proposal: a "Add to
  dictionary" button on a highlighted word, no silent learning.
- Q10: Parakeet v3 through `ort` (CC-BY-4.0 weights) as a second engine: worth an evaluation step after V2?
- Q11: settled 2026-10-16 and recorded in V0.3 on 2026-10-17: Settings › Dictation owns
  configuration and Diagnostics; the area links there and to Activity › Dictation. The shared
  transcription model and language stay in Settings › Chat & voice (R6).
- Q12: default command key. Ctrl+Alt+Z is free in Plasma's defaults; is it free in the user's setup, or
  should the portal ask with no default?
- Q13: may "Send it to Zeca as a message" be offered when Zeca's chat runs through a CLI provider, or only
  with the API chat?

### Critical Files for Implementation
- /home/rogeradas/Projects/Vults/crates/voice/src/record.rs (and lib.rs, vad.rs)
- /home/rogeradas/Projects/Vults/crates/platform/src/shortcuts.rs (and jump.rs, jump/kwin.rs, jump/x11.rs)
- /home/rogeradas/Projects/Vults/app/src/lib.rs (listen_shortcuts, 707-760) and app/src/voice.rs, app/src/gate.rs
- /home/rogeradas/Projects/Vults/crates/store/src/lib.rs (MIGRATIONS)
- /home/rogeradas/Projects/Vults/design/ui-motion-proposal/dictation.js with island.js, settings.js, verify.mjs, features.json, copy.json
