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
single accepted prototype (`design/ui-motion-proposal/dictation.js`): Talk with Zeca remains a later
work page; Meetings has its bounded recording/transcript check, Voice commands its retained-history
page, and Styles its configuration editor.
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
[V4 Statistics and diagnostics](#v4-statistics-and-diagnostics). V8.1-V8.4 now connect voice-command
grammar, name resolution, timed clarification, the local dispatcher and a separate capture key with
island/Nest feedback and non-replayable history. V8.5 adds dedicated command settings, the work page,
a shared phrase guide and default-local/explicit Chat fallback. V5.4 research and V5.5 engineering now provide a default-off developer speaker preview, after
the bounded V5.3 recording-to-transcript handoff. Independent PT/EN quality qualification remains
open; see the [consolidated V5.5 checklist](#v55-consolidated-delivery-checklist). V5.6 now has a gated
summary backend and its Settings/Summary/To-dos interface; V5.7/V5.8 add explicit text-only
meeting storage/export and the saved Meetings workspace.
See the [V5.6 delivery sequence](#v56-delivery-sequence). Linux now exposes explicit recording up to four hours under Dictation > Meetings, with bounded mono PCM16 temporary files, local transcription after Stop and explicit text-only saving. Hardware/compositor acceptance remains pending. V3.5 delivers the documentation checkpoint; real-speech/provider sign-off remains separate. V3.4 now delivers the visible clean-up settings, Dictation Styles editor and applied History labels. V3.3 now freezes per-app style and output language at key-down. V3.2 now connects the pure contract to tool-free
API/local providers, a default-off switch, durable quota, cancellation and complete-local-text fallback;
Z6 is already Done (#232), with its separately recorded Responses follow-up unchanged. V1/V2/V4/V8 real microphone/desktop/provider acceptance
remain separate; N3 is not complete.

V7 now implements typed MCP questions, explicit local dictation into drafts and reviewed external-agent
setup; full-client/human-speech acceptance stays open. V6.1-V6.3.5 provide the turn/end policy, gated
realtime transport/accounting and native turn retention/local recovery. V6.4.1 adds a bounded CPAL
output primitive and pure heard-text/history-replacement math. V6.4.2 connects its native owner to
provider identities, acknowledged server truncation and in-memory history/one-shot repair. V6.7.3
now retains a realtime session between explicitly recorded manual turns. Local silence completion is opt-in; semantic-model and
duplex continuation remain open. Backend V6.4 is
delivered; physical/live acceptance stays open. V6.5.1/V6.5.2 deliver read-only PipeWire pair inventory,
an isolated synthetic AEC experiment and consent-bound pinned native duplex routing with explicit
device policy. Backend V6.5 is delivered; its continuous-turn UI adapter, Q6 production setup and
physical/live acceptance remain open. V6.6.1 now routes native finalized
local transcripts through V8 before remote admission. V6.6.2 adds bounded read-only realtime flock
tools with independent continuation admission. The V6.6.3 publication checks below are accepted
with Codex consultation disabled. V6.7.1/V6.7.2 expose explicit Settings-owned configuration, and
V6.7.3 adds Nest-owned manual Start/Finish/Cancel with editable text recovery and a retained
conversation for explicitly requested following turns. Local silence completion is opt-in. Semantic-model/
duplex integration, island speaking states, optional Codex activation and physical/live acceptance
remain open. The maintainer accepted the four-hour meeting capture adjustment before resuming that work.

## References

| Project | Licence | What we take | How |
|---|---|---|---|
| The reference app credited in `NOTICE` | MIT | Key state machine, text pipeline (snippets, replacements, dictionary), X11/Wayland paste, clipboard lease, history and statistics, meeting notes, end-of-turn policy | Ported, credited only in `NOTICE` (`check-brand.sh`) |
| [voicebox](https://github.com/jamiepine/voicebox) | MIT | Clean-up prompts ("a text filter, not an assistant", three rules), whisper loop remover, key chords (Space locks mid-hold), focus-safe overlay, conditional clipboard restore, MCP with an identified client | Rust copied; Python rewritten in Rust; `NOTICE` line when code is translated |
| [Patter](https://github.com/PatterAI/Patter) | MIT | Turn and barge-in state machine, VAD hysteresis, semantic end of turn (smart-turn v3, BSD-2-Clause code/weights verified in V6.2.2), sentence chunker with Portuguese rules, per-stage metrics | Design and constants; code rewritten |
| VoiceStudio | AGPL-3.0 | Ideas only: spoken code in terminals, a fallback model, an honest "Pasted" vs "Copied", pause | No code |

The research notes behind this plan (per-project specs with file:line citations) are kept outside the
repository; ask the maintainer for them. The reference keys used in the steps are `[TLP]` (reference app,
Linux port), `[DD]` (reference app, dictation and dictionary), `[VA]` (reference app, voice assistant),
`[VB]` (voicebox), `[PT]` (Patter) and `[VS]` (VoiceStudio, ideas only).

## Decisions (V0)

Rules that hold as they are: [ADR 0002](../adr/0002-rust-backend-no-sidecars.md) (Rust only, no Python
sidecar), [ADR 0028](../adr/0028-spoken-replies-through-a-realtime-model.md) (local transcription, audio in
memory, no GPL in the binary, voice never answers a permission; V7.1 question drafts require human
confirmation), [ADR 0004](../adr/0004-a-human-answers-permissions.md)
and [ADR 0019](../adr/0019-requests-on-the-users-behalf.md) (model clean-up is a request on the user's behalf).

| # | Today | Proposal | Why |
|---|---|---|---|
| R1 | X5 voice dictionary dropped (2026-10-12) | **Reopen, local only**: post-processing plus whisper vocabulary; cloud transcription stays dropped | Names of projects and tools come out right; cheap and fully local |
| R2 | Dictation only into Vults' chat | **Dictation into any app** and straight into flock sessions (new ADR 0032) | Clear gain; the pipeline already exists |
| R3 | Wake word: no | **Still no.** Hands-free is a double tap on the key. Zeca's 8 s follow-up mic is off by default | Keeps ADR 0028; no always-open mic |
| R4 | X3 a voice per vult dropped | Stays dropped; reconsider after V6 | Speaking depends on the opt-in realtime model |
| R5 | No meetings in the plan | **Meetings in V5**, opt-in per recording, mic and the PipeWire monitor | Useful, but the heaviest part (diarization) |
| R6 | One configuration owner per value | **Revised 2026-10-18 at the maintainer's request:** the Dictation tab presents its own configuration and shared voice model, alongside work pages; Chat links there. Privileged editors stay in the Settings webview, transcripts in Nest, and figures in Activity > Dictation | One editor per value and one shared navigation; visual ownership does not grant new command authority |
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
  `command`: the pure voice-command grammar and name resolver (V8), returning descriptions with
  unresolved names in V8.1; after resolution the app maps them to existing non-answer `Intent`s
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

**V3.1 Clean-up prompts and guards** (M) — implemented 2026-10-18; providers/UI not connected
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

Implementation and evidence (2026-10-18):
- Confirmed the Z6 prerequisite in Plan Zeca's Done table (#232); this stage does not implement its
  separate OpenAI Responses/native-compaction follow-up.
- `vults-dictation::cleanup` is pure: typed style/rule/target-language options produce system/user/
  assistant turns, not an IO request or tool invocation. Verbatim and empty input return no request.
  Natural, Polished and Prompt instructions keep the transcript as data; Prompt only organizes
  supported Goal/Context/Steps/Constraints/Done when sections, without inventing work or answers.
  Hesitation, explicit self-correction and technical-term rules are independent. Translation is
  explicit for the four app languages; with none selected, the spoken language stays.
- The final user turn XML-escapes the transcript, app label and at most 200 bounded spelling hints.
  Few-shot examples are actual alternating turns, with question/imperative anchors last. Examples
  are authored from this contract; no prototype source, attribution or dependency was copied.
- Typed local snippet/replacement/spelling/spoken-code segments become collision-free opaque marks.
  Their literal payload never enters the provider prompt. Validation requires the original mark
  sequence, rejects missing/duplicated/reordered/invented marks and restores payloads in one pass,
  preserving exact whitespace and literal marker text. Input, output, metadata and protected-count
  limits bound the pure work. Failed builds/validation retain the complete local original for the
  future orchestrator; nothing truncates the text or decides insertion safety.
- Reply guards reject empty output, new control bytes, lost explicit question punctuation, implausible
  character lengths (15% minimum; Natural/Polished 4x + 200, Prompt 8x + 600 maximum) and same-language
  Natural/Polished word ratios outside 0.35x to 1.4x + 4. Translation and Prompt use character bounds
  instead of cross-language/structural word ratios. These are structural checks, not proof of semantic
  faithfulness; live model quality/injection testing remains separate from deterministic cassettes.
- Nine pure tests cover opt-outs/limits, escaped metadata, rule/style/language options, local code,
  exact snippet restoration, marker collisions/nesting/tampering, question/control guards, Unicode and
  both word/character thresholds. Feature-enabled evals replay 19 authored response fixtures and
  review six complete prompt-turn snapshots, including translated variants. Fixtures include questions,
  imperatives, injection as literal text, self-corrections, missing/duplicate/invented marks, empty/
  control output, length failures, translation and Prompt organization. They call no model/network.
- The only dependency change is the eval crate's test dependency on the existing dictation crate;
  no external package is added. V3.1 adds no renderer, microphone, app-control, settings, network,
  automatic insertion or approval behavior. The running V8 app can remain available for validation.
- Verification: 1,074 workspace tests pass (eight opt-in/native ignored), plus eight feature-enabled
  evals. UI build, all 15 production-renderer scripts, workspace and feature-enabled eval Clippy with
  warnings denied, formatting, dependency audit, repository guards, 35 reviewed documentation pairs
  and offline links/fragments pass. No renderer changed, so new native UI fixtures are not required
  for this pure slice. The existing app/server stay open for the maintainer; no live provider call,
  real-speech acceptance or new Windows CI result is claimed.
- The next step is **V3.2 Providers and the gate**. Its opt-in switch, request quota, content-free trace,
  provider adapter, timeout/raw fallback and cancellation admission must land before model clean-up
  can run. The per-app resolver and Styles settings/work page follow in V3.3/V3.4; N3 and its earlier
  microphone/compositor/provider acceptance remain open.

**V3.2 Providers and the gate** (M) — implemented 2026-10-18; visible controls remain V3.4
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
- Implementation: a separate Settings-only `dictation_cleanup` switch defaults off (schema 37),
  independent of Zeca, voice recording and command Chat consent. Capture the provider/model, ledger
  binding and cancellation revision at key-down; later consent cannot revive a recording. The
  temporary runtime style is Natural; per-app style/language resolution follows in V3.3, and V3.4
  owns the visible switch and Styles controls. Command capture and replay bypass this path.
- `chat::filter` sends one bounded, non-streaming, tool-free Anthropic or OpenAI-compatible request
  through the selected API provider; no CLI, Chat persona/tools, proxy, redirect, automatic retry or
  remote fallback. Local endpoints must be literal loopback IPs. Keys stay in the keyring. Typed local
  edits retain V3.1's opaque marks through the worker's final processing. Malformed/refused/tool/
  truncated/oversized replies and provider errors retain the complete local output.
- A single absolute deadline includes keyring preflight and durable admission: 4 seconds normally,
  8 with explicit translation, 12 for Prompt. Disablement, provider/model/key changes, lock or ticket
  cancellation drops HTTP work. Existing final target/card/session and insertion rechecks remain.
- Store migration 25 reserves each attempt and its starting trace atomically before sending, under a
  shared 2,000-attempt rolling quota. Failures, interrupted work, restarts, Clear history and local
  calls retain charges. A Settings binding to the ledger identity prevents database loss/recreation
  or disable/re-enable from granting fresh allowance. Missing/unreadable budget state denies work.
  Late completion updates only a surviving trace, without restoring erased history. Metadata contains
  provider/model/local/timing/outcome and known token counts; no transcript/audio/clipboard/key or
  invented zero usage/cost. Heard/final text and cleanup timing follow existing History retention.
- Tests use authored replies and actual loopback fake HTTP servers, never a real provider. Coverage
  includes both wire formats, endpoint validation, redirects/errors without retry, unknown usage,
  malformed/oversized bodies, 4/8/12-second slow-provider and slow-preflight fallback, an entire day
  switched off with zero preparations/reservations/calls, mid-call disable/cancel, stale consent,
  last-slot concurrency, the exact 2,000-attempt boundary, restart/clear/clock rollback, unavailable
  ledger, recreated database, retained heard text, protected local edits and late trace completion.
- Verification: 1,092 workspace tests pass (eight opt-in/native ignored), plus eight feature-enabled
  evals. UI build, all 15 production-renderer scripts, workspace and feature-enabled eval Clippy with
  warnings denied, formatting, cargo-deny, repository guards, all 35 reviewed documentation pairs and
  482 offline links/fragments pass. No rendered UI changed; the existing app/server stay open with
  clean-up off. No live provider, real-speech/compositor acceptance or new Windows CI result is claimed.
- Next: **V3.3**, per-app styles and language, then the visible V3.4 controls. N3 remains open.

**V3.3 Styles per app and language** (S) — implemented 2026-10-18; visible controls remain V3.4
- Build:
  - a `dictation_styles(app, style)` table;
  - defaults: verbatim for terminals/editors, polished for mail, natural for chat apps ([TLP] 345-349);
  - resolved at key-down;
  - flock sessions → Prompt;
  - "Write in" and "Agent sessions in English".
- Tests: resolution table.
- Needs: V1.7, V3.2.
- `vults-dictation::styles` resolves exact reviewed desktop aliases, explicit per-app overrides and
  category/global defaults. Priority: unavailable/off stays local; recognized session uses Prompt;
  explicit app choice overrides terminal/editor Verbatim, mail Polished or chat Natural; other
  identified apps use the global default (Natural). Missing focus and unreadable preferences never
  guess from a title/URL or send a model request. Verbatim suppresses translation as well as cleanup.
- Settings schema 38 adds typed `dictation_style`, optional `dictation_write_in` (en/pt-BR/es/zh) and
  default-off `dictation_sessions_english`. Session English precedes global Write in only in a
  recognized session. These choices do not enable cleanup or inherit Chat consent. Settings-only
  commands prepare the V3.4 controls; edits apply to the next capture, leaving current choices intact.
- Migration 26 adds persistent `dictation_styles(app, style)`, preserved by retention/history clearing,
  plus nullable `output_language` on retained dictations. The worker keeps heard language independent;
  only successful model filtering records its applied style/output language. Failure keeps local
  processing metadata, and erasure/Not kept removes retained output language with the transcript.
- Freeze provider/consent plus resolved application/style/language in the key-down callback. X11 reads
  foreground identity there; Plasma Wayland warms a separate lifetime of the existing checked KWin
  observer when cleanup is enabled, without a RemoteDesktop grant. Read-only observation supplies no
  typing authority. Compositor owner loss clears snapshots; shutdown closes the style observer without
  unloading insertion's observer. Exact already-authorized pane context or unambiguous session ancestry
  selects Prompt; shared/ambiguous context does not opt into session English. Later UI focus observations
  cannot change the choice. Final target/card/session checks and cancellation remain V3.2's owners.
- Tests cover aliases without arbitrary suffix impersonation, all category/global/override priorities,
  opt-out/unknown focus, language/session precedence, invalid Settings types/ids, round-trips, schema-25
  migration, preference persistence through clear/restart, retained heard/output language, local-edit
  protection and frozen request options. The private headless-KWin smoke exercises live focus changes,
  frozen context, zero keyboard grants for style observation and independent observer close/lifetime.
- Verification: 1,099 workspace tests pass (eight opt-in/native ignored), plus eight feature-enabled
  evals and the explicit private headless-KWin smoke. The final app follow-up passes all 222 app tests.
  UI build, all 15 renderer scripts, workspace and feature-enabled eval Clippy with warnings denied,
  formatting, dependency audit, repository guards, 35 reviewed documentation pairs and 482 offline
  links/fragments pass. This backend slice changes bridge data only; no rendered page or new visual
  fixture is claimed. The existing app/server stay open with cleanup off. No real provider/speech,
  actual compositor/portal acceptance or new Windows CI result is claimed.
- Next: **V3.4**, visible Settings controls, the Styles work page and applied History labels.

**V3.4 Clean-up settings** (M) — implemented 2026-10-18; real-provider/speech sign-off remains separate
- Build: the prototype's *Styles* page: the clean-up switch, off by default (R7), and the four style cards
  (As spoken, Natural, Polished, Prompt; while off they say every line is pasted as spoken). On, it also shows
  the provider, "Write in", agent sessions in English and a collapsed per-app list.
  History shows the style and "in English" when a line was translated.
- Tests: renderer checks.
- Settings > Dictation now owns the separate switch, off by default. Before activation it names the
  selected API provider/model, describes text/context (no audio), cloud/local destination, rolling
  2,000-attempt quota and possible provider charges. It reads only local settings/provider/key-presence
  metadata and app preferences; opening Styles never fetches model lists, probes a server or calls a
  model. Missing provider/key blocks activation, with a Chat & voice link; existing cleanup can still
  be disabled when ancillary reads fail. No privacy preference or gate budget changes meaning.
- On, show the shared four style cards, Write in (spoken/en/pt-BR/es/zh), default-off session English
  and a collapsed per-app list with automatic resets and custom app identity entry. Off, hide these
  write controls. Native acknowledgements/newer events win over older reads; writes guard duplicates,
  retain focus/scroll and keep app drafts on failure. App errors/status stay beside their controls;
  an uncertain write or unavailable budget offers a local reload, without a false rollback claim.
- Nest > Dictation > Styles uses the same localized read-only cards, default/output-language summary
  and a direct `dictationstyles` Settings link. History labels only known applied styles and output
  languages, with no invented metadata for old rows and no style labels on voice-command rows. Room/
  Full and narrow navigation retain the selected page. Models, insertion guards and permission
  authority remain unchanged.
- All 16 production-renderer scripts pass, including a new dedicated cleanup-control check and added
  History, direct Settings routing and surface-continuity cases. They cover off/missing-key/no writes
  on read, provider disclosure, stale startup/newer events, pending repaint/duplicate guards, budget
  failure/retry, style/language/session settings, per-app reset/save/inert text, local error placement,
  drafts/focus/scroll and read-only Nest. Copy is translated in pt-BR/es/zh (1,448 source sentences).
- Native QA uses a fresh private Xvfb/D-Bus profile with Zeca off and local fixture provider metadata:
  Black 1180x820 and Light 480x360, Settings consent/on/off, saved Prompt/English, collapsed app list,
  a long custom identity, retained keyboard focus, direct Styles navigation, four-card layout, output
  labels from synthetic history and pt-BR. Inspected screenshots and geometry show no page overflow;
  the ledger records zero model attempts. This is UI/backend wiring evidence, not live-model quality
  or real-microphone/compositor acceptance. The temporary native app is closed after QA.
- Verification: UI build, 1,099 workspace tests (eight opt-in/native ignored), eight feature-enabled
  evals, workspace/eval Clippy with warnings denied, formatting, dependency audit, repository guards,
  all 35 reviewed documentation pairs and offline links/fragments pass. The maintainer's running app
  is restarted with the current backend and kept available on the existing local Vite server, with
  their cleanup preference still off. No user transcript/provider call or new Windows CI is claimed.
- Next: **V3.5**, documentation checkpoint and explicit sign-off; N3 remains open.

**V3.5 Docs and sign-off** (S) — documentation delivered 2026-10-18; real-speech/provider acceptance pending

- Reconciled the English/pt-BR Styles guide, settings reference, architecture and Unreleased notes
  with the production V3.1-V3.4 runtime. Kept the old `coming-in-n3` anchor while adding a named
  Styles section, style/default tables and an explicit disposable-text validation checklist.
- Distinguished input and output languages, local text rules versus optional model filtering,
  Prompt's 12-second deadline even with translation, and measured cleanup fallback timings versus
  unmeasured stages. The complete local result, never a partial/raw substitute, remains the fallback.
- Recorded consent/provider/data/quota disclosure, exact app-override precedence, literal protected
  edits, no CLI/tools/persona, durable admission accounting, retention and no permission authority.
- Applied the full-area review's four refinements: process-owned, whitelisted return to the last
  configuration webview; direct Command settings navigation; voice-model descriptions in all four
  languages; and search before folded compact daily figures, retaining their explicit disclosure choice.
- Final verification passes: UI build, all 16 renderer scripts, 1,100 workspace tests (eight ignored),
  eight feature-enabled evals, workspace Clippy with warnings denied, formatting, repository guards,
  35 reviewed documentation pairs and offline links/fragments. Native checks confirm opposite
  webview visibility on return, rejection of the navigation setter from Nest, direct command settings,
  translated model labels and search at y=153-185 above the y=322 footer in both themes at 480 x 360.
  Compact figures retain their explicit open choice after wide/narrow transitions. No microphone,
  inference, insertion or new compositor/portal acceptance is claimed; the isolated app is closed.
- The Settings-only navigation command stores presentation state, never consent or a transcript.
  An explicit work-page route wins over remembered configuration. Nest's header, keyboard and palette
  return through the native owner; work views keep their own filters and drafts.

| Contract | Evidence available | Acceptance still open |
| --- | --- | --- |
| Filter meaning and literal edits | Pure prompt/reply guards, authored question/injection/translation cassettes and protected-edit tests | Review real speech and actual provider replies for meaning, names and phrasing |
| No-call/local fallback | Fake off/stale/unavailable/quota/deadline/cancellation paths, tool-free HTTP doubles, explicit consent and native no-inference UI reads | Real-provider availability/latency and cancellation under the maintainer's chosen model |
| Frozen app/style/language | Alias/category/override/session resolution tables and frozen snapshots; store migration/retention tests | Real desktop focus and recognized/ambiguous sessions, with each intended app and language |
| Quota and privacy | Atomic admission, restart/clear/concurrency/lost-ledger tests; keyring-only credentials; metadata-only request traces | Review the chosen provider disclosure before opting in; no live quota exhaustion is required |
| Configuration and history UI | Production-component failure/race/focus checks; isolated native Black/Light, 1180 x 820 and 480 x 360, cross-webview return, exact commands destination, translated models and visible search | Maintainer comfort at their compositor/scale; real audio and destination insertion remain separate |

The checklist in [the guide](../guide/dictation.md#check-styles-locally) is ready for real-provider
acceptance with disposable text and explicit opt-in. Continuation is not a claim that a live model
preserves meaning or that the V1/V2/V4/V8 desktop/speech matrix passed. N3 is not complete.
The next recorded implementation step after this checkpoint was the **V5.1 System audio capture** spike, now delivered below. The next step is V5.2; this documentation checkpoint itself starts no recording.
The maintainer requested the app/server remain stopped and generated caches be cleared afterward.


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
is not their sign-off. This checkpoint handed off to **V8.1 Normaliser and phrase tables**;
see its record below for the current implementation handoff. The phases follow the recorded order; V3 Styles follows V8. N3 is not complete. The app remains stopped,
and renderer-check caches are cleared after validation.


---

## V5 Meetings (R5)

**2026-10-20 long-recording priority:** after the maintainer identified the one-minute cap as
unusable for meetings, manual capture now admits up to four hours. Both sources stream through
the shared stateful resampler into private mono 16 kHz PCM16 WAVs; the existing 28-second local
decoder remains the processing owner after Stop. Bounded queues and normalized frame caps keep
temporary audio below 1 GiB at four hours, independently of source channels/rate. Stop, Cancel,
page contact, source/model review and cleanup ownership remain in place. This does not add live
transcription or resumable audio: failures/cancellation delete audio, and startup removes leftovers.
Speaker analysis stays experimental and limited to 30 minutes per source; longer recordings retain
plain text without labels. Full-duration physical/speech-quality qualification remains open.
Native synthetic capture passed 65 seconds with finalized PCM16 readers and cleanup; this is not
a four-hour soak test. The dated one-minute checkpoints below describe the earlier delivered slice.

**2026-10-20 maintainer validation:** the maintainer accepted the delivered long-recording adjustment
and authorized the next roadmap task. This closes user acceptance of that scope; it does not claim
a completed four-hour physical soak, crash resume, speech-quality qualification or K8 acceptance.


**Goal.** Record a call (mic + system audio), get a diarized transcript, a summary and to-dos; the audio is
deleted.

**V5.1 System audio capture** (M, spike first)
- Build:
  - `vults-audio` reads the default sink monitor over the local PulseAudio protocol (including
    pipewire-pulse); the initial CPAL-host spike was replaced by the bounded connection below;
  - fallback: `<sink>.monitor`, else the first monitor;
  - two streams (mic, system) to temporary WAVs (`hound`, Apache-2.0) in a 0700 folder;
  - delete on finish; delete leftovers on start (crash).
- Port: [TLP] 126, 130 (`speech/audio.rs:241-252`, `notes/mod.rs:282-302`); [VB] 90, 110 (why not ALSA).
- Tests: monitor-name selection table; leftover cleanup.
- Licence check of the new host crate.
- Wayland and X11 behave the same (PipeWire).

**2026-10-18 spike delivered; go for production lifecycle work, not feature sign-off.**
`vults-audio/meeting-capture` is an optional Linux-only development feature. The app does not
activate it or expose a recording command. Chat/dictation keep their in-memory capture. The initial
CPAL-host implementation changed the default host preference; the later V5.2 deadline refinement
replaces that path with a separate local protocol connection and leaves the normal ALSA host unchanged.

The initial spike selected stable server IDs, preferring `<default sink>.monitor`, then the first
sorted monitor ID. The later refinement also honors the server's monitor metadata for custom IDs. No monitor means an error, never recording a microphone as system output. A separate
non-monitor default microphone is selected, with a non-monitor fallback. The inventory reports the
selected IDs and whether the output fallback was used; it opens no stream and writes no file.

Explicit recording uses two native-rate/channel float32 WAVs, with 20 ms requested fragments,
16 bounded callback chunks per track and a 1-60 second active-capture cap. Callbacks never write to
disk; overflow, invalid samples or backend errors fail the recording. The temporary directory must
be owned by the current user and mode 0700; files and the exclusive lock use mode 0600. The lock
prevents another recorder from deleting active audio. Only `mic.wav` and `system.wav` are cleaned;
symlinks are unlinked without following them. Return, error and unwind remove them; abrupt process
termination leaves private files for recovery on the next explicit recording. The persistent lock
contains no audio. No background startup hook is connected yet.

Reproduce the offline checks (no device is opened):

```sh
cargo test -p vults-audio --features meeting-capture
cargo clippy -p vults-audio --features meeting-capture --all-targets -- -D warnings
```

List sources without recording:

```sh
cargo run -p vults-audio --features meeting-capture --example meeting-capture
```

A developer can explicitly record after telling other participants and reviewing the selected
sources. This command captures the microphone and system output, temporarily saves local audio
and removes the two WAVs before returning. Use a dedicated private directory under a trusted parent:

```sh
cargo run -p vults-audio --features meeting-capture --example meeting-capture -- \
  --record-seconds 3 --consent-temporary-audio "$XDG_RUNTIME_DIR/vults-meeting-spike"
```

Evidence from an isolated PipeWire 1.6.9 / pipewire-pulse server, private D-Bus/runtime/config and
hardware monitors disabled: a synthetic 440 Hz microphone and 880 Hz output were captured as
separate streams. Both produced 144,000 stereo frames at 48 kHz for three seconds, peak 0.1526.
The default server fragments initially delayed sequential startup by about four seconds; requesting
20 ms fragments reduced measured startup to about 0.1 s. This is a single synthetic startup check,
not a latency percentile or sample-clock alignment claim. Normal completion, SIGKILL/restart recovery,
server disconnection, directory/file modes and source selection were checked. No real microphone,
desktop audio, participant speech or provider was used; the private processes were then stopped.
Unit tests also cover monitor selection, empty sources, exact file scope, a busy lock, bad modes,
symlinks, cleanup continuing after one deletion fails, WAV formats/frame caps and empty-stream failure.
CI runs these feature checks on Linux. Final checks: 14 feature-enabled audio tests, 1,100 workspace
tests (8 ignored), 8 evals, workspace/feature Clippy, UI build, formatting and repository guards pass.
A pre-existing FIFO-read test had assumed no repository above its temporary fixture; a `/tmp/.git`
directory exposed that assumption. The test now sets its own traversal ceiling and plants an ancestor
repository explicitly, without changing production lookup or deleting external files. The app's
default feature tree excludes `meeting-capture`, `pulseaudio` and `hound`.

Dependency review: CPAL 0.18.2 and `hound` 3.5.1 are Apache-2.0 (the original MIT
assumption was corrected against its published manifest). The `pulseaudio` 0.3.2 backend and added
transitive `enum-primitive-derive` 0.3.0 are MIT; `futures` 0.3.34 is MIT OR Apache-2.0.
`cargo deny check` passes with the existing policy. The PulseAudio backend is pure Rust and uses
local Unix sockets; no libpulse build requirement, remote audio connection or network gate feature
is added. Sources: [CPAL feature/API documentation](https://docs.rs/cpal/0.18.2/cpal/),
[PulseAudio backend source](https://docs.rs/crate/cpal/0.18.2/source/src/host/pulseaudio/mod.rs),
[pulseaudio source and license](https://docs.rs/crate/pulseaudio/0.3.2/source/),
[hound license](https://docs.rs/crate/hound/3.5.1/source/).

At this checkpoint, production still needed startup cleanup, cancellation, backend deadlines and
visible consent/timer. The V5.2 refinements below deliver the worker and bounded connection while
keeping recording disabled in the app. Real hardware/compositor checks, startup/UI integration and
long transcription remain separate. No long meeting, UI recording, transcription, diarization,
audio export, summary request or V5 completion is inferred.

**V5.2 Recording controls** (M)
- Build:
  - start/stop from the Meetings page, with a grey recording line, a timer and the "tell people you are
    recording" line;
  - an optional live transcript;
  - dictation stays available, with its own stream.
- Tests: state machine (recording × dictation).

**2026-10-18 backend controller checkpoint (followed by the UI delivery below).**
The optional development feature now offers `Recording::start`, `status`, `stop`, `cancel` and
one-shot, nonblocking `take_result`. `record_for` uses this same path. Each recording owns a worker,
its control state and the existing temporary-directory lock; no microphone stream, Whisper lock,
dictation state, provider, permission card or app preference is shared with it.

The controller reports Starting, Recording, Stopping, Cancelling, Finished, Cancelled or Failed.
Its timer begins once both streams are playing and freezes after teardown. Stop while recording
finalizes a report with `End::Stopped`; the active-capture cap reports `End::Limit`. Stop during
startup cancels, and Cancel overrides a pending Stop. Repeated requests are harmless. A cancellation
that wins before result publication cannot leak a successful report; an old controller cannot stop
a later recording. Dropping the controller requests cancellation without joining a live worker.
The worker retains ownership until its backend work and file cleanup finish, so a second start
or recovery cannot delete active audio. Terminal status/result publication follows cleanup; a
cleanup failure reports Failed even if the requested action was cancellation. Backend errors and
Rust unwinds also release audio before publishing their result.

`recover(path)` deletes leftovers under the same exclusive lock without opening either source.
It creates no directory for an absent profile and returns an error if a recording owns that
location. It is available to a future startup owner; the app does not call it yet. The developer
example supports `--recover PRIVATE_DIRECTORY`, or the existing recording command followed by
`--interactive`: type `stop` or `cancel`; end of input cancels. These commands retain the explicit
temporary-audio disclosure and 60-second development cap. Stop currently returns measurements and
removes WAVs; the local transcription handoff belongs to V5.3, not an audio archive.

Evidence: offline tests hold a synthetic backend in startup/teardown to check nonblocking control,
late completions, cancellation precedence, duplicate start/recovery, owner drop, stale controllers,
backend panic/error and cleanup failure. An explicitly ignored integration test additionally
requires a private local test server, fixed synthetic source IDs and a temporary runtime directory
before opening streams. In that isolated PipeWire graph, an independent dictation microphone
continued delivering local PCM while one meeting stopped and a second was cancelled. The developer
example's interactive Stop, Cancel and startup/end-of-input cancellation passed, alongside normal
limit expiry, separate 440/880 Hz tracks, SIGKILL recovery without recording and server-disconnection
cleanup. Generated tones and temporary recordings were deleted and the private servers stopped.
No user's microphone, real desktop output, participant speech, provider or UI recording was used.
Checks passed: 21 offline feature-enabled audio tests (the isolated integration test is ignored by
default and was run explicitly on the private server), 1,100 workspace tests with 8 existing ignores,
workspace/feature Clippy, UI build, formatting and repository guards. No renderer code changed in
this backend step.

**2026-10-18 backend deadline refinement delivered.** Meeting capture no longer enables
CPAL's PulseAudio feature. A dedicated `pulseaudio` client uses one local Unix connection for both
tracks; ordinary microphone capture and diagnostics retain their existing ALSA host. `socket2`
0.6.5 (MIT OR Apache-2.0, already in the lockfile) supplies the two-second connection timeout. Each
asynchronous protocol request has a two-second deadline. The synchronous authentication handshake
has socket IO timeouts and an independent two-second watchdog; cancellation also shuts down that
same socket. A 10 ms watchdog check closes it on Stop, Cancel, owner drop or the active recording
limit. No teardown waits for a server acknowledgement. Callbacks hold no files, stop accepting
chunks on teardown and retain their bounded queue; the worker closes local transport and WAVs,
then performs cleanup before publishing a terminal result. Cancellation during a backend failure
returns Cancelled; a cleanup error still returns Failed.

An explicit `PULSE_SERVER` must name an absolute `unix:` endpoint; invalid/remote values fail
without trying a different server. Otherwise, the local Pulse runtime is used. The existing OS
PulseAudio authentication cookie may be read (regular file, exactly 256 bytes, bounded read and
no symlink); it is never written, logged or sent outside that local socket. Capture pins source IDs
with `no_move`, accepts only the requested float32 format, uses monitor metadata to exclude custom
monitor names from microphones and reports its deterministic fallback. Empty IDs are rejected.
A content-free local server probe every 500 ms detects stalled/disconnected transport; a track
that stops delivering frames also fails. No network feature, retry, agent command or model request
is added.

Validation on the private synthetic server freezes pipewire-pulse with SIGSTOP. Stop and Cancel
then finish and remove temporary audio in about 35 ms; cancellation during stalled startup finishes
in about 30 ms. Inventory/authentication expires in about two seconds, and an active stream with
no user action fails after its probe deadline (about 2.3 seconds after freezing in this fixture).
The server is resumed only after each result and cleanup, proving teardown does not need its reply.
These are individual synthetic checks, not latency percentiles. Independent 440/880 Hz tracks,
normal limit expiry, crash recovery without capture and server-disconnection cleanup still pass.
The independent dictation fixture now pins its unchanged ALSA path to a private Pulse plugin
configuration; no physical microphone is opened. Unit checks cover a silent authentication peer,
cancellation of that handshake, explicit endpoint rejection and custom monitor metadata/fallback.
All private processes and generated audio are removed after validation. Final checks pass: 26
feature-enabled offline audio tests (1 isolated-server test ignored by default and run explicitly),
1,100 workspace tests with 8 existing ignores, 8 evals, workspace/feature Clippy, UI build,
formatting, repository guards and `cargo deny check`. Feature-tree inspection confirms that the
meeting feature no longer enables CPAL's PulseAudio host and remains absent from the default app.

**2026-10-18 recording-check controls and startup recovery delivered.** Linux now enables the
meeting backend for a deliberately bounded one-minute check under Dictation > Meetings. Startup
recovers only the dedicated `meeting-audio/` files, without creating a fresh directory or opening
sources. The owner sleeps while idle. Source review reads metadata only; a fresh one-use, 60-second
review ticket and explicit checkbox/start action are required for each recording. Sources are
rechecked before opening streams, addressed by their exact reviewed names and marked `no_move`.
A changed default fails rather than silently capturing a different device. Raw backend errors
remain private; translated states explain recovery/source/capture failure and offer a retry.

Nest alone owns the five recording IPC commands; Settings, island and other labels are rejected.
The page keeps state/timer and start/stop/cancel controls outside its scrollable copy/source region.
Leaving the page, hiding/minimizing/closing its window, screen lock or a two-second missing page
heartbeat cancels. Old IDs and consumed reviews cannot affect a later check. The controller is
separate from dictation, Chat, Whisper, model/network consent and store history. There is no new
preference or automatic recording. Stop currently deletes audio and reports completion; real
meeting storage, long transcription and summaries remain unimplemented. The one-minute limit and
missing outputs are disclosed before the start action; this is not a full-meeting recording claim.

V5.2's basic recording check is ready for maintainer validation. Hardware/compositor sign-off,
long meetings, transcript scheduling and any optional live transcript remain pending. V5.3 must
replace the check-only handoff with bounded local transcription before claiming saved meetings.

**V5.3 Long transcription** (S)
- Build: whisper over the full WAVs with segment timestamps; chunks ≤ 28 s at the quietest 100 ms window
  ([TLP] 138); runs off the dictation path (Q7: the `WHISPER` mutex starves dictation during a long
  transcription; queue meetings behind dictations).

**2026-10-18 V5.3 local transcription backend delivered.** `voice::meeting::transcribe_wav`
reads one already-open PCM16/24/32 or float32 WAV at 8-192 kHz, 1-32 channels. It downmixes and
resamples continuously to 16 kHz with filter state retained across reads, retaining only a bounded
PCM window. Each decode covers at most 28 seconds. The next cut is the middle of the quietest
100 ms window in the last two seconds, preserving every sample and leaving at least 500 ms for
the final chunk. WAVs must contain audio and be at most four hours long; text is limited to 16 MiB
and 100,000 segments. Invalid formats, partial frames, truncated reads and non-finite samples fail
rather than becoming silent success.

The result contains ordered text segments with start/end milliseconds on the original source
clock. Outer amplitude trimming preserves its sample offset; internal pauses remain in place.
Unlike the short-recording noise estimate, this trim does not treat continuous speech as room
noise. Whisper no-speech probabilities, known sound annotations and repetition filtering still
apply. Each track retains its own source clock; no microphone/monitor synchronization is inferred.
These are model-estimated times, not diarization or word alignment. Cancellation is checked
while reading, waiting for inference, decoding and before completion; it returns no partial result.
Progress contains only completed/total milliseconds and the segment count. Errors discard the
accumulated transcript. No dictation dictionary, snippets, insertion or model clean-up runs here.

The shared ggml scheduler retains one active model operation across Chat, preview, dictation,
VAD/model loading, model destruction and GPU discovery. A waiting interactive operation asks the
background meeting decoder to abort through Whisper's callback. That chunk releases its state and
slot, waits behind queued interactive work and retries from the same samples. No interrupted text
is published and the meeting's cancellation flag remains unchanged. Waiting background work
checks cancellation every 10 ms. An abort callback is cooperative, not a hard latency guarantee;
no bound on an individual native inference operation is claimed.

The explicit developer runner uses only an installed model and a supplied, consented WAV:

```sh
cargo run -p vults-voice --features vulkan --example meeting-transcribe -- \
  --model /path/to/installed/model.bin --wav /path/to/consented.wav --language en
```

It writes the transcript JSON to stdout and content-free progress to stderr. It never records,
downloads, deletes or modifies its input, or stores a transcript. The caller retains consent and
the temporary-file cleanup lock. The recording check in Dictation > Meetings still deletes its
WAVs at Stop: app handoff, pinned model/language ownership, cancellable job lifetime, two-track
coordination and a visible transcript remain the next V5.3 delivery. No saved-meeting or completed-V5.3 claim is made.

Validation: 1,140 workspace tests passed with 9 existing ignores, plus 8 evals, workspace
Clippy, formatting, dependency checks and repository/documentation guards. The installed Base
q5_1 model passed the public-domain fixture at two separated source offsets, silence, active
cancellation and reuse after abort. The developer runner also produced bounded timestamped JSON
without changing its input WAV. Deterministic scheduler/callback checks cover interactive priority,
preemption versus cancellation, queued cancellation and unwind release. No renderer changed,
no microphone or desktop output was opened, and the user's app stayed closed. Real-meeting
speech, noise, multi-track alignment and compositor acceptance remain separate.

**2026-10-18 V5.3 bounded app handoff delivered.** The Linux Meetings page now requires an
installed selected voice model, shows its model/language alongside the reviewed source names,
and asks consent for temporary capture plus local transcription. A changed model/language
requires a fresh review before starting and cancels pending capture/processing. The one-minute
limit remains explicit: this is a recording/transcription check, not a full meeting archive.

`Recording::start_processed` closes capture streams and finalizes both WAVs before handing
validated, buffered file readers and its cancellation flag to the app worker. A Processing phase
freezes the capture timer; Stop becomes inert there and Cancel discards the result. Readers reject
symlinks, hardlinks, FIFOs, foreign owners and nonprivate modes. The exclusive temporary-file lock
outlives local transcription and reader drop, including failure/unwind. Success is published only
after cleanup and worker exit. Late successful output cannot override an app cancellation, and
cancelled/failed worker output is consumed and dropped rather than left in a hidden result slot.

The app reuses a cached matching voice model, or admits a cold local load behind interactive work.
A native cold load already in progress cannot be interrupted; cancellation discards its loaded
context and then cleans audio. Native shutdown waits up to three seconds; a hard exit during an
uninterruptible native operation relies on startup recovery. Two WAVs are decoded sequentially,
without dictation transforms or any network/provider request. Admission and chunk progress expose
only track/time/count metadata. No transcript is sent in recurring status snapshots.

Nest alone may read a completed transcript through the ID-scoped `meeting_transcript` command.
The page reads it once, displays inert text with independent source clocks and estimated times,
and preserves source selection, disclosure, text nodes and scroll during polling/theme updates.
There is no inferred synchronization, diarization or word alignment. Transcript details are
collapsed behind the reading content so a 480 x 360 window exposes the source controls and text.
Stop/Cancel/Discard remain outside the scroll area. Missing-model failure links to the existing
privileged Voice editor. New source review, explicit discard, navigation/hide/close, lost page
lease, screen lock and app exit remove volatile transcript copies. History, retention, the
clipboard, agent approvals and the database are not used for this check.

The bounded V5.3 integration is delivered; full-duration recording/meeting lifecycle and the
meeting archive remain future work. V5.4's diarization spike is the next backend step, without
claiming real-meeting or compositor acceptance. V5.7 owns persistence/export and V5.8 the full
meeting workspace. Public guides and safety/settings references describe the current one-minute,
volatile scope rather than those future capabilities.

Validation: 1,146 workspace tests passed with 9 existing ignores, plus 8 evals, workspace
Clippy, the UI build, all 16 renderer checks, formatting, dependency and bilingual documentation
guards. Audio tests hold processing open to prove lock/reader lifetime, frozen recording time,
cancellation, error/unwind cleanup and unsafe-file rejection. App transitions cover source-review
consent, volatile discard and cancellation winning a late success. A cancelled background model
load does not open a file. Native validation used a private Xvfb/D-Bus/PipeWire profile and repeated
public-domain speech only: 20 capture/transcript/lifecycle cases and 5 model-error/layout/editor
cases passed. Both tracks produced local segments and no WAV remained after completion/cancel.
Black/Light at 1180 x 820 and 480 x 360 keep actions above the footer and show source controls/text
before optional transcript details. No user's microphone/output, provider or personal app profile
was used; the isolated processes and generated speech WAVs were removed. Real-input and full
meeting acceptance remain pending.

**V5.4 Diarization spike** (M)
- Build:
  - pyannote segmentation-3.0 + 3D-Speaker CAM++ through `ort` (ADR 0002 allows it), not through the
    `sherpa-onnx` crate (espeak-ng, ADR 0028);
  - agglomerative clustering, threshold 0.7 ([TLP] 146);
  - measure on 3 recorded fixtures;
  - verify the licences (segmentation-3.0 is MIT but gated on Hugging Face: use an ungated, hashed
    re-export; CAM++ Apache-2.0).
- Done when: the go/no-go is written here.

**2026-10-18 checkpoint: delivered as an offline development experiment.** An optional Rust
`vults-spike/diarization` binary loads explicitly supplied, fingerprint-checked ONNX models/runtime,
uses native filterbanks and fixed average-linkage cosine distance 0.7, and emits anonymous turns.
No app integration, recording, automatic download, provider call or persisted audio was added.
Three controlled fixtures constructed from two public recorded voices plus silence produced the
expected 1/2/2/0 global speaker counts. Alternating A-B-A retains identity, but the overlap fixture
has 3.140 seconds of unknown turns and fails to identify A during the overlap interval.
**Go for bounded V5.5 engineering; no-go for shipping automatic speaker labels yet.** Six unit tests,
eight rejection checks, feature Clippy and the all-feature dependency/license guard pass.
The regression checkpoint has 1,146 passing workspace tests (9 ignored), 8 passing evals,
workspace Clippy, formatting and brand/English/i18n/layers/network/README guards.
[The experiment record](diarization-spike.md) contains sources, hashes, license conditions,
fixture construction, raw measurements and the production acceptance gates. These constructed
clips do not establish real PT/EN meeting quality or human-annotated DER. The app stays closed;
no user's audio/profile was used, and temporary experiment weights/audio are removed afterward.

**V5.5 Diarization** (L)
- Build: a production version of V5.4, plus model download entries with SHA-256 and speakers merged into
  segments.

**2026-10-18 checkpoint: offline model admission and conservative text association.**
`voice::meeting::speaker_models` returns only a complete pair of pinned, size/SHA-256-checked
model bytes, with bounded cancellable reads and Unix symlink/FIFO rejection. The optional spike
loads those checked bytes into native sessions without reopening model paths.
`voice::meeting::speakers` preserves original text/times and assigns only a known voice that
covers a whole segment continuously and alone; changes, gaps, overlap and unknown activity keep
it unassigned. It validates source-clock duration, interval order and memory/item/time budgets,
uses an event sweep rather than a quadratic overlap scan, and discards cancelled results.
Fourteen new voice tests and one frozen-output regression replay cover these contracts. A native
smoke run with the tracked 3.4-second public-domain speech fixture finds one speaker from checked
memory; same-size model corruption fails before native initialization.
V5.5 remains open: native worker/preemption, runtime delivery/notices, approved gate-owned model
downloads, source-clock alignment and real PT/EN quality/resource acceptance remain pending.
The regression checkpoint passes 1,160 workspace tests (9 ignored), 7 optional spike tests,
8 evals, workspace/feature Clippy, formatting, dependency/license checks and repository guards.
This adds no app download entry, label UI or native engine dependency to the app. See the
[checkpoint and remaining release gates](diarization-spike.md#v55-checkpoint-offline-admission-and-text-association).

**2026-10-18 checkpoint: bounded native processing and interruption.**
The optional `voice/diarization` backend now owns one admitted, at-most-three-minute analysis of
an already-open source, with meeting-format normalization, content-free phase progress, checked
models and an explicitly pinned native runtime. CPU-only native segmentation/embedding operations
share interactive Whisper's scheduler. A scoped watcher requests ONNX termination for cancellation
or foreground demand; preempted attempts discard their output and retry after interactive work
with fresh options. Cancellation through the final window returns no partial timeline. Native
contexts are released under the shared slot on completion/cancellation/error; the process-wide
runtime library remains loaded. Native construction/filterbanks/sorting have bounded inputs but
cannot be forcibly interrupted, and termination is cooperative rather than a hard deadline.
Nineteen runtime-free backend tests and three explicit native checks pass, including cancellation,
foreground-first retry, normalization/budgets, late-result discard and panic/error cleanup.
The native CLI finds one voice in tracked public speech and zero in the 180-second silence control.
The checkpoint also passes 1,160 workspace tests (9 ignored), 76 optional voice tests (4 ignored,
including the 3 opt-in native cases run separately), the spike replay, 8 evals, workspace/feature
Clippy, formatting, dependency/license checks and repository guards.
[Methods, raw evidence and limits](diarization-spike.md#v55-checkpoint-interruptible-bounded-native-processing)
are recorded separately from meeting-quality acceptance. V5.5 still needs runtime distribution,
notices/model download references, gate/app wiring and source-aligned real PT/EN acceptance. No
labels, models or new requests are enabled in the app by this checkpoint.

**2026-10-18 checkpoint: reviewed transactional artifact installer.**
The optional `voice/meeting-assets` backend and explicit developer command now download the pinned
segmentation/CAM++ graphs and official ONNX Runtime 1.31.0 CPU package with exact notices. A reviewed
manifest, caller gate per HTTP attempt, fixed HTTPS hosts, bounded streams/archives and atomic
private staging prevent incomplete bundles from becoming ready. Revalidation checks all ten
files; version paths include the review hash and preserve existing/loaded versions. Cancelled or
failed stages are removed, and a later explicit attempt recovers crash leftovers. No automatic
repair, update, retry, app request or speaker-label activation is added.
The [bilingual model reference](../reference/meeting-models.md) records source/code/weights
licenses, required notices, sizes and SHA-256 before any app download offer. Entry-point privacy
and native admission now require disabled telemetry before runtime initialization, plus API-off
configuration. Twelve installer tests pass; a complete public install takes six admitted HTTP
attempts and a cached recheck takes zero. Its installed 1.31.0 runtime passes the three native
interruption checks. The checkpoint passes 1,160 workspace tests (9 ignored), 89 optional voice
tests (4 ignored, including the three native cases run separately), the spike replay, 8 evals,
workspace/feature Clippy, formatting, dependency/license and repository guards. [Methods and evidence](diarization-spike.md#v55-checkpoint-reviewed-transactional-artifact-installation)
record the limits. Next: the app-owned explicit install consent, persistent request quota/trace
adapter and install UI; real PT/EN/source-clock acceptance still gates automatic speaker labels.

**V5.6 Summary and to-dos** (M)
- Build: the JSON `{title, summary, key_points, todos[{text, owner, due}]}` from the chat's model through
  gate feature `meeting-summary`, 120 s timeout and 20 attempts per rolling 24 hours (V0.4).
  Off by default (R7): Settings › Dictation › Meetings; while off, the Summary tab says why and
  links there. Calls to a local model retain the same switch and request quota.
- Port: [TLP] 353 (`notes/builder.rs:239-243`).
- To-dos remain meeting content and can be exported; Tasks/W3 stays removed under N0.
- Tests: cassette eval.

### V5.6 delivery sequence

- [x] **V5.6.1 Contract and replay evals:** pure prompt, strict typed JSON, separate source text,
  bounded responses, explicit nullable owner/due and authored multilingual/injection cassettes.
- [x] **V5.6.2 Gated model integration:** use the selected API chat model through `meeting-summary`,
  independent default-off consent, durable 20-attempt rolling-day quota, 120-second deadline,
  cancellation/revocation and complete local-transcript fallback. Local calls keep the same quota.
- [x] **V5.6.3 Settings and presentation:** Settings-owned Meetings consent, disabled Summary
  explanation/link, explicit generation, inert summary/to-do content, four UI languages, bilingual
  guide and native sign-off. No saved meetings or executable Tasks; those boundaries stay unchanged.

**2026-10-19 V5.6.1 handoff.** The maintainer accepted the bottom dictation capsule and authorized
continuing with the roadmap. `chat::meeting_summary` now supplies a pure request/reply contract;
it adds no provider, audio, network, clipboard, storage or task-dispatch call. It reuses existing
message/role types, serde and the already-used Unicode character classification dependency.
A content-free Debug view records only the input size.

Microphone/system text stays whole and separately encoded as user-role JSON. The fixed system
prompt treats embedded instructions as meeting content, preserves uncertainty and the spoken
language, forbids tools/actions, and never unifies source clocks or anonymous voice identities.
Strict decoding requires the complete summary shape and explicit nullable owner/due fields;
unknown/duplicate fields, wrappers, wrong types, controls, excess items and oversized text fail
without returning partial notes. Owner/due values must occur verbatim in a source, with Unicode
word-edge checks for cased letters, digits and combining marks; anonymous voice labels are not
owners, including normalized whitespace and the current translated labels. Chinese retains
unspaced literal matching. This occurrence
check cannot prove a commitment, a correct summary, or the semantic completeness of a deadline.

Authored cassettes cover English, Brazilian Portuguese, Spanish and Chinese, unknown metadata,
source-local IDs, proposals/questions, quoted injection, malformed JSON and fabricated metadata.
The 31 cases include composed/decomposed accented names and reject name/deadline fragments.
The reviewed prompt snapshot makes instruction changes explicit. These are contract regressions,
not live-model factuality or language-quality acceptance. The transcript remains the source of
truth, and invalid model notes must not replace it.

Validation: both contract unit tests and all 10 eval tests pass (including the 31 summary
cassettes and prompt snapshot). Targeted Chat/evals Clippy, workspace formatting,
dependency/license checks and repository brand/English/i18n/layers/network/README guards pass.
Independent code review caught substring/Unicode metadata boundaries; the corrected cases are
covered and the final review reports no further concrete issue. This step changes no renderer;
the previously accepted app is closed.

At this contract checkpoint V5.6 remained **in progress**, with Settings and Summary presentation pending.
The gated backend handoff below follows this pure-contract checkpoint. No user transcript was
sent to a provider, and no setting was enabled.
The independent V5.5 diarization quality gate stays open and production labels remain disabled.

**2026-10-19 V5.6.2 gated backend handoff.** Explicit Nest-only begin/status/cancel commands now
own one summary attempt for the finished in-memory transcript. They preserve independent source
clocks and canonical anonymous labels, validate the complete reply, and never replace source text.
No recording or transcription completion starts a summary automatically. The selected API chat
provider/model is used without tools or CLI configuration; local providers must be literal loopback.

Settings version 39 adds independent, default-off consent bound to the existing durable ledger.
The `meeting-summary` gate charges at most 20 attempts per rolling 24 hours before HTTP, including
local calls, failures and interruptions; changing models, toggling consent, clearing history or
restarting cannot refresh the quota. A lost/replaced ledger fails closed. One 120-second deadline
covers preparation, admission and the call; there are no proxies, redirects or automatic retries.
Page departure/lease expiry, lock, shutdown and provider/model/key changes cancel pending work.
Publication holds the recording slot and settings in the established lock order, checks ownership
again, rejects stale results and releases jobs claimed after a consent change. Summary cancellation
alone leaves the transcript available. Content-free request traces record outcome and token counts.

Review found the detached blocking-admission trace gap; a shared gate reservation owner now closes
abandoned admissions for both summaries and dictation cleanup while preserving their quota charge.
The read/claim/publication race is also covered. Final independent review found no further concrete
issue. Validation: 244 app tests, 115 Chat unit tests and its existing integrations, all 10 eval tests,
targeted app/Chat/evals Clippy, renderer build, Settings/dictation-status checks, formatting and
repository guards pass. English/Portuguese safety and settings references document the new boundary.
This is backend acceptance, not live-provider summary-quality or native UI sign-off; the app remains
closed and no personal transcript was sent. Next is exactly V5.6.3, the Settings/Meetings interface.

**2026-10-19 V5.6.3 interface handoff.** Settings-owned **Meeting settings** now contains the
independent default-off consent, selected API provider/model disclosure, quota/deadline terms and
optional speaker installer. The work page keeps Transcript, Summary and To-dos tabs with roving
keyboard selection. Generation is explicit; cancellation preserves source text, errors offer manual
recovery, and notes/unknown owner/deadline values render as inert text. Leaving for configuration
explains the temporary transcript loss before navigation. Provider/model changes refresh disclosure.
The settings editor preserves newer events over late bootstrap/save acknowledgements and retains
pending/error state through redraws. English, Brazilian Portuguese, Spanish and Chinese are complete.

Native WebKit/KWin checks passed at 1180 x 820 and 480 x 360 in Black and Light, including real
Settings consent on/off and the Nest-only generation guard. Controlled IPC response fixtures cover
ready/generating/cancelled/results/quota error and anonymous metadata without audio capture or a live
provider; the real request ledger contains zero summary attempts. All four UI languages and tab
keyboard focus passed. Summary controls sit outside the scroll area, completed notes precede their
supporting disclosure, and short windows recover reading space without hiding active capture actions.
Background polling no longer leaves the whole live region busy; generation status can be announced.

Validation: renderer build, 244 app tests, app Clippy, the new deferred summary/settings regression
(in CI), existing Settings/installer/surface/geometry checks, formatting and repository guards pass.
Independent code review reports no blocker. User-facing safety, settings, model reference and guide
translations are updated. The V5.6 implementation sequence is delivered; maintainer/live-provider
quality acceptance is separate, and V5.5 independent diarization qualification remains open. No
personal transcript was sent; QA profile/cache and temporary scripts are removed, the app/server are
closed, and existing user/shared caches are preserved. Next: V5.7 meeting storage and export.

**V5.7 Meetings store and export** (M)
- Build: tables `meetings`, `meeting_segments`, `meeting_speakers`; rename people; export Markdown, SRT
  and VTT.
- Tests: exporter golden files.

**2026-10-19 V5.7 delivered: storage/export contract.** Migration 27 adds explicit text-only
snapshots, ordered source-local segments and independently renamed speakers. Saves are atomic and
idempotent; a retry cannot undo a human rename. Reads share one SQLite snapshot; delete/global
history erasure cascades to segments and names and compacts retained text. Markdown groups independent
source clocks; SRT/VTT require an explicit source and skip zero-duration estimates. Untrusted text
is escaped in exports, and malformed/oversized snapshots are refused before any row is written.
Golden files cover notes, source-local identities, untimed estimates and cue/markup injection.
The existing recording check remains ephemeral: connecting explicit save/list/rename/export actions
to the running app belongs to V5.8. No audio is stored by this API and no new dependency is added.

**V5.8 Meetings page** (M)
- Build: the prototype's page (list, Transcript/Summary/To-dos, renamed speakers, export), docs and sign-off.

Delivery sequence:
1. **V5.8.1 Archive ownership/API (delivered, backend only).** Nest-only explicit save/list/read/delete commands;
   typed conversion of the completed source transcripts and notes; immutable first snapshot and
   idempotent retry. No automatic save on recording or summary completion. Global history erasure
   shares the existing history mutex and advances a separate meeting epoch; dictation-only erase
   and retention changes do not revoke a meeting. Deletion revokes the active cached copy before
   IO, including compaction failures, so queued saves cannot restore deleted text.
2. **V5.8.2 Full Meetings workspace (implemented; maintainer sign-off pending).** Connect saved list/detail and Save/Delete, reuse Transcript/
   Summary/To-dos, add source-local renaming and native Markdown/SRT/VTT export, update EN/PT guides
   and all UI catalogs, and validate the running app before sign-off. Invalidate pending list/detail
   reads after deletion as well as global erase; a per-delete action does not advance the global
   archive epoch. Explain that saving freezes the current transcript/notes, and that global history
   erase removes the saved archive too. The existing one-minute capture qualification limit and
   default-off automatic identification remain; this step does not qualify full-length capture.

The first step changed no renderer/Bridge code or visible work page. At that checkpoint recordings
had no save/archive controls. No personal recording or live model request was needed to validate
the archive ownership contract.

V5.8.1 validation: all 1,233 workspace tests pass on Linux (10 documented ignores), including 252 app tests,
85 store tests and eight new archive/retention/notes/timing checks. All 10 eval tests, workspace/eval
Clippy, renderer build, all 18 existing renderer scripts, formatting and repository guards pass.
Independent read-only review found no blocker. No archive write was
made to the user's database, and no recording/model request or renderer change was needed. New
API portability was inspected; Windows execution of this step belongs to its eventual PR checks.

**2026-10-19 V5.8.2 implemented.** The existing Nest work page separates New recording from Saved
meetings. Completed recordings expose an explicit title/save action and immutable-snapshot terms;
the saved list searches titles among the newest 500 entries. Detail reuses Transcript/Summary/To-dos
with independent source clocks, unknown metadata and inert text. Voice names update only one
source-local identity. Delete requires confirmation; pending list/detail/save replies cannot restore
erased content. Global erasure has a separate archive event; lock/departure clears content, name
drafts and private search text. Recording maintenance and summary settings/retry use labeled
disclosures; the shared Save/Generate footer leaves at least 69 CSS pixels to read in 480 x 360.

Export opens one parented native save dialog, requires one source for SRT/VTT, then rechecks lock,
window visibility, erase epoch and the saved row before writing. Cancellation writes nothing. The
existing pure exporters escape text; newly created Linux files use 0600. Deliberate exported copies
remain independent of archive/global erasure. No path or transcript is supplied by the renderer.

Validation: 1,234 workspace tests (10 documented ignores), all 10 evals, workspace Clippy, renderer
build, all 18 renderer scripts and repository guards pass. Added archive/recovery checks cover
single mutations, source-local names, literal text, failed drafts/retry, explicit subtitle source,
confirmed deletion, stale reads and erase/departure cleanup. Independent review found search-text
cleanup and an inaccessible initial-summary retry; both are fixed with regressions.

Native Tauri/WebKit checks used an isolated XDG profile and synthetic database: Black/Light at
1180 x 820, 760 x 560 and 480 x 360, long/missing content, source switching, notes, rename, keep/delete,
Nest-only reads, native chooser cancellation and actual Markdown/system-SRT/microphone-VTT files.
Global erase cleared detail/list while leaving exported copies. The production current-recording
renderer also ran with a controlled backend, including explicit save and all four UI languages in
the minimum window; no microphone, audio model or provider request was used. The native profile,
generated files and app/server processes are removed afterward; user data/models are unchanged.
EN/PT guide and all three UI catalogs are updated. Maintainer/live speech acceptance and Windows
execution remain for their own checks; this does not close the V5.5 speech-quality gate, enable
automatic labels or qualify full-length capture.

---

## V6 Talk with Zeca (K8)

**Goal.** Speak with Zeca and hear him, interruptible.

**Rules that still hold:**
- the realtime model is opt-in, with a key in the keyring, a budget and a trace;
- there is no wake word;
- voice never answers a card.

**V6.1 Turn machine** (M) — implemented 2026-10-19; pure backend, runtime adapters remain V6.2-V6.7
- Build: `voice::turn` (pure), with states Listening → Thinking → Speaking(locked) → Armed → interrupt
  pending → draining → tail grace, plus a generation counter.
  - The lock runs 0.5 s (1.0 s with AEC), **counted from the first audio played**.
  - On barge-in, replay about 260 ms of ring buffer.
  - Desktop tail grace is 300–600 ms, not the phone's 1500 ms.
  - The two-phase speaking end.
- Port: the design, not the code: [PT] 77-138, 325 (`stream_handler.py` ranges cited there).
- Tests: an eval session with fake STT/LLM/TTS driving the real machine ([PT] 283-294); the race cases.

V6.1 implements `voice::turn` with injected monotonic time, opaque non-wrapping generation/probe
tokens and bounded deadlines. A first-consumed-audio event anchors the 500 ms/1 s lock; live suppressed
speech is probed once the lock ends. Each deferred probe has 1.5 s to confirm, and old probe results
cannot confirm a newer attempt in the same response. Only confirmation cancels/clears the old output,
advances the generation and requests 260 ms of memory pre-roll for the new listening turn.

Producer completion and requested playback drain are separate acknowledgements. Drain remains
interruptible, then enters a 450 ms desktop tail grace; speech there starts the next turn without
replaying the old tail. Empty output and tail expiry finish without opening a microphone. Speaker
barge-in defaults disabled. Cancellation/failure clears output, and old generations cannot affect
a restarted turn. Adapters must keep the original action tokens and acknowledge the requested drain;
the pure module grants no microphone, network or permission authority. Actual capture/ring replay,
provider, playback and user-facing integration remain the later V6 steps.

Ten deterministic regressions drive the actual machine with fake STT/model/output events: delayed
first playback/acknowledgement and retained live onsets, both lock durations, suppressed/ended speech,
pending timeout/rejection, stale probes,
generation replacement, output before/after producer completion, duplicate/early drain, tail/new-turn
races, empty output, cancellation/failure and counter exhaustion. Independent review found the
same-generation stale-probe race and a lost onset before delayed playback acknowledgement; both
are fixed with regressions. No audio hardware, provider request, download, settings or renderer behavior changes in this step.

All 1,273 workspace tests pass (10 documented ignores), plus 10 evals, workspace Clippy,
formatting, renderer build, all 18 renderer checks and repository guards. No dependencies changed.
No renderer/native behavior changes require a new GUI run; the app remains closed.

**V6.2 End of turn** (M) — backend engineering delivered 2026-10-19; runtime/speech acceptance remains open
- Build:
  - a silence policy of 1.6 s, or 2.6 s when the last word extends (pt/en lists: conjunctions, fillers,
    articles);
  - optional smart-turn v3 via `ort`: the 8 s window, whisper log-mel port, threshold 0.5, re-score every
    200 ms, hold at most 1200 ms, off after one failure.
- Port: [VA] 194-198 (`TurnEndPolicy.swift:18-55`); [PT] 64-73, 309-310 (`smart_turn.py:86-250`).
- Tests: log-mel parity against stored reference arrays; policy table.
- Licence verified in V6.2.2: the pinned official code/model card declare BSD-2-Clause, correcting the earlier Apache-2.0 assumption.
- Q8: how well smart-turn handles pt-BR is unknown.

V6.2 delivery sequence:
1. **V6.2.1 Policy and bounded semantic coordination** — implemented 2026-10-19. Pure
   `voice::turn::end` waits 1.6 s, or 2.6 s for an English/Portuguese continuation word. Current
   VAD silence and word-count high-water growth both gate completion; empty input/active speech
   cannot commit. Rewrites and repeated end edges cannot keep restarting timers. Optional audio-only
   scores are generation/ticket-bound, paced at 200 ms, with threshold 0.5 and a 1200 ms absolute
   hold from nominal silence eligibility. One invalid score/inference failure disables semantics
   for the rest of the conversation. Speech restarts, new generations and cancellation invalidate
   pending work; completion is once-only. Speaker capture/permission authority is unchanged.
2. **V6.2.2 Semantic model and parity** — implemented 2026-10-19. The optional `semantic-turn`
   backend verifies an exact int8 v3.0 graph and qualified ONNX Runtime 1.31.0 Linux x64 CPU library.
   Code/model licenses are BSD-2-Clause; retained notices also cover the Apache-2.0 Transformers
   reference. Eight-second left-pad/crop and Whisper log-mel preprocessing match full primary NumPy
   arrays, and a real checked-model/runtime run matches reference probabilities. There is no app
   download offer, configuration or capture; conversation/runtime integration follows in V6.3-V6.7.

Eleven policy regressions plus V6.1's ten turn cases cover language/word boundaries, silence before
words, continuing voice, partial rewrites/growth, duplicate VAD ends, score pacing/threshold, hung
inference/hold expiry, stale tickets/turns, new speech, invalid scores/failure latch, cancellation,
clock/counter bounds and a fake detector driving the actual turn machine once. Independent review
clarified the audio-only score contract and tightened the hold anchor so delayed polling cannot add
another wait; current readiness is rechecked before accepting a score. Final review has no blocker.
No renderer, native capture, provider, settings or dependency changes occur in V6.2.1; it has no app
caller yet. Actual speech, semantic-model quality and pt-BR qualification remain open.

All 1,284 workspace tests pass (10 documented ignores), plus 10 evals, workspace Clippy,
formatting, renderer build, all 18 renderer checks and repository guards. No dependencies changed.
No renderer/native behavior changes require another GUI run; the app remains closed.

V6.2.2 evidence: three full `[1,80,800]` reference arrays cover short padding, over-window noise and
the existing speech fixture, plus an exact silence oracle. Maximum feature difference is below
`1e-4` (observed `1.1920929e-7`); four native probabilities match the pinned Python/runtime reference
within `2e-4` (observed identical f32 values). An initial older-runtime probability mismatch was
isolated with the exact reference tensor; qualification now pins both the native library and graph
optimization/thread settings. Other CPU targets still need qualification; these are numerical
oracles, not real-conversation endpoint labels or pt-BR quality acceptance.

Shared exact-file admission and native runtime/session ownership preserve the diarization API.
Review fixed uncancellable foreground queue waits and preprocessing without a deadline; held-slot,
expired-feature, hung-native-work and unwind regressions cover those paths. The policy exposes each
ticket's original absolute deadline, so dispatch delay cannot grant extra inference time. New speech,
turn replacement and cancellation retain their existing revocation authority. Runtime/session construction
cannot abort; running inference receives cooperative termination.
CI's offline speech job now includes semantic-turn in both Clippy and the voice tests, while native
artifact qualification stays ignored by default. Final independent review has no remaining blocker.
The app stays closed and no user audio/model,
profile, setting or agent configuration is changed. See the [artifact reference](../reference/semantic-turn-model.md).
Next: V6.3's gated realtime client and monetary reservations; real speech/latency acceptance remains open.

All 1,285 workspace tests pass (10 documented ignores), plus 10 evals and 147 tests in the exact
amended offline CI feature combination (8 explicit native/hardware ignores). The checked-artifact
native parity test also passes explicitly. Workspace/preview/optional-feature Clippy, renderer build,
all 18 renderer checks, formatting, repository guards and cargo-deny pass. No new dependency version
is introduced; `rustfft` is an existing dependency made directly available to this optional backend.
The QA environment/artifact copies are removed after archiving evidence; no GUI run is needed for
this backend-only step.

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

Delivery sequence:
1. **V6.3.1 Monetary ledger** — implemented 2026-10-19. Migration 28 adds atomic integer
   micro-USD reservations and once-only settlement against the existing ledger identity. Parallel
   work shares one allowance; history erasure/restart cannot refund it. Open holds do not expire;
   unknown final usage debits the full ceiling while actual cost stays null. Settled cost expires
   24 hours after completion. Missing quotes, zero limits, unavailable state and overflow fail
   closed; overruns are charged in full. `voice-realtime` cannot use connector consent or count-only
   admission. Backend foundation only: there is no network call, microphone capture or new setting.
2. **V6.3.2 Bounded protocol and pricing** — implemented 2026-10-19. The offline `chat::realtime`
   owner uses exact reviewed 2.1/2.1-mini rates, conservative context/audio/output ceilings and
   explicit server-confirmed caps. It disables auto responses, tools and input transcription.
   Opaque attempts bind server responses; receipts separate request outcome from billing. Unknown
   or inconsistent usage retains the hold and closes the owner; known overruns are returned in full.
   A synthetic two-response cassette covers cached context and billable cancellation. Rate review
   expires after 30 days; no live API call or provider acceptance is claimed. See
   [Realtime accounting](../reference/realtime-accounting.md) for source contracts and the ceiling proof.
3. **V6.3.3 Runtime admission and transport** — backend implemented 2026-10-19. Fixed-origin
   native-TLS WebSocket provider, bounded 24 kHz PCM channels, independent Settings-only consent
   (schema 41), keyring lookup after durable admission, default US$1 rolling limit (zero disables),
   process-monotonic gate clock, cancellation/deadlines and once-only receipt settlement. Duplex
   frame tests cover two turns, off/spent/key refusal, mid-flight revocation/cancellation, malformed
   audio, backpressure, delayed final settlement and unknown cost; real ledger tests cover detached
   admission and exact remaining allowance. No renderer/capture entry is offered yet. Unresolved
   holds block another reservation; recovery is delivered in V6.3.4.
4. **V6.3.4 Exclusive ownership and orphan reconciliation** — implemented 2026-10-19. A Linux
   kernel lock on the data directory outlives the borrowed socket future, detached blocking workers
   and unclaimed admission guards. A new exclusive owner atomically closes abandoned holds at their
   full ceiling with unknown actual cost, starting a conservative 24-hour window once. Active owners
   refuse entry; identity/schema/write failure changes no accounting. Tests cover live process
   exclusion, forced process death, socket abort, detached handoff, rollback, erased traces, exact
   remaining allowance and no expiry extension on restart. No dependencies or schema changes.
5. **V6.3.5 Native turn retention and local text recovery** — implemented 2026-10-19. Complete
   bounded PCM turns survive admission/transport/response failures independently of the persistent
   provider. Only final success releases the original; queued final receipts precede simultaneous
   connection failures. Local Whisper/resampling returns whole reviewable text, or retained audio
   with a typed missing-model/empty/decode/timeout reason. No automatic Chat send, download or
   provider substitution. Cancellation/revocation discards work; shared Settings mutation fences
   model/language/microphone/Zeca changes against restoration races. Outer/worker native leases
   survive final publication and detached cold loads. Duplex burst and synthetic native regressions
   cover bounded queues, exact failed-turn retention, late receipts, timeout and stale-output refusal.
   Remaining activation gates: wire capture/playback and visible text continuation in V6.4-V6.7,
   then pass live-provider/native acceptance. Full V6.3 remains open until these gates pass; offline
   tests do not claim real speech/provider acceptance. Next engineering step: V6.4 playback.

**V6.4 Playback and what the user heard** (M)
- Build:
  - cpal output with a playback cursor from frames consumed;
  - the heard prefix split by words;
  - history rewritten to what was heard + "[interrupted]";
  - a re-delivery nudge when ≥ 3 words went unheard.
- Port: [PT] 122-129, 312.
- Tests: split math; the history rewrite.

### V6.4 delivery sequence

1. **V6.4.1 bounded output and heard-text contract** (delivered 2026-10-19).
   `audio::playback` owns one CPAL output worker, bounded source PCM, conversion, a predicted
   device-consumption cursor, stop and producer-end versus drain. Cancellation and errors freeze
   output; blocked native teardown retains exclusivity. `voice::turn::heard` preserves UTF-8 words,
   estimates partial-segment text, computes interrupted context and yields one hint for at least
   three unheard words. Explicit producer interruption remains distinct from received-audio drain.
   Pure clock/split regressions and an isolated ALSA null check cover the backend; physical acoustic
   timing is not claimed. There is no app caller, history mutation, microphone or provider call.
   See [Playback and heard text](../reference/realtime-accounting.md#playback-and-heard-text).
2. **V6.4.2 native provider/output/history integration** (delivered 2026-10-19).
   The native owner now binds response/item/content identities and finalized transcript/audio extents
   to generation-bound turn controls and CPAL consumption/drain. Confirmed interruption freezes once,
   cancels an active producer, settles usage and truncates the exact item at floored played
   milliseconds; a matching server acknowledgement precedes continuation. In-memory history keeps
   only estimated heard words, with a quoted one-shot repair hint for the next admitted response.
   No tool is enabled/replayed. Terminal output may drain or be interrupted; failed acknowledgement
   yields explicitly unsynchronized local history and closes the owner. Native workers retain the
   session lease. Offline socket/owner regressions and an actual CPAL null-driver check cover stale
   events, cancellation, ack failure, completed-production drain, identity changes and recovery.
   The app entry retains disabled barge-in pending V6.5. Capture/UI remain V6.7; physical/live
   acceptance stays an activation gate. Backend V6.4 is delivered; full acceptance remains open.
   See [Native playback and context repair](../reference/realtime-accounting.md#native-playback-and-context-repair).

Next engineering task: **V6.7 capture/UI**, after the V6.6.3 guarded-backend publication checkpoint below.
Codex consultation remains unavailable; its activation qualification is carried separately.

**V6.5 Echo cancellation** (M, spike first)
- Preferred: PipeWire's WebRTC engine with both the clean source and paired reference sink.
- Vults does not load modules silently. Settings explains and offers setup with a click, or the
  user runs it (Q6 remains open).
- An embedded `webrtc-audio-processing` alternative needs evidence that the native route is inadequate.
- Without qualified AEC, speaker barge-in is off; headphones require an explicit user choice.
- Port: [PT] 140-172, 326.

### V6.5 delivery sequence

1. **V6.5.1 native inventory and isolated spike** (delivered 2026-10-19).
   Read-only local Pulse discovery requires explicit WebRTC module ownership, matching source/sink
   group and stable object/server identities. Rechecking is a snapshot, never route authority.
   A private PipeWire/WirePlumber experiment feeds synthetic echo and two near-end tones through
   the actual installed native module, with private configuration/scripts/state, no hardware
   monitors or desktop routes and fixed acceptance thresholds. Three runs pass the synthetic gate;
   physical speech/double-talk/latency acceptance is not claimed. Native-only modules without Pulse
   ownership metadata are unsupported by this first inventory. No extra dependency, production
   module loader, app capture or barge-in activation is added. See
   [Echo cancellation](../reference/echo-cancellation.md) for the runnable experiment and evidence.
2. **V6.5.2 pinned duplex routing and device policy** (delivered 2026-10-20).
   One native libpulse worker acknowledges exact input/output targets, forbids moves and watches
   module-owned AEC connections plus physical endpoint identities/ports. Changed routes revoke the
   session; default changes cannot redirect it. Content spans exclude silence, subtract reported
   sink latency and require a separate successful drain acknowledgement. Stop freezes the cursor;
   native teardown and callbacks retain ownership until cancellation/delete acknowledgement.
   Explicit headphones may bypass AEC; ordinary speaker/CPAL paths remain non-interruptible.
   The app's separate native entry checks consent/revocation before capture and forwards bounded
   memory-only Input events, without automatic turn assembly, upload or persistence. Unverified
   routes invalidate retained speech/probes before deferred events. Three consecutive isolated
   native runs and clock/ownership/revocation regressions cover the backend. Backend V6.5 is
   delivered; next is V6.6. Resolve Q6 before production setup; Vults loads no system modules
   silently. Capture controls/turn assembly stay in V6.7. Physical/live speech, double-talk,
   acoustic timing, hardware hotplug and long-session acceptance remain activation gates.
   See [Routed validation](../reference/echo-cancellation.md#routed-validation).

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

### V6.6 delivery sequence

1. **V6.6.1 local commands before remote admission** (delivered 2026-10-20).
   A native complete turn may bind a finalized local transcript (at most 4 KiB); Linux admission
   requires it before remote upload. Capture/transcription assembly remains the V6.7 caller's job.
   The one runtime state owner reuses V8's transactional dispatcher and cached flock reads, with a
   separate conversation's repeat/undo/clarification context. Command enablement, realtime consent,
   cancellation, Settings epochs and an absolute three-second admission expiry are rechecked under
   Settings/admission locks. Dropped waiters revoke queued work; consumed requests cannot run twice.
   Local results retire the input generation without Generate; duplicate/stale captures cannot
   upload or replay. Only NotCommand may proceed remotely. Refused cards, unavailable/disabled
   commands and clarifications are consumed locally. External effects return Requested after being
   submitted to the existing executor, never a false completion claim. No automatic model fallback
   follows a consumed command. Missing transcripts fail before provider construction and may use
   only the existing reviewable local-text recovery. No renderer, capture, provider tool or agent
   delegation is activated. Native-owner/dispatcher/turn regressions cover these paths; physical
   speech/provider acceptance remains open. See
   [Local commands before realtime upload](../reference/realtime-accounting.md#local-commands-before-realtime-upload).
2. **V6.6.2 bounded realtime flock tools** (delivered 2026-10-20).
   The native app offers only the existing five cached flock reads through frozen confirmed specs.
   One complete successful function-only response precedes each read; identities, arguments,
   stream/final agreement, duplicate keys and replay sets are checked before execution. Each known
   receipt settles before the next independent reservation and local read. Up to eight reads share
   one 120-second chain deadline; arguments/deltas/results are bounded, including JSON escaping.
   Inert error/outside/truncation flags carry results without authority. A consumed tool receipt
   releases original input while playback stays Thinking; final audio/history use their own identity.
   Failure never re-uploads input or retries the consumed call, and reused tool attempts cannot queue
   audio. Offline protocol/duplex/native-owner regressions cover these paths and exact offerings.
   No action/permission tool, renderer entry, microphone or live provider is activated. Next is
   V6.6.3. See [Read-only tools](../reference/realtime-accounting.md#read-only-tools-in-realtime).
3. **V6.6.3 agent consultation and completion semantics** (backend publication checks accepted;
   Codex activation remains unqualified).
   - **Completion receipts** (implemented 2026-10-20). Core-only changes complete at commit;
     project persistence completes only after a successful save. Native launches distinguish
     submission from completion: starting an editor or submitting a window-activation request
     reports Requested, not Done. Known failure reports that some changes may already have happened.
     Every effect receipt is checked, including mixed cached-read/action batches. Receipt waiting
     is bounded to two seconds within the original three-second owner deadline; queued work
     rechecks the exact authority, waiter and expiry and retains the exclusive lease through exit.
     A consumed capture is shared across clones before admission unlocks. Lost acknowledgements,
     timeout, tool settlement and confirmed interruption cannot recover, relabel or replay it.
     Offline regressions and the exhaustive H1 boundary cover this slice without desktop actions,
     microphone, provider or renderer activation. See
     [Native command receipts](../reference/realtime-accounting.md#native-command-receipts).
   - **Claude Code text consultation** (implemented 2026-10-20; native backend only).
     Optional `consult_claude` reuses the existing tool-free text client with an independent
     default-off Settings consent, durable 20-attempt rolling quota and content-free local traces.
     Only its bounded question is supplied; user/project settings, skills, file mentions and memory
     discovery are disabled. No active agent session, Chat history or permission owner is inherited.
     Cancellation and the translated 1.5-second native notice cover admission and the CLI from the
     original call time. The busy slot and voice lease survive preparation and child teardown;
     late replies cannot publish. Current clients must support the isolation controls (Claude
     Code 2.1.299+ in the 2.1 series); organizational managed policy still applies. Native notice
     display and the opt-in UI stay in V6.7; no live model/microphone acceptance is claimed. See
     [Text consultation](../reference/realtime-accounting.md#text-consultation).
   - **Codex tool/context isolation** (qualified helper implemented 2026-10-20; not offered).
     Keep the user's installed CLI authentication; do not introduce an API-key route. The original
     restrictive exec candidate and App Server with empty environments both fail the local probe.
     Feature switches alone do not override model metadata. The new helper supplies a minimal
     app-owned catalog with disabled shell, patch, search and collaboration capabilities, fixed
     instructions, disabled skills/context injection, ignored user config/rules and no saved session.
     Only exact CLI 0.163.0 may run it; unknown/upgraded clients fail closed. JSON-quoted input
     escapes dollar/at-sign markers before the CLI preprocessor and preserves the decoded question:
     hiding the skills catalog alone does not stop explicit skill expansion. A canary test covers
     project instructions, files and named skills; the actual model request offers zero tools and
     no canary context. A fixed stand-in response passes through the real CLI and strict text-only
     reply parser. No external provider, real credential, microphone or desktop action is used.
     Cancellation/output bounds reuse the acknowledged-child runner.
     A separate synthetic-auth mount test proves the ChatGPT-only status preflight refuses API
     login without modifying its auth; HOME/CODEX_HOME values are never reassigned. This preflight
     is not an atomic binding against concurrent external login changes.
     **Native backend integrated 2026-10-20:** independent default-off Codex consent, Settings-only
     writes, durable 20-attempt rolling quota and metadata-only traces now reuse the native owner.
     Both providers share a busy slot through acknowledged child exit; consent/quota separation and
     cancellation ownership have offline regressions. There is no renderer/capture entry.
     **Activation guard enforced 2026-10-20:** one false qualification capability now refuses
     Codex opt-in before settings/ledger IO, omits its tool even with saved consent, closes its
     network gate and refuses the public helper before CLI/scratch access. Turning a saved choice
     off remains allowed. The isolated synthetic-auth regression now also reproduces forced-exec
     deletion, without exposing original credentials or contacting a provider. This is containment,
     not a subscription-binding solution. Ephemeral credential storage does not import the existing
     login; URL overrides are not authentication-mode guarantees. Do not substitute either.
     **Managed-policy candidate probed 2026-10-20:** system-only
     `allowed_login_methods=["chatgpt"]` constrains credential loading without forced-exec logout.
     Nine synthetic scenarios isolate both home and system policy: the negative control sends its
     API bearer, managed API attempts omit it and preserve auth, ChatGPT status succeeds, and API
     replacements before fresh exec or after a first-request 401 never send the replacement key.
     The 401 scenario observes a retry with the original synthetic ChatGPT bearer, then a fixed 400;
     no model/provider or real auth is used. A handshake retry/fallback is not proof of auth reload.
     This is file-store candidate evidence, not activation.
     Runtime effective-policy/protected-provenance checks, keyring, broader refresh/failure cases
     and live compatibility are still required. Never install system requirements automatically;
     unmanaged machines remain blocked. See
     [Managed-policy candidate](../reference/realtime-accounting.md#managed-policy-candidate).
     **Next:** qualify subscription-only behavior during concurrent external login changes before
     visible activation. App Server account/read has no stable identity for binding a separate exec.
     A synthetic-auth review probe confirms forced login on exec deletes mismatched API auth. Do not reuse Claude's consent, force a login
     mode that may log out an existing account, or claim live/managed-client acceptance. Whole
     Codex activation remains open. V6.7 engineering may proceed with that feature disabled.

     Reproduce the explicit installed-client checks on Linux (requires user/network namespaces
     and `ip`/`bwrap`; ordinary workspace tests skip these four client probes):

     ```sh
     unshare --user --map-root-user --net sh -c 'ip link set lo up && cargo test -p vults-chat consult_codex -- --include-ignored --nocapture'
     ```

     Baseline candidates must remain rejected; the exact helper must offer no tools, exclude
     canaries and accept only the fixed text reply. Passing does not authorize runtime activation.

### V6.6.3 publication checkpoint (2026-10-20)

The user requested consolidated verification and publication before V6.7 engineering. The release
scope is the tested native backend, saved meetings and verified question flows; it does not enable
unqualified Codex consultation or claim physical/live voice acceptance.

- The actual microphone Settings setter now revokes realtime ownership under the Settings lock
  after successful persistence. An isolated setter-level regression covers restoration, unchanged
  choice and failed save. The earlier helper-only test did not cover this production writer.
- Fresh isolated `configRequirements/read`, `account/read(refreshToken=false)` and `config/read`
  verify absent policy, ChatGPT-only policy and required file/keyring stores. The host has no
  `/etc/codex/requirements.toml`; it is not eligible for the managed route. No policy is installed.
- Required keyring with DBus unavailable returns no account, sends no stored API bearer and never
  falls back to an otherwise-valid synthetic ChatGPT file; the same file is accepted with file
  storage. Neither file is altered. Successful/locked OS-keyring behavior is not qualified here.
- A custom OpenAI-auth-required HTTP-only stand-in exercises actual POST401 for managed and
  unmanaged synthetic ChatGPT-to-API replacement. Both terminate after one POST without sending
  the replacement API key. This is bounded refusal evidence, not proof of credential reload or
  successful renewal; it cannot enable the Codex activation capability.
- Independent full-branch review found no additional P0/P1 issue. Its microphone invalidation,
  missing question-race CI check and stale changelog findings are resolved. All 18 renderer checks
  now run in CI. No microphone, model, original auth/keyring or agent config is used by these probes.

**Disposition:** publish Codex consultation disabled behind all four existing guards. Keep its
policy/provenance, successful keyring, credential renewal and live compatibility qualification open.
Proceed to V6.7 engineering with that unavailability explicit. V6.7 must wire reviewed capture and
controls, exercise physical/live acceptance with separate consent/budget, and run native UI review
before K8 can be marked Done. No gate is silently waived or relabeled as passed.

**V6.7 Talk with Zeca page and island speaking states** (M)
- Build: the prototype's *Talk with Zeca* page under Voice (an opt-in switch saying where audio
  goes; on, the model, budget, end of turn, interrupting and *Answer without the key*, off by default; "never
  answers a card" always shown) and the island `speaking`/`interrupted` lines; docs; sign-off; K8 moves to Done.

### V6.7 delivery sequence

1. **V6.7.1 Settings bridge contract** — implemented 2026-10-20. The Settings-only
   `voice_realtime_settings` command reads the saved switch, model and exact microdollar limit
   without opening a device, keyring, ledger or network connection. Its response omits the private
   budget identity. The renderer bridge exposes this read and the existing explicit Settings write;
   unsafe/fractional JavaScript amounts are rejected before invocation. Native model, budget and
   webview authority checks remain decisive. This is configuration plumbing, not a live voice UI.
2. **V6.7.2 Configuration page** — implemented 2026-10-20. Dictation > Talk with Zeca exposes
   opt-in, destination, model and budget in the existing Settings webview. Exact decimal-string writes
   retain large saved limits. Saves require readback; errors keep drafts, foreign changes require
   explicit reload and navigation preserves edits. Voice never answers permission cards and Codex
   remains unavailable. No recording or live conversation control is exposed in this slice.
3. **V6.7.3 Capture and conversation controls** — in progress. Native turn assembly implemented
   2026-10-20: completed 16 kHz capture plus finalized local words becomes bounded 24 kHz PCM16
   through the shared resampler, with exact duration and finite/saturating sample checks. Recovery
   reuses those same words without another model load/decode. No microphone or renderer entry is
   exposed by this adapter. The explicit native capture owner is also implemented: start is
   opt-in, Finish closes the microphone before local decoding, and Cancel/controller loss revokes
   the whole session. The same desktop lease survives delayed native teardown. The native
   bridge and controls are now implemented in Dictation > Work > Voice conversation: one explicit
   Start/Finish/Cancel turn, 500 ms status contact with a three-second native deadline, local reply/
   heard text and editable recovery copied only on a click. The page uses Nest authority; Settings
   keeps configuration ownership. Wide/narrow Black/Light native UI checks use controlled owners.
   Manual continuation now retains the same provider driver/context: after playback drains or a
   local command completes, the page shows Microphone off and offers Record another turn or End
   conversation. Only that explicit click rearms capture for the next native Listen generation;
   idle, stale/duplicate requests and provider events cannot reopen it. Page contact, cancellation,
   settings/key changes and the desktop lease span the whole conversation. Capture/decode for the
   next turn runs alongside the retained provider driver. Optional local silence completion now
   reuses V6.2's policy with the installed VAD and GPU preview, scoped to a real Listen generation
   from the retained machine (including the first turn). Default-off per-conversation selection is
   frozen before Start. Fresh capture, confirmed speech and partial words gate the 1.6/2.6-second
   policy; final decoding/upload still follow microphone closure. Missing/failed detection or
   preview visibly falls back to manual Finish. Cancellation, the cap and next-turn click still
   hold. The semantic model remains off. V6.7.3 remains open for semantic and duplex/barge-in
   integration. An explicit Stop response click now stops audible output through the existing
   acknowledged truncation/heard-prefix repair path, even with voice barge-in disabled. It opens
   no microphone and waits for heard-text completion before the next-turn action. Duplicate/stale
   clicks cannot interrupt a different generation. The native routed entry now connects fresh VAD
   evidence and token-bound scratch transcription to the existing interruption machine. Its
   eight-second ring emits the retained onset/pre-roll in a typed Replay event after the new Listen;
   it does not assemble/upload a following turn. One bounded worker retains its lease through real
   teardown; route/model preflight, cancellation, stale generations and probe deadlines fail closed.
   This is native-only: the Nest keeps manual capture. Next: routed turn assembly and explicit
   route-selection/consent UI, then V6.7.4
   island feedback and physical/live conversation sign-off.
4. **V6.7.4 Island states and acceptance** — connect speaking/interrupted feedback, review native
   layouts and accessibility, then exercise microphone/provider/interruption acceptance before
   declaring K8 Done. Codex activation keeps its independent qualification gates.

---

## V7 MCP `vults_dictate`

**V7.1 ADR 0028 / M7 alignment** (S) — delivered 2026-10-19; decision only
- Build: record that a *question* card (M7's `ask_user`, a card answered with text) may be filled by
  dictation and is sent with a click or Enter. Permission cards never take voice. ADR 0027/G1 is
  unaffected.

The [living voice record](../adr/0028-spoken-replies-through-a-realtime-model.md#human-confirmed-question-drafts)
now distinguishes typed question drafts from permission decisions. Capture is user-started and
local; only a separate click or unmodified Enter sends the final editable text. The verified client,
exact question, generation and lifetime own capture and reply. Lock/departure, replacement, expiry
and disconnect revoke pending work; replies are consumed at most once and audited without a second
transcript. IME/Shift+Enter and literal approval-like text cannot trigger permission authority.

This is the next module after V5 in the authoritative delivery order; V6 has not started. At the
V7.1 decision checkpoint, M7's read/proposal server had no question tools. V7.2.1/V7.2.2 below deliver
the typed lifecycle, verification, audit round trip and composer; V7.2.3 below connects local capture.
V7.3 owns external-agent setup and its default-off switch. V7.1 itself changed no runtime or config.

Validation: the independent authority/EN-PT review found no permission expansion or false shipped
claim. Its wording fixes distinguish local Whisper inference from remote audio and explicitly
exclude provider clean-up from question drafts. All 1,234 workspace tests (10 documented ignores),
10 evals, workspace Clippy, renderer build, 18 renderer checks and repository guards pass; reviewed
translation marks and changed-page links/fragments pass. There is no renderer/runtime change to
exercise in a native app. The safety guide also corrects the old ephemeral-only meeting statement
to match V5.8's explicit text saves and independent exported copies. The app stays closed.

**V7.2 The tool** (M)
- Build:
  - `vults_dictate(question)` in `app/src/mcp.rs`, building on M7's `ask_user`;
  - the client is identified (the process tree, not a self-asserted header);
  - an audit line;
  - the island shows the question; the mic opens only on the key; the transcript is shown and sent on
    confirmation.
- Tests: an MCP round trip with a fake agent; the audit row.

### V7.2 delivery sequence

1. **V7.2.1 Lifecycle and requester proof** — implemented 2026-10-19; backend only.
   `core::questions::Slot` retains one typed question and editable draft per authenticated
   connection, with an absolute 120-second deadline, a fresh app-owned connection nonce, process
   incarnations, request/question binding and a monotonic generation. Confirmation consumes the
   draft once; replacement, cancellation, expiry, disconnect, departure and lock can revoke it.
   No text is interpreted as an action or permission. Replay protection retains at most 256 request
   ids per connection and then requires a new connection; invalid input preserves the live draft.
   Linux `peer::requester` captures kernel socket credentials or the real stdio process ancestry,
   skips only verified system-shell executables while retaining their wrappers, and rechecks
   same-user PID/start time, parent and executable identity. Unrecognized wrappers stay the client.
   It refuses unavailable, dead or changed processes. Content-free receipts convert to the existing
   audit format; a store regression verifies persistence without prompt, draft or raw RPC id.
   These are library APIs: the app does not invoke them or advertise a question tool yet.
2. **V7.2.2 Text question round trip** — implemented 2026-10-19; typed replies only. Connect the verified relay and typed lifecycle to
   `app/src/mcp.rs`, the runtime-owned composer and the durable audit. Revalidate proof before
   publishing and confirming, generate the connection nonce in the app, revoke on every departure
   event and process exit, and handle cancellation, delivery failure and lost acknowledgement
   without retrying a consumed reply. Test a real MCP round trip with a fake agent, including
   stale clients and duplicate/late input. Preserve existing permission and hook-question paths.
3. **V7.2.3 Local dictation into the draft** — implemented 2026-10-19 after the text round trip. Route only the physical
   dictation key to the selected question; keep capture and confirmation separate, preserve manual
   edits, IME/Shift+Enter behavior and all cancellation gates, and validate the native island/composer.
   No provider clean-up, chat fallback or spoken permission answers. The default-off external-agent
   offering and config changes remain V7.3.

V7.2.1 validation covers draft-only input, once-only confirmation, every revocation, exact deadline
boundaries, connection/process/question binding, replay limits, live Linux credentials/ancestry,
process exit and metadata-only audit storage. This backend slice starts no microphone, UI,
network request or MCP listener, and left the app closed. At that checkpoint, tool/round-trip
acceptance remained pending with V7.2.2/V7.2.3; see the subsequent implementation record below.

Validation: all 1,246 workspace tests pass (10 documented ignores), along with 10 evals, workspace
Clippy, the renderer build, 18 renderer checks and repository guards. Independent code review
caught mutable process names being used to skip shell wrappers; the fix checks the resolved,
root-owned system executable identity and rejects multicall aliases. The spoofed-name regression
passes and the final review has no remaining blocker. Native UI validation does not apply to this
library-only change; no audio/model request was made.

### V7.2.2 implementation record

The Linux stdio server now offers `ask_user` only to a live client started directly by Vults,
including past verified system-shell wrappers. A separate private socket authenticates the relay
with kernel credentials and exact executable identity; the runtime rechecks the entire proof.
External clients remain refused even with a manual MCP config. The async reader keeps cancellation
and ping responsive while a human edits, binds cancellation to the exact typed RPC id, rejects replay
and bounds both wire messages and the per-connection request history. Lifetimes start when transport
receives the request, so queueing cannot renew the deadline. No agent configuration was changed.

Quick and Nest share one retained text editor, four-language labels, UTF-8 byte limits and a separate
Send/plain Enter action. IME, modifiers and repeat cannot confirm. Permission cards take priority.
Capture admission is blocked on this typed-only path; there is no microphone button or `vults_dictate`.
The runtime owns draft/connection/surface leases and consumes confirmation before an off-loop durable
audit write. It can revoke while that write is pending, and checks the original deadline again before
delivery. The relay acknowledges only after flushing the MCP result; failure, lock/departure or a
missing acknowledgement after consumption reports uncertain delivery without offering resend.

Code review found and fixed stale departure/replacement reads, expiry during audit, capture through
the separate command key, permission priority in Nest and an escaped-envelope size mismatch. A
runtime-ordered Leave acknowledgement now fences queued renderer events; an older in-flight Status
cannot suppress its follow-up refresh. Prompt/draft text clears locally on departure. Native QA found
long Quick prompts growing past the mapped surface; a bounded card now scrolls the prompt while
retaining the editor and actions, including the minimum Nest.

Evidence: six Rust tests use a real parent/agent/relay process chain and MCP/Unix transport, including
responsive ping, exact-id cancellation, once-only human confirmation, durable content-free audit,
failed audit, stale/wrong-surface input, owner refusal, lost stdout acknowledgement, lock/departure,
expiry and page lease. All 1,252 workspace tests pass (10 documented ignores), plus 10 evals, workspace
Clippy and cargo-deny. The dedicated renderer check exercises manual edits, stale acknowledgements,
IME/modifiers, byte limits, ordered departure and delayed replacement/status. Native production UI
checks cover Black/Light, Quick, Nest at 1180 x 820 and 480 x 360, long prompts and all four languages,
using controlled question responses without audio/model calls. Renderer build, all 19 renderer
checks, formatting and repository guards pass. The final review has no remaining blocker; the app,
Vite and isolated QA config/data/cache/runtime are stopped/removed. Maintainer visual acceptance
remains separate. V7.2.3's local draft dictation is next; V7.3 owns the default-off external-agent offering.

### V7.2.3 implementation record

Implemented 2026-10-19. `vults_dictate(question)` aliases `ask_user` for verified app-owned Linux
clients; neither tool starts a microphone. Only a new physical dictation-key press grants a capture
bound to the selected question token, recording generation and original deadline. Existing holds,
repeats, the command key and gestures from another recording cannot grant it. Sending retains key
ownership until its terminal acknowledgement; a fresh later ordinary press resumes ordinary dictation.

Capture uses the saved input and installed local Whisper model, with 60-second/remaining-lifetime
bounds, checks before opening/playing and lifetime cancellation in the audio thread. Audio stays in
memory; the ordinary provider/cleanup, command, clipboard, insertion and history pipelines are bypassed.
A single busy guard survives cancellation while an uncancellable native model load drains. The runtime
never waits for audio/model work. Lock, replacement, expiry, departure, client loss, lease failure,
settings changes and rebinding cancel outstanding capabilities and discard late results.

The production retained editor stays editable during recording and transcription. Final words append
to its current manual draft, wait for IME composition to end and must fit the combined UTF-8 limit.
A matching apply acknowledgement clears the recording barrier; neither release, silence, a completion
nor spoken "send" submits text. Cancel dictation keeps manual edits. Send still needs its separate
click/plain Enter and durable content-free audit/relay acknowledgement. Failed/oversized results do
not replace the draft, and no consumed reply can be retried.

Evidence: ten question/key tests include real MCP/process delivery through the alias, blocked early
Send, edits during decode, wrong apply generations, cancellation, replacement and lock. The audio
check verifies revoked admission without accessing hardware. Renderer regressions cover IME-deferred
append, duplicate/stale Ready, failed/oversized apply, separate Send and replacement acknowledgements.
Independent review fixed missing dictation view revisions, Sending shortcut ownership, a late shortcut
binding bootstrap and a non-Linux cfg warning. Final ordinary worker admission and Chat registration
share the question fence; revoked microphone epochs cannot revive. A matching renderer Status must
acknowledge the displayed question before it becomes a dictation target. Final review reports no
remaining blocker.

All 1,256 workspace tests pass (10 documented ignores), plus 10 evals, workspace Clippy, renderer
build, all 18 renderer checks, formatting and repository guards. Native controlled-response checks
exercise Black/Light Quick/Nest, 480 x 360 actions/editor, recording/transcribing states, all four
languages and manual text preserved through final append. No automated event starts hardware capture;
real-human speech/model/device acceptance remains open. QA uses an isolated profile and is stopped/
removed after validation. External-agent opt-in/config installation remains V7.3.

**V7.3 Offer it to agents** (M) — implemented 2026-10-19; real-client/speech acceptance remains open
- Build: add the tool to agents' MCP configs only through `agent-config` (rule 3: backup, diff, click); the
  switch "Agents may ask for a spoken answer" in Settings › Dictation › Agents (off).

The dedicated Linux `vults --questions-mcp` server exposes only `ask_user` and `vults_dictate`.
External requests require the saved default-off choice and verified live requester proof, including
when someone configures a server manually. Consent generations reject queued requests from an older
grant; turning access off cancels pending external questions and capture. Runtime transitions and
the actual audited reply handoff hold the same consent guard. App-owned Chat questions keep their
existing authority; this setting grants neither permission answers nor automatic microphone access.

Settings offers reviewed setup, update and removal for Claude Code and Codex. Plans bind the exact
config, diff and fingerprint; a separate click uses `agent-config` for dated backup and atomic write.
Other servers, hooks and Codex trust entries are preserved. An installer ownership marker allows a
reviewed executable-path update without granting question authority. Removal remains available with
access off. Stale reads, consent events, navigation and overlapping refreshes cannot restore an old
diff or enable its Apply action. All user-facing strings ship in four languages; EN/PT guides and
ADR 0028 document the current behavior.

Evidence: real-process MCP regressions cover external discovery, default-off runtime refusal,
on/off/on generations, audited cancellation, serialized consent/reply handoff and once-only confirmed
delivery. Temporary config fixtures cover foreign entries, stale plans, preserved servers/trust,
managed path updates and exact backups. Renderer regressions cover review-before-write, duplicate
Apply, stale navigation/consent/refresh responses and translations. Independent review reports no
remaining blocker. Native Black/Light checks cover 1180 x 820 and 480 x 360, reachable scrolled actions,
all four languages and the actual privileged setting RPC in an isolated profile. Config responses
in native UI QA are controlled fixtures; no real agent config is written. The QA app/server and
profile are stopped/removed. Full agent sessions and real-human microphone/speech acceptance remain
open; those release gates are not replaced by fixtures. Next engineering phase: V6.

All 1,263 workspace tests pass (10 documented ignores), plus 10 evals, workspace Clippy, renderer
build, all 18 renderer checks, formatting and repository guards. No dependencies changed.

---

## V8 Voice commands (R9)

**Goal.** Move around the flock and read it by voice, instantly and with no model, on a key of its own.

**User-visible outcome.** Hold Ctrl+Alt+Z, say "open api", "who is waiting?", "mute web… and docs too…
undo", let go: the island and its caption show what was done or the answer. "Approve" is refused with how to
answer a card.

**V8.1 Normaliser and phrase tables** (S) — implemented 2026-10-18; runtime not connected
- Build: in `vults-dictation::command`: lowercase, accents and punctuation folded, politeness and leading
  fillers stripped ("você pode", "por favor", "can you", "please", "uh", "então"), infinitives to imperatives <!-- check-english:allow -->
  (pt/en/es tables), split on "e depois/and then" only when every part parses.
- Port: [VA] 5.2 steps 1–6 and the multi-action rule (rewritten; French tables replaced by pt/en/es).
- Tests: table tests per language.
- `dictation::command::{normalize, parse}` now provides lexical Unicode NFD/accent/case/
  punctuation folding and anchored clause-only filler/politeness/verb handling. English,
  Portuguese and Spanish navigation, project preferences, reads, repeat and undo phrases return
  typed descriptions with unresolved target names; the crate has no core/IO/model dependency.
- Explicit whole-word sequence connectors (`and then`, `e depois`, `e entao`, `y luego`,
  `y despues`) return a batch only when every clause parses. A permission/card-answer phrase
  refuses the whole bounded utterance; unknown/incomplete/control-bearing input returns no
  batch. The limits are 4 KiB before normalization and eight commands. Lexical name keys keep
  words such as polite prefixes or infinitive verbs; only clause parsing rewrites them.
- Tests were added before the module (the expected unresolved-module failure is recorded).
  Eight table/boundary tests cover all three languages, project preferences, explicit sequences,
  unknown/refused clauses in every position, missing targets, whole-word matching, Unicode
  decomposition/idempotence, controls, negation and size/batch bounds. Existing dictation
  gesture/text tests stay separate. No runtime shortcut, microphone, dispatcher, remembered
  context, UI control or network request is added; native UI validation is not applicable.
- The new `feat/n3-voice-commands` worktree starts at merged PR #371 (`c717d8c2`), preserving
  its implementation. Reviewing that PR's completed CI found two Windows compile failures:
  an unguarded Tauri mock test despite `test` being a non-Windows dev feature, and an ambiguous
  `into()` producing an unsized `str` in the non-Linux diagnostic branch. Match the existing
  mock-test target guard and use an explicit String; these are build fixes, not runtime changes.
  Linux checks do not establish Windows CI success; the next Windows run must confirm it.
- Verification: the dictation crate's 37 tests (eight new grammar tests), workspace Clippy with
  warnings denied, formatting, UI build and four relevant renderer checks pass. Repository
  brand/English/i18n/layer/network/README/version guards, all 35 reviewed documentation pairs and
  the offline link/fragment check pass. Full regression: 1,029 workspace tests passed (eight
  opt-in/native tests ignored), plus six feature-enabled eval tests.
  No dependency or visible renderer changed. The app stays stopped and generated caches are
  cleared after verification; the source worktree remains for the next step.
- This checkpoint handed off to **V8.2 Name resolver**. V1/V2/V4 acceptance and their real
  microphone/compositor/latency checks remain open; implementing a parser does not close N3.

**V8.2 Name resolver** (S) — implemented 2026-10-18; runtime not connected
- Build: aliases (Your words replacements whose target is a project or agent name), exact name, a single
  whole word, then Levenshtein (1 for names up to 4 letters, 2 above); a runner-up within the same distance
  makes it ambiguous and the island asks *Which one?*; the answer within 90 s resolves it.
- Port: [VA] 5.3.
- Tests: "apy" → ambiguous between api and api-gateway; "cloud code" → Claude Code through the alias.
- Needs: V2.1.

Implementation and evidence (2026-10-18):
- `vults-dictation::command::names` owns a snapshot of typed project/agent identities and eligible
  Your words replacements. Only a complete known target becomes an alias; replacement text is never
  executed or chained. Priority is alias, exact full name, a single whole word, then the closest
  bounded Unicode Levenshtein match. Short normalized queries (up to four characters) allow one edit,
  longer queries two; a single-word query compares full names and their individual words.
- Normalized alias collisions and equal-distance candidates remain ambiguous, including `apy`
  against `api` and `api-gateway`. Choices are stable by kind and identity; duplicate rows do not add
  choices, while distinct identities with the same label remain distinct. Kind filters and pending
  choices cannot redirect an explicit out-of-scope alias to an unrelated name.
- `Clarification` uses injected monotonic time and only the original choices for an inclusive
  90-second window. Unknown/ambiguous replies do not renew it; a successful reply consumes it.
  Cancellation, expiry and backwards time discard the question. If any original identity disappears
  or changes its normalized name, the whole question becomes stale rather than selecting a survivor.
- This is a pure backend foundation with no IO, model call, core intent, action, permission answer or
  runtime caller. The visible island question and capture path belong to V8.4; V8.3 supplies dispatch
  and reads first. No dependency, setting or renderer changes in this step.
- Verification: 48 dictation tests pass, including ten resolver/clarification scenarios and an
  exhaustive comparison of bounded edits against full Unicode Levenshtein on short inputs. Full
  regression passes 1,040 workspace tests (eight opt-in/native ignored), plus six feature-enabled
  eval tests. Workspace Clippy with warnings denied, formatting, UI build, four relevant renderer
  checks, repository brand/English/i18n/layer/network/README/version guards, all 35 reviewed
  documentation pairs and offline links/fragments pass. Native UI validation is not applicable to
  this disconnected backend step. Windows CI still needs to confirm the earlier compilation fixes.
  The app stays stopped; generated caches are cleared and the source worktree remains.
- This checkpoint handed off to **V8.3 Intents and reads**. V1/V2/V4 maintainer and real-desktop
  acceptance remain open; implementing a resolver does not complete N3.

**V8.3 Intents and reads** (M) — implemented 2026-10-18; capture not connected
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

Implementation and evidence (2026-10-18):
- `app::voice_commands::Commands` receives a transcript, injected monotonic time, current saved
  replacements and cached usage from the state owner. It runs the grammar/resolver against a shadow
  core state and returns existing effects only after the entire sequence succeeds. Unknown, refused,
  ambiguous or unavailable suffixes commit no prefix. The capture/shortcut/runtime message path and
  visible island feedback remain V8.4; this module has no microphone or live transcript caller yet.
- The exhaustive `Intent::voice_command_allowed` classification permits only Focus, Next/Previous,
  Jump, OpenFolder and SetProjectPref. Decide, DecideAlways, Answer, Release, Skip, Review, merge and
  every other intent are excluded. The existing H1 exhaustive-intent samples also check this boundary;
  an added intent requires an explicit classification. Actual command dispatch checks it before reduce.
- A catalog maps full project and agent labels (including eligible Your words replacements) to
  qualified exact session identities. Same-id agents and duplicate project labels stay distinct;
  preference actions group sessions by exact folder, including hidden projects for unhide. Saved
  aliases take priority over built-in label aliases, without chaining arbitrary replacement text.
- Clarification retains the full batch, original choices and every earlier resolved identity. Replies
  use V8.2's original 90-second window and revalidate choices plus earlier targets; a moved/replaced
  project cannot rebind an unapplied prefix. A new recognized command replaces the question; refused
  answers cancel it. No answer by voice is converted into a card response.
- Repeat retains the last supported action/session read for 90 seconds. Undo is one-shot and restores
  the actual previous preference value, including no-ops, only when its exact session/folder and
  expected value still match. Focus undo checks the expected focus and previous live session.
  Opens/reads cannot be undone. Failed/unknown replies do not renew context; lock/cancel clears it.
- Reads use `flock_tools` over the local view and cached usage. Exact-key session detail shares the
  existing redacted formatter; waiting includes queued cards. Complete templates in English, Brazilian
  Portuguese, Spanish and Simplified Chinese report waiting sessions, session state/latest step, plan
  usage and GitHub pull requests/reviews with known/unknown check status. Branch/release rows are not
  reported as pull requests. Outside titles remain inert and secrets are redacted. No model, network
  refresh, clipboard write, dependency, setting or renderer change is introduced.
- Verification: 13 dispatcher tests plus the exhaustive core intent-boundary test pass. The seeded
  2,000-transcript property check exercises dispatch and refusal with a permission and question waiting;
  every effect stays within terminal/folder opens and project saves, with both cards unanswered.
  Scenarios cover atomic rollback/resume, expired/stale/moved targets, exact-key reads, alias priority,
  shared-folder preferences, no-op undo, external edits, context deadlines and four-language reads.
  Full regression passes 1,054 workspace tests (eight opt-in/native ignored), plus six feature-enabled
  eval tests. UI build, formatting, workspace Clippy with warnings denied, eight renderer regression
  scripts, repository guards, all 35 reviewed documentation pairs and offline links/fragments pass.
  No renderer changed, so native UI validation is not applicable here. Windows CI must still confirm
  the earlier compilation fixes. The app stays stopped, generated caches are cleared and the source
  worktree remains for V8.4.
- This checkpoint handed off to **V8.4 The command key and the island**. V1/V2/V4 maintainer and
  real-desktop acceptance remain open; this backend checkpoint does not complete N3.

**V8.4 The command key and the island** (M) — implemented 2026-10-18; real-desktop acceptance pending
- Build: a second portal shortcut `talk-command` (default Ctrl+Alt+Z, shown as the desktop bound it), the
  same capture and whisper path as dictation with no text pass and no paste; island states *Command*,
  *Command done*, *Answer*, *Which one?*, *Not by voice*, *Not a command*, with the caption carrying what was
  heard; History keeps the command line (no paste again).
- When it is not a command: say so (default), or send it to Zeca as a chat message (a request on the user's
  behalf, through the gate like any typed message).
- Tests: `check-dictation-island.mjs` covers the six states; the key never pastes text.
- Needs: V8.3, V1.12, V1.5.

Implementation and evidence (2026-10-18):
- `talk-command` is a separate portal shortcut (suggested Ctrl+Alt+Z), routed directly to the shared
  dictation worker regardless of Chat focus or Zeca's switch. Settings > Dictation shows its actual
  desktop binding. Mode is frozen into each generation ticket; the other key cannot release its
  capture, and changing mode cannot send a command through the ordinary clipboard path.
- Commands use hold only (at least 300 ms), the existing 600 ms microphone tail and local transcription.
  They bypass text transformations, focus/pane observation, clipboard, paste and Copy again's last
  text. Cancel, lock and rebinding invalidate tickets; stale queued work cannot dispatch.
- Saved aliases are read off the core loop. The runtime receives transcript/ticket/replacements and
  admits the synchronous dispatcher under the ticket's cancellation lock. Flock tools expose their
  existing cached reads synchronously, keeping IO/await out of that lock. Effects run through the
  existing runtime executor, with no synthetic Tick or permission intent on the command path.
- Production island/Nest feedback shows capture, done, answer, clarification, refusal and unknown
  states through the existing line/caption owner. Results are inert text and translated in all four
  languages. Cards, screen lock, inactive surfaces and constrained geometry retain priority. The heard
  caption expires within two seconds; absolute result/question expiry prevents handoffs and unknown
  replies from renewing the original clarification deadline.
- Command history uses the existing retention/erase ordering and a distinct `command` outcome, with
  no dictation usage/app/time aggregates or delivery-timing samples. Its row has no Use again action;
  backend replay rejects it before reaching a worker. Explicit Copy copies the result only. Ordinary
  Copy again retains the last successful dictation. No schema migration or dependency was needed.
- Unknown utterances currently say so locally, with no Chat/model request. The alternative opt-in Chat
  fallback is handed to V8.5 with its owning setting; it cannot be enabled through a placeholder page.
  V8.5 also owns the dedicated Voice commands page, switch and What you can say content.
- Isolated native validation uses a temporary XDG profile and production WebKit renderers, with Zeca
  off. Black Mini feedback, Light History and a 480 x 360 Light clarification were visually reviewed;
  all six command states fit the 480 x 360 Black Nest with inert long result text. A command row exposes
  Copy/Delete and direct backend replay is rejected. These fixtures do not establish real-microphone,
  X11/Plasma shortcut/capture or compositor acceptance, which remain explicit maintainer checks.
- Verification: 1,060 workspace tests pass (eight opt-in/native ignored), plus six feature-enabled
  eval tests. Four new worker cases cover command routing without text processing/copy, mismatched
  releases, cancellation before admission and immutable ticket mode across replacement. History tests
  enforce no usage/timing aggregates and backend replay rejection. All 15 renderer/script checks
  pass, including the six command states, four-language text, original deadline/surface/card priority,
  no History replay and the command binding in the documented Dictation settings owner. UI/native
  builds, formatting, workspace Clippy with warnings denied, repository guards, all 35 reviewed docs
  pairs and offline links/fragments pass. Windows CI still needs to confirm the earlier compile fixes.
  The isolated native app/server are stopped and generated caches cleared; source/dependencies remain.
- The next implementation step is **V8.5 Settings and docs**. N3's earlier acceptance/latency gates
  remain open; continuing implementation is not their sign-off.

**V8.5 Settings and docs** (S) — implemented 2026-10-18; real-desktop/provider acceptance pending
- Build: the prototype's *Voice commands* group (switch on by default, key, names, the not-a-command choice,
  *What you can say*); `docs/guide/dictation.md` gains a commands section (+ pt-BR); sign-off.
- Needs: V8.4.

Implementation and evidence (2026-10-18):
- Settings > Dictation owns the command switch (default on), actual command key, name/alias explanation,
  default-local/explicit Chat fallback and What you can say. Version 36 preserves old-file defaults and
  explicit choices; invalid fallback values grant nothing. Both setters require the Settings webview.
  Saves clone/persist before publishing; pending controls, rollback/retry and late bootstrap ordering
  use the existing settings owner. Changing a choice cancels command tickets/context without stopping
  ordinary dictation or erasing Copy again's last successful text.
- Dictation > Voice commands is a retained work page over the existing history backend, filtered to
  command outcomes among the newest 500 rows. Search covers heard/result text; Copy/Delete stay explicit,
  replay and bulk erase are absent, and dictation figures are hidden. Retention, individual confirmation,
  error/retry and stale-read guards stay shared with History. Search, selection and help disclosure
  survive Dictionary/Chat/tab handoffs. Dictionary and Settings links retain their existing owners.
- The work page and Settings share one read-only phrase guide. The four-language UI explains supported
  English/Portuguese/Spanish speech, current names, local cached answers, 90-second repeat/undo and the
  prohibition on answering cards. Chinese UI retains supported English examples instead of claiming a
  Chinese command grammar. No new theme, framework, dependency, model download or permission path lands.
- Only an unrecognized report with no choices/effects may request the opt-in Chat fallback. Refusals,
  unknown names and pending/stale/expired clarifications stay local. Handoff reserves the existing idle
  Chat without queuing; unsent text, files or references reject it. New conversations keep the selected
  folder/provider. Settings, lock, ticket, draft and selected source are rechecked after asynchronous
  preparation, so cancellation or a source change cannot send the old recording. Successful admission
  uses the same Chat send/tools/gate/budget/traces/approval owner, streams the reply in Chat and records
  the handoff status once in command history. Once admitted, Chat's existing Stop owns cancellation.
- Automated checks pass 1,065 workspace tests (eight opt-in/native ignored), plus six feature-enabled
  evals. Added tests cover safe defaults, invalid choices, Settings-only authority, command-only
  cancellation with ordinary Copy again retained, guarded non-queuing handoff, unsent draft/files/
  references, cancelled tickets, changed folder/provider and refusal/clarification fallback boundaries.
  All 15 renderer scripts pass, including failed settings saves/retry, late bootstrap, filtered command
  history with no bulk erase/replay and retained work-page search/help across tab handoffs. UI/native
  builds, formatting, workspace Clippy with warnings denied, repository guards, all 35 reviewed docs
  pairs and offline links/fragments pass.
- Isolated native WebKit validation uses a temporary XDG profile with Zeca off: Black wide command
  empty state/Settings help, Light minimum-size help/disabled Settings controls, and long inert command
  rows with reachable Copy/Delete in both themes at 480 x 360. Current bindings, persisted enabled/
  fallback choices, command-page filtering, retained search/help and the Settings destination are
  checked; a direct Nest setting write is rejected. Fixtures make no provider request or microphone
  recording. Real microphone/compositor/provider acceptance and earlier latency targets remain open;
  no V8/N3 human sign-off or Windows CI success is inferred. The isolated app/server are stopped and
  generated caches cleared after verification; sources and dependencies remain in this worktree.
- The next implementation step is **V3.1 Clean-up prompts and guards**, after its Z6 prerequisite.

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
- M7's `ask_user` follow-up is still needed for V7. V7.1 aligned ADR 0028 on 2026-10-19; the question
  composer and its enforcement remain V7.2/V7.3 work. Permission cards remain excluded.
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
   - smart-turn's BSD-2-Clause code/weights were verified in V6.2.2; new artifacts still need their own review.
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
- Q4: settled 2026-10-19 after native validation: Ctrl+Alt+V is reserved for independent
  dictation regardless of focus. Chat draft input uses its microphone button; no contextual
  shortcut rerouting. Voice commands keep their separate ID.
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

2026-10-18 navigation refinement: at the maintainer's request, consolidate dictation configuration
under its tab, split into eight smaller pages with shared navigation and a narrow picker. Move
the shared local transcription controls to Voice and link from Chat. Retain the Settings webview
and label checks for configuration; Nest keeps transcript work pages and gains no privileged command.
This navigation change does not complete real-speech/provider acceptance or V3.5 sign-off.

Navigation validation: build, all 16 renderer checks, workspace Clippy, 1,099 workspace tests
(8 ignored), 8 evals, source/language/layer/network guards and bilingual docs checks pass.
The running native app was checked at 480 x 360 across all eight configuration pages in Black
and Light, with bounded content scroll/footer, exact Dictionary return, Portuguese labels,
keyboard focus after repaint and Nest rejection of the cleanup setter. Expanded per-app styles
were also inspected in the isolated profile; cleanup was returned off, with zero provider calls.
Wide configuration retains the same tab/sidebar. Real-speech/provider/compositor acceptance
remains separate; no V3.5 sign-off is inferred.


**2026-10-18 V5.5 app installer handoff.** Dictation > Voice now owns an experimental reviewed
model/runtime installer, with a Meetings link, one-use expiring consent, progress and cancellation.
Every HTTP attempt uses the app's persistent 24-per-24-hour gate and content-free trace; no automatic
retry/resume/update is added. Settings-only authority, zero-HTTP review/quota rejection and native
Black/Light narrow/wide layouts are verified. The artifact feature is enabled independently of
native analysis, which stays off. See the [app-owned installation checkpoint](diarization-spike.md#v55-checkpoint-app-owned-artifact-installation).
The next step is real PT/EN speech/source-clock qualification before integrating native analysis;
no full meeting-quality acceptance, speaker labels, summaries or saved meetings are claimed.


**2026-10-18 V5.5 quality measurement handoff.** An offline Rust scorer now reports missed,
false-alarm and confused speaker time with exact label matching, zero collar and overlap retained.
Recording/source/WAV hash/duration must match; unknown activity cannot become an identity. The
optional native spike accepts a reference and retains/rechecks the same WAV descriptor. Canonical
input hashes and explicit annotation/language/split metadata make comparisons reviewable; no
score enables production. Synthetic EN/PT-tagged timelines verify the scorer only, including an
equal-speaker-count case with 40% confusion. The checkpoint passes 1,187 workspace tests (9 ignored),
21 optional spike tests and 8 eval cases. See the [source-bound scoring checkpoint](diarization-spike.md#v55-checkpoint-source-bound-offline-quality-scoring)
and [bilingual scoring reference](../reference/meeting-evaluation.md). Next: review a consented
human PT/EN corpus and freeze disjoint splits/acceptance limits before native measurement/tuning.
Physical source clocks, echo, long-session resources and app speaker-label activation remain open.


**2026-10-19 V5.5 human calibration checkpoint.** The fixed native baseline now has real public
human-annotation measurements on preselected 120-second AMI/English and C-ORAL/Brazilian Portuguese
crops. Both miss the predeclared provisional DER 0.20 target: 82.12% and 39.79%, respectively.
English finds all three annotated voices but misses/adds/confuses speech; Portuguese collapses
two annotated voices into one. Annotation conventions differ, so this is not a language ranking.
The native/standalone reports and independent millisecond raster/exhaustive mapping agree; real
22.05 kHz stereo and 16 kHz mono normalization preserve each source's duration/hash binding.
No model settings, algorithms, app linkage or labels were changed. The [human-calibration checkpoint](diarization-spike.md#v55-checkpoint-first-public-human-calibration)
records source licenses, frozen hashes, resources and limitations. Next: recording/speaker-disjoint
human evaluation material and review of simultaneous-observation merge constraints; no production
quality, physical dual-source clock, echo or long-session acceptance is claimed. App closed,
temporary downloads/models removed, user/shared Cargo caches preserved.

**2026-10-19 V5.5 coactivity checkpoint.** The development-only clustering candidate keeps
coactive embedded observations separate through direct and intermediate group merges, at the
unchanged 0.7 cosine-distance threshold. On the same frozen calibration sources, Portuguese now
finds 2 voices with DER 17.29% (previously 1 voice, 39.79%); false alarms increase. English
prediction remains identical at 82.12%. Native/standalone and independent raster scores agree;
90 optional voice tests, 21 spike tests and 3 explicit native checks pass. See the
[coactivity checkpoint](diarization-spike.md#v55-checkpoint-coactivity-constraints). This candidate
was developed after inspecting calibration, before the planned disjoint evaluation gate, and
does not qualify for production. Next: freeze recording/speaker-disjoint human evaluation and
acceptance limits; physical alignment, echo, noise, quiet turns and long-session gates remain.
No app activation; app closed and disposable public files/models removed.

**2026-10-19 V5.5 refinement checkpoint.** Preserve all positive timestamp gaps before text
association and partition false alarms into reference-absent activity/excess overlap. Two further
English recordings and six controlled gain/noise/echo variants expose remaining quality problems.
The gap fix is retained for correctness only: its quality gate fails on 17 additional missed
speaker-ms. A stronger local-slot grouping candidate regresses English calibration and was removed.
The [refinement checkpoint](diarization-spike.md#v55-checkpoint-pause-correctness-and-false-alarm-diagnostics)
records frozen plans, split changes, ten independently checked native reports and 90 voice/22
spike/3 explicit native passing checks. Next: acoustic/reference review of English false alarms
and independently evaluated speech/overlap/grouping work; independent Portuguese human evaluation,
real echo/source clocks and long-session acceptance remain open. Labels stay unavailable; no UI
change, app closed, disposable audio/annotations/models removed and user/shared caches preserved.

**2026-10-19 V5.5 decoder/reference checkpoint.** First-maximum powerset decoding now matches
the original contract for exact ties and signed zero, with a failing-before/passing-after regression.
All four primary predictions/reports are unchanged. A frozen paired AMI word/vocal-reference audit
covers no extra false alarms on IS1000a/IS1009a and only 123 speaker-ms on ES2004a; the primary
word-only profile stays fixed. See the [decoder/reference checkpoint](diarization-spike.md#v55-checkpoint-decoder-ties-and-reference-definitions).
Next: independently assess the existing Silero speech detector with frozen acceptance and
short/quiet-speech checks; do not enable a filter or automatic labels from this audit. Validation:
91 voice/22 spike/3 native passing checks, seven independent score comparisons. App closed,
disposable files removed, existing user/shared caches preserved.

**2026-10-19 V5.5 speech-detector checkpoint.** The explicit offline `--vad` diagnostic reuses
the existing catalog-pinned Silero profile/margin and preserves default diarization. All 12 frozen
cases fail the no-additional-missed-speech gate, including quiet/noise/echo and a 200 ms fragment.
New TS3003a English DER worsens 17.49% to 31.26%; the general mask is rejected. See the
[speech-detector checkpoint](diarization-spike.md#v55-checkpoint-existing-speech-detector-diagnostic).
Next: meeting-specific detection with independent short/quiet-speech acceptance and production
cancellation, leaving shared dictation thresholds unchanged. Validation: 91 optional voice/23
spike/4 native checks, 57 default voice tests with real Silero fixtures, 24 independent score
comparisons and four CLI checks. No app activation; temporary files removed, existing caches kept.

**2026-10-19 V5.5 meeting-frame checkpoint.** A separate raw-frame Silero profile preserves the
200 ms control's full 170 ms baseline coverage without changing Chat/dictation. The other 12
of 13 frozen cases add missed speech, including new EN2002a (DER 35.90% to 37.26%); reject general
hard masking without a parameter sweep. All 12 known baseline reports are unchanged; 26 score
comparisons agree independently. See the [meeting-frame checkpoint](diarization-spike.md#v55-checkpoint-meeting-frame-speech-preservation).
Next: independently validated detection/overlap and acoustic speech-loss review; physical clocks,
independent Portuguese evaluation, production cancellation and long sessions remain open. Passing
checks: 92 feature voice/23 spike/4 native/57 default voice, all-feature Clippy and guards. No app
activation; app closed, disposable public files/models removed, user/shared caches preserved.

**2026-10-19 V5.5 clock-edge refinement.** Drop frame edges that collapse to zero milliseconds
before turn merging, preserving positive turns and gaps; reject sources shorter than one
normalized millisecond before progress/model sessions. Two failing-before/passing-after
regressions cover known/unknown text association and 8/16/48/192 kHz source boundaries. See the
[clock-edge checkpoint](diarization-spike.md#v55-checkpoint-millisecond-edge-validity). Passing:
94 feature voice/23 spike tests, all-feature Clippy and guards; no native inference or new quality
measurement. Speaker labels remain unavailable, quality/physical-clock/cancellation/long-session
gates remain open. App closed, temporary logs removed, user/shared caches preserved.

**2026-10-19 V5.5 preparation-race refinement.** Recheck cancellation/foreground requests after
native operation options are created, skipping unnecessary inference and yielding to interactive
speech before retrying preparation. Two deterministic regressions fail before/pass after; 96
feature voice/23 spike/3 explicit native checks, all-feature Clippy and guards pass. See the
[preparation checkpoint](diarization-spike.md#v55-checkpoint-operation-preparation-cancellation-and-priority).
No new human quality result or app activation; session/Silero cancellation limits and the other
meeting acceptance gates remain open. App closed, temporary models/logs removed, existing caches kept.

**2026-10-19 V5.5 shared VAD ownership refinement.** Serialize interactive Silero detection and
freeing alongside construction/Whisper, covering Chat watchers, final trimming and dictation's
speech-presence checks. Background diagnostics reuse the locked decoder within their existing
slot, without recursive admission. The native ownership regression fails before/passes after;
both background profiles pass without deadlock. Passing: 96 feature voice/57 default voice with
real Silero fixtures, 23 spike, 2 native tests, all-feature Clippy and guards. See the
[ownership checkpoint](diarization-spike.md#v55-checkpoint-shared-vad-native-ownership).
No quality retuning or app activation; remaining meeting acceptance/cancellation gates stay open.
App closed, temporary model/logs removed, existing caches preserved.

**2026-10-19 V5.5 shared WAV admission refinement.** The one-millisecond source minimum now
belongs to the common WAV reader, covering transcription as well as native analysis/diagnostics
before resampler allocation or progress. Remove the speaker-only duplicate. New failing-before/
passing-after checks cover transcription and seven-rate/four-format mono/stereo boundaries.
Passing: 98 feature voice/59 default voice/23 spike, all-feature Clippy and guards. See the
[shared-reader checkpoint](diarization-spike.md#v55-checkpoint-shared-wav-clock-admission).
Synthetic checks only; quality/integration/physical-clock/long-session gates remain open. App
closed, temporary logs removed, existing caches preserved.

**2026-10-19 V5.5 buffered-input refinement.** Use stdlib BufReader at the common WAV owner to
avoid a backing read per sample. A regression fails before with 96,017 reads and passes under
407 after, preserving exact mono/resampled PCM and source duration. Passing: 99 feature voice/
60 default voice/23 spike, all-feature Clippy and guards. See the [buffered-input checkpoint](diarization-spike.md#v55-checkpoint-buffered-wav-reads).
Synthetic I/O-call evidence only; quality/physical-clock/integration/long-session gates remain
open. App closed, temporary logs removed, existing caches preserved.


## V5.5 consolidated delivery checklist

2026-10-19: close implementation as one reviewed batch; do not substitute more installer edge
cases for meeting delivery. V5.6 now starts with the pure contract and replay evals below. Earlier checkpoint statements describe their
then-current scope; this table is the current implementation status.

| V5.5 item | Status and evidence |
|---|---|
| Model/runtime delivery | Implemented: reviewed, pinned, gate-owned installation, notices and local revalidation. Installation enables no analysis by itself. |
| Bounded native analysis | Implemented: stream up to 30 minutes per source; one global clustering pass retains recurring voice IDs. One public 30-minute CPU run: 61.556 seconds, 158,916 KiB peak RSS. This measures diarization only, not ASR or capture. |
| Word/transcript association | Implemented: actual Whisper token clocks, exact filtered-text preservation, conservative whole-word coverage, Unknown for gaps/overlap. Invalid clocks retain the original segment. |
| App and UI handoff | Implemented developer preview: build with `--features meeting-speakers`, then opt in separately for each reviewed recording. Default builds stay unavailable. Plain transcription survives analysis/model failure. Source clocks and IDs stay separate. |
| Scheduling and cancellation | Implemented: interactive priority, cancellable model/hash checks and native operations, late-result discard and temporary-audio cleanup. Native construction and blocking IO remain cooperative. |
| Regression and independent review | Workspace tests, feature tests, Clippy, evals and renderer checks; independent read-only review resolved a lock-release race and a disconnected recording-cancellation check. |
| Native presentation | Inspected production renderer in isolated Tauri/WebKit, Black/Light, 1180 x 820 and 480 x 360; explicit opt-in, readable transcript rows, unknown voices, separate sources. Controlled responses and public fixture text; no physical capture acceptance claimed. |
| Release-quality acceptance | **Open:** independent disjoint human-annotated PT/EN quality gates, broader hardware/resource distributions and real capture/compositor acceptance. Known calibration and one resource run do not qualify these. Automatic production labels remain disabled. |

V5.5 engineering preview is delivered; the original production-quality release gate is not passed.
Physical cross-source alignment/echo deletion is required before mixing sources, not for these
separate source-local views. Full-duration capture/archive/export remain V5.7/V5.8 work. The
maintainer authorized V5.6 engineering after accepting the capsule; that does not pass the open
diarization quality gate. See the [V5.6 sequence](#v56-delivery-sequence).
See [methods, reference comparison and limitations](diarization-spike.md#v55-consolidated-reference-and-integration-handoff).


**2026-10-19 shortcut correction.** The shared talk shortcut unexpectedly opened Chat over Vults
windows. It now routes press/release directly to independent dictation, with no focus/Zeca branch
or renderer Chat handler. The desktop ID and configured binding stay unchanged. Native recording
rebind/cancellation checks and renderer regression cover no Chat opening or microphone start.
Chat draft voice input remains available from its explicit microphone button.


**2026-10-19 bottom feedback preview.** Source inspection of the MIT reference credited in
NOTICE confirmed a separate bottom capsule, measured audio bands and short outcome messages.
Vults reuses its status owner, themes, icons, settings and worker events in a passive Linux
480 x 160 surface. Only renderer readiness (after initial settings/view) relocates inline status;
reload/destruction returns the fallback. No audio, clipboard or agent action was added to this
surface. Its entire input region stays empty and keyboard requests are rejected. The island
keeps the single start cue; cards, lock, paused presence and absolute expiry retain priority.

Validation: 232 app tests, preview Clippy, renderer build, status/Chat/continuity/geometry checks
and repository guards pass. Independent read-only review resolved startup readiness/settings
races and found no further concrete issue. Native Tauri/WebKit inspection covers five phases in
Black/Light, viewport bounds, bottom inset, no filler captions, lock/card/pause hiding, an
unauthorized readiness acknowledgment rejection and successful readiness after page reload.
These native views used synthetic status events only, without microphone capture, clipboard
writes or stored transcripts. Temporary visual fixtures were removed and the current app remains
running for the maintainer's real-shortcut validation.


**2026-10-19 maintainer acceptance.** The bottom dictation capsule was accepted after real-key
validation. The validation app was then stopped to continue V5.6 engineering. This accepts the
indicator, not independent meeting-speaker accuracy or model-summary quality.

## 2026-10-19 consolidation review

The accumulated N3 changes through V5.7 received an independent correctness/privacy review.
Corrections before integration: Linux-only overlay constants avoid Windows dead-code failures;
temporary recording audio explicitly releases its flock after cleanup even if a child inherited
the descriptor; recovering Nest focus no longer discards the current meeting; CI also compiles and
tests the application with the optional `meeting-speakers` feature. Existing minimize, departure,
close and screen-lock cancellation remain. A native WebKit/KWin check moved focus away and back
while the real source review was Ready; it stayed Ready with the same recording ID. No audio was
captured or transcript sent. The isolated profile and app/server were removed/stopped afterward.

Local validation passed: 1,225 workspace tests, 10 evals, 31 optional capture tests, 118 optional
voice tests, four diarization spike tests and 244 preview-app tests. Deliberately ignored tests
still need their documented physical devices, model artifacts or opt-in provider setup. All 18
renderer regression scripts, renderer build, workspace/optional-feature Clippy, formatting,
repository guards, current bilingual source marks, 562 offline documentation links, npm audit and
cargo-deny pass. The release hook cold start averages 0.8 ms against the 10 ms ceiling. Final store
and default-app tests were rerun after review fixes. Prior native theme/layout/keyboard validation
remains recorded in the feature entries above.

This closes implementation through V5.7, not its remaining acceptance gates: V5.5 independent
PT/EN speech qualification is still open with automatic labels off by default, and V5.8 still owns
the explicit save/archive/export interface. No live-provider accuracy claim is made.

The first PR run additionally exposed non-Unix installer warnings and a Windows-drive issue in the
summary regression's virtual module path. Imports and the unsupported-platform enum are scoped
explicitly, the virtual entry reuses the existing cross-platform virtual-module resolver and normalized imports,
and the multi-command UI
step runs in Bash so an intermediate failure cannot be hidden by PowerShell. A follow-up portability
review covers the non-Linux app modules and test imports; local suites were rerun before resubmission.

Windows application compilation also identified Linux-only cleanup admission and clarification
feedback deadline accessors. They now follow their production callers' platform guards, while
cleanup reservation tests and semantic clarification expiry remain cross-platform. Workspace CI
uses `--no-fail-fast` to collect failures across crates in one run instead of stopping at the first.

The complete Windows test run had one remaining failure: the pre-cancelled installation test
expected the supported-platform cancellation result instead of the unsupported-platform error.
The test now checks each platform's contract and zero request admissions on both paths; production
installation behavior is unchanged. All other Windows workspace targets passed that run.
