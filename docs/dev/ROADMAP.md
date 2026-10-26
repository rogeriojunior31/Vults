# Vults roadmap

The one development roadmap. Rewritten 2026-10-25 from Plan Zeca, the UI, Dictation, Every-agent,
GitHub and Field-fixes boards, then refined into tasks. It lists **only what is left**, most urgent
first, in phases that follow the app's tabs. Finished work lives in
[changelog-dev.md](changelog-dev.md); released changes in [CHANGELOG.md](../../CHANGELOG.md).

Step ids are never reused and code cites them (`plan-zeca <ID>`, `plan-field-fixes FF<n>`); a
finished id moves to the development changelog. Long specs stay where they are and are linked from
the tasks:

| Spec | Holds |
| --- | --- |
| [plan-dictation.md](plan-dictation.md) | V0 to V8 substeps, evidence and the remaining human gates |
| [plan-ui.md](plan-ui.md) | UX1 to UX12 slices, acceptance matrices and the Agents workspace record |
| [plan-every-agent.md](plan-every-agent.md) | Watch / answer / drive per agent, spikes and notes |
| [plan-github-luna.md](plan-github-luna.md) | GH0 to GH9 delivery sequence |
| [plan-field-fixes.md](plan-field-fixes.md) | FF1 to FF11: fixes and ideas taken from a sibling app |

## Where we are (2026-10-25)

- Released: 0.1.8 (tag `v0.1.8`). `main` (at `fd7c12b3`) also carries N0 to N3, UX7, UX8 (agent
  profiles, specialist Chat, Codex, scoped GitHub), GH0 to GH8 and the Dictation workspace refinement
  (#385), all under *Unreleased* in the changelog.
- The 2026-10-16 plan (one release per stage: 0.1.9 = N0 + N1, 0.2.0 = N2, 0.2.1 = N3, 0.2.2 = N4) no
  longer fits, because a tag from `main` now ships all of it. **REL-1** below decides the new plan.
- Many tasks are implemented with automated evidence and wait only for a person at the desktop: they
  are the **ACC** tasks of Phase 0.

## How to read a task

Each task is one PR (or one recorded acceptance), small enough to pick up cold.

`P0` now (release blockers, certain bugs) · `P1` next · `P2` after · `P3` later or only if decided.
Size: `S` up to a day of focused work · `M` a few days · `L` a week or more, split before starting.
**Needs** names tasks that must be done first. **Done when** is the running-app criterion; tests
alone close nothing that touches the desktop. Work each task with *How to work a step* at the end of
this file; on merge, move it to [changelog-dev.md](changelog-dev.md).

## Next up

The queue, in order. Take the first task whose *Needs* are met.

| # | Task | P | Size | Why first |
| --- | --- | --- | --- | --- |
| 1 | REL-1 Choose the release plan | P0 | S | Everything else ships through it |
| 2 | FF1 Canvas follows the monitor's scale | P0 | S | Certain bug, small fix |
| 3 | FF5 Keep *Copied* dictation out of clipboard history | P1 | S | Privacy, small fix |
| 4 | R3 Release smoke | P0 | S | Gates the next tag |
| 5 | ACC-UX6 to ACC-DICTUI Acceptance passes | P0 | M | Finished work waiting for a person |
| 6 | REL-2 Tag and publish | P0 | S | Ships N0 to N3 and UX7/UX8 |
| 7 | S14 Release a card whose hook is already gone | P1 | S | Correctness of cards |
| 8 | FF3 Quiet finish when the terminal is in front | P1 | M | Daily annoyance |
| 9 | FF4.1 to FF4.4 Update notice | P1 | M | Users miss releases |
| 10 | GH9 GitHub release readiness | P1 | L | Closes the GitHub board |

---

## Phase 0 — Release gate

### REL-1 · Choose the release plan
`P0 · S · Needs: —`
- [ ] Decide the version of the next tag from `main` (0.1.9 or 0.2.0) and whether later releases follow
  phases of this roadmap. Recommendation: **0.2.0 now** (it already holds N2's Flock and Chat, which
  ADR 0020 tied to 0.2.0), then a minor release per one or two phases.
- [ ] Record it: a dated line in [ADR 0020](../adr/0020-0.2.0-at-wave-4-then-0.2.x.md)'s *History*
  (and its pt-BR twin with a new source mark), and update *Where we are* here.

**Done when:** the ADR records the plan and this section matches it.

### R3 · Release smoke
`P0 · S · Needs: REL-1`
- [x] 2026-10-26 local Security triage: all 16 Gitleaks findings are synthetic redaction fixtures
  or non-secret UI labels/identifiers. Exact commit/path/rule/line fingerprints suppress only those
  historical occurrences; a full-history scan passes with Gitleaks 8.24.3. New same-path PAT and
  generic-key canaries still fail detection. `scripts/check-secret-fixtures.sh` runs after Gitleaks
  in Security, and PR changes to scanner configuration, exceptions or this check trigger it.
  Pending: merge this fix and confirm Security on main. No real credential was identified among
  these 16 findings; no credential validation request or history rewrite was performed.
- [ ] Install the build as a user would (the `.deb` or `.rpm` from the draft release, not `tauri dev`).
- [ ] Island: Mini and Quick, a session arriving, finishing and failing, a permission card answered by
  click, a question card.
- [ ] Chat: one CLI turn and one API turn; voice input into the draft.
- [ ] GitHub: Connections loads, one row opens.
- [ ] Nest: Flock, Chat, Connections, Activity, Dictation tabs; Settings; `/continue` and its owned Stop;
  spend and budget.
- [ ] Zeca off: flock, cards, connectors and the Nest work without Chat.
- [ ] Record the date, desktop (KDE Wayland, X11) and result in the release PR.

**Done when:** every line passes on one Linux desktop; a failure becomes its own task before the tag.

### REL-2 · Tag and publish
`P0 · S · Needs: R3, ACC-UX6`
- [ ] Follow [releasing.md](releasing.md): versions in the four files, CHANGELOG *Unreleased* renamed,
  pt-BR docs current, `scripts/check-version.sh`.
- [ ] Tag, review the draft (install one package), publish, push the AUR PKGBUILD.

**Done when:** the release is public and the AUR package builds.

### ACC-UX6 · Final desktop matrix
`P0 · S · Needs: —` · Steps in [plan-ui.md](plan-ui.md#ux6-final-desktop-acceptance)
- [ ] State and B05; pending card and ownership; Zeca off; geometry and desktop (480 × 360, wide and
  short Nest, two displays and scales, minimize and restore, click-through under Wayland).

**Done when:** the user ticks the matrix in plan-ui.

### ACC-UX7 · Mini/Quick widths on a real compositor
`P1 · S · Needs: ACC-UX6`
- [ ] The four widths (Default, 560, 680, 760), Mini at 38 px, the overflow icon routing to the Nest
  Flock, keyboard in the width picker, on KDE Wayland and X11.

**Done when:** recorded in plan-ui's UX7 section.

### ACC-UX8 · Agent profiles and specialist Chat
`P1 · S · Needs: —`
- [ ] Copy a preset, edit name, instructions, species and accessory; reload and see the revision.
- [ ] Open a profile's Chat; one live Claude Code turn; a permission card answered; switch back to Zeca
  and check the transcripts stay apart; repeat with Zeca off.
- [ ] Light theme and minimum size (480 × 360) for the profile views.

**Done when:** recorded in plan-ui's UX8 section.

### ACC-CODEX · Codex specialist with live inference
`P1 · S · Needs: ACC-UX8`
- [ ] One live turn with a Codex profile using the user's existing CLI login; resume; a changed login
  is refused for an old thread.

**Done when:** recorded in plan-ui's *Agent refinement: Codex and GH8*.

### ACC-MEET · Saved meetings workspace (V5.8.2)
`P1 · S · Needs: —`
- [ ] Record a short consented meeting, save it, reopen, search, export MD, SRT and VTT, delete it, and
  check no audio is left under the app's data folder.

**Done when:** the maintainer's sign-off is recorded in plan-dictation's V5.8 section.

### ACC-DICTUI · Dictation workspace refinement (#385)
`P1 · S · Needs: —` · Record in plan-ui's 2026-10-25 Dictation entries
- [ ] Fresh desktop visual pass of the regrouped Dictation navigation, Voice settings, History, Dictionary,
  Snippets and Saved meetings, at Room size and 480 × 360, in Black and Light.
- [ ] Keyboard-only and screen-reader pass: sidebar arrows and Home/End, search with Escape recovery,
  deletion confirmation returning focus to the row.
- [ ] Native destructive clicks (delete an entry, bulk erase in History options) on test data only.

**Done when:** recorded in plan-ui; existing saved entries untouched.

---

## Phase 1 — Island and Flock

### FF1 · Canvas follows the monitor's scale
`P0 · S · Needs: —` · [Spec](plan-field-fixes.md#ff1--canvas-follows-the-monitors-scale)

Implementation status (2026-10-26, branch `fix/release-019-stabilization`): mutable Scene/Sky
bitmap scale and one shared resolution watcher are implemented. The watcher re-arms the raw
resolution query after every change, including fractional changes within the same integer pixel
scale. Existing layout zoom and bird state remain unchanged; old-scale sprite caches are cleared.
Active idle canvases repaint and hidden canvases do not start animation. Regression checks first
failed on a 1x-to-2x transition and now cover repeated transitions, fractional re-arming, preserved
CSS dimensions, cache invalidation and hidden-frame suppression. Native two-monitor KDE acceptance
is still pending; keep FF1 open until the Done-when criterion is recorded.

Before the fix, `dpr` was `private readonly` in `ui/src/island/scene.ts` and `ui/src/island/sky.ts`. A monitor change
only moved the surface (`place_surfaces` from `on_monitors_changed` in `app/src/lib.rs`); the page was
not reloaded, so the bitmap kept the first monitor's ratio.
- [x] A small watcher in `ui/src/` (`ui/src/pixel-ratio.ts`) that re-arms a
  `matchMedia("(resolution: <n>dppx)")` query after every change and calls its listeners.
- [x] `Scene` and `Sky`: make `dpr` mutable; on change, resize the bitmap (`size()`), drop the sprite
  caches keyed by scale, and `refresh()` even while idle.
- [x] Canvas audit: Scene and Sky are the only DPR owners; both use the shared watcher.
- [x] Test: stubbed `matchMedia` fires repeated and fractional changes; both canvases resize.

**Done when:** moving the island between a 1× and a 2× output on KDE leaves the birds whole and sharp,
noted in the PR; the test passes.

### S14 · Release a card whose hook is already gone
`P1 · S · Needs: —`

`ipc` waits `ACK_TIMEOUT` (800 ms) for the ack, then the hook gives up; `runtime.rs` acks on
`Effect::AckPermission` without checking whether that is already too late, so a card can show for a
hook that left.
- [ ] In `crates/ipc`, expose `ReplyHandle::is_gone()` (the channel's `is_closed()`).
- [ ] In `app/src/runtime.rs` at `Effect::AckPermission`: if the handle is gone, or the pending offer's
  `since` is older than `ACK_TIMEOUT`, do not ack; feed core an input that ends the offer without an
  answer (reuse the expiry path; never an `Intent`, so rule 2 stays untouched).
- [ ] Core test: a late request yields no card left on screen; an on-time one still shows.

**Done when:** the test passes and a hook killed before the ack leaves no card in the running app.

### FF3 · Quiet finish when the session's terminal is in front
`P1 · M · Needs: —` · [Spec](plan-field-fixes.md#ff3--quiet-finish-when-the-sessions-terminal-is-in-front)
- [ ] App: from `vults_platform::focus::Tracker`, on change only, map the active window to a session
  with `focus::session()`; send it to core as a plain input.
- [ ] Core: keep `watched: Option<SessionKey>`; expose `SessionView.watched` (regenerate `view.gen.ts`).
- [ ] UI: in `ui/src/island/render.ts` `cues()`, treat a watched session like a muted one for *done* and
  *fail* (no `Sound.play`, no `fsm.reveal`); cards keep their sound and reveal.
- [ ] Setting in General › Notifications, on by default; strings in the four languages.
- [ ] Docs: `guide/island.md`, `reference/settings.md`, both languages.

**Done when:** a finish while looking at that terminal makes no sound or reveal; a permission card in
the same situation still sounds; GNOME Wayland behaves as today.

### FF2 · Idle CPU of a minimized Nest
`P2 · S · Needs: —`
- [ ] Minimize the Nest on KDE Wayland with a busy flock; watch `top -H` on the WebKit process for a
  minute. Record the figure in the PR or here.
- [ ] Only if it is not near idle: let Rust emit the Nest's shown/minimized state and have
  `ui/src/surfaces/nest/render.ts` combine it with `document.hidden` before `setActive`.

**Done when:** a minimized Nest is near idle, measured.

### FB · Flock board animations
`P2 · L · Needs: —` · One PR per item; each redrawn on the current head against the approved art
([board](https://claude.ai/artifact/PZH7HwmUNUJeTnw5MBvkeV), [review](https://claude.ai/artifact/1phpxZ7YcaVzUcMD9yfGTt)).
- [ ] FB-03 take-off with a crouch and a hop; landing shows the legs and squashes
- [ ] FB-09 turning on the wire passes through the front
- [ ] FB-11 the wire sags under each bird and tugs the neighbor
- [ ] FB-44 per-species take-off
- [ ] FB-34 male and female where nature tells them apart, from the session hash
- [ ] FB-35 age: chick, juvenile, adult over weeks of work
- [ ] FB-36 leg or wing band per species, color and number from the hash
- [ ] FB-29 temperament per species
- [ ] FB-30 a work animation per species
- [ ] FB-40 following: the scout lands after a subagent result, the others follow
- [ ] FB-47 third failure in a row: two frames of a drop, behind a setting
- [ ] FB-42 visitors by region and season
- [ ] Check in the app: #48 (big species fit the pill), #23 (flush on failure and approval), #28
  (soaring together or alone). Later: #33, #27, #46.

**Done when (each):** sprites match the approved prototype pixel for pixel; reduced motion respected.

### K7 · Zeca's mood from the flock
`P2 · M · Needs: K1`
- [ ] Core derives calm, alert, worried or celebrating from existing attention; Zeca looks at the bird
  he talks about.
- [ ] Each mood drawn with its reduced-motion still.

**Done when:** a running-app review per mood.

### Z5 · Memory of a closed on-demand window
`P3 · S · Needs: —`
- [ ] Find whether wry keeps the webview or WebKit caches the process (about 110 MiB) after the Nest
  closes; write the finding here; fix only if it is ours.

### X9 · Open terminal on more desktops
`P3 · M each · Needs: —`
- [ ] X9-HYPR Hyprland (`hyprctl dispatch focuswindow`)
- [ ] X9-SWAY Sway (`swaymsg [pid=…] focus`)
- [ ] X9-GNOME GNOME Wayland through a Shell extension (a decision first: shipping an extension)
- [ ] X9-TERM terminals by name: Ghostty, Alacritty tabs

**Done when (each):** Open terminal brings the session's window forward on that desktop.

### FF9a · Island options seen in a sibling app
`P3 · S each · Needs: —` · Check the current settings before each; drop the ones that exist.
- [ ] FF9a-1 optional clock in the middle of the compact island
- [ ] FF9a-2 own sounds: a file named like a built-in cue in a sounds folder replaces it (`ui/src/sound.ts`
  synthesizes cues today, so this means loading a file per cue)
- [ ] FF9a-3 keyboard chords in the open island: move through lists, open the row, open the diff
- [ ] FF9a-4 alerts from any session: another session that finishes or fails opens the island for a
  moment, then the previous one returns; off by default

---

## Phase 2 — Agents and coverage

Spec and per-agent notes: [plan-every-agent.md](plan-every-agent.md). One agent per PR. Every config
write follows rule 3 (backup, diff, click).

### W2 · Show the working folder on every card
`P1 · M · Needs: —`
- [ ] Parse the folder other agents send with a shell call (`workdir`, `directory`) and a search's path,
  per driver in `crates/agents/src/*.rs`.
- [ ] Carry it in `ApprovalView` (`crates/core/src/view.rs`) and show it on the card in island and Nest.
- [ ] Policies keep allowing only for Claude Code until each agent's folder is shown; then widen per
  agent with a test.

**Done when:** a Codex, Qwen and OpenCode shell card shows its folder; policy tests cover each.

### LC · Live checks
`P1 · S each · Needs: —` · Write each result in plan-every-agent's *Notes*.
- [ ] LC-1 Codex card round trip in its TUI (headless pty hangs; needs a real terminal)
- [ ] LC-2 a `/continue` Codex call where its own hook also fires: one card answers, the other is
  released
- [ ] LC-3 Antigravity with the real app (fixtures only so far)
- [ ] LC-4 OpenCode: plugin card versus its own TUI prompt; whichever answers first wins

### FF9b · Agent follow-ups to verify
`P2 · S each · Needs: —` · Verify against a real session, then one PR each.
- [ ] FF9b-1 Claude Code plan ready (`ExitPlanMode`): a card with the terminal's own choices; *Change
  plan* brings the terminal forward. Still a click (rule 2)
- [ ] FF9b-2 session name from Claude Code's `/rename` title, else the folder it started in
- [ ] FF9b-3 Antigravity asking a question: the bird turns to a question with its cue
- [ ] FF9b-4 sessions started by wrappers (Conductor and similar) land on the right agent; Open
  terminal brings the wrapper forward

### UX12 · Agent availability in the UI
`P2 · M · Needs: —` · [Spec](plan-ui.md#ux12---ui-delivered-with-coverage-and-mobile-features)
- [ ] Settings › Agents and Flock show Watch / Answer / Drive from `crates/agents/src/registry.rs`, with
  verified versus planned; planned agents never appear installed.

### New agents
`P2–P3 · M each` · Each: a spike with `scripts/spike.sh`, a recorded fixture, the installer, docs
(`other-agents.md`, `approvals.md`, README list, both languages), CHANGELOG.
- [ ] B4-CLINE Cline CLI · [ ] B4-KIRO Kiro CLI (Claude-style hooks) — P2
- [ ] C1 a recorded session from a real Cursor — P2
- [ ] C5-AMP Amp (TS plugin) · [ ] C5-PI Pi (TS extension) · [ ] C5-HERMES Hermes (Python plugin;
  enabling stays the user's) — P2
- [ ] F3 YAML writer (lossless, comments kept), then C6-GOOSE Goose — P3
- [ ] C6-CRUSH Crush · [ ] C6-GROK Grok Build · [ ] C6-MUSE Muse Code — P3
- [ ] R2 ACP rows where the ACP mode passes a spike: Goose, Auggie, Cline, Hermes (Droid waits for a
  Factory login) — P3

**Done when (each):** installed and seen working with the real CLI; answers where answerable, "waiting
in the terminal" otherwise.

### G1 / G2 · An open reply for any tool
`P3 · L · Needs: ADR 0027 accepted`
- [ ] G1 the user accepts or rejects [ADR 0027](../adr/0027-an-open-reply-for-any-tool.md).
- [ ] G2 protocol `VERSION` 9, the hook, core keyed by tool name, Settings lists named tools,
  `protocol.md` (+ pt-BR) with shell and TypeScript examples.

**Done when:** a shell test tool gets a card, an allow and a deny; an old hook stays silent.

### Smaller agent tasks
- [ ] QW Qwen `write_file` diff from `tool_response.returnDisplay.fileDiff` into hunks — P3 · S
- [ ] F4 public issues per agent from [Adding an agent](../contributing/agents.md), on the maintainer's
  go — P3 · S

---

## Phase 3 — Chat and Zeca

### UX8-QS · Agent profiles in Quick
`P1 · M · Needs: ACC-UX8` · [Spec](plan-ui.md#ux8---agents-workspace-a-new-feature-in-stages)
- [ ] A Quick summary of profiles and their running conversations, reusing the Nest's profile owner;
  waiting cards keep priority.

**Done when:** native check at Quick sizes in both themes; no new authority.

### UX8-PROJ · Profile runs on the wire
`P1 · M · Needs: UX8-QS`
- [ ] Working specialist runs appear through the existing session pipeline with their profile identity.
- [ ] Prove one run never shows as two birds or two conversations (test plus native check).

### K1 · When Zeca speaks unasked
`P1 · M · Needs: —`
- [ ] Core rule: now, debrief or never; at most 4 unasked lines an hour; silent in full screen or focus;
  presence from the screen lock.
- [ ] Core test: with the budget spent, a card still opens the island (ADR 0021, D5).

### K2 · Debrief on return
`P2 · M · Needs: K1`
- [ ] The away digest and memory, written by Zeca, in Quick's reserved slot.

**Done when:** the debrief is opened on most returns (local metric).

### K3 · Card explanation
`P2 · M · Needs: W2`
- [ ] A risk line and an out-of-scope warning per permission card (`card-risk` in the prototype).
- [ ] Obvious reads offered as a batch, still answered with one click.

**Done when:** average card wait goes down (local metric).

### Zeca follow-ups
- [ ] Z8 warn a fetch's own card before the call (taint lands only when it ends) — P2 · S
- [ ] M7f the broader `vults-zeca` MCP for Codex through the installer's diff, backup and click — P2 · M
- [ ] Z6f OpenAI through the Responses API with native compaction, once someone can test with a key —
  P3 · M

### Later in Chat
`P3` · Each needs its own design pass first.
- [ ] K4 routines (morning brief, nightly CI, dependencies, watching a page), each through the gate
- [ ] K5 proposals from repeated denials, corrections or the same CI failure
- [ ] K6 skills in `SKILL.md` format, written with rule 3
- [ ] K10 Zeca as an ACP agent for Zed, JetBrains and Neovim
- [ ] UX9 Studio library: skill import and reimport, verified tool connections
  ([spec](plan-ui.md#ux9---studio-library-and-tool-connections-later))
- [ ] UX10 scoped investigations with retained evidence
  ([spec](plan-ui.md#ux10---investigations-later-and-scoped))

Each K task ships its own UI slice; the acceptance per slice is in
[plan-ui.md](plan-ui.md#ux11---ui-delivered-with-existing-companion-features) (UX11).

---

## Phase 4 — Dictation

V1 to V8 are implemented. What is left is human acceptance, then small fixes and new capability. Spec
and the full gate list: [plan-dictation.md](plan-dictation.md) (*Remaining human and external
gates*).

### DG · Human acceptance gates
`P0 · S each · Needs: —` · Record each in plan-dictation.
- [ ] DG-1 natural PT and EN dictation, with existing drafts and cancellation (V1, V2)
- [ ] DG-2 physical shortcuts, portal grants, focus change, editor and terminal insertion, and the
  question card filled by voice and confirmed by a click (V1, V7); voice never answers a permission
- [ ] DG-3 release-to-delivery p95 with Small on Vulkan, target under 2 s (V4)
- [ ] DG-4 voice commands spoken for real: targets, effect and undo (V8)
- [ ] DG-5 paid providers, only after separate consent: cleanup, summary, realtime, spoken flock reads,
  interruption and eight-second capture (V3, V6)

### FF5 · Keep *Copied* dictation out of clipboard history
`P1 · S · Needs: —` · [Spec](plan-field-fixes.md#ff5--dictation-keep-dictated-text-out-of-clipboard-history)
- [ ] In `crates/platform/src/clipboard.rs` `copy()`, offer `x-kde-passwordManagerHint` as the paste
  leases already do (`insert/lease.rs`, `insert/wayland_lease.rs`); reuse their code.
- [ ] `confirm()` read-back still matches.

**Done when:** with Klipper running, a *Copied* dictation is not in its history; paste still works.

### V5q · Meeting speaker quality gates
`P1 · L · Needs: —`
- [ ] Freeze independent, speaker-disjoint, human-annotated PT and EN material.
- [ ] Score the current preview; labels stay a developer preview until it passes.

### FF6 · An optional second speech engine
`P2 · L · Needs: an ADR`
- [ ] ADR: a second model (Parakeet TDT 0.6B v3, CPU, 25 languages) on the existing `ort` dependency of
  `crates/voice`; download size, license, consent.
- [ ] If accepted: engine choice in Dictation › Voice (Whisper default), checked download, evals on both
  engines with WER and stage p95.

### Smaller dictation tasks
- [ ] FF7 compare the zh/en *common advanced* CAM++ export with the current one; manual merge of two
  speakers and moving a line in Saved meetings — P3 · M
- [ ] FF9c names learned across meetings (misheard words fixed, names kept), inert text behind the
  summary consent — P3 · M

---

## Phase 5 — Connections

### GH9 · GitHub release readiness
`P1 · L · Needs: —` · [Spec](plan-github-luna.md#gh9--recovery-ui-documentation-and-release-readiness)
- [ ] GH9-1 recovery: app restart mid-write, network loss, account switch and GitHub errors, each with
  a test.
- [ ] GH9-2 a controlled test repository and an end-to-end run of the plan's flows.
- [ ] GH9-3 docs in both languages: `gh` setup, host and account, tools, budgets, permissions, recovery,
  limits; source marks updated.
- [ ] GH9-4 review rate and byte budgets before any new background request.
- [ ] GH9-5 a reviewer walks every external write path; CI green on Linux and Windows.

### GHA · GitHub acceptance
`P1 · S · Needs: GH9-2`
- [ ] Visual acceptance of Connections controls (GH0 to GH3).
- [ ] On the controlled repo: issue write, PR draft and review, an Actions rerun, a merge, each through
  its card (GH4 to GH8).

### GH1r · Repository access diagnosis
`P2 · M · Needs: —`
- [ ] A user-triggered, budgeted check that tells no CLI, no login, expired auth, rate limit, SSO or
  forbidden, and inaccessible repository apart; the last identity is kept but marked stale on failure.

### Later in Connections
- [ ] FF9d-GITLAB GitLab connector (merge requests, reviews asked, default-branch CI, activity), token
  in the keyring, through the gate — P3 · L
- [ ] FF9d-GITEA Gitea, same shape — P3 · L
- [ ] CZ the Connections face for Zeca (`conn-face-zeca`) with a real source of state — P3 · M

---

## Phase 6 — Activity

### W9b · Cost of watched sessions
`P2 · L · Needs: —`
- [ ] A loopback-only OTLP receiver for the telemetry Claude Code and Codex export.
- [ ] Turning it on writes the agents' telemetry settings through the installer (diff, backup, click).
- [ ] Map tokens to sessions; label estimates; off and unknown before opt-in.

**Done when:** a watched Codex session's tokens count in the day.

### FF9e · Weekly recap parity
`P3 · S · Needs: —`
- [ ] Compare `crates/core/src/recap.rs` and the Week image export with the sibling app's recap; list
  real gaps as tasks or close this.

---

## Phase 7 — Settings, updates and distribution

### FF4 · Update notice, asked for
`P1 · M (four PRs) · Needs: —` · [Spec](plan-field-fixes.md#ff4--update-notice-asked-for)
- [ ] FF4.1 pure rules with tests: this platform's tag prefix, no pre-releases, version order,
  `should_check` **at most daily** (rule 4), snooze 24 h, ask again after a day if unanswered.
- [ ] FF4.2 a gate feature `update` in `app/src/gate.rs` (switch, small budget, trace); one
  unauthenticated GET of the releases list; add it to `scripts/network-allowlist.txt`.
- [ ] FF4.3 consent asked once in the island; Settings › General › Updates with *Check now*; the notice
  card (version, notes, *Open release page*, *Later*); Zeca's badge only when nothing else waits.
  Distro builds (AUR) compile the feature out. Strings in four languages.
- [ ] FF4.4 docs: `docs/safety.md` request list, settings reference, ADR 0019 *History*, both
  languages; CHANGELOG.

**Done when:** with the switch off, no request is made (traces empty); with it on, at most one a day,
traced; the AUR build has no updater code.

### Later in distribution
- [ ] FF4.5 What's new after an update: one entry per version, 1 to 4 items, four languages, a CI check
  — P2 · M
- [ ] UPD the signed updater with channels, after a signing key exists (ADR 0019) — P2 · L
- [ ] LANG more UI languages; Arabic needs a right-to-left pass first — P3 · M each
- [ ] PKG-APPIMAGE (needs a faster build) · PKG-FLATPAK (sandbox reaches the socket, agent configs
  and layer-shell) · PKG-MACCI · PKG-WINSIGN (a code-signing certificate) — P3
- [ ] X7 Windows and macOS betas, when Linux is settled (*Priorities* in CLAUDE.md) — P3 · L
- [ ] X6 a headless browser for Zeca with its own profile — P3 · L

---

## Phase 8 — Mobile

The desktop is the source of truth; the phone only decides. UI acceptance per task in
[plan-ui.md](plan-ui.md#ux12---ui-delivered-with-coverage-and-mobile-features).

### L2f · Pairing leftovers
`P2 · S each · Needs: —`
- [ ] L2f-1 an audit row for a pairing and a revoke (`phone_pair`, `phone_revoke` in
  `app/src/phone.rs`, written through `crates/store/src/audit.rs`).
- [ ] L2f-2 two phones racing on one card: the ledger takes the first, the second gets `Used`, with a
  test.

### Android
`P3 · L each`
- [ ] L3 Android MVP in Kotlin/Compose over the Rust core: pair, flock, deny, approve and answer with a
  biometric Keystore key and key attestation; UnifiedPush with FCM fallback. Done when an approval at
  100 s works and past 108 s is refused.
- [ ] L4 handoff and Zeca's proposals from the phone. Needs L3.
- [ ] L5 service actions through connectors, GitHub first; an action offered over 5 minutes ago is
  refused. Needs L3.

---

## Decisions pending

| Item | What to decide |
| --- | --- |
| REL-1 Release plan | The next version and the release cadence (task above) |
| FF10 Screen capture for agents | Yes or no: the Screenshot portal to the clipboard; it has no link to sessions unless wired to Chat or an agent |
| FF11 Zeca reacts to the machine | Yes or no on tone; would reuse a visibility signal so it reads nothing while hidden |
| FF8 A spoken wake phrase | **Declined** by *Out of scope* (*Wake word: No*); reopening needs an ADR |
| X9-GNOME | Whether Vults ships a GNOME Shell extension |

## Watch

The folded island vanished once in an R3 pass (likely the harness switching displays); the sunglasses'
summer window is still to choose; Codex keys hook trust by position, so removing ours may make Codex
ask again for a later hook.

## Dropped

- 2026-10-12: X1 side panel, X2 a bird on the desktop, X3 a voice per vult (reconsider after V6), X4
  Vercel, cloud transcription, X8 perch together; W10's Gemini CLI check (Gemini CLI retired, #310).
  X5's voice dictionary was reopened as local-only V2 on 2026-10-17.
- 2026-10-16: Tasks (W3) and what only they used: runs in their own clone (W4), the handoff through a
  context pack (W5), the review loop (W6), the router behind `/delegate` (W8).

---

# Reference

The decisions, guarantees and working method below hold for every row above. They were part of Plan
Zeca and are unchanged except for the file names in the method.

## Decisions

Each settles a conflict between earlier plans, a review and the code.

| Topic | Decision |
|---|---|
| Storage | A `store` crate (Core): one `vults.sqlite`, bundled `rusqlite` with FTS5, numbered migrations. Recall's vectors are a brute-force cosine over at most 90 summaries, no `sqlite-vec` |
| Offer ledger | In `core` (pure), persisted by `store`: cards and the phone work with Zeca off (D7) |
| Action classes | `core::policy::Class` (Read, ReadExternal, Write { destructive }, Spend, External) for agents and Zeca |
| Expiry | An offer lives from when `ipc` read the hook's line until `SERVER_DECISION_TIMEOUT − GRACE` (107 s) |
| Layers | Zeca's brain is the `zeca` crate (Connect), never depending on `vults-core`; his body stays in Experience; `check-layers.sh` reads `cargo metadata` |
| Agent drivers | The `AgentDriver` trait in `agents` (Core, no tokio), implemented in `chat`, injected by `app` |
| Zeca's own work | Never a session: his CLI carries `VULTS_ZECA=1`, the hook marks the event, `core` routes it to the chat |
| Surfaces | One Nest window, built when opened (D6): Flock, Chat, Connections, Activity (Tasks left in N0); Dictation is the fifth tab as N3 phases land. Settings keeps its own webview ([0021](../adr/0021-four-sizes-one-nest.md)) |
| Dictation | A pure Connect crate for key/text logic, capture in `audio`, insertion in `platform`, orchestration in `app`; no text into a waiting session; Settings owns values and Activity the figures ([0032](../adr/0032-dictation-into-other-apps.md), [0021](../adr/0021-four-sizes-one-nest.md)) |
| Permissions | Zeca never answers one; only a policy the user wrote may allow or deny (ADR 0004); destructive always asks (ADR 0004) |
| Requests nobody asked for | ADR 0019: each off by default, its own switch, budget and local trace, through one gate in `app` |
| A phone approving | ADR 0004: a `BIOMETRIC_STRONG` signature counts as a click; D1 is "the island and the phone" |
| Journal | Opt-in with Zeca's memory; secrets redacted before writing |
| Skills, `AGENTS.md`, `CLAUDE.md` | Rule 3: backup, diff, click. Zeca's MCP goes through `--mcp-config` on the process; no config written |
| MCP server | `vults --zeca-mcp`, a small hand-written stdio server (no `rmcp`) |
| Two paths | Path B (Claude Code, Codex) is never worse than the CLI in a terminal; path A (API key or local) never runs code, it delegates |
| Reference project | Named only in `NOTICE` (`check-brand.sh`) |

### D1 to D8

Design decisions from the road to 0.2; `caps.rs`, the core's tests and ADRs cite them.

- **D1. A card is answered on the island, or in the Nest while the user is in it** (ADR 0021); a
  paired phone may answer too (ADR 0004). The tray, widget, notifications and palette only *open*
  the card.
- **D2. Meaning in core, look in the surface.** Core says who needs you (`Attention`), who is in
  front (`focus`) and how a card ended (`Outcome`); surfaces pick clips, colors and sounds. Size,
  tab and filter are look: the app keeps them as a small `Place`.
- **D3. One `ViewModel` for every surface**, until a payload gets heavy.
- **D4. Every surface speaks in `Intent`s.** A new intent joins the rule-2 test the day it lands.
- **D5. No preset leaves an acknowledged card unseen.** *Island*, *Panel* and *Quiet* open the
  island on a card with its sound; *Paused* never acknowledges: it releases the card to the terminal
  at once, saved *Always* rules answer nothing and connectors stop.
- **D6. One live webview at rest.** The island always lives; the widget while chosen; the palette,
  the Nest and Settings are built when opened and closed when done.
- **D7. Zeca is a layer, not the core.** With him off the flock, cards, notifications, connectors
  and the Nest (without Chat) all work.
- **D8. No *Stop* until an agent offers one.** Vults stops only the sessions it started (W4; after
  N0, only `/continue`); the menu shows only what works.

**C7, how a watched agent could be stopped** (research 2026-10-08). Never a signal: SIGINT ends
Claude Code's whole session and Codex and agy exit too, orphaning children. Claude Code: our
PreToolUse answers `{"continue": false, "stopReason": …}`, the turn ends before the next tool (a
text-only turn cannot be stopped). Codex: `continue: false` only after a tool; the clean path is the
app-server's `turn/interrupt { threadId, turnId }`, untried on a watched session. agy:
PostInvocation `terminationBehavior: "terminate"`, a hook we do not register. In tmux,
`send-keys -t $TMUX_PANE Escape` is a real Esc, untested. So a *Stop*, when it comes, is a flag a
click arms and our next PreToolUse carries; it says "stops before the next tool" and clears on Stop
and SessionEnd.

## Guarantees

Each invariant has an automatic guard; a step that weakens one does not pass review.

| Invariant | Guard |
|---|---|
| Never block an agent (rule 1) | The hook's tests, its panic hook (G2), fuzzing (S5), the cold-start ceiling (G4) |
| A permission only by a human click or recorded consent (rule 2) | `only_decide_can_respond` over every `Intent` (H1), the proptest (S4), the ledger (S3), the policy's consent test (W2) |
| A hook waits only while its card can be seen | D5's core test in every preset |
| An offer never outlives the hook | Expiry from the hook's arrival (S3, S14) |
| Zeca never answers a permission; his work is never a session | `FlockReader` with no `Intent` (S9), ADR 0018; the marked event (Z1) |
| Nothing leaves the machine unless turned on (rule 4) | One gate with a switch per feature; all off, zero requests (S10); `check-network.sh` |
| Outside content does not trigger an action | Q-LLM, taint, injection evals (Z7, Z8, S7) |
| An agent's config only with backup, diff and click (rule 3) | `agent-config`; MCP through `--mcp-config`; `AGENTS.md` and skills through the same flow (M9, K6) |
| A stable prompt through a conversation | A snapshot of the system prompt's hash (M2) |
| Secrets are never written | The keyring; redaction before the journal, recall and traces (M4, S6) |
| Zeca's lines never hide or delay a card (ADR 0021) | D5's test; K1's with the budget spent |
| The layer surface is never resized or unmapped | A checklist line in every platform PR |
| The island's look changes only on purpose | Running-app review after visual changes |
| Settings written by older versions still load | Fixtures (H4) |
| Dictation never answers a permission | Required before N3 delivery: a waiting session yields Copied (V1.11); H1 covers voice intents (V6.6) and commands (V8.3) |
| Meetings leave no audio archive | Required before V5 delivery: temporary-WAV cleanup on success, failure, cancellation and startup after an interrupted run (V5.1) |
| The Rust and TS views agree | Generated TS with a freshness test (H2) |
| Layers, language and brand | `check-layers`, `check-english`, `check-i18n`, `check-brand` |

Metrics, from local traces and audit only: recall hit@3 ≥ 0.8; ≥ 50 % of memory proposals accepted;
≥ 8 of 10 planted bugs sent back; tasks finished without a terminal, up week after week; unasked
lines dismissed unread < 20 % (K1); card wait time down after K3; the debrief opened on most returns
(K2).

## Out of scope

| Item | Decision | Why |
|---|---|---|
| Zeca approving another agent's permission | Never | Flock content may carry an injection |
| Zeca running code in the harness | Not in v1 | Code is the agents' work; Zeca coordinates |
| A logged-in browser or the user's credentials | Never | Zeca has his own profile and keys |
| A third-party agent framework | No | Owning the small loop allows taint, the ledger and traces |
| A messaging gateway (Telegram and the like) | No | Zeca lives on the desktop, the editor and the phone |
| A duel (one task on two agents) | Put off | Doubles the cost; the score teaches the same |
| A2A | Put off | ACP and MCP cover coding agents |
| Wake word | No | Hands-free dictation is a double tap (V1); realtime follow-up is separately opted in and bounded |
| Agents started without permissions | Never | Rule 2 |

## References

| For | Reference | What to take |
|---|---|---|
| K*, context | [Anthropic Engineering](https://www.anthropic.com/engineering), [context engineering](https://anthropic.com/engineering/effective-context-engineering-for-ai-agents), [managing tool context](https://platform.claude.com/docs/en/agents-and-tools/tool-use/manage-tool-context) | Effective agents and tools; just-in-time context; caching and context editing |
| Memory, K5 | [Hermes memory](https://hermes-agent.nousresearch.com/docs/user-guide/features/memory), [Letta context repositories](https://www.letta.com/blog/context-repositories), [sleep-time compute](https://www.letta.com/blog/sleep-time-compute), [gitoxide](https://github.com/GitoxideLabs/gitoxide) | Limits, staged writes, memory in git, sleep off the critical path |
| K6 | [AGENTS.md](https://agents.md/), [Agent Skills](https://agentskills.io) | Canonical project memory; portable skills |
| Drivers | [Codex app-server](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md), [Claude Code hooks](https://code.claude.com/docs/en/hooks) and [sandboxing](https://code.claude.com/docs/en/sandboxing), [sandbox-runtime](https://github.com/anthropic-experimental/sandbox-runtime) | `thread/*`, `turn/*`, `review/start`; marked events; bubblewrap |
| K10, R2 | [Agent Client Protocol](https://agentclientprotocol.com/protocol/overview), [Rust SDK](https://github.com/agentclientprotocol/rust-sdk) | ACP driver and Zeca as an ACP agent |
| Injection | [CaMeL](https://github.com/google-research/camel-prompt-injection), [an analysis](https://simonw.substack.com/p/camel-offers-a-promising-new-direction) | A Q-LLM with no tools; capabilities per value |
| W9b | [OTel GenAI conventions](https://github.com/open-telemetry/semantic-conventions-genai) | Span names; the telemetry the agents export |
| L3 | [UniFFI](https://github.com/mozilla/uniffi-rs), [UnifiedPush](https://f-droid.org/2022/12/18/unifiedpush.html) | The Rust core in Kotlin; push without Google |
| Orchestration | [Vibe Kanban](https://github.com/BloopAI/vibe-kanban), the reference (`NOTICE`) | A worktree per task; safe handoff; action tokens. Not taken: agents without permissions |

## How to work a step

1. Study: read `CLAUDE.md`, `docs/architecture.md`, the step and its references; write what an open
   item still needs next to it here, and the rest in the commit and the module docs.
2. Your own worktree and branch from `main` (one `CARGO_TARGET_DIR` per worktree).
3. Tests and evals first; docs, the four languages of UI text and the pt-BR docs in the same PR.
4. Run *Before every commit*; a review pass, at most two rounds of fixes, then ask the user.
5. PR, CI green, a merge commit (never a squash); move the step to changelog-dev.md in the same PR.

The prompt for a coding agent, one session per step:

```markdown
Implement step <ID> of docs/dev/ROADMAP.md: <title>.

Before coding: read CLAUDE.md, docs/architecture.md, the step and the references for its area.

Done when:
- <copied from the board>

Rules:
- Code, comments, docs and commits in English; UI sentences through t() with pt-BR, es and zh.
- Only the step's crates; no refactor mixed with a feature.
- Every action with an effect goes through core::policy and the ledger; Zeca never builds Decide or DecideAlways.
- No new network request without ADR 0019's switch, through the network gate in app.
- Tests and eval cases first; run CLAUDE.md's "Before every commit" list.
- Never write the reference project's name (check-brand.sh).

At the end, list what was left out and why, and move the step to changelog-dev.md.
```
