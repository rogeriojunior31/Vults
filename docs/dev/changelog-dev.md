# Development changelog

Everything finished on the way to Vults, newest areas last. It holds what used to be Plan Zeca's *Done*
index and its stage delivery records (N0 to N4, the UI delivery sequence). Code comments cite step
ids as `plan-zeca <ID>`; every finished id is here. What is left to do is in [ROADMAP.md](ROADMAP.md);
what shipped to users is in [CHANGELOG.md](../../CHANGELOG.md).

Moved verbatim on 2026-10-25. Where a paragraph says "below" or "above", read it against this file.

# Release planning

### REL-1 · Choose the release plan
`P0 · S · Needs: —`
- [x] 2026-10-26: the owner selected **0.1.9** as the next stabilization release from current main,
  including its already-merged N0-to-N3 work. Later versions follow bounded release candidates and
  their acceptance gates, rather than reserving a number for each phase.
- [x] Record the dated decision in [ADR 0020](../adr/0020-0.2.0-at-wave-4-then-0.2.x.md), align
  ADR 0031's release clause and both reviewed pt-BR translations, and update *Where we are*.
- [x] Merged in PR #387 at `dae212de`; main CI and Security passed. R3, native acceptance, Security and REL-2 remain separate pending work.

Local validation (2026-10-26): ADR links, History anchors and reviewed translation source hashes
pass. UI build, all 19 renderer checks, workspace formatting/Clippy/tests, feature-enabled evals
and repository guards pass. Package/version files and lockfiles are unchanged at 0.1.8.

**Done when:** the ADR records the plan and this section matches it.

# Stage delivery records

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
On 2026-10-17, the user validated the corrected Flock/Compact themes in the running app,
accepted N1.5 and requested the N1 PR before N2. N1.1-N1.5 are complete. The broader
UX6 desktop matrix (multiple displays/scales and window ownership) and R3 release
smoke remain tracked separately.

The Flock/Compact theme follow-up replaces fixed dark surfaces, pale diff text and control
colors with shared roles. Active canvases refresh their palettes on a theme change; hidden
ones invalidate without starting animation. Native checks cover Flock at 480/760/1180 px,
Quick and Mini in both schemes, populated sessions, an unanswered permission and live
switching with motion paused. Representative text contrast is checked in the native renderer.

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

N2.1 implementation keeps skipped cards in the core's pending queue and ledger with their
original deadline. Skip only changes presentation/order; stale local controls cannot redeem
a skipped offer. Selecting its session reopens the card. Both Flock and Quick expose Skip;
the Needs you strip filters skipped tags and keeps an explicit return through the flock.
Question drafts survive skipping. Core and renderer checks cover these invariants; native
validation uses isolated requests without executing an agent command. User acceptance of
N2.1 is pending. N2.2 adds Quick's Now inbox: one `ViewModel.inbox`, derived from
core pending offers, session attention and connector snapshots, with Chat's waiting count
and locally read memory proposals supplied by app. Each pending offer keeps its own ID,
including several offers from one session; Review only reorders/reopens that exact offer.
Quiet connection news can be marked seen, while failures and pending offers remain.
The destinations reuse Flock, Chat, Memory and the selected Connections detail. Activity
and Connections return to Now when the Nest goes back to Quick.

N2.2 core/renderer checks cover ordering, skipped offers, exact-request review, silence,
connector deduplication, Seen versus decisions, Zeca off and retained focus/scroll.
The native Linux pass covers Black and Light, a full seven-item inbox, bounded feed/header/
footer, the empty state, separate cards from one session, Memory/Connections destinations,
a stale connection filter, Seen preserving a failure, and Zeca off. Controlled IPC offers,
local proposals and a local `gh` fixture executed no agent command or remote action.
User acceptance of N2.2 is pending.

N2.3 gives idle and successfully finished sessions one monotonic 30-second rest deadline in
core. Metadata, navigation and the wall clock cannot restart it. App wakes at the next rest
boundary without waiting for a hook; Mini and Quick keep a green mark until then and reuse
the existing sky. The front bird now wears its Mini badge too. Work, permissions and questions
cancel rest and land that bird; night, music, reduced motion and hidden scenes keep their
existing motion rules. Calling a circling bird down keeps a local 30-second perch break.

Core/renderer checks cover the exact boundary, staggered sessions, waiting offers remaining
open, late mounts, call-down, urgent landing and motion exceptions. The native Linux pass
covers Black and Light, both green Mini birds, Quick, a Nest return during the grace period,
and the unprompted transition from green at 29 seconds to flight at 31. New work lands only
its own bird; a new completion starts another grace period; a controlled permission lands
without a decision or command. Build, workspace tests, evals and repository checks pass.
User acceptance of N2.3 is pending.

N2.4 adds a Session overview to the existing Flock owner: reported running time, API-price cost,
current context occupancy, four recent steps and four changed files, with links to Activity and
Changes. Rows show agent, step, reported duration and context; named birds share the selected
session. Status chips filter the list without replacing its selected card. Narrow Room switches
between list and detail; short windows give the wire's space to content. List and detail retain
independent scroll; a new request brings its card back to the top without a focus-induced jump.

The existing Claude Code status-line relay now keeps validated numeric duration and context
alongside cost and plan windows. Paths, model data and transcript contents stay out. Core stores
per-session snapshots without creating sessions, changing activity, resetting rest or answering
offers. Missing metrics, including Codex hooks' duration/context, say Not reported; genuine zeros
stay zero and compaction may lower context. File counts reuse reported edits and the existing
folder observation, distinguishing command-written files from counted diffs.

Core, hook, parser and renderer checks cover those contracts, retained filters/detail/diffs,
keyboard navigation and permission priority. Native Linux validation uses six sessions through
the real hook and covers Black/Light, 480 x 360, 760 x 600 and 1180 x 820, known/zero/unknown
metrics, named birds, filtered selection, independent scroll/focus, narrow navigation and a
destructive request arriving over a scrolled overview. Quick/Nest return preserves search,
status, detail and owner; the empty Flock remains usable with Zeca on. No test executes a tool
or answers an agent permission. Build, workspace tests, evals and repository checks pass.
User acceptance of N2.4 is pending.

N2.5 adds the plan usage ring and percentage to the existing model control in Quick and
Nest Chat. It selects the fullest active window of the selected CLI, distinguishes a reported
zero from missing data, and uses the shared 70% warning / 90% critical statuses. The tooltip
lists each active window and its reported reset time; the accessible name includes usage and
status. API Chat never borrows Claude Code's subscription. A reset deadline removes expired
windows without a new hook, and closing or hiding Chat stops its timer.

Usage changes now reach every surface. The shared Bridge subscribes before reading the cache,
so a delayed initial read cannot overwrite an event. The feature reuses the existing status-line
relay, Codex opt-in, polling and network gate; opening Chat adds no provider requests. Updates
touch only the model indicator, preserving the draft, files, transcript and open menu.

Renderer checks cover provider scope, thresholds, zero/unknown/invalid data, expiration,
timer lifecycle, retained controls and the actual Nest/Bridge event wiring. Native Linux
validation uses the real status-line hook and a local Codex app-server fixture: initial cache,
live updates in both surfaces, retained draft and focused choices, automatic expiration,
provider changes, API absence and the existing Codex switch. Visual checks cover Quick and
Nest in Black/Light, with Nest at 480 x 360, 760 x 600 and 1180 x 820. No test sends a chat
message or answers an agent permission. Build, workspace tests, evals and repository checks pass.
User acceptance of N2.5 is pending.

N2.6 adds shared Chat suggestions for session and project references typed with `@`.
Filtering uses live flock data, including full folders and session IDs for duplicate names.
Arrow keys select; Enter, Tab and clicks insert a readable quoted reference without sending.
Escape, outside clicks, IME composition and leaving Chat dismiss the menu. Its overlay is
bounded to the available conversation area, independently of transcript sizing. Draft sync
retains the exact reference targets between Quick and the disposable Nest.

On send, the app validates at most eight selections, resolves agent plus session ID or the
exact project folder, and limits the current turn's flock context and API tools to their union.
Claude Code's existing read-only MCP flock tool applies the same selection on each call,
through a private, atomic turn-scope file and identity keys beside its redacted snapshot.
Missing targets and unbound quoted references fail before a provider turn starts; edited-away
references restore ordinary flock context. References never change the working folder, answer
cards or grant access, and prior conversation messages remain intact. Saved history retains
readable reference text; a recalled reference without its selection must be selected again.

Renderer checks cover both surfaces, keyboard/IME/caret and full pointer sequences, duplicate identities,
live selection, draft transfer, explicit sending and unchanged folders. Rust checks cover
validation, exact scopes, missing/edited references, API tool/context filtering, MCP selection
and old/new draft serialization. Native Linux validation uses four sessions from the real
hook, duplicate folders and session IDs, Quick/Nest draft return, Black/Light and Nest at
480 x 360, 760 x 600 and 1180 x 820. A local Claude fixture captures the real prompt; the
real MCP stdio server returns only the selected session. A missing target starts no provider
process and retains its editable draft. No real provider request, agent tool
execution or permission answer is used. Build, workspace tests, evals and repository checks pass.
User acceptance of N2.6 is pending.

N2.7 compacts the existing Nest Chat in Room and Full: a 42 px wide-layout title bar,
13 px transcript with tighter message spacing, and a one-line initial composer sharing its
760 px reading column. The empty state centres Zeca's original 3x scene and vertical
suggestions; populated Chat keeps that same scene at 2x above the composer, outside the
scrolling log. Reserved space prevents it from covering text at any scroll position.
Attachments use a bounded horizontal strip; shorter windows cap the growing draft and fold
the portrait at 560 px, with its animation stopped. Quick keeps its own layout and controls.

The surface check verifies that replacing messages leaves the same companion mounted.
Native Linux validation covers empty/long Markdown/API states, real quota and reference
sources, keyboard selectors, retained multiline drafts/files, Recent/Memory and Quick/Nest
transitions, and Zeca off. Black/Light checks include 480 x 360, 760 x 600 and 1180 x 820;
24 attachments and ten draft lines also cover 481/560/561 px boundaries, with at least 40 px
of transcript and reachable writing and action controls. A local CLI fixture supplies text
and an unanswered permission; no real provider request or agent tool execution is used.

That native pass exposed an existing Chat rule gap: destructive calls still offered Always.
Conversation-rule eligibility now uses the existing core classifier on the whole command;
such calls cannot create or redeem an Always rule, including an older in-memory entry.
The regression covers multiple shell tools, wrappers, compound calls and a cut display target.
Native cards keep Deny primary and omit Always in both themes at all three standard sizes.
No permission is answered during validation. Build, workspace tests, evals and repository
checks pass. User acceptance of N2.7 is pending.

N2.8 adds a pure `ViewModel.chat` with time-of-day greetings and offers derived from the
flock. The app supplies its local hour on the existing clock updates. Core rules prioritize
a waiting session, then a failure, then active work or an empty flock; session questions
carry the exact agent and session ID. All sentences have the four language variants.
These are offers inside an explicitly opened Chat, not K1's proactive speech: no model
request, sound, notification, permission answer or automatic opening is added.

Room and Full show the offered questions in the empty conversation; clicks append to the
editable draft using N2.6 references and retain files and the execution folder. Quick keeps
only the compact greeting and caption. Both layouts offer simple continuations under the
latest completed successful reply, including a saved reply, without guessing its meaning.
Question chips fill the draft; Open Flock uses the existing save/error transition owner.
Streaming, errors, stopped turns and Zeca off cannot activate continuations. Core refreshes
update the offers without replacing messages, scrolling the transcript or moving the caret.

Core checks cover greeting boundaries, languages, offer priority and unchanged waiting
permissions. Renderer checks cover exact identities, obsolete offers, focus, retained drafts,
reply eligibility and the real Nest entry's failed save and retry. Native Linux validation
covers quiet/working/failed/waiting states, four languages, Black/Light in Quick and Nest,
and Nest at 480 x 360, 760 x 600 and 1180 x 820. A local CLI fixture supplies one reply;
no real provider request, agent command or permission answer is made. Build, workspace tests,
evals and repository checks pass. User acceptance of N2.8 is pending.

N2.9 connects Zeca's existing clips to live Chat events in Quick and the Nest. Typing and
IME input give a brief listen cue; voice recording listens and transcription thinks.
Reported tools search until they end; new reply text speaks visually, returning to thought
during a quiet stream and resting shortly after a successful completion. Saved replies and
restored drafts never replay these brief cues. Active conversation states cut a cosmetic greeting immediately. Every unanswered permission
outranks later text, tool notes, attachments and voice. No pose sends a message or answers a card; K7's
flock-wide mood, semantic reactions and generated speech remain outside this step.

The Chat driver now forwards tool activity as presentation data, tracking simultaneous CLI
calls by ID and bracketing API reads. The app's existing live snapshot retains this state
when the disposable Nest is recreated. This leaves approvals and outside-content tracking
with their existing owners. Reduced motion and manual still mode draw each current scene
pose and head mark once, without a frame loop; active scenes refresh when the preference
changes, while short, hidden, locked and disabled companions keep their lifecycle guards.
Closing or hiding Chat clears its transient reaction timer.

Checks cover concurrent/duplicate/unknown/stopped tool events, both API wires, live versus
saved poses, pending-card priority, timer cleanup, late initial reads and retained reading,
focus and caret. Native Linux validation uses a controlled local CLI's real tool/text/card
events, without running its reported tools or answering its permission. It covers typing,
search/thought/speech transitions, completion/expiry, saved and active recreation, Quick/Nest
continuity, Black/Light, 480 x 360, 760 x 600 and 1180 x 820, reduced/manual motion and Zeca
off. Voice poses use the existing display-only fixture; no microphone or speech service runs.
Build, workspace tests (876), evals and repository checks pass. N2.1-N2.9 are implemented;
Final N2 acceptance is recorded below.

N2 acceptance follow-up (2026-10-17): Chat keeps provider reasoning and tool arguments/status in
one closed Activity disclosure per turn. Only the selected pending approval expands; explicit
Review and Previous/Next retain exact request ownership, and answered cards become closed receipts.
Ended requests expire as Cancelled. Commands, folder and decision actions fit the 480 x 360 Nest;
Always moves into More choices. New approvals enter upward, respecting reduced/manual motion.
The portrait hover follows the sprite silhouette. CLI guidance now avoids unnecessary tools and
prefers reading/search tools for simple inspection; permission authority and access remain unchanged.

Bounded local activity survives saved/live Quick/Nest recreation; recognized secrets are redacted
before persistence. Public Codex summaries and Claude thinking events remain separate from reply
text. Missing reasoning/results are explicit. Checks exercise exact selection, stale controls,
shortcuts, inert restoration, tool completion/failure, truncation and redaction. Native fixtures
cover actual stream/card events, command/actions together at minimum size, Black/Light, manual
reading, disclosure/restoration and cancellation without executing tools or clicking approvals.
On 2026-10-17, the user accepted N2.1-N2.9 and these follow-ups for the N2 PR. Earlier
per-slice pending notes record their delivery-time status. The broader desktop matrix and
R3 release smoke remain separate; N3 follows the N2 merge.

The N2 PR also repairs Windows test startup: all MSVC executables receive the Common Controls
v6 manifest, including the library test harness. This follows the Tauri build workaround and
retains the existing tests rather than excluding the Waiting regression from Windows.
Reference fixtures also use drive-qualified project paths on Windows without loosening path validation.

On 2026-10-17, Security CI picked up retirement advisories RUSTSEC-2026-0370/0371/0372:
zvariant, zbus_names and zvariant_derive moved into zbus 6. The current Tauri/ashpd/ksni stack
still pulls zbus 5. Like the existing GTK3 retirement exceptions, deny.toml lists only these
informational IDs, with removal tied to the desktop stack's zbus 6 migration. Vulnerability,
license and source checks retain their existing settings. Upstream record:
https://github.com/z-galaxy/zbus/issues/2043.

#### N3 Dictation

The phases of [plan-dictation.md](plan-dictation.md), in this order: V0 (its open R and Q
answered first), V1, V2, V4, V8, V3 (after Z6), V5, V7 (after M7), V6 (= K8). The Dictation tab,
Settings › Dictation and Activity › Dictation land on the N1 frame.

N3 started on 2026-10-17 in `feat/n3-dictation`. The maintainer accepted V0.1 and authorized
continuing with the planned defaults. [ADR 0028](../adr/0028-spoken-replies-through-a-realtime-model.md#dictation-and-meetings)
records local vocabulary, bounded opt-in follow-up listening, temporary meeting audio and model
license evidence. V0.2 is recorded in [ADR 0032](../adr/0032-dictation-into-other-apps.md): exact
targets rechecked before delivery, clipboard ownership, keyboard-only portal permission with a
keyring token, and a current-state guard against inserting into a waiting session. Both records
have reviewed pt-BR translations. V0.3 is recorded in ADR 0021 and DESIGN.md: Dictation is
the fifth Nest tab as its phases land, its seven work pages link to central Settings and
Activity, and transient feedback preserves focus, the fixed overlay and card priority. The
Nest carries feedback while visible; dictation does not wake the hidden island. No audio or UI
behavior changes yet. V0.4 is recorded in ADR 0019: model features stay independently off by
default, with 2,000 clean-up attempts, 20 summary attempts and a US$1 default realtime limit in
any rolling 24 hours. Local providers retain the switch, request quota and trace; only remote
monetary accounting is exempt. The count-only gate must gain spend reservations before V6
ships; this decision does not register or enable any request. V0.5 integrates the phase rows
and related ids in [Dictation (N3)](ROADMAP.md#phase-4--dictation-dictation-tab). The maintainer accepted the V0.9 prototype
on 2026-10-17; P12 is approved in the scope manifest. Its fixes cover Alt+6, separate size
shortcuts, Black/Light, central Settings, conditional-control focus and simulated meeting consent.
No production behavior is inferred from that approval.

V1.1 now extracts `vults-audio`: named/default microphone capture, continuous 16 kHz mono
resampling, bounded memory and nine speech bands at about 30 Hz. Chat keeps its Recorder API,
one-minute limit, Silero watcher and GPU preview; the shared capture accepts up to ten minutes
for later callers. Seven audio tests and sixteen voice tests (with real installed Base/Silero
models and a public speech fixture) pass, as do workspace Clippy (all targets) and the dependency audit. Guides,
architecture, settings reference and translations follow the extraction. The maintainer reported
validation and accepted V1.1 on 2026-10-17; a separate desktop/device matrix was not reported.

V1.2 now applies hysteresis and smoothing to Silero probabilities, with 250 ms minimum speech
and 400 ms minimum silence. Queued edges and reset isolate each analysis. Short pauses stay
inside a segment; Chat retains its 1.5 s end-of-turn pause. All 26 audio/voice tests pass with
real installed models, including four WAV fixtures with 40 ms edge tolerances and the existing
Chat end-of-speech and transcription regressions. This is automated evidence, not a new native
microphone pass.

V1.3 adds the pure transcript gate to the common preview/final decoder. It reduces six-or-more
word/phrase or Unicode character loops to one occurrence, with a 60-character unit cap, and
omits whole-result silence phrases in four languages. Short repetitions, longer sentences
containing those phrases and technical brackets survive. A standalone matching phrase is
omitted even when intentionally spoken; the guide records that tradeoff. All 28 audio/voice
tests pass with installed models, including real-model Chat regressions and a public reported
output fixture; workspace Clippy passes. Native microphone/release acceptance remains separate.
V1.4 now adds the dependency-free `dictation` Connect crate. Caller-owned monotonic time drives
hold/double-tap/Space-lock/cancel gestures; Finish carries the 600 ms capture-tail deadline.
Sixteen integration tests cover timing boundaries, delayed ticks, repeated key events and reset
following capture failure. This code performs no IO and does not bind a key or open a microphone.
V1.5/V1.6 now wire the portal and first clipboard-only runtime together. Talk remains independent
of Zeca, focused Chat retains its draft path, release follows its press owner, and new shortcuts
cancel or copy the last result. Bound keys and remapping are reported by the desktop. The local
worker captures up to ten minutes, honors its tail, shares voice decoding and can retry an already
installed Base model. Generation checks and token cancellation prevent stale writes. The maintainer's
native check found GTK reporting Copied while Wayland kept the old clipboard: a probe reproduced
that false success. Native data-control now handles Wayland background copying, a retained arboard
owner handles X11, and read-back must match before Copied. The native Plasma regression and an
independent GTK observer on isolated Xvfb both pass. No XWayland-only fallback is used on Wayland;
missing background access produces a translated error. Nothing pastes, submits or writes history.
A temporary translated status follows the island/Nest owner without focus or input-region changes;
snapshots preserve it across mounts without rewinding newer progress. All 199 selected subsystem
tests, the UI build/renderer logic checks, workspace Clippy and cargo-deny pass. Native desktop
consent, microphone/clipboard and X11/Wayland acceptance remain pending because no computer-use
surface was available. V1.7 now reads foreground context without activating a window: EWMH
on X11 and a read-only, sender-checked KWin script on Plasma Wayland. Other Wayland desktops,
late/failed reads and ambiguous session ancestry remain unknown. Status names the observed app
and offers a terminal paste hint; titles/window identities remain private in memory. Cancellation
and new turns reject old observations. This asynchronous read is context, not proof of exact
key-down focus or pane identity: exact capture and delivery checks remain prerequisites for
insertion. History's app display awaits V1.13, and native X11/Plasma acceptance remains open.
All 207 selected subsystem tests, the UI build, renderer checks and workspace Clippy pass.
V1.8 now adds live-only XI2 observation on X11: Space locks Holding, other non-modifier keys
cancel Holding/TapWait, and Esc also cancels the capture tail or decoding. Actual Talk/Copy-again
chords and repeats are ignored. Map changes refresh lookup. The worker invalidates stale callbacks
and closes the listener on cancellation, completion or failure; the default Space/Open collision
keeps focus at the destination. Unknown bindings and Wayland retain portal-only controls.
A private Xvfb smoke proves native events reach the listener and foreground without a grab,
and that dropping the handle stops it. Real X11 desktop/portal/microphone acceptance remains open.
V1.9 now adds opt-in paste in supported plain native X11 editors. A separate synchronous target
tracks the original window/focus child/process; focus loss, held keys and clipboard changes block
Ctrl+V. Its in-memory clipboard owner restores previous text atomically only while the lease still
owns it, preserving new equal-text copies and overlapping dictations. The Settings-only switch
starts off, explains text-only restoration/history limits, and outcome feedback distinguishes
Pasted from Copied. The private-Xvfb smoke covers native dispatch, clipboard requests and races;
real desktop/microphone and visual acceptance remain open. All terminals, IDEs, unknown apps and
Wayland stay copy-only in V1.9.
V1.10 now implements the Plasma Wayland path: an explicit keyboard-only RemoteDesktop grant,
keyring-only rotating restore token, reusable session, honest permission state and explicit retry
following refusal/revocation. A preexisting KWin focus epoch plus a compositor checkpoint validates
the same editor; Vults focus invalidates the destination. Native ext-data-control restores the
previous text in place without reclaiming selection, preserving a new owner's copy. The isolated
headless-KWin/GTK smoke uses real focus/clipboard APIs and a simulated RemoteDesktop boundary;
actual desktop consent/editor/microphone and visual acceptance remain open. No full native matrix
is closed.
V1.11 now implements exact tmux delivery for XTerm/UXTerm and Kitty hosts. The captured
window, pane, client and agent process are checked again, then core's live permission/question/card
state gates literal single-line bytes without Enter. Deferred cards still wait. A server ack yields
In session without touching the clipboard; unsupported or unsafe targets copy instead. Direct
Kitty cannot acknowledge send-text, and WezTerm's copy/search overlay shares its underlying pane
id. Those direct transports therefore remain supported clipboard fallbacks under ADR 0032.
WezTerm's separate Open terminal action now pins the hook's same-user socket and never starts or
selects another instance; this is navigation only. Existing Vults Chat uses its unsent draft;
removed task-run composers stay removed. V1.11's supported implementation is complete, with actual
native desktop/microphone/visual acceptance still open. V1.12 is in progress: Mini/Quick now
reserve a neutral feedback row above the flock, with production icons for existing phases,
translated text, immediate card/lock priority and bounded Quick geometry under zoom/dock changes.
Real microphone bands and the shared GPU-only local preview now follow the recording's ordered
status path and survive a surface handoff. Capture end clears both; cancellation/new generations
reject late updates without logging or storing preview text. Reduced motion hides moving bands.
Conditional two-row captions now use free space outside the island's input region, with card/lock
priority and an original two-second deadline that survives a surface handoff. General's existing
dictation controls own the caption switch until V1.15; schema 25 preserves explicit choices.
Confirmed delivery now shows a localized Unicode word total from the committed text. Unknown
counts stay absent, and failure/cancellation cannot reuse old totals. Correction totals await
the replacement/clean-up features. The optional start cue now has one persistent island owner,
uses shared mute/volume/Do Not Disturb guards and never replays on snapshots or surface changes.
General holds the schema-26 sound switch until V1.15. The maintainer accepted the previous UI
slices; native listening acceptance of the new cue remains pending. Ownership and computed
geometry checks pass; native font/overflow and the full X11/Wayland visual matrix remain pending.
V1.13's store implementation adds migration 21 for text-only dictations and independent
daily/app totals. Retention (Not kept, 7/30/365 days, Always) prunes on history open and record;
clear/delete preserve totals and erase transcript pages. Zero-word results add nothing, and
unknown corrections remain absent. V1.13 supplied the API without enabling transcript writes;
the next task connects the History page and visible privacy controls.
V1.14 now connects that API: the fifth Dictation tab owns searchable retained history, measured
figures and explicit copy/reuse/erase actions. General temporarily owns the schema-27 privacy
selector (30 days by default) until V1.15. Real captures record once, and erase/retention changes
reject old in-flight writes; Copy again never creates a new measurement or reuses an old window.
At the maintainer's request, Chat also uses a slimmer single-row composer in Nest and Quick,
with attachment left, microphone/Send right and a separate file strip. Native acceptance is pending.
V1.15's first slice centralizes current controls in Settings > Dictation (keys, Voice, Paste,
Privacy), linking History, General and Chat & voice to it. Shared model/language editing stays
in Chat & voice; no key/default changes from schema 27. Privacy erasure uses explicit confirmation
and guarded failure/retry. A Settings-only **Change in KDE…** action now launches the installed
KDE shortcut editor with pending/error/retry feedback and no binding writes. Schema 28 now saves
hold/double-tap, Space-lock and Esc-cancel choices with existing defaults. Settings-only writes
apply after a successful save; each new recording snapshots its gestures. Raw-key switches are
unavailable on Wayland, and the global Cancel shortcut always works. Schema 29 adds an optional
independent-dictation microphone, read by the next capture. Settings-only inventory loads on
entry or explicit refresh without recording; failures retain the last list and disconnected
preferences stay selected. The existing missing-input fallback and Chat default stay intact.
Schema 30 now saves optional repetition removal and one-time Base fallback, both on by default.
The next capture snapshots these choices for preview and final decode; Chat retains its existing
filter. Disabled fallback skips its extra speech check, and errors never trigger retry. Current
runtime controls are wired. V1.16 adds original spoken-code conversion for captured terminals
and sessions with a schema-31 switch (on, matching the prototype), literal identifier punctuation
in English/Portuguese/Spanish, unchanged prose/unknown targets and preserved control bytes.
Heard/written history keeps the difference, and replay bypasses conversion. V1.17 adds final
whole/trailing spoken cancellation in English/Portuguese/Spanish, on by default in schema 32.
It discards the independent turn before conversion/delivery, releases its resources, records no
history/totals and preserves the previous success. A recording keeps its captured choice;
manual replay and Chat bypass the check. V1.18 adds per-recording local MPRIS lowering/pause/leave
(schema 33), restoration after capture/cancellation and guards for user changes and crashed/restarted
players. Quieter/paused players stay untouched; no track metadata is read. The maintainer reported
local V1.18 validation on 2026-10-17. V1.19 reconciles user docs, settings and architecture,
isolates the media regression test from real players and records automated/native-fixture evidence
plus the unreported desktop matrix cells in the detailed plan. Delivery/meeting switches and
full desktop-matrix acceptance remain open; no broader sign-off is inferred. The app stays stopped.
At the maintainer's request, V2.1 now prepares schema-22 dictionary/replacement/snippet storage
and Nest-only typed list/edit commands. Unicode duplicate keys, stable replacement order and
literal snippet bodies are local preferences retained across history clearing. The command/data
checks are automated. V2.2 now applies a per-recording snapshot after raw spoken cancellation:
opaque literal snippets, ordered replacements, eligible dictionary spellings and context-bound
code edits. Unicode matching preserves controls, output is bounded, annotated segments retain
changes, and history distinguishes heard/written text and processing style. Work runs outside
the control lock with a generation recheck before delivery. Replay and Chat bypass it; failures
preserve the previous success. V2.3 adds per-recording dictionary context, off by default in
schema 34, capped by the model tokenizer at 224 tokens including the coding seed. Preview/final
share the snapshot; Base retry rechecks its own model, and Chat never inherits the context.
Only Turbo/fixed English qualifies from the 20-sentence-per-language synthetic WER comparison
and empty silence/noise fixtures. Base/Small and Portuguese remain blocked despite improved
sentence WER. Exact results, hashes and human-speech limitations are recorded in the detailed
plan and voice fixtures. Dictionary/Snippets pages and vocabulary controls are delivered in V2.5;
V2.4 now adds independently opt-in project names in schema 35, off by default. Bounded local
Cargo/npm/module/file/branch extraction refreshes on session start and repository refresh,
with separate cache entries for each Git working tree. The captured session cwd or an
unambiguous editor/terminal title selects the names; stale/late/unknown context adds nothing.
Preview/final share a generation-bound frozen snapshot, with personal dictionary words first
under V2.3's existing cap and gate. No source bodies, commands or network requests are used,
and project names do not become saved spelling rules. Small workspace and recording fixtures
pass. No project-name accuracy improvement is inferred. V2.5 now provides Dictionary/Snippets
work pages, literal entry forms, confirmed deletion, retained drafts and read/write retry.
Its Nest-only sentence preview uses saved rules with cancellation/optional terminal context and
inert highlights, with no audio/model/clipboard/history effects. Settings owns both vocabulary
switches. Renderer/Settings/continuity checks and isolated native Tauri Black/Light validation at
normal/minimum sizes passed; the native instance is closed afterward. V2.6 now reconciles the
English/pt-BR guides, settings reference, architecture and Unreleased notes, with a concrete
local check example and an acceptance/evidence matrix in the detailed plan. Its checkpoint has
303 affected-package tests, build/renderer checks, targeted Clippy and repository guards.
V2 implementation/docs are delivered; maintainer acceptance and real-speech checks remain
pending. V4.1-V4.3 now implement retained stage timings and p50/p95 (migration 23), Activity >
Dictation usage with daily application totals (migration 24), and Settings > Dictation Diagnostics.
Diagnostics reads existing audio/shortcut/keyboard/GPU/VAD state, keeps final-decode/retry/Copied
counters in memory and offers an explicit six-second, memory-only microphone test. V4.4 reconciles
English/pt-BR docs and records the evidence and maintainer checklist. Automated checks and isolated
native Black/Light/minimum-size reads pass; real-microphone/compositor acceptance and the Small/Vulkan
release-to-pasted p95 target below two seconds remain unmeasured.
V5.5 engineering now supplies a default-off developer speaker preview; independent release-quality
acceptance remains open. See the [current checklist](plan-dictation.md#v55-consolidated-delivery-checklist). V5.2 supplies Linux recording controls/startup recovery; V5.3 now supplies the bounded capture-to-transcript owner and volatile transcript UI with interactive Whisper priority. Full-duration recording and meeting archives remain future work. V5.1 supplies an optional, isolated Linux capture spike; real hardware/compositor acceptance remains pending. V3.5 delivers the bilingual documentation checkpoint and disposable-text checklist; real-speech/provider acceptance remains pending. V3.4 adds Settings-owned consent/provider disclosure, style/language/app controls, the Dictation Styles editor and applied History labels. V3.3 adds frozen per-app styles, output language, session English, preserved overrides and retained output-language metadata. V3.2 adds tool-free API/local clean-up, default-off consent, durable quotas, cancellation and full-local-text fallback. V3.1 now supplies pure text-filter prompts, opaque local-edit marks, structural reply guards and deterministic evals; Z6 is Done (#232). V8.5 supplies dedicated command settings, the work page, explicit Chat fallback and docs; real-desktop/provider acceptance remains open. V8.1 provides
pure English/Portuguese/Spanish phrase tables, bounded Unicode normalization and atomic sequences
with unresolved names and whole-utterance card-answer refusal. V8.2 adds local project/agent name resolution through aliases, exact names,
whole words and bounded Unicode edits, plus a 90-second clarification state limited to the original
choices. V8.3 adds the local atomic dispatcher, an exhaustive non-answer intent boundary, cached flock
reads in four languages and repeat/undo context. V8.4 connects the separate command key to local capture,
atomic ticket admission, existing effect execution and island/Nest feedback. Command history cannot
be replayed and does not inflate dictation statistics. Native UI fixtures pass; real-microphone and
compositor acceptance remain separate. V8.5 adds dedicated settings, retained command history/search, a shared phrase guide and default-local/opt-in Chat fallback using the existing Chat owner.
The detailed plan records their tests and the two
Windows compilation fixes found in PR #371's CI (mock-test target gate and the diagnostic String type).
No V1 native matrix sign-off or N3 completion is inferred; real-speech/desktop acceptance remains separate.

#### N4 The rest

Taken in the order of the sections below, each redrawn on the new UI before it lands: UX7 to
UX12, Zeca proactive (without K8), the Flock board, Mobile, More agents, Follow-ups and Later.
Two prototype items wait here: the Connections face for Zeca (`conn-face-zeca`) and the card's
risk line (`card-risk`, K3).

### UI delivery sequence

**2026-10-21 UX8.1:** begin with a read-only built-in profile catalog in Flock, backed by native
Connect descriptions and a Nest-only command. Persist the bounded destination, retain the session
view and keep waiting cards reachable. The catalog works with Zeca off; no memory, execution,
provider or permission route is added. Custom copies and Chat use remain subsequent slices.

**2026-10-21 UX8 profile copies:** local preset copies now support name/instruction editing and
immutable revision saves in migration 29. Stale edits are rejected; drafts survive in-window
navigation and save failures. Preset provenance and appearance stay pinned. Store and renderer
checks cover retention, revisions and recovery, and native copy/edit/reload passed at wide/narrow
sizes without using the personal profile. Appearance/skills editing and Chat binding remain open.

**2026-10-21 UX8 profile appearance:** add species and fixed-accessory editing to local copies,
using production sprites for the draft preview and saved card. Migration 30 retains earlier
species/text/provenance and defaults missing accessories to none. Native validation rejects
unknown choices and seasonal Auto; each new revision retains its complete appearance.
Skills and specialist Chat binding remain open.

**2026-10-21 UX8 conversation binding storage:** migration 31 retains exact preset/custom profile
snapshots with each conversation. Custom references resolve a retained revision; later profile
edits cannot rewrite old conversations. Creation is atomic, Zeca's frozen context is rejected and
legacy conversations retain their existing context without a binding. Zeca's current Chat refuses
these records until specialist persona/tool isolation and the explicit recipient/executor/source
contract are implemented. No specialist Chat action or execution route is enabled yet.

**2026-10-21 UX8 specialist runtime primitive:** a Claude Code-only constructor now uses the
pinned brief without the companion persona, frozen context, API tools/history or companion MCP.
Resume forces isolated settings and drops extra directories. Unsupported providers/ACP and unsafe
specs fail before driver execution; the normal approval channel remains in place. Native profile
Chat routing stays blocked until recipient/source ownership and all recall/journal/MCP callbacks
are separated. No new Chat action or live provider request is introduced by this slice.

**2026-10-21 UX8 native recipient and memory separation:** the native runtime now retains and
checks the exact saved profile, source, pick and session before recall. Bound turns skip companion
memory, context, MCP scope and journal callbacks, and never feed companion memory extraction.
Native restore uses the isolated Claude constructor and first-turn persistence keeps the snapshot.
Recipient metadata is cleared/replaced with reset/open/stop/delete. User-facing profile Chat stays
blocked until recipient UI and command/voice/cross-surface transitions are implemented.

**2026-10-21 next implementation:** after merging #375, the maintainer deferred further manual
voice validation and requested pending project work. Proceed with UX7.1's shared Mini/Quick width
selection on `feat/ux7-shared-width`; keep N3's unperformed human/external acceptance explicit.
This implements presentation work without declaring N3, K8 or the N4 release complete.
UX7.2 adds the selected 56/600 logical-pixel height proportions while preserving Default,
existing bird animation and content scrolling. Manual desktop acceptance remains deferred.
UX7.3 adds Mini's reserved, keyboard-accessible overflow icon and routes hidden sessions to
Room/Flock through the existing saved surface transition. Cards keep priority and authority.
UX7.4 completes the implementation slices with shared-picker keyboard navigation, local Escape,
retained focus/scroll after width saves and announced save failures. Only native UX7 acceptance
remains deferred; subsequent implementation work can move to UX8 under the maintainer's direction.
During desktop review, the maintainer requested a slim bar and the same width control in global
General. Mini now stays 38 px high at every width; General and Quick share the saved preference.
The subsequent desktop wire report was reproduced and corrected before UX8: Quick measures its
mounted overview and updates the wire, perches and targets together; the four-width native matrix
passed and restored the maintainer's 560 px preference. UX8 began with the catalog described above.

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
desktop PR validation. UX7 implements shared widths, bounded proportions, Mini overflow and
keyboard continuity, with native geometry/wire checks recorded in the UI plan. UX8 delivers the
local profile catalog/editor, retained revisions, appearance, pinned skills, immutable conversation
snapshots and profile-owned Claude Code Chat. Specialist conversations have per-profile
Ask/Policies/Full access, separate saved history and approval streams, and remain available with
Zeca off without importing project/flock context or Zeca memory. User desktop acceptance remains
open. Quick summaries and execution projection are the remaining UX8 slices; release gates are
unchanged.

**2026-10-22 UX8 profile access and Zeca-off preflight:** each profile now stores its own Ask,
Policies or Full choice, defaulting to Ask, separately from Zeca's Chat setting. Native turn
settlement uses that profile choice and retains destructive/outside-content confirmations. The
Zeca switch now blocks companion turns only; specialist turns no longer inherit it. The specialist
Chat UI remains gated until it owns separate conversation state and cross-surface transitions.

**2026-10-22 UX8 specialist Chat delivery:** Flock profile detail now opens the existing Chat tab
with that profile selected as recipient. The production ChatPanel owns the shared UI; each recipient
retains its own draft, history and native runtime. New conversations and history resume the exact saved instructions, skills and
revision. A Nest-only runtime and event stream keep messages and permission cards out of Zeca's
conversation. The human-only approval path, policy priority, destructive-action rule and
outside-content disclosure remain active. Keyboard shortcuts answer or stop the selected specialist
conversation while it is open. The shared renderer has automated regression coverage and awaits the user's
wide/narrow desktop validation; Quick summaries and execution projection remain separate.

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
| UX11 | Debrief, risk/batches/questions, routines, proposals, moods/voice/ACP and observed-cost views; voice lands with V6/V7 | Alongside K1-K8/K10, M7, W2/W9b and the original Flock board |
| UX12 | Registry-backed agent support and Android permission/service views | Alongside Every agent and L3-L5; G1/G2 remain proposed |

Check the review's findings against the current head in the running application.
Mocked features and synthetic runs are references, not native completion evidence.

# Finished steps

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
| C10 | Moved to X5; reopened as local-only V2 on 2026-10-17 | |
| C11 | Moved to local voice commands V8; reused by K8/V6 | |
| P1 | Moved to S1 | |
| P2 | Moved to M3 | |
| P3 | Moved to W1 | |
| P4, P5 | Moved to X1, X2, dropped | |
| P6 | The contribution grid, in Activity | #188 |
| P7 | Moved to W9 | |
| P8 | Moved to S2 | |
| P9 | Moved to K8 | |
| P10, P11 | Moved to X3, X4, dropped | |

2026-10-18 navigation refinement: at the maintainer's request, consolidate dictation configuration
under its tab, split into eight smaller pages with shared navigation and a narrow picker. Move
the shared local transcription controls to Voice and link from Chat. Retain the Settings webview
and label checks for configuration; Nest keeps transcript work pages and gains no privileged command.
This navigation change does not complete real-speech/provider acceptance or V3.5 sign-off.

2026-10-18 refinement handoff: returning to Dictation restores its last privileged configuration
section, explicit work routes retain their destination, command history links directly to its
settings, model descriptions follow the UI language, and compact History keeps search above its
folded daily figures. V3.5 documentation is delivered; live-provider/speech sign-off remains open.
At the maintainer's request, the local app/server stay stopped and generated caches are cleared.

2026-10-18 V5.1 handoff: an optional Linux-only `vults-audio/meeting-capture` development spike
selects the default sink monitor and a separate microphone by server ID, writes bounded native-format
WAVs to a locked 0700 directory, and deletes audio on return/error or the next explicit recording
after interruption. Synthetic independent streams, crash recovery and server-disconnection cleanup
passed on an isolated PipeWire server. License review and feature tests are recorded in the Dictation
plan. No app integration or real recording is enabled. Next is V5.2: cancellable production lifecycle,
startup recovery and explicit recording controls. Hardware/Wayland/X11 and simultaneous dictation
acceptance remain pending. The app and validation processes stay closed after cache cleanup.

2026-10-18 V5.2 backend handoff: per-recording worker/control ownership now supports nonblocking
status, stop, cancellation and one-shot results. Terminal states follow stream teardown and WAV
cleanup; cancellation wins pending success, and busy-directory recovery cannot remove live audio.
An isolated synthetic test kept a dictation microphone running while meetings stopped/cancelled.
The example supports explicit interactive control and recovery without opening a source. The default
app does not enable recording. V5.2 still needs startup integration, the Meetings route and visible
consent/recording controls; the dedicated local audio connection now bounds backend waits before production activation. The local
app remains closed and generated interface caches are removed after checks.

2026-10-18 V5.2 deadline handoff: replace the meeting-only CPAL host with a bounded local
PulseAudio protocol connection, preserving the ordinary ALSA microphone path. Connection,
authentication and protocol deadlines plus socket shutdown let Stop/Cancel complete even when the
private audio server is frozen. Synthetic stalled-startup, stalled-recording and independent
dictation fixtures pass; private processes and generated audio are removed. The app still excludes
recording. Next remains V5.2 startup ownership and visible consent/recording controls, followed by
real hardware/compositor acceptance and V5.3 long transcription.

2026-10-18 V5.2 recording-check handoff: Dictation > Meetings now provides a Linux-only one-minute
microphone/system-output check, explicit per-attempt source review and consent, state/timer and
stop/cancel. Startup recovers the dedicated private WAV directory without opening audio. Nest-only
IPC, source-name pinning, consumed/expired review tickets, stale IDs, page leases and lock/close
cancellation preserve ownership. The owner sleeps while idle; no model, transcript or meeting
archive is created. Next is V5.3 long transcription; maintainer/hardware/compositor acceptance and
full meeting functionality remain pending. The user's app stays closed after isolated validation.

**2026-10-18 V5.3 backend handoff.** The local meeting WAV reader now yields timestamped
segments with bounded PCM memory and chunks no longer than 28 seconds. Interactive Whisper work
preempts and runs ahead of background meeting chunks; cancellation never returns partial text.
The explicit local developer runner records nothing and changes no input files. See the V5.3
record in `plan-dictation.md`. The visible Meetings page remains a one-minute recording check;
app-owned audio/transcript handoff, two-track coordination and transcript UI are next.

**2026-10-18 V5.3 bounded production handoff.** Dictation > Meetings now captures and transcribes
both reviewed sources locally for up to one minute, with pinned model/language, progress and
independent source views. Text stays only on that page and is discarded on leave/new review/lock;
WAV cleanup precedes publication. Cancel overrides late success. Cold model loading and decoding
run off the UI; interactive speech has priority. No persistence/export/summary or full-duration
meeting claim is made. V5.4 is next; V5.7/V5.8 still own archive/export/full-workspace delivery.

**2026-10-18 V5.4 handoff.** The optional offline Rust diarization experiment now evaluates
fingerprint-checked pyannote/CAM++ exports through dynamic `ort`, native filterbanks and fixed
0.7 average-linkage cosine distance. Three controlled recorded-speech fixtures find the expected
speaker counts and preserve A-B-A identity; overlap leaves 3.140 seconds unknown and misses A's
identity in that interval. [Evidence and sources](diarization-spike.md) record a go for bounded
V5.5 engineering and a no-go for shipping automatic labels until real PT/EN quality, runtime/model
lifecycle and resource/cancellation gates pass. No app/UI activation or new download is offered.

**2026-10-18 V5.5 admission/association handoff.** The voice backend now admits only a complete,
size/SHA-256-checked model pair with cancellation and bounded reads; the optional native spike
consumes those bytes directly. Conservative whole-segment speaker association preserves source
text/clocks and leaves changes, gaps, overlap and unknown activity unassigned, with bounded event
processing and no partial cancelled results. Fourteen new voice tests, a recorded-turn replay
and a public-speech native smoke check cover this slice. V5.5 remains open for the native worker,
preemption, runtime/download lifecycle and real-meeting quality gates; no app activation is claimed.
See the [V5.5 checkpoint](diarization-spike.md#v55-checkpoint-offline-admission-and-text-association).

**2026-10-18 V5.5 native processing handoff.** The optional voice backend now analyzes one
bounded source under its caller's recording worker/cleanup ownership, with normalization,
progress, explicitly admitted runtime/models, native cancellation and interactive Whisper
priority. Interrupted operations discard their attempt and retry with fresh options; cancellation
through final publication returns no partial timeline. Nineteen runtime-free cases and three
opt-in native checks pass, including a real terminate error and foreground-first native retry.
[Evidence and limits](diarization-spike.md#v55-checkpoint-interruptible-bounded-native-processing)
retain cooperative-interruption, runtime-delivery and real PT/EN meeting acceptance requirements.
The next V5.5 step is runtime/model distribution and gate integration; app activation stays off.

**2026-10-18 V5.5 installer handoff.** An optional transactional developer installer now prepares
a reviewed, fully verified Linux x86-64 GNU model/runtime bundle with notices, per-request caller
gate, cancellation, exclusive staging/recovery and atomic publication. A fresh install and zero-HTTP
cached revalidation pass, and official ONNX Runtime 1.31.0 passes the three native checks with
startup privacy enforced. The bilingual model reference and content-free installation record are
linked from the [checkpoint](diarization-spike.md#v55-checkpoint-reviewed-transactional-artifact-installation).
The next step is the app's explicit-action consent/quota/trace owner and install UI. No app download
caller, analysis activation or real-meeting quality acceptance is claimed by this backend slice.


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


**2026-10-19 consolidated V5.5 handoff.** The reviewed batch now connects bounded 30-minute
source analysis, global voice clustering, model-derived word timestamps and conservative
association to the actual meeting worker and native transcript UI. A build-only developer feature
and a fresh per-recording checkbox protect default builds. Failed/missing analysis preserves plain
text. Installer implementation is complete for this preview. The independent PT/EN release-quality
gate remains open; automatic production labels stay disabled. The maintainer subsequently authorized V5.6
engineering; its first pure-contract task is recorded below. The
[current finite checklist](plan-dictation.md#v55-consolidated-delivery-checklist) separates delivered
engineering, measured evidence and unqualified release claims.


**2026-10-19 V5.6.1 summary contract.** After accepting the bottom dictation indicator, the
maintainer authorized the next roadmap step. The pure `chat::meeting_summary` contract now
builds untrusted source-separated prompts and validates bounded typed notes, preserving local
text on rejection. Authored multilingual/injection cassettes and a reviewed prompt snapshot
cover the contract; they do not qualify model truthfulness. No model was called or consent
enabled. Next: the independent `meeting-summary` consent, quota, timeout/cancellation and the
Summary interface, as specified by the [V5.6 sequence](plan-dictation.md#v56-delivery-sequence).
The V5.5 independent diarization quality gate remains open.

**2026-10-19 V5.6.2 gated summary backend.** Independent default-off consent, ledger-bound
20-attempt rolling-day admission (local calls included), a 120-second deadline and explicit
Nest-only generation now connect the finished transcript to the selected API chat model without
tools. Cancellation/revocation and generation checks protect transcript fallback and stale-result
publication. Shared reservation ownership also fixes canceled blocking admissions in dictation
cleanup. Independent review and 244 app tests pass, with Chat/evals, Clippy, renderer build and
repository guards. No user transcript was sent and the app stays closed. Next: V5.6.3 Settings
control, disabled-state explanation and Summary presentation with native validation. V5.6 remains
in progress; this backend checkpoint does not waive V5.5 quality or qualify model truthfulness.

**2026-10-19 V5.6.3 notes interface.** The V5.6 implementation sequence now has its dedicated
Settings-owned consent and the Nest's Transcript/Summary/To-dos workflow, with four-language
provider/limit disclosure, explicit generation/cancellation, safe text and source fallback.
Native Black/Light checks cover 480 x 360 and 1180 x 820 plus real consent changes; populated
notes use controlled responses, not live-model quality evidence. Deferred renderer regressions,
244 app tests, Clippy/build/guards and independent review pass. QA files are removed and the app
is closed. Next: V5.7 meeting storage/export. Maintainer/live-provider acceptance and the V5.5
independent diarization quality gate remain separate from this delivered implementation.

**2026-10-19 V5.7 meeting storage/export.** The store now owns atomic text-only meeting snapshots,
source-local speaker renames and Markdown/SRT/VTT exporters with golden files. History erasure removes
these rows too. This backend is not yet called by the recording check; explicit save/archive/export
controls are V5.8. Independent V5.5 speech qualification remains open and production labels stay off.

**2026-10-19 V5.8.1 archive API after consolidation.** PR #372 merged as `10731101` after Linux/Windows,
security, docs and five fuzz checks passed. Old N3 worktrees/branches and generated build caches
were removed, preserving local models/data and ignored prototypes; the cleanup freed 44.8 GiB.
A fresh `feat/v5.8-meetings` worktree starts with Nest-owned archive actions, explicit immutable
snapshots, and separate global-erase invalidation. The next step connects the full workspace,
renaming/export and user docs, as specified in the V5.8 delivery sequence.

The V5.8.1 backend step passes all 1,233 workspace tests (including 252 app and 85 store tests;
10 documented ignores), all 10 eval tests, workspace/eval Clippy and existing renderer/repository
checks. An independent review found no blocker. No new archive controls are visible yet;
V5.8.2 owns the full Meetings workspace and its native validation. No user transcript was retained
by this implementation check, and the app stays closed.

**2026-10-19 V5.8.2 full Meetings workspace.** Explicit Save connects the current text/notes to the
archive. Saved meetings has title search, Transcript/Summary/To-dos, source-local voice names,
confirmed deletion and parented native Markdown/SRT/VTT export with an explicit subtitle source.
Global erase, lock and departure invalidate cached content and pending reads without coupling
meeting retention to dictation retention. Review fixes clear private search text and keep initial
summary retry/settings reachable. Secondary recording/summary actions use disclosures so Save and
Generate remain visible with at least 69 CSS pixels of reading space at 480 x 360.

All 1,234 workspace tests (10 documented ignores), 10 evals, workspace Clippy, renderer build,
18 renderer checks and repository guards pass. Isolated native WebKit/KWin Black/Light checks cover
wide/narrow/short layouts, real archive CRUD, inert notes, native chooser cancel and all three export
formats, plus global erase and Nest ownership. Controlled production recording UI covers explicit
save and all four languages without audio/model requests. EN/PT guide and catalogs are reconciled;
maintainer acceptance remains separate. The app/server and synthetic profile are cleaned after QA.
Independent V5.5 speech qualification, full-length capture and default-on speaker labels remain
unqualified; this step does not change those gates. See the V5.8 record in Plan Dictation.

**2026-10-19 V7.1 question-draft decision.** The delivery order after V5 is V7, then V6. ADR 0028
now defines user-started local dictation into a typed M7 question draft and a separate human
click/Enter to send. It requires verified requester/question/generation binding, revocation on
lock/departure/expiry/disconnect, once-only replies and content-minimal audit. Voice never answers
permissions; ADR 0004 and ADR 0027 are unchanged. EN/PT record and user guide distinguish the
decision from shipped behavior. Next: V7.2's M7 question lifecycle and verified MCP round trip,
then dictation/composer wiring; the external-agent switch/config offer belongs to V7.3. No runtime,
agent config, setting, microphone or network request changes in V7.1.
Independent authority/translation review and all 1,234 workspace tests, 10 evals, Clippy, build,
18 renderer checks and guards pass; documentation marks/links are current. The review clarifies
local-only transcription without provider clean-up. The bilingual safety guide also catches up
with V5.8's explicit meeting snapshots/export retention. No native rerun is needed for this
documentation-only step; the app remains closed.

**2026-10-19 V7.2.1 text-question backend.** A pure question slot binds each editable draft to the
verified process incarnations, connection nonce, exact question/request and generation. It expires
at an absolute deadline, consumes confirmation once and clears pending text on revocation; invalid
input cannot replace a live draft. Linux requester proofs retain and recheck the actual same-user
process chain, start times, parent links and executable identities, using kernel credentials for
socket peers. Content-free terminal receipts reuse the audit format and are tested through the store.
The app does not call these APIs yet: no MCP tool, listener, composer, capture or agent setting is
added. V7.2.2 owns the verified text round trip and runtime audit; V7.2.3 adds local dictation, with
V7.3 still owning external-agent setup. See Plan Dictation's V7.2 delivery sequence.

All 1,246 workspace tests (10 documented ignores), 10 evals, workspace Clippy, renderer build,
18 renderer checks and guards pass. Independent review's shell-name attribution finding is fixed
with executable identity checks and a regression, including resolved multicall aliases. Final
review has no remaining blocker. No renderer or native app behavior changes in this backend step.


**2026-10-19 V7.2.2 typed MCP round trip.** App-owned Linux MCP clients can ask one text question,
edit it in Quick/Nest and receive only a separately confirmed reply. The private transport verifies
kernel peer/process/executable ancestry; external clients are refused. The runtime owns cancellation,
absolute expiry, surface leases and permission priority, with durable content-free audit before
delivery and stdout acknowledgement before success. Once consumed, uncertain replies cannot be
resent. Review fixes cover stale renderer departure/replacement reads, deadline enforcement during
audit, all capture admission and the wire envelope. Native long-prompt QA tightened Quick/Nest scroll
bounds so the editor/actions remain visible. No microphone, external-agent config or provider call
was added. Six real-process/MCP regressions, 1,252 workspace tests (10 documented ignores), 10 evals,
Clippy and cargo-deny pass. Renderer build, all 19 renderer checks and guards pass; final review has
no remaining blocker. Black/Light native Quick/Nest checks include long text, a visible editor and
all actions at 480 x 360. QA app/server and isolated caches are cleaned. Next is V7.2.3, local
dictation into this editor; V7.3 retains opt-in setup.


**2026-10-19 V7.2.3 local question dictation.** The physical dictation key now holds one bounded,
memory-only recording for the selected live Quick/Nest text question. It uses the installed local
model and saved microphone/language without provider clean-up, downloads, commands, clipboard or
history. The result appends to the current manual draft after IME composition; a matching apply
acknowledgement unblocks a separate human Send. Both MCP names (`ask_user`/`vults_dictate`) share the
typed question and content-free audit, and remain restricted to verified app-owned clients.
Cancellation, replacement, lock, lease/expiry, disconnect, settings changes and shortcut rebinding
invalidate capture and late results. One worker bounds native model loads that cannot stop mid-load.
Review fixed cancelled Ready snapshots, shortcut routing during Sending, in-flight ordinary capture
admission and stale binding hints. A matching renderer acknowledgement precedes capture selection.
All 1,256 workspace tests (10 documented ignores), 10 evals, Clippy, build, 18 renderer checks and
guards pass. Native controlled-response QA covers Black/Light Quick/Nest, the 480 x 360 minimum,
four languages and retained manual text after final dictation; no synthetic event opened a microphone.
Real-human microphone/speech acceptance remains separate. Next: V7.3 external-agent opt-in/config.


**2026-10-19 V7.3 reviewed external-agent setup.** A saved default-off Dictation > Agents choice
admits verified external Linux clients to a dedicated question-only MCP server. Consent generations
cancel old external work; serialized runtime/handoff checks prevent revoked access from sending a
reply. The physical dictation key and separate human Send remain mandatory for the voice draft path.
Claude Code/Codex setup, executable-path updates and removal use exact reviewed plans, fingerprints,
dated backups and atomic writes; other servers, hooks and trust entries remain intact. Removal works
with access off. Renderer lifecycle/read generations invalidate stale diffs and status responses.
Independent review has no remaining blocker. Native Black/Light QA covers wide/minimum windows,
scrolled actions, four languages and the real setting RPC in an isolated profile; config changes use
controlled responses rather than actual agent files. The app/server and isolated cache are cleaned.
Full agent sessions and real-human speech remain acceptance gates. See Plan Dictation's V7.3 record;
the next engineering phase is V6, following the recorded V7-before-V6 delivery order.
All 1,263 workspace tests (10 documented ignores), 10 evals, workspace Clippy, renderer build,
18 renderer checks, formatting and repository guards pass. No dependencies changed.


**2026-10-19 V6.1 pure voice turn machine.** `voice::turn` binds callbacks to opaque turn/probe tokens,
locks barge-in from actual first playback, preserves live suppressed onsets and waits for matching
confirmation before cancellation. Production completion requests drain; actual drain starts 450 ms
of desktop tail grace, where speech is a new turn without old-tail replay. Cancellation/failure and
new generations reject old work. Ten deterministic scripted STT/model/output regressions cover the
lifecycle and races; review's stale-probe and delayed-playback onset findings are fixed with tokens
and retained live speech. The module has no callers in the app yet and opens no microphone, provider or network connection.
Runtime ring replay, end-of-turn policy, realtime budgeting, playback and UI remain V6.2-V6.7.
Next: V6.2's end-of-turn policy, with semantic-model licensing/parity gates kept separate.
All 1,273 workspace tests pass (10 documented ignores), plus 10 evals, workspace Clippy,
formatting, renderer build, all 18 renderer checks and repository guards. No dependencies changed.
No renderer/native behavior changes require a new GUI run; the app remains closed.


**2026-10-19 V6.2.1 conversation end policy.** The pure `voice::turn::end` policy waits for VAD
silence and settled text, with 1.6 s base/2.6 s continuation-word pauses in English/Portuguese.
Partial rewrites and duplicate edges do not keep restarting the clock. Optional audio-score requests
are bound to their turn/ticket, paced at 200 ms and limited by an absolute 1.2 s hold; an inference
failure or invalid probability disables semantics for the conversation. New speech, cancellation
and generations revoke old results. Eleven new tests include a scripted detector driving the real
turn machine. Independent review has no remaining blocker after tightening delayed-poll deadlines.
There is no app caller or model/capture/UI/network change. V6.2 remains in progress: V6.2.2 retains
model/weight license checks, checked artifacts, the eight-second window, log-mel reference parity
and optional ONNX inference, followed by V6.3-V6.7 integration. Speech/pt-BR quality remains open.
All 1,284 workspace tests pass (10 documented ignores), plus 10 evals, workspace Clippy,
formatting, renderer build, all 18 renderer checks and repository guards. No dependencies changed.
No renderer/native behavior changes require another GUI run; the app remains closed.


**2026-10-19 V6.2.2 qualified semantic backend.** Optional `semantic-turn` verifies the pinned v3.0
graph and independently qualified Linux x64 CPU runtime, computes eight-second Whisper/Slaney
features and scores through `ort`. Full primary NumPy arrays match within `1e-4`; all four native
probabilities match the reference within `2e-4`. An older-runtime mismatch was diagnosed rather than
loosening tests; the graph, runtime fingerprint, optimization and thread options are now fixed.
BSD-2-Clause code/weight provenance corrects the earlier Apache-2.0 assumption. EN/PT reference and
retained licenses record hashes, reproduction parameters and qualification limits. Shared model-file
admission/runtime/session ownership preserve diarization callers. Review's queue/preprocessing deadline
findings are fixed with cancellation, absolute budgets and regressions; final review has no blocker.
V6.2 backend engineering is delivered; app activation, real speech/pt-BR quality, latency and other
platform/CPU acceptance stay open. Next is V6.3's gated realtime client and cost reservation work.
No app/UI/capture/download/settings/agent-config behavior changes in this step.
All 1,285 workspace tests pass (10 documented ignores), plus 10 evals and 147 tests in the exact
amended offline CI feature combination (8 explicit native/hardware ignores). The checked-artifact
native parity test also passes explicitly. Workspace/preview/optional-feature Clippy, renderer build,
all 18 renderer checks, formatting, repository guards and cargo-deny pass. No new dependency version
is introduced; `rustfft` is an existing dependency made directly available to this optional backend.
The QA environment/artifact copies are removed after archiving evidence; no GUI run is needed for
this backend-only step.


**2026-10-19 V6.3 backend handoff.** V6.3.1-V6.3.5 now provide durable micro-USD holds,
bounded reviewed pricing/protocol, opt-in gated TLS transport, Linux-exclusive orphan recovery and
native complete-turn retention. Failed remote turns can return local Whisper text for review, or
retain the original audio with a typed local failure; they never auto-send through Chat or another
provider. Shared settings mutations revoke old owners before a prior choice can be restored.
All detached accounting/decoder work retains its corresponding ownership until it exits.
The native retention API accepts already captured PCM and starts no microphone, playback or UI.
Live-provider/speech acceptance and visible text continuation remain open. The next engineering
step is V6.4 playback, followed by V6.5-V6.7 integration; the full V6.3 activation gate stays open
until these owners and acceptance checks land. See the current V6.3 sequence in Plan Dictation.


**2026-10-19 V6.4.1 playback foundation.** One bounded CPAL output worker converts 24 kHz PCM16,
keeps accepted/submitted/predicted-consumed frames separate and distinguishes producer end from
device drain. Stop/revocation freezes accounting before clearing queued samples; native teardown
retains exclusivity. Pure heard-text accounting estimates partial words, computes interrupted
context and offers a one-shot hint after at least three unheard words. It performs no history write
or model/tool call. Offline clock/resampling/history regressions and an isolated ALSA null check
cover the primitive; real acoustic timing and provider/speech acceptance remain open. No app caller,
conversation microphone or UI is added. Next: V6.4.2 provider/item/transcript identities, server
truncation and native output/history integration, before V6.5-V6.7. See Plan Dictation's finite
V6.4 sequence; full V6.4 remains open.

Validation also exposed a transient prior gate-lock failure during parallel process tests. The
last intentional gate owner now explicitly unlocks its shared file description; unrelated inherited
descriptors cannot prolong ownership. Every intentional worker/admission still retains the lock.
A duplicated-descriptor regression verifies release and continued exclusion without timing retries.


**2026-10-19 V6.4.2 native playback and history.** The complete-turn service binds gated provider
attempt/response/item identities, finalized transcripts, turn generations and CPAL consumption/drain.
Confirmed interruption freezes once, cancels active production and awaits exact server truncation
acknowledgement after billing settlement. Matching in-memory history retains estimated heard words;
a quoted repair hint is consumed only in the next admitted response. No tool or automatic retry is
enabled. Settled output survives terminal socket failure for drain/interruption; a failed/missing
ack yields unsynchronized local history and ends the owner. Closed consumers/revocation suppress
recovery and detached native output retains the session lease. No microphone, renderer entry or
persistent Chat history write is added. App barge-in stays disabled pending V6.5. Offline duplex
and scripted native checks, plus an isolated CPAL null-driver run, validate the backend contract;
physical/live speech acceptance remains open. Next: V6.5 echo-cancellation spike, then V6.6-V6.7.


**2026-10-19 V6.5.1 native echo-cancellation spike.** Read-only Pulse inventory identifies explicit
PipeWire WebRTC source/reference-sink pairs by module, group and stable node/server metadata.
Discovery is not a route lease or acoustic qualification. An isolated private PipeWire/WirePlumber
experiment with no hardware monitors tests seeded synthetic echo and local tones through the
installed WebRTC engine. Three runs pass fixed synthetic limits; physical speech/double-talk,
latency and actual app routing remain open. Configuration/script isolation and strict module
argument parsing were corrected during independent review. No production module load, dependency,
microphone entry or barge-in activation is added. Next: V6.5.2 pinned duplex streams and explicit
device policy, before V6.6 tools and V6.7 capture/UI. Whole V6.5 remains open. See the finite
V6.5 sequence in Plan Dictation and [Echo cancellation](../reference/echo-cancellation.md).


**2026-10-20 V6.5.2 pinned duplex ownership.** A consent-bound native libpulse worker pins clean
capture and reference output with exact target acknowledgements and `DONT_MOVE`. Subscription
precedes baseline validation; internal AEC connections, module/server/node identities and physical
ports stay checked. Route changes revoke capture/output; default changes never redirect them.
Explicit headphones policy stays separate from route validity; ordinary speakers/CPAL cannot
interrupt. Content spans exclude silent gaps, subtract reported sink latency and separate producer
end, consumed content and acknowledged drain. Stop freezes the last confirmed cursor. Native
leases, callback storage and retired stream references survive teardown. The separate app entry
checks authority before capture and emits bounded memory-only Input events to its native caller;
it performs no automatic upload, turn assembly, persistent history or renderer activation.
Invalid routes clear retained interruption evidence before delayed playback events. Three final
private-graph native runs pass drain, frozen stop, next response, forbidden moves, route/module
revocation, stale selection, explicit policy and lease release checks. No human/device/provider
qualification is inferred. Backend V6.5 is delivered; next is V6.6 voice tools, then V6.7 capture/UI.
Q6 setup and physical/live speech, acoustic latency, hotplug and long-session acceptance remain
open. See [Echo cancellation](../reference/echo-cancellation.md#routed-validation).


**2026-10-20 V6.6.1 local command admission.** Complete native turns can bind a bounded finalized
local transcript. Linux requires that binding before upload and asks the one state owner to reuse
V8's transactional command dispatcher and cached flock reads. Context belongs to this conversation;
commands cannot answer cards, borrow a later enablement or fall through to a model after being
handled/refused/clarified. Settings, cancellation, closed waiters, admission expiry and once-only
consumption are checked before mutation. Local completion retires the generation without Generate;
duplicate captures never upload. External effects report Requested after executor submission,
while completed core-only changes may report Done. Missing transcripts fail before remote admission
and retain the existing review-only local recovery. No renderer, automatic transcription assembly,
provider tool or consultation is activated. Next: V6.6.2 bounded flock tools, then V6.6.3 agent
consultation and V6.7 capture/UI. Whole V6.6 and physical/live acceptance remain open.


**2026-10-20 V6.6.2 realtime flock reads.** The native app supplies the existing five cached
read-only flock tools through frozen, exactly confirmed specs. A serial function-only response
must complete with matching streamed/final identities and valid bounded arguments before its read
can execute. Exact receipts settle first; every continuation needs a separate monetary reservation.
Eight reads share the original chain deadline, and serialized success/error result envelopes include
JSON escaping in their 16 KiB bound. Call/item identities stay consumed between turns. The native
owner distinguishes tool settlement from audio completion, releases original input and remains
Thinking without history/drain/listen. Final audio uses a fresh identity; stale tool attempts are
rejected before playback. Failure cannot replay the consumed input/call or claim an action ran.
Offline duplex/native-owner checks cover these paths without a provider, key or microphone. Review
fixed escaped-envelope bounding and early stale-attempt rejection. No action/permission tool or
renderer activation is added. Next: V6.6.3 consultation/completion, then V6.7 capture/UI. Whole
V6.6 and physical/live acceptance remain open.


**2026-10-20 V6.6.3 native completion receipts.** Native command effects now carry distinct
completed/submitted/failed receipts through the existing runtime executor. Successful project saves
can complete; starting an editor or requesting window activation remains Requested because launch
submission cannot prove visible completion. Failed or missing receipts never invent Done; mixed
read/action batches wait for every receipt before preserving their cached answer. The bounded
receipt wait ends inside the native owner's original deadline. Queued workers recheck consent,
waiter, epoch and expiry and retain the exclusive session lease until exit. Capture clones share
consumption before admission unlocks; timeout, lost acknowledgement, tool settlement and confirmed
interruption cannot return that input to recovery or replay it under another generation. Offline
native-owner regressions and H1 cover this backend slice without desktop actions, microphone,
provider calls or renderer changes. Agent consultation, including its separate consent/budget/trace
and 1.5-second reassurance, remains next within V6.6.3; V6.7 and physical/live acceptance stay open.

Validation: 1,396 workspace tests pass (12 documented ignores), plus 10 feature evals, workspace
Clippy with warnings denied, the renderer build, all 18 existing renderer checks and repository
language/brand/layer/network/version guards. Reviewed EN/PT marks, anchors and local links pass.
Independent review found and resolved mixed-batch receipt loss, competing timeout deadlines,
launcher submission mistaken for completion and interruption clone replay; the final code review
has no confirmed blocker. The app stays closed; no physical/live acceptance is claimed.


**2026-10-20 V6.6.3 Claude text consultation.** The native realtime registry can add
`consult_claude` only after its own default-off Settings consent. The existing text client keeps
built-in/MCP tools unavailable, ignores user/project settings, disables skills, file expansion and
memory loading, and requires Claude Code 2.1.299+ in its current 2.1 series. The short supplied
question is the complete context; no live agent/Chat session is resumed. The separate durable gate
limits this feature to 20 attempts per rolling day with content-free traces, surviving toggles,
restart and history erasure. One worker owns admission and CLI teardown. Its original call time
anchors the 60-second cancellation deadline and translated 1.5-second native reassurance; blocked
preparation and child exit retain the busy slot and exclusive voice lease. Replies remain inert
outside data under the existing 16 KiB envelope and original 120-second realtime chain deadline.
This is native plumbing without visible capture/UI or live-provider acceptance. Organizational
managed policy remains in effect. Codex is deliberately unoffered until equivalent text-only
isolation is qualified; this is the next V6.6.3 task, before V6.7.

Validation: 1,405 workspace tests pass (12 documented ignores), plus 10 feature evals, workspace
Clippy with warnings denied, the renderer build and all 18 existing renderer checks. Independent
review resolved implicit file/memory context, admission/busy ownership, original timing and stale
notice checks; final code review has no remaining blocker. Source/docs clarify enforced managed
policy and do not claim an OS sandbox, visible activation or live compatibility. The app stays
closed; Codex text-only qualification remains the next roadmap item.


**2026-10-20 V6.6.3 Codex candidate qualification.** A reproducible installed-client probe
captures the first Codex CLI 0.163.0 request for `gpt-6.1-sol` in a loopback-only network
namespace, using a synthetic token/question and no model response. The restrictive exec candidate
still includes inline `additional_tools` (`exec` and collaboration) and an ambient skills catalog.
A missing top-level `tools` field does not establish a tool-free request. The probe logs only
capability names and a skills-presence boolean, never captured instructions or credentials, and
reaps the child before cleaning its private directory. A normal offline regression checks both
registry representations. No production driver, Settings choice or voice offering changes.
This candidate is rejected; V6.6.3 remains open for supported tool/context isolation and its
consent/cancellation integration, before V6.7. This does not establish that every Codex route is
unsuitable and does not claim live-provider, microphone or desktop acceptance.


**2026-10-20 V6.6.3 Codex tool/context isolation helper.** Local request capture explains the
rejected candidate: packaged model metadata independently enables code mode, search and
collaboration, while hiding a skills catalog does not disable explicit skill expansion. A new
CLI-only helper pins qualified client 0.163.0 and supplies minimal fixed model metadata without
reading mutable model-cache instructions. It disables offered capabilities and injected context,
quotes the question as JSON with escaped dollar/at-sign markers, retains original CLI authentication
and accepts only a complete bounded text reply. It shares the acknowledged-child cancellation
runner with Claude. An isolated installed-client probe now checks the exact helper, adversarial
project/file/explicit-skill canaries and a fixed synthetic model response; both earlier candidate
paths stay as negative controls. There is no paid API adapter, app offering, renderer activation or
live provider use. The helper removes inherited API-key/base-URL/session environment overrides and requires a
ChatGPT-only status preflight. A separate synthetic-auth filesystem/network test proves that
preflight refuses API mode without changing stored auth, while retaining original HOME/CODEX_HOME
values. It does not yet prove a stable binding against concurrent external account changes. Do not force a login method: this client can
log out an existing mismatched account. Next is that binding plus independent native Codex
consent/budget/trace and ownership integration. V6.6.3 remains open before V6.7.


**2026-10-20 V6.6.3 native Codex consultation backend.** Independent default-off consent and a
Settings-only command now feed the existing native registry; Claude consent never enables Codex.
Each provider has its own durable 20-attempt rolling quota and metadata-only trace. Both share one
busy slot, original deadline, translated notice and retained voice ownership through child exit.
Upgrade, quota isolation/history erasure and cancellation ownership have offline regressions.
There is no renderer/capture entry or live-provider acceptance. Subscription eligibility remains a
nonmutating preflight, not atomic binding across an external login switch. A synthetic-auth review
probe confirms that forcing ChatGPT mode on exec deletes mismatched API auth, so that workaround is
prohibited. V6.6.3 remains open on subscription binding and managed/live-client qualification before
V6.7 activation.

Validation: 1,410 workspace tests pass (16 documented ignores), 10 feature evals, and all six
explicit installed-Codex checks pass inside the isolated namespace. The renderer build, all
18 existing renderer checks, workspace Clippy with warnings denied, formatting and repository
guards pass. Independent code review found no backend blocker; subscription-switch activation
remains explicitly outside this backend sign-off. No dependency or renderer changes.


**2026-10-20 V6.6.3 enforce the Codex auth activation gate.** The prior backend wiring had an open
qualification gate in documentation only. A shared false capability now rejects opt-in before
settings/ledger access, excludes Codex from native offering despite a saved true choice, closes its
network gate and refuses the public text helper before any process or scratch access. Independent
consent/quota/ownership wiring and local qualification commands are retained. Existing Claude
consultation is unaffected; turning a saved Codex choice off remains allowed. The isolated auth
regression now reproduces destructive forced-exec mismatch using only disposable synthetic auth,
while the status check remains nonmutating. Original home/auth/keyring stay hidden; no external
provider, microphone or app launch. Neither ephemeral credential storage nor URL overrides is a
documented binding of the existing subscription login, so neither is introduced as a workaround.
This closes an activation bypass, not the subscription-binding issue: V6.6.3 stays open before V6.7.

Validation: 1,411 workspace tests pass (16 documented ignores), 10 feature evals and seven explicit
Codex checks pass, including the disposable forced-exec mismatch characterization. Build, all
18 renderer checks, formatting, workspace Clippy with warnings denied and repository guards pass.
The independent review found and resolved status-stdin race/child cleanup in the test harness;
production guards have no confirmed blocker. Translation source marks, English anchors and
173 local documentation links/anchors are verified. The app remains closed.


**2026-10-20 V6.6.3 managed subscription-policy candidate.** Unlike forced login on exec, local
system allowed_login_methods requirements apply before credential loading. Expanded the installed
client's isolated auth regression to nine named controls/scenarios, with synthetic home and empty
or policy-bearing system mounts. Without policy the local stand-in sees the synthetic API bearer;
with ChatGPT-only policy, stored API auth is not transmitted or deleted. A synthetic ChatGPT status
succeeds; replacing stored auth before a fresh exec or after its first authenticated request gets
401 never sends the replacement API key. The latter observes a retry using the original synthetic
ChatGPT bearer, before a fixed 400 terminates the request. No raw auth/header logging, model reply,
external provider or original config/auth access. Capture/child lifetimes and resources are bounded.
This is an evidence-backed candidate for already admin-protected installs, not specific-account
binding or production sign-off. Runtime effective-policy and protected-provenance checks, keyring,
broader refresh/failure cases and live compatibility remain open. No system policy is installed,
no user account/config is changed, and the activation capability remains false. V6.6.3 stays open.

Validation: all seven explicit Codex checks pass, including nine auth scenarios; 1,411 workspace
tests pass (16 documented ignores), 10 feature evals, build and all 18 renderer checks pass.
Workspace Clippy with warnings denied, formatting and repository guards pass. Independent review
resolved inherited host policy in controls and auth-content assertion logging. The review also
clarifies that a GET/POST retry after 401 may be handshake/fallback, not credential reload; that
qualification remains open. The PT source mark, English heading anchors and 106 local documentation
links/anchors are checked. The app stays closed and the production activation guard stays false.


**2026-10-20 consolidated publication checkpoint before V6.7.** Full-branch independent review
found three actionable gaps: the production microphone setter bypassed realtime invalidation,
question-race checks were absent from CI and the changelog contradicted delivered dictation/setup.
The setter now uses the shared change owner under Settings lock after persistence; a child-only
private-config regression covers ABA restoration, unchanged device and failed persistence. CI
runs all 18 renderer checks; Unreleased describes delivered question flows and backend-only voice.
Nineteen isolated Codex auth/policy scenarios include effective requirements/account/config reads,
keyring-required/noDBus refusal and true POST401 managed/unmanaged controls. Both HTTP controls
terminate after one request without the replacement API bearer; this is refusal, not auth reload.
The current host has no protected system policy. No original auth/keyring/system config, real
provider or microphone is touched. Codex's four activation guards stay closed. At the user's request,
publish this guarded baseline and proceed to V6.7 engineering; Codex activation, successful keyring,
credential renewal and physical/live acceptance remain explicit open gates, not completed work.

Validation: 1,412 workspace tests pass (16 documented ignores), 10 feature evals and all seven
explicit Codex checks pass with 19 auth/policy scenarios. The renderer build, all 18 renderer
checks, workspace/optional-feature Clippy and tests, formatting, repository guards, release-hook
build and cold-start ceiling pass. Cargo deny passes advisories, bans, licenses and sources.
Nine changed public EN/PT source marks are current; the latest checkpoint's headings and 108
local documentation links are checked. Independent review confirms all three findings resolved
and permits publication with Codex disabled; no successful credential renewal or live UI/audio
acceptance is claimed. Local Rust is 1.99.1; PR CI also validates the pinned 1.98 toolchain.


**2026-10-20 V6.7.1 realtime Settings bridge.** The Settings-only read command returns the saved
switch, model and exact microdollar limit without device/keyring/store/network IO or a private
ledger identity. The typed bridge exposes this read and the existing Settings write, refusing
fractional, negative and unsafe JavaScript amounts before invocation. Native authority and
validation remain decisive. This adds no capture or visual control; the next finite task is
V6.7.2 configuration UI, followed by capture/controls and island/native acceptance. Codex activation
remains unavailable and K8 stays open. See Plan Dictation
[V6.7 delivery sequence](plan-dictation.md#v67-delivery-sequence).

Validation: 1,413 workspace tests pass (16 documented ignores), 10 feature evals, workspace Clippy
with warnings denied, formatting, renderer build, all 19 bridge/renderer checks and repository
guards pass. Native readback covers default-off, exact large limits, private-identity omission and
unchanged independent consent; Nest/island callers are refused. The new bridge check runs in CI
and covers explicit invocation and unsafe-number refusal without capture or provider access.
The local AGENTS instruction also names `check-task-actions.mjs`, which is absent from this
checkout; that unavailable check is not reported as passed. No visual renderer changes or app
launch are part of this contract; native UI validation begins with V6.7.2.


**2026-10-20 V6.7.2 voice configuration page.** Dictation > Talk with Zeca now uses the existing
Settings webview and shared navigation/components/theme. It shows default-off OpenAI audio/context
consent, the reviewed model, an independent rolling USD budget and a link to shared key settings.
The form preserves drafts across navigation and failed saves, confirms writes by readback and
blocks stale edits after a foreign change until explicit reload. Both RPC directions use exact
integer decimal strings; USD accepts a point or comma without floating-point arithmetic. Native
validation rejects noncanonical/out-of-range amounts before IO. Voice never answers permission
cards and Codex remains unavailable. Saving opens no microphone or provider connection.

Validation: 1,413 workspace tests pass (16 documented ignores), 10 evals, build, all 19 renderer/bridge
checks, workspace Clippy with warnings denied and formatting pass. The expanded production bridge/
component regression covers stale reads, one save, failure/retry, exact large USD amounts, drafts,
conflicts, acknowledgement readback, translations and explicit key navigation. Isolated native
Tauri/WebKit validation exercised actual configuration RPCs, failure under a read-only config
folder, focus/caret retention and exact limits. Visually reviewed Black/Light at 1180 x 820 and
480 x 360 has no horizontal overflow and keeps Save reachable; all four languages render and
translated narrow actions remain reachable. A private XDG profile, masked original HOME path and
private DBus without service activation keep user configs/keyring apart. No microphone, model,
provider or agent configuration is used. Shared Settings styles fix field sizing and action spacing.
The local AGENTS task-actions script remains absent, not passed. Next is V6.7.3 explicit capture,
turn assembly and conversation controls; live/physical voice acceptance and K8 completion stay open.


**2026-10-20 V6.7.3 native turn assembly.** Completed mono 16 kHz capture and its finalized local
transcript can now become a bounded 24 kHz PCM16 realtime turn through the existing shared
resampler. The adapter preserves duration/pitch/level and the untouched source, validates
100 ms-30 seconds and 4,096-byte local words, rejects non-finite samples and saturates filter peaks.
Recovery returns the same finalized local text without another decode/model load, while consumed
captures, cancellation and revoked authority remain refused. Untranscribed turns retain the
existing guarded local decoder path. No microphone, model, provider, renderer command or new
permission authority is opened by this adapter. The finite next step remains the explicitly
started capture owner, followed by work-surface controls; V6.7.3 and live/physical acceptance stay open.

Validation: 1,416 workspace tests pass (16 documented ignores), 10 feature evals, renderer build,
all 19 renderer/bridge checks, workspace Clippy with warnings denied, formatting and repository
guards pass. Nine local recovery/assembly tests include arbitrary/boundary capture lengths,
1 kHz waveform phase/level, unchanged source/text, non-finite input, invalid/oversized UTF-8 text,
signed PCM16 saturation and recovery without decoding for key/budget/transport/timeout failures.
Cancelled, revoked and consumed turns are refused. The final portable sample-count assertion was
rerun with targeted tests and Clippy. The local task-actions script remains absent, not passed.
Reviewed EN/PT source marks match. No renderer changed; the app stays closed and no physical
microphone/model/provider or live conversation acceptance is claimed.


**2026-10-20 V6.7.3 explicit native capture owner.** The Linux native entry now starts one
recording under the existing desktop lease, using the shared microphone preference, installed local
model and ordinary-capture question fence. Explicit Finish joins the microphone before local decode;
Cancel or losing the controller revokes the whole session, including response/recovery after Finish.
A 30-second cap discards unfinished input; the total capture/decode wait is 60 seconds. Delayed
native workers retain the same lease until actual exit and suppress late output. Prepared turns use
the existing current generation, local-command gate, provider admission and exact local-text recovery.
No renderer entry, automatic capture, model download, barge-in or permission authority is added.

Validation: 1,422 workspace tests pass (16 documented ignores), 10 feature evals, renderer build,
all 19 renderer/bridge checks, workspace Clippy with warnings denied, formatting and repository
guards pass. Native tests cover explicit finish, consumer/controller loss, sticky cancellation,
late Finish after the cap, revoked keys, blocked worker abort/timeout and lease retention. The
prepared-turn driver test covers upload of the original PCM, recovery and a local command that never
constructs a provider. Tests use fake devices/decoders and synthetic PCM; no physical microphone,
model, provider or user configuration is used. The local task-actions script remains absent, not
passed. Next is the native bridge and listening/finish/cancel controls with reviewable recovery in
the existing work surface. V6.7.3, live/physical acceptance and K8 completion remain open. The app
stays closed.


**2026-10-20 V6.7.3 Nest manual controls and recovery.** Dictation > Work > Voice conversation
now owns one explicit Start/Finish/Cancel turn. Configuration remains in the privileged Settings
webview; three Nest-only commands expose status and bounded heard/local reply or review text,
never PCM, raw failures, keys or settings writes. A native identity rejects stale controls/results.
The page sends 500 ms heartbeats; a three-second loss drops its sole capture controller. Departure,
hidden/closed Nest, lock, shutdown and relevant key/settings changes cancel the same native owner.
Existing teardown/settlement retains delayed native work; cancellation cannot undo a sent request.
One acknowledged Start replaces the previous draft; recovered text stays editable, unsent and copied
only on a click. Navigation preserves local reviewed edits. There is no shortcut, automatic Chat
send, permission answer, continuous capture, automatic turn-end or duplex/barge-in in this entry.

Validation: 1,424 workspace tests pass (16 documented ignores), 10 evals, build, all 19 renderer/bridge
checks, workspace Clippy with warnings denied, formatting and repository guards pass. Native owner
tests cover foreign surfaces, stale identities, departure and heartbeat expiry. Production renderer
regressions cover duplicate Start, late acknowledgements/status, explicit Finish/Cancel, failed
bootstrap/status, inert recovered text, editable/copy behavior and retained drafts. Isolated native
Tauri/WebKit checks refuse actual island/Settings commands and disabled-consent Start before any
microphone. Controlled owners exercise the production work page, recovery/copy and focus/caret;
Black/Light at 1180 x 820 and 480 x 360 keep controls visible without horizontal overflow. The
short window uses a separate named transcript/recovery scroller so actions cannot cover the editor;
all four languages render. No microphone/model/provider, original keyring or agent config is used.
The private profile and app/server are stopped and removed after QA. The local task-actions script
remains absent, not passed. V6.7.3 still needs continuous-turn integration; next connect the existing
native end-of-turn/duplex adapters, followed by V6.7.4 island feedback and physical/live acceptance.
K8 and Codex consultation qualification remain open.


**2026-10-20 long-meeting capture prioritized.** The maintainer rejected the one-minute recording
check as insufficient for real meetings and authorized prioritizing longer capture. Manual Meetings
now reports/adopts the shared four-hour native cap. Independent source packets are downmixed and
statefully resampled before disk writes to mono 16 kHz PCM16, with saturating conversion and peak
metadata matching stored samples. Both tracks together stay below 1 GiB at the cap; bounded queues
and resampler windows do not retain the full recording in memory. The existing local decoder
processes finalized tracks in at most 28-second chunks after Stop, with dictation priority.
Cancellation, reviewed sources/model/language, the page lease and private-file cleanup are retained.
There is no live transcription, audio archive or crash resume. Startup still deletes leftover audio.
The 256 KiB summary input bound remains separate; saving/export still accepts longer transcripts.

Hour-aware clocks cover recording/progress/segments. UI copy discloses four hours, temporary storage
and the experimental 30-minute speaker limit. Unavailable identification now shows the promised
plain transcript without misleading Unknown labels. In the minimum window, pre-transcript tabs yield
space to the named source/consent scroller; completed transcripts retain their tabs and actions.

Validation: 1,429 workspace tests pass (17 documented ignores), 10 evals, build, all 19 renderer/
bridge checks, workspace Clippy with warnings denied, formatting and repository guards pass. Unit
checks cover 75-second streamed PCM, exact clocks at 8/44.1/192 kHz, clipping, failed writes/cleanup,
the four-hour owner beyond a minute and matching app limits. An explicit isolated native PipeWire/
Pulse test recorded 65 seconds from synthetic sources only, finalized both mono PCM16 readers and
removed both WAVs; 1,041,280/1,040,960 frames retained their independent source clocks. It used no
physical microphone/output, model, provider or user configuration. Native Tauri/WebKit inspection
uses controlled owners at 1180 x 820 and 480 x 360, Black/Light, ready/recording/transcribing states,
source-consent focus, hour clocks/progress and all four languages. The minimum keeps at least
48 pixels for source/consent review and actions outside it. Private app/audio/server processes and
profiles/caches are removed afterward; the user's app remains closed. The local task-actions
script remains absent, not passed. Four-hour physical soak, speech-quality acceptance, background/
crash-resume UX and long-input hierarchical summaries are not claimed complete. V6.7 continuous-turn
integration and K8 remain open after this user-prioritized meeting adjustment.


**2026-10-20 long-meeting maintainer acceptance and manual voice continuation.** The maintainer
accepted the delivered long-recording scope and authorized the next task. Four-hour physical soak,
crash resume and speech-quality qualification remain separate; this is not K8 completion.

V6.7.3 now retains the same provider driver and remote context between explicitly recorded turns.
After heard playback drains or a local command completes, the Nest shows Microphone off and offers
Record another turn or End conversation. The next click rearms only the sole live controller and
uses the next native Listen generation. Idle does not open a microphone. Duplicate/stale requests,
loss of page contact, departure, lock, changed settings/key and provider failure cannot start a
following capture. Capture/transcription remains under the desktop lease while the provider driver
continues polling; local routing and exact-text recovery use the existing path. The UI keeps only
the most recent reply/recovery in memory. Automatic turn-end, duplex/barge-in, island feedback and
physical/live K8 sign-off remain open. Codex consultation is unchanged and unavailable.

Validation: 1,431 workspace tests pass (17 documented ignores), 10 evals, all 19 renderer/bridge
checks, build, workspace Clippy with warnings denied, formatting and repository guards pass. A
retained-driver regression uploads two native-assembled turns through one fake provider and admits
no next capture during idle. Controls checks reject early/duplicate or disconnected continuation;
controller cancellation stays irreversible. Renderer checks keep the same session ID, heard reply,
heartbeat while waiting and explicit end. Native Tauri/WebKit checks use controlled owners at
1180 x 820 and 480 x 360 in Black/Light and all four languages; actions stay visible and the named
content scroller remains accessible. Actual commands reject a foreign webview and default-off
consent before device/network access. No physical audio, model, provider, original keyring or agent
config is used. The private app/server/profile/cache are removed afterward. The local AGENTS
check-task-actions.mjs script remains absent, not passed. Live multi-turn speech/provider acceptance
and automatic turn-end/duplex integration remain open.


**2026-10-20 V6.7.3 opt-in local silence completion.** The Voice conversation page now offers
Finish automatically after a pause. It starts off in each app run and is frozen for the live
conversation; no saved consent, automatic microphone reopening or shortcut changes are added.
The same native capture owner polls the retained local VAD and GPU preview against V6.2's existing
1.6/2.6-second English/Portuguese policy. Fresh audio, confirmed speech and local partial words are
required. Partial text stays native and bounded to one 4,096-character snapshot. Final decoding
and provider upload still follow microphone closure. Missing/failed detector or preview falls back
to manual Finish with an explicit status. Cancel, page ownership, settings/key revocation, the
30-second discard cap and the desktop lease remain in force. Preview callbacks retain the lease
until their actual worker exits. No semantic model is enabled or downloaded.

The first capture now also follows an actual Listen generation from the retained turn machine;
subsequent captures keep the same provider context and explicit next-turn click. Automatic
completion marks the controller finished before waiting, so the next user click can rearm it.
Semantic-model integration, explicit duplex/barge-in, island feedback and physical/live acceptance
remain open; K8 is not complete. Codex consultation remains unavailable.

Validation: 1,435 workspace tests pass (17 documented ignores), 10 evals, all 19 renderer/bridge
checks, build, workspace Clippy with warnings denied, formatting and repository guards pass. The
actual policy adapter covers quiet without speech, missing words, once-only completion, both
language delays, partial growth and resumed speech. Capture-owner checks prove automatic completion
closes before final decoding and never beats cancellation or expiry. A blocked-detector regression
proves Finish/departure need not wait for inference, its late result is suppressed and its real
worker retains the lease until exit. Detector inference runs outside the capture-stop worker;
old audio windows behind a busy decoder cannot end newly captured speech. The retained-driver
regression now captures both initial and following Listen generations with one fake provider.

Native Tauri/WebKit inspection uses controlled owners, default-off/frozen selection, explicit
following turns, the visible manual fallback, actual foreign-surface/default-off refusal, four
languages and wide/minimum Black/Light layouts. Actions remain visible and the checkbox is reachable
through the named content scroller. No physical audio, model, provider, original keyring or agent
configuration is used. Private processes/profile/cache are removed after QA. The local AGENTS
task-actions script remains absent, not passed. Physical PT/EN speech timing and live-provider
quality remain unqualified; semantic/duplex integration and K8 sign-off stay open.


**2026-10-20 V6.7.3 explicit response interruption.** Once native playback actually starts, the
Nest replaces Finish recording with Stop response. The explicit click bypasses voice-detection
locks without granting microphone authority or replay. It uses the retained driver's existing
played-frame freeze, response cancellation, acknowledged truncation and heard-prefix/next-turn
repair. The same provider/context survives; the page waits for interrupted heard text before
allowing another explicit capture. Producer completion does not prevent stopping buffered audio.
No audio route is opened or inferred. Automatic voice barge-in still requires the separate explicit
headphones/AEC adapter; semantic integration, island feedback and physical/live K8 sign-off remain
open. A response that finishes before the click is harmless; duplicate clicks cannot enqueue another
stop or a recording. Cancel remains available before audio and while interruption settles.

Validation: 1,436 workspace tests pass (17 documented ignores), 10 evals, all 19 renderer/bridge
checks, build, workspace Clippy with warnings denied, formatting and repository guards pass. Pure
turn checks cover explicit stop during the voice lock with barge-in disabled/headphones/AEC, no
microphone replay, generation replacement and stale/duplicate or inaudible requests. The retained
playback regression now also exercises the explicit stop before/after producer completion, exact
played-frame freeze, held truncation acknowledgement, heard-only history and repair of only the
next admitted turn. Each displayed audible reply gets an opaque binding checked by the native
controller; a late request cannot target the next reply. Renderer checks cover the same session,
matching reply binding, disabled duplicate stop, interrupted feedback and no new capture.

Native Tauri/WebKit inspection uses controlled owners at wide/minimum sizes in Black/Light and
four languages for speaking, interrupting and waiting. It retains visible named actions and an
accessible content scroller, exercises explicit stop without a new session/microphone and rejects
actual foreign-surface/default-off entry. No physical audio, model, provider, original keyring or
agent configuration is used. Private processes/profile/cache are removed afterward. The local
AGENTS task-actions script remains absent, not passed. Physical/live interruption quality and
explicit duplex/voice barge-in integration remain open; K8 and Codex activation are unchanged.


**2026-10-20 V6.7.3 native routed interruption detector.** The explicit routed service now consumes
verified headphone/AEC packets through a bounded local detector. Model/VAD availability is checked
before streams open. Only actual playback enables observation. Fresh three-second VAD snapshots
feed the retained turn machine; its 500/1,000 ms locks and exact probe tokens remain authoritative.
Local scratch transcription can confirm meaningful words within a shorter 1,400 ms worker budget;
core still enforces its original 1,500 ms deadline. Probe text is not published, executed, uploaded
or used to answer a card. Missing/failed/empty, expired, revoked or replaced evidence cannot confirm.

An eight-second ring retains the onset/pre-roll. On confirmation, native Replay follows the new
Listen and carries that exact generation. A native caller still owns complete-turn assembly and
explicit commit; no new Turn is created by the detector. One off-loop worker, one admitted analysis
and bounded channels keep playback/Stop/cancellation responsive. Actual inference/model destruction
retains the desktop lease. Weak route checks do not keep native streams open during teardown.
Original input closure still finishes the driver normally; closed analysis owners fail closed.

The ordinary Nest UI remains manual with Stop response; no continuous capture, route selection,
module setup or automatic follow-up is enabled there. Next is routed turn assembly and explicit
route/consent UI. Semantic-model integration, physical/live qualification and V6.7.4 island feedback
remain open; K8 and Codex activation are unchanged.

Validation: 1,442 workspace tests pass (17 documented ignores), 10 evals, build, all 19 renderer/
bridge checks, workspace Clippy with warnings denied, formatting and repository guards pass. Six
new deterministic tests exercise the actual native adapter with controlled analyzers: bounded
retention and playback-only admission; a short utterance ending during its pending probe; matching
confirmation and new-generation replay; cancellation and real worker-held lease release; route/
authority revocation, replaced generations and expired probes; empty words/decode failure; closed
analysis/input owners; and the actual async pump's action-before-Replay ordering with no generated
or uploaded Turn. The tests use the retained pure turn machine and real bounded worker/channel
owners, without physical audio, model weights, providers, OS keyring or user configuration.

No renderer changes or production activation are included, so no new GUI run is required. The app
remains closed. Existing manual/Stop-response renderer checks pass unchanged. The local AGENTS
task-actions script remains absent, not passed. Real PT/EN speech/double-talk, acoustic timing,
routed turn assembly, selection/consent UI and live-provider acceptance remain open.
