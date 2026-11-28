# Vults roadmap

**What is left, and nothing else.** One line per task, most urgent first inside each section. A
finished task is deleted from here in the PR that finishes it, and that PR adds its line to
[CHANGELOG.md](../../CHANGELOG.md).

`P0` blocks the next release · `P1` next · `P2` after · `P3` later or only if decided.
Size: `S` up to a day · `M` a few days · `L` a week or more (split before starting).
Detail, discussion and evidence go in the task's issue or PR, never here: if a line needs a
paragraph, it needs an issue.

The long specs this file replaced (plan-ui, plan-dictation, plan-every-agent, plan-github-luna,
plan-field-fixes, plan-connector-parity, plan-windows-functional-validation, the development
changelog) are archived at
[`497677d3`](https://github.com/rogeriojunior31/Vults/tree/497677d34a2b2ef75590930452896ba0a7bac4e8/docs/dev).
Read them there when a task below names a step from them.

## Where we are (2026-11-27)

- Released: **0.1.9** (tagged 2026-11-27; its draft release waits for the R3 smoke and the owner's Publish). Everything merged since is under *Unreleased* in the changelog.
- Next release: **0.1.10**, from current `main`.
- Current focus, set by the owner on 2026-11-28: **what t3code does well, first** (the section below,
  planned in [t3-parity.md](t3-parity.md)); then Windows functional validation and the rest. Further
  connector work (GitLab/Gitea CP3 onwards, Notion) waits.

## t3code first

Every id is a step in [t3-parity.md](t3-parity.md): what it does, where it goes, its size and its
rules. Its section 0 is the contract for all of them: the shared pieces built once, and the
checklist each step meets in its PR (layers, rules, states, four languages, look, tests, docs, the
running app). Done before this list: Labs (1), Chat folders (2), Settings pages, provider marks, the
Agents page and shorter copy (3).

**A. Quick wins** (no dependency outside their section)
- [ ] T1 · New-conversation engine and model in Settings · P1 · S
- [ ] X3 · Palette over conversations, Settings pages and themes; Settings search · P1 · S-M
- [ ] X4 · Fallback model when an engine is at its limit (new conversations only) · P1 · S
- [ ] X10b · Save a cut turn's partial reply and offer *Continue* (asking before quitting is done) · P2 · S
- [ ] U1, U2 · Every plan window; Codex's reset credits · P1 · S
- [ ] X12, X13, X14, X15 · Skills in `/`; pace on quotas; favorite models; Nest hotkeys · P2 · S
- [ ] X8, X9 · Remove a voice model and space used; logs folder and a copied report · P2 · S
- [ ] Section 3, Providers 4-5 and Voice 3-4 · Models detail; version advisory; microphone test; *Test model* · P2 · S-M

**B. Connections: read, review and approve** (section 12)
- [ ] C1, C2 · UI reads for an item; the detail pane (header, labels, body, timeline, linked PRs); moves the pane to `ui/src/pane.ts` · P1 · M
- [ ] C3 · The Issues tab: involves me, state, sort, search, labels, grouped by repository · P1 · M
- [ ] C4, C5 · Comment, close or reopen, labels, assignees, each a prepared-write card (built here) · P1 · S-M
- [ ] C6, C7 · PR Summary and Timeline; review with Comment, Approve or Request changes · P1 · M
- [ ] C8, C9, C10, C11 · Code tab with threads; inline comments and Resolve; viewed files; merge method and rerun · P2 · L
- [ ] C12 · Docs and checks for each slice, in the slice's PR · P1 · M

**C. The chat as a workspace** (sections 9, 7, 10)
- [ ] W1, W2 · The workspace bar: repo, branch chip, worktree list with *Chat here* · P1 · S-M
- [ ] W3 + X2 · *Open in* the editors found on this computer (the editors table) · P1 · M
- [ ] W4, W5 · Create PR from the chat (prepared-write card); Open PR or issue, checks dot · P1 · M · Needs: C4
- [ ] T2 · Pin, archive (list and Unarchive), confirm delete · P1 · M · Needs: C2
- [ ] X5, X6 · Ask about lines in Changes, quote a reply; *Work on this* from an issue or PR · P2 · M · Needs: C2
- [ ] T3, T4, T5 · Titles by a model; fork from a reply; search every conversation · P2 · M
- [ ] W6 · Docs and checks for the bar · P1 · S

**D. Usage like t3code's** (section 11)
- [ ] U4 · Local usage scan of Claude's and Codex's transcripts, per hour, model and token kind (replaces most of W9b) · P1 · L
- [ ] U5 · Cost math: 1-hour cache writes, tiers, split by kind, savings, each figure's source · P1 · M
- [ ] U6 · The Usage page: Cost, Tokens, Limits; 24 h to 90 days; the SVG chart; tiles; table; limits with resets and pace · P1 · L · Needs: U4, U5, C2
- [ ] U7, U8 · Retention, price overrides and aliases; docs and checks · P2 · M

**E. Scheduled tasks** (section 5: the vultures that fly in, work and leave)
- [ ] S1-S4 · Design and rule 2, the Lab, the store, the gate feature `ROUTINES` · P1 · M
- [ ] S5 · A specialist's run on the wire with its own bird (closes UX8-PROJ) · P1 · M
- [ ] S6-S7 · A run with no window, access capped; the scheduler · P1 · L
- [ ] S8-S9 · The flight in and out on the island; notifications · P1 · M
- [ ] S10-S12 · Specialists, Scheduled tasks (editor with the vulture), the *Scheduled* mark, docs · P1 · L · Needs: C2

**F. Deeper work**
- [ ] Section 4 · Code: projects with their context, handoff v2, git-backed Changes, a worktree per conversation · P2 · L
- [ ] X7, X11 · Steer a running turn; Zeca hands a task to a specialist (Labs) · P2 · M
- [ ] B1-B2 · Open links in; a session's local server with *Open* · P2 · S-M
- [ ] B3, T7 · Preview window; resume after a usage limit (Lab) · P3 · L

**G. Icons across the whole app** (section 13, after A to F so each surface is redrawn once)
- [ ] I1 · Inventory and a check against new hand-drawn icons and glyphs · P1 · S
- [ ] I2, I3, I4 · The island, the Nest, the widget, the dictation overlay and Settings leftovers in Lucide and marks · P1 · M
- [ ] I5, I6, I7 · Tray icons per state, the app icon at every size, DESIGN.md and screenshots · P2 · M

**H. Environments** (section 6): other machines over SSH, after F · P3 · L

**To decide first (ADR):** U9, Claude's reset credits and *Redeem* (they need the CLI's token and a
direct call to Anthropic); B4, agent access to a preview browser.

## Now (after t3code first)

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
- [ ] G1/G2 · An open reply for any tool (`--reply vults`, fixed allow/deny JSON, rules keyed by tool name, protocol VERSION 9). Decide first · P3 · L
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
- [ ] T1-T6 · Conversations like t3code's Threads: new-conversation defaults in Settings, pin and archive, titles by a model, fork from a reply, search every conversation, copy as Markdown (docs/dev/t3-parity.md §7) · P2 · M
- [ ] T7 · Resume after a usage limit resets; settle idle conversations (Lab first) · P3 · L
- [ ] B1-B2 · Open links in the system browser or a Vults window; a session's local server on its card with Open (t3-parity §8) · P2 · S-M
- [ ] B3-B4 · A preview window, then agent access to it through the `vults-zeca` MCP, each act a card (Lab first) · P3 · L
- [ ] LATER · Each needs a design pass first: routines (K4), proposals from repeated denials (K5), skills (K6), Zeca as an ACP agent (K10), Studio library (UX9), scoped investigations (UX10) · P3

## Dictation

- [ ] V5q · Speaker-disjoint annotated PT and EN material; labels stay a preview until the score passes · P1 · L
- [ ] FF6 · A second local engine (Parakeet TDT 0.6B v3): decide size, license and consent first, then engine choice, checked download, evals on both · P2 · L
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

- [ ] W9b · Cost of watched sessions: reconsider after U4 (the local usage scan covers most of it); what is left maps tokens to live sessions · P3 · M · Needs: U4
- [ ] FF9e · Compare the weekly recap with the sibling app; real gaps become tasks · P3 · S

## Settings and distribution

- [ ] FF4.5 · What's new after an update, 1 to 4 items per version, four languages, a CI check · P2 · M
- [ ] UPD · Signed updater with channels, after a signing key exists · P2 · L
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
- Wake word stays declined

## How to work a task

1. Read `CLAUDE.md`, `docs/contributing/architecture.md` and the rules that never bend.
2. One branch, one PR per task; tests first; docs, UI strings in four languages and pt-BR docs in
   the same PR.
3. Run *Before every commit* from `CLAUDE.md`, then a review pass.
4. In the same PR: delete the task's line here and add its line to `CHANGELOG.md`.
