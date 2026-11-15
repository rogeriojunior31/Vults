# Vults roadmap

**What is left, and nothing else.** One line per task, most urgent first inside each section. A
finished task is deleted from here in the PR that finishes it, and that PR adds its line to
[CHANGELOG.md](../../CHANGELOG.md). Why a rule holds lives in the [ADRs](../adr/README.md).

`P0` blocks the next release · `P1` next · `P2` after · `P3` later or only if decided.
Size: `S` up to a day · `M` a few days · `L` a week or more (split before starting).
Detail, discussion and evidence go in the task's issue or PR, never here: if a line needs a
paragraph, it needs an issue.

The long specs this file replaced (plan-ui, plan-dictation, plan-every-agent, plan-github-luna,
plan-field-fixes, plan-connector-parity, plan-windows-functional-validation, the development
changelog) are archived at
[`497677d3`](https://github.com/rogeriojunior31/Vults/tree/497677d34a2b2ef75590930452896ba0a7bac4e8/docs/dev).
Read them there when a task below names a step from them.

## Where we are (2026-11-15)

- Released: **0.1.8**. Everything merged since is under *Unreleased* in the changelog.
- Next release: **0.1.9**, from current `main`. Package versions stay 0.1.8 until REL-2.
- Current focus, set by the owner on 2026-11-11: **Windows functional validation**. Further connector
  work (GitLab/Gitea CP3 onwards, Notion) waits until it is done.

## Now

1. WIN · Windows functional validation (below)
2. Native acceptance of merged work: FF1, FF3, FF5, VC4, VC6
3. ACC-* and DG-* acceptance passes
4. R3 release smoke, then REL-2

## Release 0.1.9

Rules for every release: [Releasing](releasing.md).

- [ ] R3 · Release smoke on an installed production package (not `tauri dev`): island Mini and Quick, session arrive/finish/fail, permission and question cards, one CLI and one API Chat turn, voice into the draft, GitHub row, every Nest tab, Settings, `/continue` and its Stop, spend and budget, Zeca off. Record date, desktop (KDE Wayland, X11) and revision in the release PR · P0 · S · Needs: VC6
- [ ] R3-SHOTS · Replace earlier-build README captures with release-candidate screenshots, sample data only · P0 · S
- [ ] REL-2 · Bump the four version files, rename *Unreleased*, tag, review the draft, publish, push the AUR PKGBUILD · P0 · S · Needs: R3, ACC-UX6

## Acceptance waiting for a person

Implemented and tested; each closes only with a check in the running app, recorded in its PR.

- [ ] ACC-UX6 · Final desktop matrix: pending card ownership, Zeca off, 480 × 360, wide and short Nest, two displays and scales, minimize/restore, Wayland click-through · P0 · S
- [ ] ACC-UX7 · Mini/Quick widths (Default, 560, 680, 760), Mini at 38 px, overflow to Nest Flock, keyboard, on KDE Wayland and X11 · P1 · S · Needs: ACC-UX6
- [ ] ACC-UX8 · Agent profiles: copy, edit, reload; a profile Chat with a live Claude Code turn and a card; transcripts stay apart from Zeca; light theme and 480 × 360 · P1 · S
- [ ] ACC-CODEX · One live Codex profile turn, resume, a changed login refused for an old thread · P1 · S · Needs: ACC-UX8
- [ ] ACC-MEET · Record, save, reopen, search, export (MD, SRT, VTT) and delete a consented meeting; no audio left behind · P1 · S
- [ ] ACC-DICTUI · Dictation workspace visual pass (Room, 480 × 360, Black and Light), keyboard and screen reader, destructive clicks on test data · P1 · S
- [ ] FF1 · Moving the island between a 1× and 2× output on KDE keeps the birds whole and sharp · P0 · S
- [ ] FF3 · A finish while looking at that terminal makes no sound or reveal on KDE Wayland and X11; a card still sounds · P1 · S
- [ ] FF5 · With Klipper running, a *Copied* dictation stays out of its history and still pastes · P1 · S
- [ ] VC4 · Command, clarification and failure states readable in light/dark, minimum and wide layouts · P0 · S
- [ ] VC6 · Three phrasings per voice command in PT and EN (media, volume, open app, flock navigation, project mute); off/failure/ambiguity/cancel; no Claude login needed; KDE Wayland and X11 · P0 · S · Needs: VC4
- [ ] DG-1 · Natural PT and EN dictation with drafts and cancellation · P0 · S
- [ ] DG-2 · Physical shortcuts, portal grants, focus change, editor and terminal insertion, question card filled by voice and sent by a click · P0 · S
- [ ] DG-3 · Release-to-delivery p95 under 2 s with Small on Vulkan · P0 · S
- [ ] DG-4 · Voice commands spoken for real: target, effect, undo (includes VC6) · P0 · S · Needs: VC6
- [ ] DG-5 · Paid providers, after separate consent: cleanup, summary, realtime, spoken flock reads · P1 · S
- [ ] MF · Live metrics for Claude, OpenCode, Qwen, Hermes, Copilot, Kimi, Droid, Cursor compact and Antigravity status line · P1 · S each
- [ ] W2 · A Codex, Qwen and OpenCode shell card shows its working folder; then widen policy allows per agent with a test · P1 · M
- [ ] LC · Live checks: Codex card in its TUI; `/continue` Codex with both hooks; Antigravity with the real app; OpenCode plugin vs its own prompt · P1 · S each
- [ ] GHA · GitHub on a controlled repo: Connections controls, issue write, PR draft and review, Actions rerun, merge, each through its card · P1 · S · Needs: GH9-2

## Windows

- [ ] WIN-1 · Startup and lifecycle: single instance, owned child processes stop, closed sessions retire · P0 · M
- [ ] WIN-2 · Room and Settings native WebView pass: navigation, close/reopen, drafts, no stray click interception · P0 · M
- [ ] WIN-3 · Scale and placement at 100–200 %, short windows, monitor changes, physical input · P0 · M
- [ ] WIN-4 · Agents and hooks: CLI discovery, launchers with spaces, install/status, pipe events, stale sessions · P0 · M
- [ ] WIN-5 · Shortcuts: existing chords respected, conflicts visible, custom and disabled chords persist · P1 · S
- [ ] WIN-6 · Dictation and commands: readiness, latency, delivery, cancel, focus loss, clipboard restore, live speech · P1 · M
- [ ] WIN-7 · Platform capabilities audit: playback, app launch, files/URLs, notifications; unsupported paths named · P1 · M
- [ ] WIN-8 · Installer with source identity, update/restart, settings and models kept, Unicode paths · P1 · M
- [ ] WIN-9 · Still Linux-only: meeting recording, voice conversation, MCP agent questions, specialist GitHub tools, typing into terminals, terminal tabs/panes for *Open terminal* · P2 · L

## Island and Flock

- [ ] FF2 · Measure a minimized Nest's idle CPU on KDE Wayland; fix only if not near idle · P2 · S
- [ ] FB · Flock board animations, one PR each, matching the approved art: take-off/landing, turning, wire sag, per-species take-off, sex, age, bands, temperament, work animation, following, failure drop, visitors · P2 · L
- [ ] K7 · Zeca's mood (calm, alert, worried, celebrating) from attention, with reduced-motion stills · P2 · M · Needs: K1
- [ ] Z5 · Find who holds ~110 MiB after the Nest closes; fix only if ours · P3 · S
- [ ] X9 · Open terminal on Hyprland, Sway, GNOME Wayland (needs a decision on a Shell extension), Ghostty and Alacritty tabs · P3 · M each
- [ ] FF9a · Island options: middle clock, custom sounds, keyboard chords in the open island, alerts from any session · P3 · S each

## Agents

Every agent: a spike with `scripts/spike.sh`, a fixture, the installer, docs in both languages and a
CHANGELOG line. Every config write follows rule 3.

- [ ] FF9b · Verify against real sessions: Claude plan-ready card, session name from `/rename`, Antigravity questions, wrapper-started sessions · P2 · S each
- [ ] NEW-P2 · Cline CLI, Kiro CLI, a recorded real Cursor session, Amp, Pi, Hermes live turn · P2 · M each
- [ ] NEW-P3 · Lossless YAML writer then Goose; Crush, Grok Build, Muse Code; ACP rows for Goose, Auggie, Cline, Hermes · P3 · M each
- [ ] G1/G2 · An open reply for any tool (`--reply vults`, fixed allow/deny JSON, rules keyed by tool name, protocol VERSION 9). Decide first; the retired proposal is ADR 0027 at the archive commit · P3 · L
- [ ] QW · Qwen `write_file` diff into hunks · P3 · S
- [ ] F4 · Public issues per agent from *Adding an agent*, on the maintainer's go · P3 · S

## Chat and Zeca

- [ ] UX8-QS · Agent profiles in Quick, reusing the Nest's owner; cards keep priority · P1 · M · Needs: ACC-UX8
- [ ] UX8-PROJ · Specialist runs on the wire with their profile identity, never two birds · P1 · M · Needs: UX8-QS
- [ ] K1 · When Zeca speaks unasked: now, debrief or never; at most 4 lines an hour; silent in full screen; a card still opens the island with the budget spent · P1 · M
- [ ] K2 · Debrief on return in Quick's reserved slot · P2 · M · Needs: K1
- [ ] K3 · A risk line and out-of-scope warning per card; obvious reads batched, still one click · P2 · M · Needs: W2
- [ ] Z8 · Warn on a fetch's own card before the call · P2 · S
- [ ] M7f · The broader `vults-zeca` MCP for Codex through the installer · P2 · M
- [ ] Z6f · OpenAI Responses API with native compaction, once testable with a key · P3 · M
- [ ] LATER · Each needs a design pass first: routines (K4), proposals from repeated denials (K5), skills (K6), Zeca as an ACP agent (K10), Studio library (UX9), scoped investigations (UX10) · P3

## Dictation

- [ ] V5q · Speaker-disjoint annotated PT and EN material; labels stay a preview until the score passes · P1 · L
- [ ] FF6 · A second local engine (Parakeet TDT 0.6B v3): ADR first, then engine choice, checked download, evals on both · P2 · L
- [ ] FF7 · Compare the CAM++ export; manual speaker merge and line move in Saved meetings · P3 · M
- [ ] FF9c · Names learned across meetings, behind the summary consent · P3 · M

## Connections

- [ ] GH9-1 · Recovery tests: restart mid-write, network loss, account switch, GitHub errors · P1 · M
- [ ] GH9-2 · A controlled test repository and an end-to-end run of every flow · P1 · M
- [ ] GH9-3 · Docs in both languages: `gh` setup, host and account, tools, budgets, permissions, recovery, limits · P1 · M
- [ ] GH9-4 · Review rate and byte budgets before any new background request · P1 · S
- [ ] GH9-5 · A reviewer walks every external write path; CI green on Linux and Windows · P1 · S
- [ ] CP3–CP8 · GitLab and Gitea parity with GitHub: live reads, issue writes, PR writes, CI controls, merge, real-service acceptance (Gitea needs a test instance). Paused for Windows · P2 · L
- [ ] NOTION · Notion connector, after CP · P3 · L
- [ ] CZ · The Connections face for Zeca with a real source of state · P3 · M

## Activity

- [ ] W9b · Cost of watched sessions: loopback OTLP receiver, telemetry settings via the installer, tokens mapped to sessions · P2 · L
- [ ] FF9e · Compare the weekly recap with the sibling app; real gaps become tasks · P3 · S

## Settings and distribution

- [ ] FF4.5 · What's new after an update, 1 to 4 items per version, four languages, a CI check · P2 · M
- [ ] UPD · Signed updater with channels, after a signing key exists (ADR 0019) · P2 · L
- [ ] LANG · More UI languages; Arabic needs a right-to-left pass first · P3 · M each
- [ ] PKG · AppImage, Flatpak, macOS CI, Windows code signing · P3
- [ ] X7 · macOS beta · P3 · L
- [ ] X6 · A headless browser for Zeca with its own profile · P3 · L

## Mobile

- [ ] L2f-1 · Audit rows for pairing and revoke · P2 · S
- [ ] L2f-2 · Two phones racing on one card: the first wins, the second gets `Used`, with a test · P2 · S
- [ ] L3 · Android MVP (Kotlin/Compose over the Rust core): pair, flock, answer with a biometric Keystore key, UnifiedPush · P3 · L
- [ ] L4 · Handoff and Zeca's proposals from the phone · P3 · L · Needs: L3
- [ ] L5 · Service actions through connectors, refused after 5 minutes · P3 · L · Needs: L3

## To decide

- FF10 · Screen capture for agents (Screenshot portal to the clipboard): yes or no
- FF11 · Zeca reacts to the machine: yes or no on tone
- X9-GNOME · Ship a GNOME Shell extension or not
- Wake word stays declined; reopening it needs an ADR

## How to work a task

1. Read `CLAUDE.md`, `docs/architecture.md` and the ADRs the task touches.
2. One branch, one PR per task; tests first; docs, UI strings in four languages and pt-BR docs in
   the same PR.
3. Run *Before every commit* from `CLAUDE.md`, then a review pass.
4. In the same PR: delete the task's line here and add its line to `CHANGELOG.md`.
