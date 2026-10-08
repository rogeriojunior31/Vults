# Road to 0.2: one core, many ways to live on the desktop

Internal plan (2026-10-04), checked against the code at `c1ed701`. The first release is **0.1.0**
(`road-to-0.1.md`, section 3). Everything below lands by **0.2.0**, shipped as small, frequent
**0.1.x** releases (ADR 0015, which supersedes the one-theme-per-version plan of ADR 0007):

| Area | Steps | In one line |
|---|---|---|
| Hardening | H1–H4 | Guards that make the refactors safe. |
| Experience | E1–E18 | The island stops being the only way to see the flock: tray, widget, notifications, focus, presence, voice. |
| Control | C1–C11 | The birds become handles: quick actions, a command palette, attention that escalates. |
| Platform | P1–P11 | Memory and context: history, projects (repo, branch, PR, CI), cost, agent capabilities. |
| Operations | section 9 | The Nest (full app), policies and autonomy, starting agents from here. This is **0.2.0**. |

## Releases

- A **0.1.x** goes out whenever one or two steps are merged and the smoke check passes (the R3
  checklist: the island, a card, the chat, voice, GitHub). A fix that cannot wait is the next
  0.1.x too. Patch numbers may pass 9 (0.1.10 is fine).
- Each release: `scripts/check-version.sh v0.1.x`, tag, draft release, AUR, docs.
- Steps follow the waves below; a wave can take several 0.1.x releases. Within a wave, steps
  that touch different files run in parallel; boards are updated after the merges.
- **0.2.0** is Operations, and only after ADR 0014 is accepted.

Released so far:

- **0.1.1** (2026-10-04): wave 1's first steps (H1–H4, E12, E16, the 0.1.0 leftovers, the sound
  dependency) and Zeca polished (#78): a lit back and a folded wing with two new tones (`i`, `d`),
  his blacks a step up, a breath and a shuffle in the idle; every species, tall bodies included,
  inherits the light from its own body color. The polish board (`plan-zeca-polish.md`) was
  deleted with the release.
- **Unreleased** (for 0.1.2): looks aligned and fourteen new ones (#85, #89, #91). A look follows
  each species' eye (the condor wore his glasses on his forehead) and a leaning head keeps the
  brim; looks are composed of pieces (hats, eyewear, a grill per pose) plus a neck slot whose
  pendant swings; five head-only looks and nine with chains join the Settings list. The board
  (`plan-looks-and-drips.md`) was deleted when its last step merged.

| Wave | Steps | Why this order |
|---|---|---|
| 1 | H1–H4, E12, E16 | Guards first; two small visible wins |
| 2 | E1, E2, E3, E11 | Meaning moves into core before any new surface |
| 3 | E4, E5, E6, E7 | Windows per surface, panel mode, notifications |
| 4 | E8, E9, E10, E17, E18 | Presence, widget, Zeca off, looks picker, the user's status line |
| 5 | E13, E14, E15, C10 | Voice in: silence detection, partial text, the speaking engine chosen |
| 6 | C1–C6, C8 | Control: quick actions, palette, attention ladder, away digest, quiet bird |
| 7 | C7, C9, C11 | Zeca speaks; voice commands; how agents could be stopped |
| 8 | P1, P2, P3, P7, P8 | History, roosts, capabilities, cost, audit log (Operations needs them) |
| 9 | P4, P5, P6, P9, P10, P11 | Side panel, the desktop bird, contribution grid, open-mic voice, Vercel |
| 0.2.0 | section 9 | ADR 0014 accepted, then the Nest, policies, starting agents |

The choices behind this plan are recorded in `docs/adr/` (0008 to 0015); this file holds the
steps and is deleted once they are done.

Linux (KDE first) through 0.2; Windows and macOS stay at the end (CLAUDE.md, *Priorities*).
Zeca stays optional: every surface works with him off.

Steps are worked like `plan-fourth-review.md` (*How to work a step*): study, mark `doing`, own
worktree, tests and docs in the same PR, `flock-tester` → `flock-reviewer`, PR, merge. One step,
one PR.

---

## 1. What the code says today

What the plan has to work around. Each line was read in the code.

| Fact | Where | What it means for the plan |
|---|---|---|
| The view goes to one window only, and only when it changed | `app/src/runtime.rs` (`emit_to(ISLAND, "view")`) | Any new window gets nothing until the emit is per surface. |
| The TS view types are written by hand; nothing checks them against Rust | `ui/src/bridge.ts` (`SessionView`…) vs `crates/core/src/view.rs` | A rename or removal fails silently. More surfaces, more readers: generate them first (H2). |
| "Who needs you", the settle delay, the sounds and `cardOnScreen` live only in `render.ts` | `ui/src/island/render.ts` | A second surface would copy them. Move the *meaning* to core before adding one (E1). |
| The session in front is chosen in the UI (`inFront`, `pinned`) | `render.ts` (`pick`) | Tray, widget and shortcuts cannot agree on focus until core owns it (E2). |
| Whether a card was answered "here" or "in the terminal" is guessed by the island | `render.ts` (`answeredHere`, `lastShown`) | A card answered from elsewhere would read "answered in the terminal". Core must say who answered (E3). |
| Ack is sent when core queues the card, not when it is drawn | `crates/core/src/lib.rs` (`Effect::AckPermission`) | The hook may wait up to 110 s for a card nobody sees. Today the island always opens and pins on a card, so this holds. Any preset that hides the island breaks it (D5). |
| Ctrl+Alt+Y/N answer only when the island shows the card | `render.ts` (shortcut handler) | Keep it: only the card's host answers a shortcut (D1). |
| The layer-shell code assumes one window: one global input region, `layout` and `island_keyboard` hard-wired to the island | `crates/platform/src/linux.rs` (`REGION`), `runtime.rs` (`layout`) | A second layer surface needs a per-window refactor first (E4). |
| New windows need a capability entry and a Vite entry | `app/capabilities/default.json`, `ui/vite.config.ts` | Part of E4. |
| The surface is mapped once at a fixed size and never resized or hidden | CLAUDE.md, `docs/architecture.md` | Presets change what the island *draws* and where it is anchored. They never resize it or unmap it. |
| The tray has a static icon and a 3-item menu; no left-click handler | `app/src/lib.rs` (`tray()`) | Panel mode (fourth review, step 7) starts from almost nothing. |
| `settings.json` writes `version: 1` and never reads it; unknown keys are dropped on save | `app/src/settings.rs` | After 0.1.0 people have settings files. Read the version before 0.2 adds keys (H4). |
| Visual tests address lab states by index | `tests/visual/island.spec.ts` | A new lab state shifts every screenshot. Use names (H3). |
| `State` is not persisted; sessions leave after 10 min (finished) or 30 min (silent) | `crates/core/src/lib.rs` (`FINISHED_TTL`, `SESSION_TTL`) | Per-session prefs (rename, pin) die with the session. Make prefs per project (C2). History is new work (P1). |
| The app cannot stop an agent. It can only act while a permission or question hook waits | `crates/hook/src/output.rs`, `crates/protocol` (`Reply`) | No *Stop* action until an agent offers a way (D8). |
| Jump raises the window on KDE and focuses tmux, kitty, wezterm and herdr panes. Nothing raises a window elsewhere | `crates/platform/src/jump.rs` | *Open terminal* is honest on KDE only. Say so in the menu. |
| `RespondPermission` comes from `Decide`, `DecideAlways` and a matching *Always* rule. Its doc says only `Decide`, and the pin test does not cover user intents | `crates/core/src/lib.rs` (`Effect`), `tests.rs` (`only_decide_can_respond`) | Fix the doc and widen the test before adding new intents (H1). |
| zbus 5 is already in the tree (media, and through ashpd) | `crates/media/Cargo.toml` | Desktop notifications and screen-lock signals need no new crate. |
| Chat and voice start lazily. Zeca is drawn as the island's idle look | `app/src/chat.rs`, `render.ts` | "Zeca off" is mostly UI: another idle look, plus gating the chat, mic, tray item and talk shortcut. |
| Release labels packages with `tauri.conf.json`'s version. Nothing checks the tag; the AUR `pkgver` hard-codes `0.0.0` | `.github/workflows/release.yml`, `packaging/aur/PKGBUILD` | Fixed by fourth-review step 5: `scripts/check-version.sh` stops a tag that is not the version; `pkgver()` reads `Cargo.toml`. |

## 2. What must not break, and what guards it

| Invariant | Guard today | Guard to add |
|---|---|---|
| Never block an agent: the hook exits 0, empty stdout, on any failure | hook tests | — |
| A permission is answered only by a human click or by an *Always* rule that a human created | `only_decide_can_respond` | Cover every `Intent` and every new one (H1) |
| A hook waits only while a card can be seen | the island pins itself on a card | A core test: in every preset, an acknowledged card has its host; *Paused* never acknowledges (D5) |
| The island's look does not change unless we mean it | `npm run test:visual` (local) | Refactor PRs (E1 to E4) pass with **no** `-u` |
| The layer surface is never resized or unmapped | manual (nested KWin harness) | A checklist line in every platform PR |
| Settings written by 0.1.0 still load | serde defaults | A fixture of the 0.1.0 file loaded in tests (H4) |
| The Rust and TS views agree | none | Generated TS with a freshness test, like `brand.ts` (H2) |

## 3. Design decisions

- **D1. The island window is the only card host.** Permission and question cards are drawn
  there and nowhere else, with the Y/N shortcuts. In panel mode the island anchors by the tray
  instead of the top (fourth review, step 7), but it is the same window. The tray, the widget,
  notifications and the palette can only *open* the card. This keeps rule 2, the ack and the
  shortcuts in one place.
- **D2. Meaning in core, look in the surface.** Core says who needs you (an ordered
  `Attention`), who is in front (`focus`) and how a card ended (`Outcome`: here, terminal,
  expired, rule). Surfaces pick clips, frames, colors and sounds. The 1.5 s settle stays in the
  UI: it is animation timing, not meaning.
- **D3. One `ViewModel` for every surface.** It is small. A projection for a single surface
  only comes when a payload gets heavy (the Nest's history, wave 8). Not before.
- **D4. Every surface speaks in `Intent`s.** No commands per surface. New intents (`Focus`,
  mute and pin a project) join the rule-2 test the day they are added.
- **D5. No preset leaves an acknowledged card unseen.** The presets (*Island*, *Panel*, *Quiet*,
  *Paused*) change how much the app shows at rest. In *Island*, *Panel* and *Quiet*, a card opens
  the island and plays its sound (*Quiet* included, decided 2026-10-04). In *Paused*, as in the
  reference's pause, core does not acknowledge a card and releases it at once
  (`Effect::ReleasePermission`): the agent asks in its terminal straight away, nothing waits.
  Connectors stop polling while paused. A preset that hid acknowledged cards would make agents
  wait 110 s for nothing: there is none.
  While *Paused*, saved *Always* rules answer nothing either: every card goes to the terminal
  (#103).
- **D6. One live webview at rest.** The island always lives. The widget lives while it is
  chosen. The palette and the Nest are built when opened and closed when done. Measure the
  memory of a second webview in E9 before adding a third.
- **D7. Zeca is a layer of the island, not its core.** With him off: the flock, cards,
  notifications and connectors all work; no chat, mic, talk shortcut or tray *Chat…*.
- **D8. No *Stop* until an agent offers one.** Hooks only reply while a permission or question
  waits. Research per agent in C7; the menu shows only what works.

## 4. 0.1.0: the first release

What is left of `road-to-0.1.md` and the fourth review, sorted.

**Must**
- Fourth review step 2 (GitHub alerts) is done (#53); step 3 (pacing) is done (#50).
- Fourth review step 4 is done (#58): the GitHub card.
- Fourth review step 5 is done (#52): the release fails unless the tag matches all three versions.
- Fourth review step 6 is done (#54): seasonal looks, the witch hat on until Nov 1.
- R1: done: version `0.1.0` in the three files and the AUR `PKGBUILD`.
- R2: done: `road-to-1.0.md` is now `road-to-0.1.md`. "pt-BR after 1.0" (CLAUDE.md, `i18n.rs`)
  stays: 1.0 still exists, later.
- R3: done. Two passes in the real app on the nested KWin harness: live diff with Claude and
  Codex (real CLIs) and Gemini (fixture: the CLI fails on this account), cards, voice end to end,
  MPRIS (Haruna), the GitHub card, the installer, quit. The bugs found were fixed in #61 to #64.
- Tag `v0.1.0`: draft release, packages, AUR, docs published. The README stops saying "build
  from source".

**Moved out**
- Step 7 (panel mode) → E6, after core owns attention. Its icon frames would otherwise copy
  `render.ts`'s logic in Rust.
- Step 8 → the hello bounce in E12, the contribution grid in P6.
- 2.3 Antigravity → C6. 2.6 web search → later.

**Every fourth-review step is done by 0.2.0:**

| Step | Where |
|---|---|
| 0a, 0b, 1, 3 | done (0b still to try in the real app: R3) |
| 2, 5, 6 | done (#53, #52, #54) |
| 4 | done (#58) |
| 7 | E6 |
| 8, hello bounce | E12 |
| 8, contribution grid | P6 |
| 8, mascot on the desktop | P5 (the floating flock spike) |

## 5. Hardening before any new surface (H)

Small PRs that change no behavior and make the E1 to E4 refactors safe (wave 1).

| # | Step | Size | Done when |
|---|---|---|---|
| H1 | **Done (#69).** Rule-2 test covers every `Intent`; fix the `RespondPermission` doc (rules count) | S | The test loops over all intents; the doc names the three sources |
| H2 | **Done (#68).** Generate the TS view types from core, with a freshness test (as `brand.ts`) | M | Removing a field in Rust fails `cargo test` until `ui/src` is regenerated |
| H3 | **Done (#70).** Visual tests address lab states by name | S | Adding a lab state changes no other screenshot |
| H4 | **Done (#69).** Settings read `version`; a 0.1.0 fixture loads in tests | S | Fixture test passes; a future version is kept and the user is warned, not overwritten |

## 6. Experience (E)

Refactors first (no visible change), then surfaces.

| # | Step | Size | Depends on | Done when |
|---|---|---|---|---|
| E1 | **Done (#82).** `Attention` per session and overall in core; `render.ts` reads it (sounds, who is in front when a card waits) | M | H2 | Core tests per status; visual tests pass with no `-u` |
| E2 | **Done (#87).** `focus` in core: `Intent::Focus`; a flock-row click sends it; the chat folder follows it | M | E1 | Same behavior; two windows would agree |
| E3 | **Done (#84).** `Outcome` of a card in core (here / terminal / expired / rule) | S | H1 | The island's "answered in the terminal" comes from core |
| E4 | **Done (#96).** Platform per window: `LayerSpec` (anchor, size, keyboard), region per window, `layout` by label, capabilities and Vite entries | M | | The island unchanged on the nested KWin harness and X11 fallback; builds on Windows CI |
| E5 | **Done (#97, #98).** The view to every live surface (sent once to every window, the last one kept for windows that open later) | S | E4 | A test window gets the same view as the island |
| E6 | **Done (#99).** Panel mode = fourth review step 7, frames chosen from `Attention` | M | E1, E4 | As in step 7's *Done when* |
| E7 | **Done (#101).** Desktop notifications over zbus: finished, failed, needs you. Actions only *Open* (the island on the card), never *Allow* | M | E1 | No notification can answer a card; one per event, merged per session |
| E8 | **Done (#103).** Presence presets: *Island*, *Panel*, *Quiet* (only cards and notifications), *Paused* (cards released to the terminal at once, connectors stopped, island empty), from Settings and the tray | M | E6, E7 | Switch without restart; the D5 test passes in every preset |
| E9 | **Done (#109, #111).** Corner widget: a layer surface fixed in a corner the user picks (decided 2026-10-04), 1 to 3 birds and counts; a click opens the island | M | E5 | Memory of the second webview measured and written in *Notes* |
| E10 | **Done (#105).** Zeca off: another idle look for the island; chat, mic, talk shortcut and tray *Chat…* gated | M | | Visual test of the island without Zeca; no chat process starts. Settings can open at a section (the reference opens *Agents* when hooks are outdated) |
| E11 | **Done (#88).** Shortcuts through the portal: next or previous session (moving `focus`), and one that opens the island (the reference has one) | S | E2 | Works on Plasma 6; the desktop asks once |
| E12 | **Done (#75).** Hello bounce in Zeca's greeting (fourth review, step 8; `f789a2a`) | S | | A clip in `zeca.py` and its visual test; sounds stay synthesized |
| E13 | **Done (#94, #95).** Voice: Silero VAD through `whisper-rs` (885 KB model, MIT) replaces `trim_silence`; a tap-to-talk mode that stops by itself after ~600 ms of silence | S | | Hold-to-talk unchanged; tap mode stops on silence; no new crate |
| E14 | **Done (#110).** Voice: live partial text while the user speaks (re-decode every 0.8 s on the GPU), dimmed; only the final text goes to the input | M | E13 | Partials show on the Vulkan path; the CPU path keeps today's behavior |
| E15 | **Done (spike, see Notes).** Spike: Zeca's speaking engine, and its license checked (section 12) | S | | A choice in *Notes*: Kokoro through `ort` with the system's `espeak-ng` as a separate process, or Supertonic 3 with a license the user accepts |
| E16 | **Done (#73).** Sound volume in Settings (today a fixed 0.05 in `ui/src/sound.ts`) | S | | The reference has a slider; a setting with the 0.1.0 fixture still loading (H4) |
| E17 | **Done (#107).** Right-click Zeca for his looks, with a live preview (after fourth review step 6; nineteen looks since #91, so the picker groups them: seasonal, head, with a chain) | S | | Visual test of the picker |
| E18 | **Done (#93).** Keep the user's own Claude status line: save the old `statusLine` beside the hook, run it from ours (same stdin, 10 s timeout) and print its output; uninstall puts it back | M | | Reopens road-to-0.1 2.4's "decided against" with the reference's way. Still a diff, a backup and a click (ADR 0005). Fixture with a user status line |

Docs in `docs/guide/` in the same PRs (presence, notifications, widget, Zeca off).

## 7. Control (C)

| # | Step | Notes |
|---|---|---|
| C1 | **Done (#115).** Quick actions on a bird: open terminal, view activity, view diff, open the diff's file in the editor, go to the card, focus | Only what works. Where the window cannot be raised, open the folder in `code` (absolute, existing folder, no shell) or the file manager, as the reference does |
| C2 | **Done (#119).** Per-project prefs: mute, pin, hide | Per project, not per session: sessions leave after 10 to 30 min |
| C3 | Command palette: open a session, focus, jump, go to the card | A layer surface with on-demand keyboard, like the chat. Never answers a card (D1) |
| C4 | **Done (#121).** Attention ladder: a waiting card climbs island → notification → sound; do not disturb | Pure in core, with time |
| C5 | **Done (#122).** "While you were away": a digest when the screen unlocks (`org.freedesktop.ScreenSaver`). While locked, the scene's timers and the connectors rest | Deterministic, no model |
| C6 | **Done (#116).** Integrations: Antigravity (2.3), more generic agents | |
| C7 | **Done (research, see Notes).** Research: how each agent could be stopped | Writes *Notes* only; no menu item without a working path (D8) |
| C8 | **Done (#120).** A quiet bird: a session *working* with no event for 5 min is flagged, 15 min loudly. The human snoozes it, says *keep going*, or dismisses it | Only shown, never acts on the agent. From Paperclip's silent-run signal (section 12) |
| C9 | **Removed (2026-10-07)**: the local voice (Kokoro, espeak-ng for Portuguese) sounded too poor; a realtime model (OpenAI Realtime first, Gemini Live later) replaces it, opt-in. Was **Done (#123).** Zeca speaks, off by default: replies cut into sentences and spoken while they stream; a *speak* clip; any key, click or the talk shortcut stops him | Engine from E15; models downloaded and checked by SHA-256 like whisper. Sentence cutter ported from Patter (MIT), not from VoiceStudio |
| C10 | Voice: personal dictionary ("cube control" → `kubectl`); cloud transcription opt-in, key in the keyring | Road-to-1.0 6.1 |
| C11 | Voice commands for moving around only ("next session", "open the chat") | **Never** for answering a card (ADR 0004) |

## 8. Platform (P)

| # | Step | Notes |
|---|---|---|
| P1 | Local history (SQLite): sessions, steps, outcomes, with retention | `note` is "never logged" today: keeping it is opt-in |
| P2 | Roosts: sessions grouped by repo, with branch, PR and CI from the GitHub snapshot | |
| P3 | Agent capabilities: what each agent can ask, approve, diff and stop | Feeds 0.2.0's policies |
| P4 | Side panel (sessions, cards queue, activity) | A second layer surface, on E4 |
| P5 | Spike: a bird out on the desktop. First option, from the reference: a small fixed-size layer surface moved by its margins (no resize, no remap), input only on the body, slow frames at rest, asleep when locked; it flies to the island for a card and back. Full-screen transparent surface only if that fails | Revisits the "mascot on the desktop" decision. Check margin moves on KWin with the nested harness |
| P6 | GitHub contribution grid in the GitHub card (fourth review, step 8; `86fbb79`) | One `gh` query every 30 min; after step 4 |
| P7 | Cost per session and per roost: tokens and cost from the agent's stop, marked *subscription* or *API* | A subscription's dollars are not real spend: show tokens there |
| P8 | An audit log in the history: every click on a card, every *Always* rule, every agent config written | Append-only; who (human, rule, system), what, on what |
| P9 | Voice: an open-mic conversation with Zeca: echo cancelling (NLMS, ported from Patter), cutting him off by speaking (~200 ms of speech), optional noise removal (`nnnoiseless`, BSD-3) | No wake word (section 12) |
| P10 | A voice or speed per vult, per agent | |
| P11 | Optional: Vercel deployments per roost (ready, error, canceled; branch, commit), token in the keyring | The reference's one other code-related connector |

## 9. Operations: 0.2.0

- **First, a decision record that rewrites rule 2.** Policies and autonomy mean answering
  without a click. Write down what counts as a human's consent (for example: a policy the user
  wrote, saw as a diff and clicked), then change CLAUDE.md. Nothing else in 0.2.0 starts before it (ADR 0014).
- The Nest: history, roosts, usage, an audit of every answer.
- Policy engine over P3's capabilities. What an agent can *see* is separate from whether
  *this call, now* passes. Answers: allow, deny, ask first. Actions are classed read, write or
  destructive. A deny beats an allow, and every answer goes to the audit log (P8).
- Budgets per roost, with a warning at 80 %. A budget's "hard stop" only means no new session
  started from here, plus a notification. It never blocks or kills an agent we only watch.
- Pausing a session we started: ask it to stop, wait, then end it. Never for a watched one (D8).
- Starting an agent session from here (in the user's terminal).
- Zeca as an agent, with a handle that cannot build `Decide` or `DecideAlways` (by type).

## 10. Changed from the first draft, and why

- **Per-surface view types from day one** → one `ViewModel` (D3). They were extra code for
  payloads that are small.
- **Rename and pin per session** → per project. Sessions do not live long enough.
- ***Stop* in the quick actions** → research only. No agent can be stopped through a hook today.
- **A *Hidden* preset** → *Quiet*, plus *Paused*, which releases cards at once instead of
  hiding them (the reference's pause).
- **Panel mode before 0.1.0** → E6, on top of core's `Attention`.
- **Floating flock early** → a late spike (P5). It goes against a recorded decision and its cost is unknown.

## 11. Decided (2026-10-04)

- 0.1.0 ships with the GitHub card (step 4) and the seasonal looks (step 6).
- *Quiet* plays the card's sound.
- The widget is a layer surface fixed in a corner, like the island.
- Every step of `plan-fourth-review.md` is done by 0.2.0 (table in section 4).
- Everything in this plan lands by 0.2.0, in small 0.1.x releases (ADR 0015).

## 12. Ideas from other projects

### Paperclip (`paperclipai/paperclip`, MIT, read at `994d6ed`)

A server that runs a company of agents: Node, Postgres, an org chart, budgets. It launches
agents; we watch them. What fits a desktop companion:

| Idea | Where in Paperclip | Here |
|---|---|---|
| Silent-run signal: quiet 5 min is suspicious, 15 min critical; only informs; snooze, continue or dismiss | `doc/execution-semantics.md` section 12 | C8 |
| One inbox for approvals, questions and failed runs | `server/src/services/attention.ts` | C4, C5 |
| "Surface problems, don't silently fix them" | `doc/SPEC.md` sections 8 and 12 | Already our rule; quote it in `docs/safety.md` |
| Cost ledger with the billing type (subscription, overage, API) | `packages/db/src/schema/cost_events.ts` | P7 |
| Append-only activity log: actor, action, entity, details | `packages/db/src/schema/activity_log.ts` | P8 |
| Environment check per adapter as a list of info, warn, error lines | `packages/adapters/claude-local/src/server/test.ts` | P3, as a *doctor* view |
| Never resume a session saved for another folder | `claude-local/src/server/execute.ts` | 0.2.0, starting sessions |
| Tool visibility apart from call policy; risk classes; deny beats allow; audit everything | `doc/MCP-ACCESS-GOVERNANCE.md` | 0.2.0 policy engine |
| Budgets: warn at 80 %, hard stop, incident | `schema/budget_policies.ts` | 0.2.0, hard stop softened |
| Untrusted input (outside PRs, issues) never raises trust | `doc/LOW-TRUST-PRESETS.md` | 0.2.0, Zeca as an agent |

**Not taken:** the company model (org chart, CEO, board, hiring, tasks as the only channel); the
server (Express, Postgres, auth, API keys per agent, cloud sandboxes); agents launched with
permissions skipped by default (against rule 2); telemetry on by default (against rule 4);
agents woken by schedules or watchdogs with nobody present; a secrets manager outside the
keyring; injecting its own skill or MCP config into agents (against rule 3).

### The reference (upstream unchanged since `59f63df`, checked 2026-10-04)

Taken into the steps above: pause from the tray (E8), a shortcut that opens the island
(E11), sound volume (E16), the looks picker (E17), keeping the user's status line
(E18), Settings at a section (E10), opening the folder or the diff's file (C1), resting
while locked (C5), the small desktop bird (P5), Vercel (P11).

**Decided against:**

| What | Why |
|---|---|
| Stripe, Resend, Notion, Cal.com and n8n connectors | Business services, not agent work; each is one more secret. n8n may return if asked |
| GitHub stars and repository count | A vanity number for one more query |
| Dragging the mascot onto a window to attach it as context | macOS Accessibility and AppleScript; reads other apps' windows; nothing like it on Wayland |
| Sending a dropped file by email | macOS Mail only |
| Hide after N minutes without pointer movement | Saved and never read in the reference; Wayland has no global pointer; screen lock (C5) is the right signal |
| Four pill slots and a main pill | One editor's model; every session counts here |
| Global key monitors (Escape anywhere) | Not possible on Wayland, and close to a keylogger |
| Declining a card because another is up | We queue parallel cards |
| A debug or demo tray menu | The lab does it |

### VoiceStudio (`debpalash/VoiceStudio`, read at `c4d63ef`)

An Electron app on a large Python backend (torch, whisperx, 17 speech engines). **AGPL-3.0 with a
CLA: none of its code can come here.** What it taught us:

| Idea | Here |
|---|---|
| Silence detection to end an utterance (theirs is a loudness gate; Silero is on their own plan) | E13, with Silero through the `whisper-rs` we have |
| Partial text by decoding the buffer again every 0.8 s | E14 |
| Cut a streaming reply into sentences and speak each one as it is ready | C9, ported from Patter (MIT), where that code came from |
| Echo cancelling and talking over the assistant | P9, also from Patter |
| Dictation never as an always-on mic | Same here: no wake word |

**Licenses checked (2026-10-04):**
- The `sherpa-onnx` crate's default build links `espeak-ng`, which is **GPL-3.0**
  (`sherpa-onnx/rust/sherpa-onnx-sys/build.rs`). That rules it out the way Piper was ruled out,
  even for VAD only. The 6.1 plan "Kokoro through sherpa-onnx" is dropped.
- Kokoro's weights are Apache-2.0, but its Brazilian Portuguese voices need `espeak-ng` for
  phonemes. Running the system's `espeak-ng` as a separate process keeps it out of our binary;
  confirm that reading before relying on it (E15).
- Supertonic 3 needs no phonemizer and speaks Portuguese, but its weights are OpenRAIL-M (use
  restrictions, not OSI). Only as a download the user accepts.
- Silero VAD: MIT. `nnnoiseless`: BSD-3.
- Not taken: VoiceStudio's code (AGPL), OmniVoice's weights (non-commercial), openWakeWord's
  models (CC BY-NC-SA), ten-vad (non-compete clause), the Python stack.

## Notes

### Left after 0.1.0 (for a 0.1.x)

- ~~`tests/visual/flock.spec.ts:215` sometimes runs past 120 s~~: done (#70).
- ~~Ticker: switching sessions mid-slide~~: done (#72).
- ~~GitHub card: stale after an error, the tab while a card waits, birds on its corner~~: done
  (#72). The poll error could go through core so every surface sees it.
- ~~Settings selects in the mono font and GTK's pill~~: done (#66); still to see in WebKitGTK.
- Seen once in R3, not reproduced: the folded island vanished while the app ran (likely the
  harness switching displays). Watch for it.
- Summer window for the sunglasses (`Outfit::seasonal`): to choose.
- Also done after 0.1.0: SHA256SUMS names match the downloads (#67); a failed rule save is
  logged (#71).
- Also done after 0.1.2: voice stops ~1.5 s after the last word (#106); tray badges readable at
  22 px (#104); native 22/24/32 px tray frames (#114); the Panel island folds for Settings (#109).
  The talk key stays bound at the desktop while Zeca is off: KDE's portal cannot release one
  shortcut alone, so the press is ignored and the guide says how to free it (#117).
- E9 memory (D6): the corner widget adds about 40 to 44 MiB PSS (320 to 362 MiB, swap included);
  its own web process is about 162 MiB RSS. It exists only while chosen.
- E14: no voice preview on the CPU (Base takes ~0.6 s of 8 threads per pass); one global lock keeps
  whisper contexts from running at once on Vulkan (they crashed).
- E15 (speaking engine, spike 2026-10-06): Kokoro-82M (Apache-2.0 weights, fp32 ONNX 326 MB)
  through `ort`; English phonemes from `misaki-rs` without default features (nothing GPL linked);
  pt-BR phonemes from the system `espeak-ng` run as a separate process (never bundled; an AUR
  optdepends). RTF ~0.19 on 4 CPU threads, first audio 0.33 s. Without espeak-ng, the MIT
  `piper-plus-g2p` gives understandable pt (no stress marks). Fallback: Supertonic 3 (OpenRAIL-M,
  opt-in download with a license screen; upstream archived 2026-09). Ruled out: fp16 on CPU
  (silent), the WebGPU provider (noise), Piper and F5 (license), engines without pt-BR.
- C6: Antigravity shows as another tool named `antigravity` (no AgentKind of its own yet); its
  hooks cannot answer cards (agy asks itself). Recipes for OpenCode, Pi and Cursor are untested in
  a live session.
- C9: on Linux ONNX Runtime is loaded at run time (Microsoft's 1.28.3 build, downloaded with the
  model and pinned by SHA-256, ~337 MB in all): the bundled one needs a newer glibc than Ubuntu
  22.04. Loaded, speech takes ~500-560 MB. `~~~` and indented code blocks are still read aloud.
- C4: do not disturb is set from Settings only (the command is ready for a tray item).
- Codex keys trust by hook position: removing ours may make Codex ask again for a later hook.
- C7 (stopping an agent, research 2026-10-08; Claude Code 2.1.292, Codex 0.160.1, Gemini CLI 0.63.0,
  agy 1.3.1). **Never a signal:** SIGINT to Claude Code ends the whole session, idle or mid-turn
  (tested), and Codex, agy and Gemini exit on it too; a signal can orphan a tool's children and
  leave the terminal broken. What could work, per agent:
  - Claude Code: our PreToolUse hook answers `{"continue": false, "stopReason": …}` (documented; it
    wins over a permission decision). The turn ends before the next tool and the session stays.
    A turn that only writes text has no event before Stop: it cannot be stopped this way.
  - Gemini CLI: the same through BeforeTool `continue: false` (documented: it ends the agent loop).
  - Codex: `continue: false` only on PostToolUse, Stop and UserPromptSubmit (after the tool ran).
    The clean path is the app-server's `turn/interrupt {threadId, turnId}` (an Esc; hook payloads
    carry `session_id` and `turn_id`) through the shared daemon socket; experimental, untried.
  - agy: PostInvocation `terminationBehavior: "terminate"`, which needs a hook we do not register
    (a config change the user approves). No local stop call found.
  - Any agent in tmux: `tmux send-keys -t $TMUX_PANE Escape` is a real Esc (the hook forwards
    `TMUX_PANE`); untested.
  So a *Stop* item, when it comes, is a flag a human click arms (as in rule 2) and our next
  PreToolUse / BeforeTool reply carries; it says "stops before the next tool", not "stopped", and
  is cleared on Stop and SessionEnd so it never ends the user's next turn. The hook's
  `terminal.pid` is the agent for Claude Code (tested), `sh` for agy (`|| exit 0`), the inner
  `node` for Gemini.

(Add what each step learns here.)
