# Field fixes and borrowed ideas

Written 2026-10-25. A sibling desktop app (outside this repo) ships the same kind of island, with
a changelog of bugs it met on real desktops and features users asked for. This plan lists what
Vults can take from it. Nothing here is copied code: each step says what behaviour to reach, where
it lands in Vults, and how to accept it. IDs are `FF<n>`; cite them in code as `plan-field-fixes FF<n>`.

Status of each row was read from the code on 2026-10-25, not assumed. A row marked **verify** needs
a measurement or a read before work starts.

## Constraints that hold for every step

- English is the project language; every sentence the user reads goes in `core/src/i18n.rs` or the
  UI catalogs (pt-BR, es, zh) in the same PR. Docs that are published change in the same PR, with
  their `docs/pt-br` twin and `source` mark.
- The brand check applies: no step may name the project this one derives from or its character.
- Rule 1 (never block an agent), rule 2 (a human answers permissions) and rule 4 (no telemetry,
  every self-initiated request through `app/src/gate.rs` with a switch, a budget and a trace) are
  not touched. FF4 is the only step that adds a network request.
- Layers: pure decisions in `core`/`dictation`, IO in `app`/`platform`, drawing in `ui`.
- Island gotcha: measure the DOM synchronously; `requestAnimationFrame` pauses while WebKit thinks
  the page is hidden (AGENTS.md).

## Order of work

| Order | ID | Title | Size | Why now |
|---|---|---|---|---|
| 1 | FF1 | Canvas follows the monitor's scale | S | A certain bug: `dpr` is fixed at construction |
| 2 | FF2 | Idle CPU of a minimized Nest | S | Narrowed on review: only the minimized case is left |
| 3 | FF3 | Quiet finish when the session's terminal is in front | M | `platform::focus` already exists |
| 4 | FF4 | Update notice, asked for | M | ADR 0019 already reserves it |
| 5 | FF5 | Dictation: clipboard history exclusion | S | The paste leases have it; the *Copied* path does not |
| 6 | FF6 | Dictation: an optional second engine | L | Accuracy and CPU-only machines; needs an ADR |
| 7 | FF7 | Diarization: one model variant and manual fixes | S | Vults already uses the same model pair |
| 8 | FF8 | A spoken wake phrase | M | **Declined**: Plan Zeca already decided *Wake word: No*; reopening needs an ADR |
| 9 | FF9 | Agent and card follow-ups | S each | See the list at the end |
| 10 | FF10 | Screen capture for agents | M | Candidate; Linux has a portal for it |
| 11 | FF11 | Zeca reacts to the machine | S to M | Candidate; product decision |

FF1 to FF3 are fixes and do not need a decision. FF4 to FF8 add behaviour and each starts with the
decision named in its row.

---

## FF1 — Canvas follows the monitor's scale

**Found.** `ui/src/island/scene.ts` and `ui/src/island/sky.ts` hold `private readonly dpr`, read once
from `devicePixelRatio` in the constructor, and size the canvas bitmap from it. The island is a
layer-shell surface that can follow the pointer to another output (and the Nest can be dragged
between monitors), so the bitmap stays sized for the first monitor while drawing uses the current
one. Effect: birds drawn too large and cut off, or too small and soft. The sibling app fixed the same
defect and noted a second trap: a `matchMedia("(resolution: Ndppx)")` query made once only fires for
the first change, so it must be re-armed after every change.

**Do.**
1. Make `dpr` mutable in `Scene` and `Sky`; on a ratio change recompute it, call `size()` again,
   clear the sprite caches keyed by scale, and redraw at once (including while idle, where the frame
   loop may be asleep: call the existing `refresh()`).
2. One small helper in `ui/src/` that watches the ratio: build the `resolution` query for the
   current ratio, listen for `change`, and build a new query for the new ratio inside the handler.
   Share it between `Scene`, `Sky` and any other canvas found by `git grep devicePixelRatio ui/src`.
3. Check the `zoom` argument still composes: the layout is already corrected by `zoom`, so the
   bitmap size is `layout * dpr` and the CSS size `layout / zoom`.

**Accept when.** A unit test (jsdom or a stubbed `matchMedia`) shows that two successive ratio changes
each resize the canvas; moving the island between a 1x and a 2x output on KDE leaves the birds whole
and sharp, checked by hand on two monitors (note the result in the PR); no extra work per frame when
the ratio is stable.

**Docs.** `CHANGELOG.md` Unreleased line. No i18n.

---

## FF2 — Animate only while the window is on screen

**Review 2026-10-25.** Narrowed. The Nest is destroyed when closed (`nest_close` calls `destroy()`;
D6), and the UI has no `requestAnimationFrame` loop (`scene.ts` drives frames with timers), so a
*closed* Nest cannot animate. Only a **minimized** Nest is left to measure. Steps 1 to 3 below stay
as the fix if that measurement is not near idle.

**Found.** The Nest decides to animate with `document.hidden` (`ui/src/surfaces/nest/render.ts`:
`portrait.setActive`, `flock.setActive`, the `visibilitychange` handler). On WebKitGTK a hidden or
minimised window can keep `visibilityState` at `visible`, and the sibling app saw its hidden window
keep a WebKit process at a full core. Rust already knows the truth: `app/src/lib.rs`
(`question_visible`) reads `is_visible()` and `is_minimized()`.

**Do.**
1. **Measure first.** Close the Nest (it hides, it does not exit) and watch
   `top -H -p $(pgrep -f WebKitWebProcess)` for a minute, on KDE Plasma Wayland. Record the number in
   the PR. If it is near zero, close FF2 as not needed and keep the helper only if cheap.
2. If it is not: a small `NestShown` atomic in `app`, set from the window events (`CloseRequested`
   hide, `Resized` for minimise, `Focused(true)`, and the places that show it) and emitted to the
   page as `nest-visible`; plus a command the page calls once at load, in case it missed the event.
3. In `render.ts`, combine it with `document.hidden`: animate only when both say visible. Keep the
   existing rule that canvases scrolled out of view are skipped.

**Accept when.** CPU with the Nest closed is at or near idle over one minute; reopening resumes
animation at once; a test covers the page-side rule (hidden by either signal means no frame asked).

**Docs.** CHANGELOG line. No i18n.

---

## FF3 — Quiet finish when the session's terminal is in front

**Review 2026-10-25.** The cue and the reveal are decided in the renderer, not in `core`:
`ui/src/island/render.ts` `cues()` plays the sound and calls `fsm.reveal()`, and it already keeps a
muted project quiet at rest while its card keeps its sound. So the change is a per-session flag in
the view, fed by the app from `platform::focus`, read by `cues()` like `muted`, and also skipping the
reveal. Core only carries the flag (pure), and its tests check a card is never quieted.

**Found.** When a turn ends while the user is looking at that session's terminal, a sound and a
reveal say nothing new. The sibling app shows only a badge then. Vults has the parts:
`crates/platform/src/focus` (X11 `_NET_ACTIVE_WINDOW` and KWin) reads the active window, and
`focus::session(window, sessions)` already maps a window to a session for dictation
(`app/src/dictation.rs`). `core/src/notify.rs` handles desktop notifications only for *Panel*, so the
sound and the reveal are decided elsewhere; **verify** where (search `core/src` for the finish/failed
cue and the reveal effect) before writing code.

**Do.**
1. Add the input to the core: a pure `Input` (or a field on the existing focus input) carrying "the
   window in front belongs to session S". Core decides: a Finished or Failed for S while S is in
   front produces the badge but not the cue nor the reveal. A card that needs the user (permission,
   question) is never quieted: those always sound and open.
2. In `app`, feed it from `focus::Tracker` on change (not by polling), only while a session is
   working or just finished, so nothing runs when the island is idle (performance rule).
3. Where no compositor lets Vults read the active window (GNOME Wayland), the input never arrives and
   behaviour stays as today. Say so in the guide.
4. A setting, **on by default**, in Settings > General > Notifications: "Stay quiet when I am looking
   at the session's terminal".

**Accept when.** Core tests: finished while in front gives badge only; finished while elsewhere gives
cue and reveal; a permission card while in front still sounds and opens; focus leaving afterwards does
not replay the cue. No change to rule 2.

**Docs.** `docs/guide/island.md` (+ pt-br), `docs/reference/settings.md` (+ pt-br), i18n for the
setting label, CHANGELOG.

---

## FF4 — Update notice, asked for

**Found.** `rg -i updater` finds nothing in the app. ADR 0019 already says: the update check is asked
for, signed, shown with the version and notes before any install, and distro packages ship with the
updater compiled out; the updater lands after 0.2.0 (ADR 0020).

**Do.** Ship the notice first, the installer later.
1. A new gate feature `update` in `app/src/gate.rs` (switch, budget of a few requests a day, trace).
   One unauthenticated `GET` of the repository's releases list, no cookies, nothing about the user.
   Check at most once a day once allowed (rule 4 in AGENTS.md and CLAUDE.md says *at most daily*),
   first look about 30 seconds after launch (never on top of the greeting).
2. Pure rules in a small module with unit tests (they port well): parse the tag prefix for this
   platform's releases, ignore pre-releases and rolling tags, compare versions, `should_check(now,
   last_check)`, and a snooze: **Later** hides that version for 24 hours, a newer version shows at
   once; if the first question is left unanswered it comes back after a day at most.
3. Consent: the island asks once ("May Vults look at its releases on GitHub?"). No request before the
   user says yes. Also in Settings > General > Updates with **Check now** (a click, so it works even
   when the automatic check is off).
4. Presentation: Zeca gets a small badge or tint with an arrow when he has nothing else to say;
   approvals and questions always come first. The card shows version, notes and two buttons:
   **Open release page** and **Later**.
5. Installing: the `.deb`/`.rpm`/AppImage case follows ADR 0019 (signed, click after seeing version and
   notes, nothing while an agent runs). Until the signing key exists, the notice opens the release page.
   AUR builds compile the whole feature out (the ADR says so already).
6. "What's new": after an update, show once what the new version brings. Source: one JSON per version
   with 1 to 4 items, each an icon, a short title and one line, translated into every UI language, and a
   script check in CI that fails when the current version has no entry (same idea as the existing
   i18n check). This is optional for the first cut; the badge and card come first.

**Accept when.** With the switch off, a packet capture shows no request; with it on, one request per
interval, traced, within budget; pure-rule tests cover prefix, ordering, snooze and the unanswered
question; a compiled-out build has no updater code or UI.

**Docs.** New section in `docs/safety.md` (+ pt-br) listing the request; ADR 0019 History line;
settings reference; i18n for every sentence; CHANGELOG.

---

## FF5 — Dictation: keep dictated text out of clipboard history

**Review 2026-10-25.** Half done already: the paste leases offer `x-kde-passwordManagerHint`
(`crates/platform/src/insert/lease.rs` on X11, `insert/wayland_lease.rs` on Wayland). The gap is the
clipboard-only path below, which gives *Copied*. Reuse the same hint there; do not add a second
mechanism.

**Found.** `crates/platform/src/clipboard.rs` writes with `wl_clipboard_rs::copy::Options` on Wayland
and `arboard` on X11, without asking clipboard managers to skip the entry. Dictated text can hold
secrets read aloud, and managers such as Klipper or cliphist then keep it. The sibling app writes the
text with the "exclude from history" hint (arboard's `exclude_from_history` on Linux) and, after the
paste keystroke, puts back what the clipboard held before. Vults already reads back the clipboard to
verify "Copied" (see `docs/guide/dictation.md`).

**Do.**
1. X11 path: use arboard's `SetExtLinux::exclude_from_history()`.
2. Wayland path: `wl-clipboard-rs` has no such flag; offer the `x-kde-passwordManagerHint` MIME type
   with the value `secret` alongside `text/plain`, which Klipper honours. **Verify** with Klipper on
   Plasma 6 that the entry is skipped and that paste into a normal app still reads plain text.
3. Keep the read-back check; the extra MIME type must not make the match fail.
4. Restoring the previous clipboard after an opt-in paste is a separate choice: check how
   `insert` leases and restores today before adding anything, and do not change it in this step.

**Accept when.** With Klipper (or cliphist) running, a dictated phrase does not appear in the
history; plain paste still works in KWrite, a browser field and a terminal; tests cover the MIME set.

**Docs.** `docs/guide/dictation.md` clipboard section (+ pt-br), CHANGELOG.

---

## FF6 — Dictation: an optional second engine

**Review 2026-10-25.** `ort` (ONNX Runtime, `load-dynamic`) is already an optional dependency of
`crates/voice` for the speaker preview, so Parakeet can run on it without sherpa-onnx or a new native
runtime. The ADR then weighs a second model and its download, not a new runtime.

**Found.** Vults transcribes with whisper.cpp (Vulkan), qualified only for Turbo with fixed English
for the vocabulary prompt. The sibling app runs NVIDIA Parakeet TDT 0.6B v3 (int8) through
sherpa-onnx on the CPU: offline, 25 languages, about 700 MB once, files pinned to a Hugging Face
revision and checked by SHA-256 before use. Two things matter for Vults: machines with no usable
Vulkan GPU, and languages where Turbo is weaker.

**Do.**
1. **ADR first** (new, next free number): is a second engine worth a native dependency (sherpa-onnx
   with ONNX Runtime) given `forbid(unsafe_code)` in `crates/dictation` and the "no sidecars" rule
   (ADR 0002)? Options to weigh: link it in `crates/voice`, load it as a checked runtime the way the
   diarization spike does, or decline. Record the decision with a dated History line.
2. If accepted: an `Engine` choice in Settings > Dictation > Voice, whisper as the default; model
   download follows the existing checked-download path (pinned revision, size and SHA-256, explicit
   click, nothing fetched before). The dictation, styles, dictionary and snippets pipelines stay
   engine-independent: they receive text.
3. Compare on the existing evals: word error rate on the project's fixtures per engine, latency per
   stage (the Diagnostics page already records p50/p95), and idle memory.

**Accept when.** The ADR is merged; the engine is chosen in Settings and persisted; a wrong checksum
refuses to load; the same fixtures run through both engines with the numbers in the PR.

**Docs.** `docs/reference/meeting-models.md` style page for the artifacts, guide, settings reference
(+ pt-br), i18n, CHANGELOG.

---

## FF7 — Diarization with pinned models

**Review 2026-10-25.** Vults already installs the same pair: pyannote segmentation 3.0 and 3D-Speaker
CAM++ (`docs/reference/meeting-models.md`), and V5.5 delivered a developer preview with it. What is
left from this row: compare the zh/en *common advanced* CAM++ export against the zh-cn one on the
existing fixtures, and the manual merge/move of step 3. Step 1 below is replaced by that comparison.

**Found.** `docs/dev/diarization-spike.md` (V5.4) concludes no-go for automatic speaker labels:
overlap is not reliable. The sibling app ships labels ("you", "Person 2", "Person 3", renamable)
using a pyannote segmentation 3.0 model (int8, about 1.5 MB) plus a CAM++ speaker embedding (about
28 MB), both pinned by hash, and two details Vults can use even without the labels being automatic:
the other side's voice coming back through the speakers is not written as the user's, and one voice is
not split in two people.

**Do.**
1. Re-run the V5.4 fixtures with those two exact models and the same thresholds; add overlap cases.
   Record the numbers in `diarization-spike.md` as a dated section. This step may end in "still
   no-go", and that is a valid result.
2. Look at the echo handling in `crates/audio/src/echo*` against the "your voice vs the call" rule so
   the microphone track is not credited with what the speakers played.
3. Allow merging two speakers and reassigning a line by hand in Saved meetings, which is useful even
   with labels off.

**Accept when.** Numbers recorded; if go, labels stay behind the existing explicit development preview
until the independent qualification the spike asks for.

---

## FF8 — A spoken wake phrase (declined)

**Status 2026-10-25.** The roadmap's *Out of scope* table already says *Wake word: No* (hands-free dictation
is a double tap, and realtime follow-up is opted in and bounded). The notes below stay as the case for
reopening it, which needs an ADR first.

**Found.** Vults has push-to-talk voice commands (Ctrl+Alt+Z) and realtime "Talk with Zeca". The
sibling app also listens for a wake phrase ("OK <name>") and then takes a question. Always-on
listening conflicts with how careful Vults is about the microphone (explicit consent, indicators,
cancel on lock), so this is a product decision before it is an engineering step.

**Do.** Write the decision: default off, a visible "listening" state in the island and tray, local-only
detection, never records before the phrase, cancelled on screen lock, and an audit/trace entry per
activation. If the answer is no, mark FF8 declined and remove it from the order table.

---

## FF9 — Agent and card follow-ups

Each is small and independent. **Verify** against the agent driver and a real session before
committing to any. Anything touching an agent's config follows rule 3.

| Item | What the sibling app does | Where in Vults |
|---|---|---|
| Plan approval | A plan ready in Claude Code offers the terminal's own choices (auto, manual, change plan); "change plan" brings the terminal forward to type | `crates/agents/src/claude.rs` maps `ExitPlanMode` to `Activity::Plan` only; check what the hook payload carries. Answering stays a click (rule 2) |
| Session name | A session is named after its `/rename` title, else the folder it started in (not the subfolder it moved into); with several sessions in one pill, the one working | Claude driver and `core` session naming |
| Agent asks a question | The Antigravity pill turns to a question with a sound | `crates/agents/src/antigravity.rs` mentions plan input only; see whether the agent emits one |
| Alerts from any pill | Option: another pill that finishes or fails opens the island for a moment, then the previous pill returns. Off by default | `core` presence rules |
| Conductor and similar wrappers | Sessions started by a wrapper land on the Claude pill; "Open terminal" brings the wrapper forward | `crates/platform/src/jump` |
| More agents | Amp, Hermes Agent and Muse Code with plugin installers (preview, dated backup, confirm) | `docs/dev/plan-every-agent.md` open steps; none of the three is in `crates/agents` |
| Forge connectors | GitLab and Gitea pulses like GitHub's: open merge/pull requests, reviews asked of you, default-branch CI, activity; tokens in the keyring | `crates/connectors`; today only GitHub through `gh` |
| Optional clock | A clock in the middle of the compact island | UI setting |
| Own sounds | A file named like a built-in sound in the sounds folder replaces it | `ui/src/sound.ts` |
| Keyboard in the open island | Chords to move through lists and open the row or the diff | island UI |
| Display pinning | On Hyprland and Sway, pin the island to one output with a picker | `crates/platform` (some support exists; check the picker) |
| Weekly recap | A card on Monday for the past week | `crates/core/src/recap.rs` already exists; compare contents |
| More UI languages | 14 on the Mac and 12 on Windows/Linux, with right-to-left Arabic; each added by a community translator | Vults has 4 (`ui/src/i18n`: pt-BR, es, zh). Adding one needs the three checks to pass; Arabic needs a right-to-left layout pass first |
| Stale finish timers | A Stop that arrives and is then followed by new work must not fire its delayed "finished" later | `crates/core` is a pure reducer, so check by a test: finish, then new prompt inside the delay, then the delay passes; the session must stay working |
| Learned names in meetings | After a call, misheard words are fixed and names learned are kept for the next call | Saved meetings and the dictionary; must stay inert text and behind the existing summary consent |
| Share a recap as an image | Weekly recap exported as an image | `docs/guide/activity.md` already mentions it; confirm parity before doing anything |

---

## FF10 — Screen capture for agents (candidate)

**Found.** The sibling app has a shortcut: select part of the screen, the image goes to the clipboard,
then paste it into a terminal where the agent accepts images, with optional arrows, boxes and pen, or
a direct "ask" with the image. Vults has no capture feature (`rg -i screenshot` finds none).

**Do, if wanted.** Use the desktop's Screenshot portal (`org.freedesktop.portal.Screenshot`, with
`interactive: true`) so no compositor-specific code is needed and the user picks the region in the
desktop's own UI. Put the PNG on the clipboard through `platform::clipboard`; do not write it to
disk unless the user saves it. A global shortcut through the existing GlobalShortcuts path. Annotation
is a separate, later step. Decision needed: whether this belongs in Vults at all, since it has no
connection to sessions unless "ask" is wired to a chat or an agent.

## FF11 — Zeca reacts to the machine (candidate)

**Found.** The sibling's character reacts to CPU load, temperature, memory pressure and battery, read
only while he is on screen, with a tooltip saying why. Vults' rule is 0 percent CPU when the island
is hidden, which this respects if the sampling is gated on visibility (reuse the FF2 signal).

**Do, if wanted.** Read `/proc/pressure/*`, `/sys/class/power_supply` and thermal zones from `app`
(no new crate), sample slowly while visible, and send a small mood input to the character. Off by
default; copy for the tooltip goes through i18n. Decide first whether it fits Zeca's tone.

---

## Checked, nothing to do

Looked at and found already handled in Vults on 2026-10-25, so nobody redoes them:

- Antigravity hooks: the Vults installer ends every command in `|| exit 0` and never answers "allow"
  for the agent (it leaves permissions to the agent's own prompt). The sibling's bug, where an empty
  answer was read as a denial, is the reason to keep a test for that, not to change anything.
- OpenCode 1 and 2 plugins: one default export that both accept (`crates/agents/src/opencode.rs`).
- Corrupt preferences: `app/src/settings.rs` salvages field by field when the whole file does not parse.
- Logs without commands, paths or payloads, five daily files kept (`app/src/log.rs`).
- Monitor choice for the island (`Settings.monitor`) and hover-to-open.
- Local models and OpenAI-compatible servers and OpenRouter in Chat.
- Step ticker cap: Vults keeps 8 steps by design (`MAX_STEPS`), not a bug of the 20th step.
- The tray: Vults draws its own icon from a watch channel, so the lock-across-menu deadlock the sibling
  fixed does not exist today. When FF4 adds a menu entry for an update, copy the value out of any
  mutex before touching the menu.
- AltGr-safe default shortcuts: a Windows concern. On Linux, AltGr is a separate level-3 key, so
  Ctrl+Alt defaults do not type characters.
- Claude Desktop pill: the app has no Linux build.

## Risks
- FF10 and FF11 can widen the product past agents; each starts with a yes or no.

- FF2 may be unnecessary; measure before changing anything.
- FF3 depends on the compositor exposing the active window. It degrades to today's behaviour.
- FF4 must not become telemetry: the request carries nothing about the user and is traced locally.
- FF6 adds a native dependency; the ADR may decline it, which is an acceptable outcome.

## Out of scope

Copying code, text or art from the sibling app; anything that answers a permission without a click;
a hosted service.
