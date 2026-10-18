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
and related ids in [Dictation (N3)](#dictation-n3). The maintainer accepted the V0.9 prototype
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
release-to-pasted p95 target below two seconds remain unmeasured. Next in the recorded implementation
order is V8.1, the pure local voice-command normalizer and phrase tables; V3 Styles remains later.
No V1 native matrix sign-off or N3 completion is inferred; real-speech/desktop acceptance remains separate.

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
| UX11 | Debrief, risk/batches/questions, routines, proposals, moods/voice/ACP and observed-cost views; voice lands with V6/V7 | Alongside K1-K8/K10, M7, W2/W9b and the original Flock board |
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
| K8 | Voice on the same brain (V6): an opt-in realtime model (replaces C9), turn-taking and barge-in, bounded opt-in follow-up listening after Zeca asks (P9); V8's local commands tried before the model; never answers a card | Z6, G9, V1, V8 | A spoken question uses `flock_list` |
| K10 | Zeca as an ACP agent: Zed, JetBrains and Neovim talk to him, with memory and the flock | M7 | A Zeca session in Zed answers about the flock |

### Dictation (N3)

Integrated on 2026-10-17 (V0.5). The detail and substeps stay in
[plan-dictation.md](plan-dictation.md); ids are `V<phase>.<step>`. Delivery order is V0, V1,
V2, V4, V8, V3, V5, V7, V6. Recording a contract is not completion of its runtime or native
acceptance. V0.1-V0.5 are recorded and V0.6-V0.9/P12 were accepted in the prototype on
2026-10-17. The maintainer accepted V1.1 on that date; V1.2 is implemented with automated
regression checks. V1.3 adds the shared transcript gate and V1.4 adds the pure key logic, both
with automated evidence. V1.5/V1.6 implement clipboard-only dictation.
V1.7-V1.18 add guarded native delivery, island feedback, History, Settings and local text/music
controls. V1.18 has maintainer validation; V1.19 records the docs and regression checkpoint
with unreported cells of the full desktop matrix still open. V2.1-V2.5 implement Your words
storage, final-text rules, restricted opt-in Whisper/project context, work pages and sentence
preview. V2.6 records the documentation/regression checkpoint and remaining acceptance; synthetic
WER and isolated native UI evidence do not establish real microphone/desktop acceptance.
V4.1-V4.3 implement stage timings, Activity statistics and Settings diagnostics. V4.4 records the
documentation checkpoint and remaining acceptance: real input/grants and the Small/Vulkan p95 target
are still open. The next implementation step is V8.1, in the recorded order above. Each backend
and production UI step follows its own prerequisites and running-app criteria. These rows belong to N3, including
K8's voice work; the other K rows remain N4. Tasks stays removed.

| # | Step | Needs | Done when |
|---|---|---|---|
| V0 | Decisions in ADRs 0028, 0032, 0021 and 0019; integrated prototype in four languages | G9 | Decisions recorded and P12 prototype visually accepted on 2026-10-17; runtime/native acceptance remains with each phase |
| V1 | Dictation into apps: shared audio capture, key machine, focus at key-down, clipboard lease, XTest/portal insertion, guarded session delivery, island, History and Settings; works with Zeca off | V0; each backend substep's own prerequisites | X11 and Plasma Wayland smoke passes with Zeca on and off, including a waiting session and honest delivery outcomes |
| V2 | Your words (X5 reopened locally): dictionary, replacements, snippets, whisper/project vocabulary and Try a sentence | V1 | Measured WER improves with the dictionary on in the fixture set |
| V3 | Styles: clean-up as a text filter, Prompt, translation and per-app choice, through the selected API or local provider and the gate | V1, Z6 | Injection and question evals pass; timeout or invalid output uses the raw text |
| V4 | Statistics and Diagnostics; stage timings p50/p95 | V1 | Release-to-pasted p95 is shown and below 2 s with Small on Vulkan |
| V5 | Meetings: mic/system audio, temporary WAV cleanup, licensed diarization through `ort`, opt-in summary, MD/SRT/VTT export | V1, V0's meeting decision | A 30-minute call yields speakers and a transcript, plus a summary when enabled, with no audio left on disk |
| V6 | K8's voice delivery: turn machine, end of turn, opt-in realtime client with spend accounting, playback/heard prefix, AEC and shared flock tools | Z6, G9, V1, V8 | A spoken question uses `flock_list`; budget and interruption checks pass; K8 has one implementation through V6 |
| V7 | `vults_dictate`: a question card filled by voice and sent with explicit confirmation | V1, M7's `ask_user` follow-up, V7.1's ADR alignment | An agent's question is answered by voice with human confirmation and an audit row; never a permission |
| V8 | Local voice commands (takes C11): own key, parser/name resolver, non-answer intents and flock reads, 90 s context and undo | V1, V2 | Open api, read who waits, mute web and docs, then undo, without a model; H1 covers the command path |

M7's existing MCP server is not the question-card follow-up required by V7. That interface and
V7.1's explicit ADR alignment must land first; these rows do not relax the current prohibition
on answering cards by voice. Z6 and M7 retain their existing Done entries and outstanding
follow-ups; this integration does not claim those follow-ups are implemented.

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

**Dropped (2026-10-12):** X1 side panel, X2 a bird on the desktop, X3 a voice per vult (reconsider
after V6), X4 Vercel, cloud transcription, X8 perch together; W10's Gemini CLI check (Gemini CLI
retired, #310). X5's voice dictionary was dropped then and reopened as local-only V2 on
2026-10-17; its cloud transcription remains dropped.

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
