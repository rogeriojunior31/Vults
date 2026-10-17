# Plan Zeca: the roadmap

Internal board, rewritten 2026-10-12 at `592f6a6` (#337). It is the one roadmap: what is left
first, then the decisions and guarantees that hold, then an index of every finished step. Step ids
are never reused; code comments cite them as `plan-zeca <ID>`, so a finished id stays in **Done**.
Agent coverage has its own board, [plan-every-agent.md](plan-every-agent.md).

0.2.0 is tagged when N2 is done, after the R3 smoke; one release per stage of the
[order of work](#order-of-work-2026-10-16), 0.2.x after it
([ADR 0020](../adr/0020-0.2.0-at-wave-4-then-0.2.x.md), amended by
[0031](../adr/0031-tasks-leave-vults.md)). Linux first; Android is the one exception.

## Left to do

### Order of work (2026-10-16)

Set by the user on 2026-10-16. The UI prototype (`design/ui-motion-proposal` on
`design/prototype-visual-pass`) is the target for every page; each stage starts when the one
before it is accepted. Every section below this one keeps its rows; the stage column says when
they are taken.

| Stage | Release | What | Rows it takes | Done when |
| --- | --- | --- | --- | --- |
| N0 | 0.1.9 | Tasks leave Vults completely | W3, W4, W5, W6, W8 removed; W7 and W9 kept | No `/task`, `/delegate`, Tasks tab, run, review or router code left; `/continue` and the day's spend still work |
| N1 | 0.1.9 | Pages and theme as the prototype draws them | UX6's desktop matrix, the frame and the pages Chat and Flock do not own | Each page matches the prototype side by side at 480, 760 and 1180 px, in light and dark |
| N2 | 0.2.0 | Flock and Chat working, UI and backend | The prototype's `new` Flock, Lab, Chat and Zeca features | Every one of them is `shipped` in `features.json`, with native evidence |
| N3 | 0.2.1 | Dictation | V0 to V8 ([plan-dictation.md](plan-dictation.md)), with Z6, M7 and K8 brought in where a phase needs them | Each phase's own criterion |
| N4 | 0.2.2 | The rest of the roadmap, refined against the new UI | UX7 to UX12, Zeca proactive, the Flock board, Mobile, More agents, Follow-ups, Later | Each row's own criterion |

**Releases** (set by the user on 2026-10-16): one release per stage, tagged after its R3 smoke.
N0 and N1 ship together as 0.1.9, N2 is 0.2.0, then N3 is 0.2.1 and N4 is 0.2.2. A fix needed
between two stages is the next patch, and the stages after it move up by one.

#### N0 Remove Tasks

Tasks (W3) and what only they use go: runs in their own clone (W4), the handoff through a
context pack (W5), the review loop (W6), the router behind `/delegate` (W8). The shipped
migrations 14 to 20 are never edited; their tables stay and are no longer read, and a later
migration may drop them after a release. One PR per slice, each passing the full CI.

N0.1 is recorded in [ADR 0031](../adr/0031-tasks-leave-vults.md), with amendments to 0018,
0020 and 0021. N0.2 removes the Tasks surface and its Chat/palette entry points; shared
cost figures stay in Activity and the budget editor moves to Settings. Its small native wiring
changes move budget writes to Settings and publish spend changes independently of Tasks.

N0.2 validation (2026-10-16): build, renderer checks, Clippy, workspace tests and evals passed.
The native app was checked on Linux/KWin with isolated data at 1180 x 820 and 480 x 360:
four tabs, Cost, the budget-settings link, a saved $5.25 budget, a rejected write from the Nest,
scroll access to the short-window editor, and costs/budget with Zeca off. The renderer checks
also cover palette/keyboard order, budget read/save failures, retries and stale responses.
N0.3 removes the app's task commands, isolated execution and review loop. `/continue` now uses
`app/src/continuation.rs`: one owned execution, generation-bound cards, the existing sandbox and
settings checks, budget checks and cost settlement. Stop releases its unanswered cards even when
its owned driver must be aborted; watched processes are never stopped. Legacy saved Tasks tabs
fall back to Flock. N0.3's permission, session/lifecycle and cost checks pass with the full local
pre-commit sequence.

N0.4 retires the store's task API and Zeca's router, pack and review modules, including the
Codex review-only driver path and their eval cases. A fingerprint test pins the original SQL
of migrations 14 to 20, and an upgrade test preserves old rows and reads `task:N` spend after
reopening the database. The current schema remains 20. Full local checks pass.

N0.5 removes `VULTS_TASK`, core's dispatched flag and the corresponding sprite take-off.
Protocol 9 rejects older envelopes; the hook retains only terminal metadata, `VULTS_ZECA`
and the `VULTS_RUN` driver channel. Protocol/hook regressions and the full local checks pass.
The normal arrival and departure animations remain. The Linux/KWin app was checked with
isolated data at 1180 x 820 and 480 x 360: hook-created session arrival/end, Flock, Cost, a saved
budget and the Settings-only write guard. `/continue` reports no eligible session or owned
continuation for the tested commands.

N0.6 removes the Tasks guide and its navigation links, updates the published guides and
request inventory in both languages, and aligns the README with N0-N4. `site.json` already
describes approvals, Chat and Zeca without Tasks; the GitHub issue template stays.
The final source audit retains shipped SQL and upgrade tests, historical cost sources, negative
entry-point tests, native agent tool names, embedding/async tasks and ordinary prose; no active
Tasks execution, review, router or UI remains. Local checks and the translation/link audit pass.
The human R3 release smoke remains pending.

| # | Slice | Removes | Keeps |
| --- | --- | --- | --- |
| N0.1 | ADR | [0031](../adr/0031-tasks-leave-vults.md): Tasks leave Vults, and 0.2.0 moves from wave 4 to N2; it amends 0018, 0020 and 0021 (Surfaces, D7, D8, the R3 smoke) | — |
| N0.2 | Surfaces | The Nest's Tasks tab (`surfaces/nest/tasks.ts`, its CSS, `budget.ts` moved), `/task` and `/delegate` in the chat and the palette, "Open Tasks", their strings in the three languages, `scripts/check-task-actions.mjs` and its CI step | The day's spend and its budget: the figures in Activity › Cost, the value in Settings |
| N0.3 | App | `app/src/tasks.rs`, the `task_*` commands, `runs.rs`' task start, fresh start, stop, review and worktree removal, `tasks-changed` | `/continue` (W7, `handoff.rs`): its driver code moves out of `runs.rs` into its own module; D8 still stops only what Vults started |
| N0.4 | Crates | `store::tasks` readers and writers, `zeca::router`, `zeca::review` and the review evals | The migrations; spend rows already written as `task:N` stay readable |
| N0.5 | Protocol and core | `VULTS_TASK` and the take-off it drives in `core::view`, with the protocol note it needs | `VULTS_ZECA` and the driver's own channel for `/continue` |
| N0.6 | Docs | `guide/tasks.md` (and pt-BR), the Tasks parts of the Nest, Chat, Island, Activity, Approvals and Other agents guides, the settings and protocol reference, `site.json` | The GitHub issue template named Task (it is about issues, not the feature) |

Done when `rg -i '\btasks?\b'` finds only the kept items above, async tasks and the shipped
migrations, and the desktop shows the Nest with Flock, Chat, Connections and Activity.

#### N1 Pages and theme

The prototype's rule: one shared frame (controls, page header, states), each tab with its own
layout, color only for status. Chat and Flock (Compact and Lab) keep their current look and are
not reworked here; N1 changes their frame only.

| # | Slice | From the prototype |
| --- | --- | --- |
| N1.1 | Theme and frame | Tokens and type scale into `DESIGN.md` and the UI; the page header with one filled action; figure tiles, cards, selects with the app's chevron; empty, loading and read-error states; side panels that resize, reset on double-click and fold to a rail (`lab-flexible`) |
| N1.2 | Activity | The week as a dashboard: figures against last week, turns by agent, record cards, a labelled year; Cost holds the spend that left the Tasks tab |
| N1.3 | Connections | A triage inbox: repo marks, check bars, a strong detail header, diff bar, checks in view; one filled action; Refresh on the sync line |
| N1.4 | Settings | Groups as cards, categories with icons and a larger title; Settings owns every value, Activity every figure |
| N1.5 | Acceptance | UX6's [desktop matrix](plan-ui.md#ux6-final-desktop-acceptance), then each page side by side with the prototype |

N1.1-N1.4 implementation is delivered on `feat/n1-pages-theme` (2026-10-17): the
shared single-row frame, optional neutral Light theme beside default Black, retained
resizable/foldable side panels, recorded weekly comparisons and agent totals, connection
triage, and grouped Settings. History retention stays in Settings; Activity links there.

The native Linux pass covered Activity, Connections and Settings at 480 x 360,
760 x 600 and 1180 x 820 in both themes, including scroll reachability, narrow detail
return/focus, panel reset/folding and a retained Chat draft. Black layouts were compared
with the prototype in the same WebKit runtime. Controlled local history and GitHub
responses supplied populated states; they are not usage or live-account evidence.
N1.5 remains open for the user's full UX6 desktop matrix, including pending cards,
multiple displays/scales and window ownership. R3's release smoke remains separate.

#### N2 Flock and Chat

Each item ships with its real source of state; a mocked value in the prototype becomes a read in
`core` or `app`, or the item waits. Rules do not move: a card is answered only by a click, a rule
or a policy, and no preset leaves a card unseen (D5).

| # | Item | Prototype id | Backend it needs |
| --- | --- | --- | --- |
| N2.1 | Skip a card for now; the Needs you strip folds when nothing is left | `card-skip`, `needs-skip` | A `Skip` intent in `core` that reorders and never answers; the rule-2 test |
| N2.2 | Now: one inbox for everything that asks for an action | `now-inbox` | One `Attention` list in the `ViewModel` (D3) |
| N2.3 | An idle bird stays green 30 s, then circles in the island's sky | `island-sky` | Idle time per session in `core` |
| N2.4 | The Lab session: overview (running time, cost, context used, recent steps, changed files), rows with agent, step and minutes and a context line, bird names on the wire, status chips over the list | Lab › Flock | Context use and running time per session from the agents' own records; changed files from the step diffs |
| N2.5 | Plan usage ring on the model | `chat-usage` | The plan windows Vults already reads, per provider |
| N2.6 | `@` a session or project in the chat | `chat-mentions` | Zeca's flock reads, scoped to what was named |
| N2.7 | The dense Lab chat | `chat-lab-dense` | — |
| N2.8 | Zeca greets by the time of day, suggestions follow the flock, follow-ups under each reply | `zeca-greeting`, `zeca-suggestions`, `zeca-followups` | Rules in `core` for what is offered; none is an unasked line (K1) |
| N2.9 | Zeca's moods while you type, search, think, speak and wait on a card | `zeca-moods` | The chat's turn states; K7's flock mood stays in N4 |

#### N3 Dictation

The phases of [plan-dictation.md](plan-dictation.md), in this order: V0 (its open R and Q
answered first), V1, V2, V4, V8, V3 (after Z6), V5, V7 (after M7), V6 (= K8). The Dictation tab,
Settings › Dictation and Activity › Dictation land on the N1 frame.

#### N4 The rest

Taken in the order of the sections below, each redrawn on the new UI before it lands: UX7 to
UX12, Zeca proactive (without K8), the Flock board, Mobile, More agents, Follow-ups and Later.
Two prototype items wait here: the Connections face for Zeca (`conn-face-zeca`) and the card's
risk line (`card-risk`, K3).

### UI delivery sequence

Requested 2026-10-13, following the Compact/Nest review. Prioritize existing-feature repairs.
The detailed tasks, dependencies, acceptance criteria and audit mapping are in
[UI implementation sequence](plan-ui.md); [DESIGN.md](../../DESIGN.md) holds UI rules.
The 2026-10-15 six-part refinement batch is implemented on
`feat/ui-component-polish`: Tasks, Connections, shared frame, searchable Chat
history, control feedback and narrow/wide continuity. Its detailed delivery and
pending desktop review are in [the batch record](plan-ui.md#six-part-ui-refinement---2026-10-15).
This does not close native acceptance or begin UX7.
The subsequent [Quick Chat proportions](plan-ui.md#quick-chat-proportions---2026-10-15)
refinement is implemented within the fixed overlay, with ADR 0021 reconciled;
paired Mini/Quick width selection and desktop acceptance remain separate.

These IDs are new; completed W/U/L steps stay in Done. UI-only refinements do not
reopen shipped features, and the R3/tag gate below keeps its existing scope.

The [current delivery status](plan-ui.md#current-delivery-status) distinguishes
implemented slices from pending acceptance. The user accepted the current
UX1-UX5 block on 2026-10-14 after merging #351; those steps are indexed in Done.
UX6 implementation and automated integration are complete, awaiting the final
desktop PR validation. UX7 and the new workspaces have not started.

The user also validated #353's saved close, model-list retry and short/wide Chat
repairs after merging it on 2026-10-14. UX6 now retains Flock search, Needs you
and detail navigation too; the remaining state and desktop acceptance is listed
in [UX6](plan-ui.md#ux6---continuity-with-native-ownership).

After merging #354 on 2026-10-14, the user accepted Flock navigation retention
and short-window filter clearing. By project now retains its query independently
of History/Audit; the broader native desktop pass still gates UX6 and UX7.

The user accepted all delivered adjustments through the merged #357 on
2026-10-14. The final UX6 batch guards delayed Chat shortcuts, waits for the
Nest's saved destruction before Quick/Mini opens, and adds production-renderer
integration to CI. The [final desktop matrix](plan-ui.md#ux6-final-desktop-acceptance)
defines the remaining user acceptance before UX7; the release R3 smoke remains
independent.

| # | Step | When / needs |
| --- | --- | --- |
| UX6 | Implementation and automated continuity integration complete | Final desktop PR acceptance; B05 |
| UX7 | Shared wider Mini/Quick extension | UX6; reconcile ADR 0021, display scale and fixed-overlay/input-region limits |
| UX8 | Native Agents workspace, profiles and specialist Chat/context | New feature, staged, after N0; built on the current session pipeline |
| UX9 | Studio skill library/import/reimport and verified tool connections | New feature after UX8; native storage and reviewed configuration contracts |
| UX10 | Scoped investigations, retained evidence and implementation/review handoff | Later feature; one verified route before engine expansion |
| UX11 | Debrief, risk/batches/questions, routines, proposals, moods/voice/ACP and observed-cost views | Alongside K1-K8/K10, M7, W2/W9b and the original Flock board |
| UX12 | Registry-backed agent support and Android permission/service views | Alongside Every agent and L3-L5; G1/G2 remain proposed |

Check the review's findings against the current head in the running application.
Mocked features and synthetic runs are references, not native completion evidence.

### Release gate

- [ ] **R3 smoke** in the real app, by a person at the desktop: the island, a card, the chat, voice,
  GitHub, and the Nest, on the Living Flock UI as refined by N1 (Tasks left in N0). Check the
  remaining Nest areas, `/continue` and its owned Stop, spend and budget, and Zeca off
  ([ADR 0031](../adr/0031-tasks-leave-vults.md)).
- [ ] **Tag** the stage's release: 0.1.9 after N1, 0.2.0 after N2 ([releasing.md](releasing.md)).

### Flock board

Twelve items the user approved that are not wired yet, from the
[flock board](https://claude.ai/artifact/PZH7HwmUNUJeTnw5MBvkeV) (prototypes in the
[flock review](https://claude.ai/artifact/1phpxZ7YcaVzUcMD9yfGTt)). Every prototype is redrawn on
the new head and pose before it lands; sprites must match the approved prototype pixel for pixel.

| Board id | Item |
|---|---|
| #03 `decolagem` | Take-off with a crouch and a hop, the first beat down; landing shows the legs and squashes on contact |
| #09 `virar` | Turning on the wire passes through the front: mid-hop he looks at you |
| #11 `fio-afunda` | The wire sags under each bird's weight and tugs the neighbor (a griffon two rows, a black vulture one) |
| #44 `decolar-especie` | Per-species take-off: a condor hops and climbs on deep beats; a *Cathartes* jumps and already teeters |
| #34 `sexo` | Male and female where nature tells them apart (a crestless, red-eyed condor female), drawn from the session's hash |
| #35 `idade` | Age: a new project brings a straw-colored downy chick that grows to juvenile and adult with weeks of work |
| #36 `anilha` | A leg or wing band for every species, its color and number from the session's hash |
| #29 `temperamento` | Temperament per species: pace, restlessness, boldness |
| #30 `trabalho` | A work animation unique to each species (the bearded vulture breaks a bone, as the Egyptian already throws a stone) |
| #40 `seguir` | Following: when a subagent ends with a result, the *Cathartes* scout lands and the black vultures come down after it |
| #47 `vomito` | On the third failure in a row, two frames of a drop, behind a setting |
| #42 `visitantes` | Visitors by region and season (the Egyptian migrates, a condor in southern Brazil); the old visitor flies off when a new one comes |

To check against the app: #48 `pill-alto` (the big species fit the pill's height), #23 `rubor` (bare
skin flushes on failure and approval), #28 `sociavel` (black vultures soar together, the
turkey vulture low and alone). Later: #33 `ficha` (the picker plays the species and shows its
card), #27 `nomes` (species names in the four languages), #46 `som` (hisses and grunts, no song).

### Zeca proactive (wave 5)

*When* Zeca speaks is always a rule in `core`; the model only writes *what*.

| # | Step | Needs | Done when |
|---|---|---|---|
| K1 | Attention engine for his lines: now, debrief or never; at most 4 unasked lines an hour; silent in full screen or focus; presence from the screen lock. Never delays or hides a card | S3 | A `core` test: with the budget spent, a card still opens the island (ADR 0021, D5) |
| K2 | Debrief on return and the day's opening: the away digest (C5) and memory, written by Zeca, in Quick's reserved slot (U6) | K1, M4 | The debrief is opened on most returns |
| K3 | Card explanation: a risk line and an out-of-scope warning per permission; obvious reads approved in a batch, always with a click | Z6, W1 | Average time a card waits goes down |
| K4 | Routines: morning brief, nightly CI, dependencies, watching a page (each through the gate, ADR 0019) | G9 | A routine made in the chat, editable in the Nest |
| K5 | Learning from the flock: a command denied 3 times, a correction repeated to agents, the same CI step failing → a roost proposal or an `AGENTS.md` diff | M6, M9 | At least one useful proposal per week of use |
| K6 | Skills in the `SKILL.md` format; writing to `~/.claude/skills` follows rule 3 | M6 | A skill Zeca wrote used by Claude in a session |
| K7 | Mood from the flock (calm, alert, worried, celebrating) drives animation and tone; he looks at the bird he talks about | K1 | A running-app review per mood |
| K8 | Voice on the same brain: an opt-in realtime model (replaces C9), open mic (P9), moving-around commands (C11); never answers a card | Z6, G9 | A spoken question uses `flock_list` |
| K10 | Zeca as an ACP agent: Zed, JetBrains and Neovim talk to him, with memory and the flock | M7 | A Zeca session in Zed answers about the flock |

### Mobile (wave 6)

The desktop is the source of truth and the one that acts; Android only decides; the relay is
blind. On the phone: deny and dismiss without biometrics; approve, answer, hand off and take a
service action with biometrics. Sending a message or moving money is not in the protocol.

| # | Step | Needs | Done when |
|---|---|---|---|
| L3 | Android MVP in Kotlin/Compose over the Rust core (UniFFI, `cargo-ndk`): pair, see the flock, deny, approve and answer with a Keystore key that needs `BIOMETRIC_STRONG` per use; push through UnifiedPush, FCM as fallback. Brings **key attestation**: the desktop still takes the phone's word for which key needs biometrics (L1) | L2 | An approval at 100 s works; past 108 s it is refused (`limits::SERVER_DECISION_TIMEOUT`) |
| L4 | Handoff and Zeca's proposals from the phone; the screen lock takes Zeca to the phone (Live Updates on Android 16+), unlocking waits 30 s | L3, W7 | A proposal accepted on the phone shows in the Nest |
| L5 | Service actions through `connectors`, GitHub first (re-run CI, approve a PR), with the same one-time token | L3 | An action offered more than 5 min ago is refused |

L2 leftovers: an audit line for a pairing and a revoke; one answer per device at a time when two
phones race on one card (the ledger takes the first, the second is `Used`).

### More agents

On [plan-every-agent.md](plan-every-agent.md): F3's YAML writer, F4's issues, B4 (Cline, Kiro), a
recorded Cursor session (C1), C5 (Amp, Pi, Hermes), C6 (Crush, Goose, Grok Build, Muse Code), R2
(ACP rows) and G1/G2 (an open reply for any tool, protocol `VERSION` 9).

### Follow-ups

- **W9b**: cost from the OTLP telemetry Claude Code and Codex export, received on loopback only, for
  the sessions Vults only watches; the agents' telemetry settings go through the installer's diff,
  backup and click. Done when a watched Codex session's tokens count in the day.
- **M7**: `ask_user`, a card answered with text; the `vults-zeca` MCP for Codex (it reads MCP only
  from its own config, so rule 3 applies).
- **W2**: other agents' shells carry a working folder (`workdir`, `directory`) and searches may name
  a folder the card does not show; show them on the card, for humans too. Policies allow only for
  Claude Code until then.
- **Z6**: OpenAI through the Responses API with native compaction; OpenAI stays on Chat Completions
  until someone can check it with a key.
- **Z8**: a fetch's own card is not warned yet (taint lands when the call ends).
- **S14**: a request `core` sees more than `limits::ACK_TIMEOUT` (800 ms) late has already lost its
  hook, yet shows a card; release it to the terminal at once, or check the reply handle before
  acking.
- **Z5**: a closed on-demand window keeps its WebKit web process (about 110 MiB) until restart;
  find out whether wry keeps the webview or WebKit caches the process.
- **Qwen `write_file` diff**: Qwen sends a unified diff in `tool_response.returnDisplay.fileDiff`;
  the hook could turn it into hunks.
- **Live checks**: Codex's card round trip in its TUI (it hangs in a headless pty); a `/continue`
  Codex call where its own hook also fires (either card answers, the other is released:
  fail-closed); Antigravity (not on npm, fixtures only); OpenCode's race between the plugin and its own TUI
  prompt (whichever answers first wins).

### Later

| # | Item | Waits for |
|---|---|---|
| X6 | A headless browser for Zeca (`chromiumoxide`), its own profile, never the user's browser | Room |
| X7 | Windows and macOS betas | `CLAUDE.md`, *Priorities* |
| X9 | Open terminal on more desktops: GNOME Wayland (a Shell extension), Hyprland and Sway (`hyprctl`, `swaymsg`), terminals by name (Ghostty, Alacritty tabs) | Room |
| | macOS in the CI matrix | X7; a `.dmg` needs signing and notarization |
| | A signed Windows installer | A code-signing certificate (SmartScreen warns without one) |
| | AppImage | A faster build (dropped in #130: the slowest job) |
| | Flatpak | Wayland and KDE settled; the sandbox must reach the socket, the agents' configs and layer-shell |
| | The updater, stable / beta / nightly channels | [ADR 0019](../adr/0019-requests-on-the-users-behalf.md): off until turned on, install after a click, signed, never in distro packages |

Watch: the folded island vanished once in an R3 pass (likely the harness switching displays);
the sunglasses' summer window is still to choose; Codex keys hook trust by position, so removing
ours may make Codex ask again for a later hook.

**Dropped (2026-10-12):** X1 side panel, X2 a bird on the desktop, X3 a voice per vult, X4 Vercel,
X5 voice dictionary and cloud transcription, X8 perch together; W10's Gemini CLI check (Gemini CLI
retired, #310).

**Dropped (2026-10-16):** Tasks (W3) and what only they used: runs in their own clone (W4), the
handoff through a context pack (W5), the review loop and its live check (W6), the router behind
`/delegate` (W8). Their ids stay in Done; N0 removes the code.

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
| Surfaces | One Nest window, built when opened (D6): Flock, Chat, Connections, Activity (Tasks left in N0), plus Settings as its own webview ([0021](../adr/0021-four-sizes-one-nest.md)) |
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
| Wake word | No | Voice by shortcut or opt-in realtime |
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
5. PR, CI green, a merge commit (never a squash); move the step to **Done** in the same PR.

The prompt for a coding agent, one session per step:

```markdown
Implement step <ID> of docs/dev/plan-zeca.md: <title>.

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

At the end, list what was left out and why, and move the step to Done in plan-zeca.md.
```

## Done

One line per finished step; the details live in the commits and module docs.

### UI refinements accepted 2026-10-14

The user reported reviewing all current UX1-UX5 areas and accepted them after #351.
The detailed delivered slices and acceptance record remain in [plan-ui.md](plan-ui.md).

| # | What | Acceptance |
| --- | --- | --- |
| UX1 | Accessible names, selection, keyboard navigation and task focus | User review after #351 |
| UX2 | Narrow Chat, Settings and approval layouts; composer and return actions | User review after #351 |
| UX3 | Consistent usage warnings and cost meaning | User review after #351 |
| UX4 | Quick Chat, Flock and Week presentation on existing sources | User review after #351 |
| UX5 | Task drafts/actions/recovery, connection detail, record search and setup guidance | User review after #351 |

### Wave 0: hygiene and decisions

| # | What | PR |
|---|---|---|
| G1 | `check-layers.sh` reads `cargo metadata`; knows `store`, `zeca`, `link`, `relay` | #199 |
| G2 | The hook exits 0 and silent on a panic | #196 |
| G3 | Dropped: PRs stay merge commits | |
| G4 | CI fails above the hook's cold-start ceiling (`hyperfine`) | #197 |
| G5 | Releases carry build provenance | #198 |
| G6 | ADR 0004 accepted; rule 2 names every consent | #200 |
| G7 | ADR 0004: a signed phone decision is a click; Kotlin for the Android shell | #201 |
| G8 | ADR 0018: Zeca as an agent | #202 |
| G9 | ADR 0019: requests on the user's behalf | #203 |
| G10 | ADR 0020: 0.2.0 at the Operations gate, the rest 0.2.x | #204 |
| G11 | This plan in the repo | #195 |

### Wave 1: foundation

| # | What | PR |
|---|---|---|
| S1 | `store`: one `vults.sqlite`, migrations; history imported | #205 |
| S2 | Append-only audit of every answer | #206 |
| S3 | `core::policy` classes and `core::ledger` offers; island clicks through it | #207 |
| S4 | Proptest of `reduce`'s invariants and rule 2 | #208 |
| S5 | Fuzzing of the hook's road, protocol and agent JSON | #209 |
| S6 | Traces of Zeca's turns and tools, with tokens and cost | #210 |
| S7 | Evals with cassettes (`--features evals`) | #211 |
| S8 | `AgentDriver` in `agents`; Claude Code and Codex in `chat` | #212 |
| S9 | `zeca` crate: a read-only `FlockReader`, memory in git | #214 |
| S10 | The network gate; Codex usage behind it; `check-network.sh` | #217, #218 |
| S11 | `AgentDriver` v2: `Spec`, `Interrupter`, `Answer`, tool and turn-end events | #225 |
| S12 | The classifier never under-rates the network or a guess | #220 |
| S13 | `consents` apart from the audit | #223 |
| S14 | An offer's life starts when `ipc` read the hook's line | #221 |
| S15 | Memory commits say who | #222 |

### Wave 2: Zeca useful now

| # | What | PR |
|---|---|---|
| Z1 | Native mode: the chat reads the user's own setup; `VULTS_ZECA` marks its events | #226 |
| Z2 | "Always in this conversation" on the chat's card | #228 |
| Z3 | Model and effort per engine | #227 |
| Z4 | Many conversations, resumed after a restart | #229 |
| Z5 | The Nest window with its Chat tab | #230 |
| Z6 | The API chat's tool loop (Anthropic, Chat Completions) | #232 |
| Z7 | Web search and fetch through a Q-LLM with no tools | #238 |
| Z8 | Taint per conversation; the next card warns | #234 |
| Z9 | Flock tools in the API chat | #237 |

### Wave 3: memory

| # | What | PR |
|---|---|---|
| M1 | Memory files of facts: limits, substring edits, an injection scan | #241 |
| M2 | Frozen and volatile context on all three engines | #242 |
| M3 | Roosts by repository | #243 |
| M4 | The journal, each past day summed up by a small model | #244 |
| M5 | Recall: FTS5 + a local embedding (brute-force cosine), RRF; hit@3 0.82 | #245 |
| M6 | `/remember`, `/forget`, proposals; accept is a commit, undo a revert | #246 |
| M7 | The hand-written `vults-zeca` MCP for the chat's Claude Code | #247 |
| M8 | Zeca's sleep: repeats dropped at the source, merges proposed, a debrief | #248 |
| M9 | Shared project memory: `@AGENTS.md` in `CLAUDE.md`, diff first | #249 |

Docs audit after wave 3 (memory switch, settings 18, Zeca's own MCP not outside): #250.

### Wave 4: work and Operations

| # | What | PR |
|---|---|---|
| W1 | Capabilities per agent (`caps`) and the doctor | #253 |
| W2 | The policy engine, written and accepted in Settings → Approvals | #255 |
| W3 | Tasks, `/task`, the Tasks tab, an issue on a click | #256 |
| W4 | A task starts an agent in a clone of its own, sandboxed, in the flock; Stop for those only | #257, #259 |
| W5 | The context pack and a task's handoff; Claude → Codex eval run live | #258, #260 |
| W6 | The cross review loop, up to three rounds; 10 of 10 planted bugs sent back | #261 |
| W7 | `/continue` for a stopped session whose process is gone | #262 |
| W8 | The router and `/delegate` | #263 |
| W9 | Costs by day, task and roost; a daily budget that only stops new starts | #264 |
| W10 | An ACP driver: OpenCode, Qwen Code, Copilot CLI take tasks | #265 |
| W11 | The Nest's History, Usage, Traces and Audit | #266 |

### Wave 5 and 6, done so far

| # | What | PR |
|---|---|---|
| K9 | The command palette (as U13) | #281 |
| L1 | `crates/link`: pairing by QR, sealed envelopes, counters in `store`, the folder bound to the card | #233 |
| L2 | `crates/relay`, its client, the phone in Settings | #236, #239 |

### Living Flock (U0 to U13)

| # | What | PR |
|---|---|---|
| U0 | ADR 0021 and the 08 prototype as reference | #268 |
| U1 | The Nest's shell: five tabs and the gear | #270 |
| U2 | Four sizes and the switch (*Room* for Panel) | #271 |
| U3 | Mini, the pill without text | #272 |
| U4 | `SessionView` gains `roost` and `branch` | #269 |
| U5 | The sky's motion: flock moments, molt, thermals, flight signatures, a task's bird from Zeca | #319, #320, #322, #337 |
| U6 | Quick, the island at 440 px | #273, #274 |
| U7 | Nest › Flock | #275 |
| U8 | Nest › Tasks by roost | #276 |
| U9 | Nest › Chat | #277 |
| U10 | Nest › Activity, the budget form | #278 |
| U11 | Connections, filtered; Add a connection | #279, #282 |
| U12 | Settings in the Nest's window (ADR 0021) | #280, #283 |
| U13 | The palette (Ctrl+K) | #281 |

### The look (L0 to L17)

Not the mobile L1 to L5 above: these are the Living Flock look steps.

| # | What | PR |
|---|---|---|
| L0, L1 | The prototype's tokens, icons and the Nest's frame | #285 |
| L2 | Nest › Flock as drawn | #286 |
| L3 | Nest › Chat as drawn | #287 |
| L4 | Nest › Activity as drawn | #289 |
| L5 | Nest › Connections as drawn | #290 |
| L6 | Settings as drawn | #291 |
| L7 | Quick as drawn | #292 |
| L8 | Tasks and the pill | #293 |
| L9 | Quick's drill-in | #296 |
| L10 | Quick's tabs | #298 |
| L11 | Room resizes and keeps its size | #299 |
| L12 | Live check of Quick's tabs, by the user | |
| L13 | Small leftovers | #303 |
| L14 | The chat's engine pill | #306 |
| L15 | The Nest in the README | #298 |
| L16 | Activity by project | #312 |
| L17a | Ask Zeca and ✓ on Connections' rows | #315 |
| L17b | Squash and merge on an approved, green PR | #317 |

### Road to 0.2 (H, E, C, P)

| # | What | PR |
|---|---|---|
| H1 | Rule-2 test over every `Intent` | #69 |
| H2 | Generated TS view types with a freshness test | #68 |
| H3 | Historical Lab visual tests addressed states by name; the Lab was removed | #70 |
| H4 | Settings read `version`; a 0.1.0 fixture | #69 |
| E1 | `Attention` in core | #82 |
| E2 | `focus` in core | #87 |
| E3 | A card's `Outcome` in core | #84 |
| E4 | Platform per window (`LayerSpec`) | #96 |
| E5 | The view to every live surface | #97, #98 |
| E6 | Panel mode | #99 |
| E7 | Desktop notifications that only open the island | #101 |
| E8 | Presence presets: Island, Panel, Quiet, Paused | #103 |
| E9 | Corner widget (about 40 MiB more) | #109, #111 |
| E10 | Zeca off | #105 |
| E11 | Shortcuts through the portal | #88 |
| E12 | Hello bounce | #75 |
| E13 | Silero VAD, tap to talk | #94, #95 |
| E14 | Live partial text while speaking | #110 |
| E15 | Spike: a speaking engine (later removed with C9) | #118 |
| E16 | Sound volume | #73 |
| E17 | The looks picker | #107 |
| E18 | Keep the user's Claude status line | #93 |
| C1 | Quick actions on a bird | #115 |
| C2 | Per-project mute, pin, hide | #119 |
| C3 | Moved to K9 | |
| C4 | Attention ladder, do not disturb | #121 |
| C5 | "While you were away" digest | #122 |
| C6 | Antigravity hooks, generic agents | #116 |
| C7 | Research: how each agent could be stopped (see D8) | #152 |
| C8 | A quiet bird | #120 |
| C9 | Zeca spoke; removed (local voice too poor), replaced by K8 | #123, #129 |
| C10 | Moved to X5, dropped | |
| C11 | Moved to K8 | |
| P1 | Moved to S1 | |
| P2 | Moved to M3 | |
| P3 | Moved to W1 | |
| P4, P5 | Moved to X1, X2, dropped | |
| P6 | The contribution grid, in Activity | #188 |
| P7 | Moved to W9 | |
| P8 | Moved to S2 | |
| P9 | Moved to K8 | |
| P10, P11 | Moved to X3, X4, dropped | |
