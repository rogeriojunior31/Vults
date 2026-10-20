# Architecture

Agent tracking flows one way, through one runtime loop:

```
agent ──hook JSON──▶ vults-hook ──protocol line──▶ ipc ──┐
GitHub (gh) ─────────────▶ connectors runtime ─────────────────┤
island clicks (Allow, open, jump…) ────────────────────────────┤
                                                                ▼
                                        core::reduce(state, input, now) ──▶ effects
                                                                │            (answer the hook,
                                                    State::view ▼             open a URL, jump)
                                                         island renders
```

## Layers

The crates form three layers over a small base. A crate depends only on its own layer or a lower
one; `scripts/check-layers.sh` checks it in CI (tests may reach further).

| Layer | Crates | What it owns |
|---|---|---|
| **Experience** | `app`, `platform`, `ui/` | Every surface: the island, the widget, settings, the tray, Zeca and the flock |
| **Connect** | `connectors`, `chat`, `audio`, `voice`, `dictation`, `media`, `zeca`, `link`, `relay`, `evals`, `spike` | What reaches past the agents: GitHub, the chat CLIs and APIs, the microphone, what is playing, a paired phone |
| **Core** | `core`, `protocol`, `peer`, `ipc`, `hook`, `agents`, `agent-config`, `store` | Sessions, events, approvals and decisions: what the agents are doing and what the human said |
| base | `brand`, `secrets` | The name, the keyring |

Only `app` wires the layers together: Connect never calls into Experience, and Core knows neither.
That keeps a new surface cheap (it draws `State::view` and sends intents, [ADR 0021](adr/0021-four-sizes-one-nest.md))
and keeps Zeca optional ([ADR 0018](adr/0018-zeca-as-an-agent.md)): he uses Chat, Voice and
Connect, and none of them knows about him. His body is in Experience; his brain is the `zeca`
crate in Connect, which reads the flock but never answers a permission
([ADR 0018](adr/0018-zeca-as-an-agent.md)).

| Crate | Role | Must not use |
|---|---|---|
| `brand` | The app's name, slug and bundle id; generates `ui/src/brand.ts` | anything |
| `protocol` | Versioned hook ↔ app messages, limits, socket and pipe names | tokio, Tauri |
| `peer` | Same-user checks on both ends of the connection | tokio, Tauri |
| `hook` | Reads the agent's hook JSON, forwards it, prints the agent's decision format | tokio, HTTP |
| `ipc` | Async server: connection limits, ack-then-decide, routing to the app | Tauri |
| `core` | Pure domain: sessions, approvals (each card an offer in the ledger: bound to its exact request, answered once, only in time), action classes, the audit lines, alerts, the view, whose TypeScript types it generates into `ui/src/view.gen.ts` (a test checks it is fresh); clock injected | IO, async, Tauri |
| `agents` | The registry of every agent (id, name, wire, setup, how it runs, what it can do; `ui/src/agents.gen.ts` is its twin) and, per agent (Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI, Cursor, Factory Droid, Kimi Code, any other): event names, tool → activity, install entries, Codex trust; the `AgentDriver` trait for an agent the app runs itself (no runtime) | Tauri, tokio |
| `agent-config` | Safe edits of agent configs (JSON, TOML with its comments kept, whole plugin files): strict read, diff, fingerprint, backup, atomic write | Tauri |
| `store` | The local database: one `vults.sqlite` (bundled SQLite), numbered migrations, the history of turns, the append-only audit log, the consents (never deleted), traces, Zeca's conversations (with what each read from outside and its frozen context), the paired phones and the highest counter each signed with, Zeca's journal and daily summaries with their full-text and vector recall, the memory proposals waiting for a yes, his nights, what the agents cost by day and roost, and text-only dictation history with independent daily/app totals (migration 21), recorded under the chosen retention, plus user-maintained dictionary/replacements/snippets (migration 22) independent of history retention | Tauri, async, the network |
| `chat` | Chat through the `claude` and `codex` CLIs, with permission requests, or an API: Anthropic or an OpenAI-compatible cloud with the user's key, or a local Ollama / LM Studio | Tauri |
| `zeca` | Zeca's brain (ADR 0018): a read-only view of the flock, his memory as a git repository of small Markdown files of facts, each with a hard limit (each write a commit) | `core` (so nothing in it can build an `Intent`), Tauri |
| `link` | A paired phone's link (ADR 0004): pairing by QR, envelopes sealed with X25519 and XChaCha20-Poly1305 (a relay sees only ciphertext), answers signed by the phone's Keystore keys and checked before `core`'s ledger redeems them | IO, the network, Tauri |
| `relay` | The phone's relay (ADR 0004): an axum server that stores and forwards sealed envelopes by box, capped and dropped after 10 minutes, and the client both ends use, each request through the app's network gate; it depends only on `link` | Tauri, `core` |
| `evals` | Evals with cassettes (dev only, never in the app): policy classes, redaction, the prompts sent to models, recorded streams replayed for injection | being linked into the app |
| `spike` | The spike kit (dev only, never in the app): a fake model, a fake app on the hook's socket and a terminal for TUIs, to record a real agent's hooks without an account; `scripts/spike.sh` runs it per agent | being linked into the app |
| `secrets` | The OS keyring (Secret Service, Credential Manager), keyed by the bundle id | Tauri, files |
| `connectors` | Vults Connect: the `Connector` trait, the polling runtime, GitHub | Tauri, core |
| `audio` | Microphone capture on its own thread (cpal), named device/default fallback with monitors skipped, bounded 16 kHz mono in memory (rubato), scalar level and nine speech bands (rustfft); optional Linux meeting capture owns private temporary WAVs; no transcription | Tauri, core, transcription, the network |
| `voice` | Chat push-to-talk over `audio`, a one-minute recording cap, Silero end-of-speech and whisper.cpp/live preview on this computer, model downloads checked by SHA-256; bounded local meeting-WAV transcription with segment timestamps, cancellation and priority for interactive speech | Tauri, core |
| `dictation` | Pure key gestures with injected monotonic time (hold, double tap, Space lock, cancellation and a 600 ms microphone-tail deadline), spoken-cancel matching, terminal-context spoken-code punctuation, voice-command grammar and project/agent name resolution | IO, async, Tauri, core, the network |
| `media` | What is playing (MPRIS over the session bus, by its signals) and play/pause/skip | Tauri, core |
| `platform` | Linux surfaces/input regions, tray, global shortcuts, jump-to-terminal, focus capabilities, clipboard leases, guarded XTest/portal/tmux insertion and temporary MPRIS volume/playback changes | Tauri, core |
| `app` | The Tauri shell: the runtime loop, effects, commands, tray, settings | — |

The UI (`ui/`) is TypeScript with no framework. `src/bridge.ts` is the only file that talks to Tauri;
`src/island/` renders the island from the view, `src/character/` draws the birds from sprite data,
and `src/surfaces/` holds every other surface, one folder each (`widget/`, `settings/`, `nest/`) with its
`main.ts` entry and its stylesheet. Code more than one surface uses sits at the root of `src/`; the
exceptions are the chat panel (`src/island/chat.ts`, with its Markdown and icons) and the GitHub
card (`src/island/board.ts`), which the Nest draws too, and the settings pages' look
(`surfaces/settings/pages.css`), which the Nest's Week uses inside a shadow root.

## Voice conversation turns

V6.1 adds the pure `voice::turn` machine, not yet called by the app. Injected monotonic time and
opaque turn/probe tokens bind every asynchronous result. First consumed output starts a 500 ms
headphone lock or 1 s echo-cancelled lock; speaker interruption defaults off. A still-live suppressed
speech onset can request a separate 260 ms pre-roll probe after the lock. Only the matching confirmed
probe cancels output and advances the turn; rejection or a 1.5 s timeout leaves the response running.

Production completion requests playback drain; an earlier transient empty queue is not completion.
The requested drain acknowledgement enters a 450 ms desktop tail grace. Speech there is a new turn
without replaying the old output tail; expiry finishes without opening capture. A pending onset at
drain becomes a new turn with its pre-roll, rather than cancelling an already silent response.
Cancellation/failure clears output and rejects late callbacks. Adapters must retain the tokens from
each action, acknowledge actual output consumption/drain and own the memory-only microphone ring;
relabeling old results with current tokens violates the contract. The machine itself has no audio,
provider, permission or network IO, and its effects grant no microphone consent. Runtime capture/replay,
realtime budget/gate, playback and UI integration remain later V6 work.

V6.2.1 adds the pure `voice::turn::end` conversation policy: 1.6 s of silence, or 2.6 s after an
English/Portuguese continuation word. It needs both a VAD speech end and nonempty words; silence
before speech or an ongoing voice never commits. Only growth beyond the partial transcript's prior
word-count maximum restarts its text timer. Rewrites and duplicate VAD end events cannot keep
resetting the pause. Other languages use the base delay.

The optional audio-only semantic scheduler emits at most one outstanding, generation/ticket-bound
request every 200 ms. A probability at least 0.5 may finish only after the current silence policy;
nonfinite/out-of-range scores or one inference failure disable it for the rest of that conversation.
The absolute extra wait is 1.2 s from nominal silence eligibility, not from a delayed poll; a hung
request cannot extend it. Renewed speech cancels the pending ticket and hold; cancellation/new turns
reject old callbacks. Completion is emitted once. Capture duration/empty-input limits remain with the
adapter. Existing Chat/dictation recorders keep their own stop rules.

V6.2.2 adds the optional `semantic-turn` backend with a pinned smart-turn v3.0 graph and independently
qualified Linux x64 CPU runtime. Eight-second crop/left-pad, normalization and Whisper/Slaney features
match stored primary NumPy arrays; an explicit checked-artifact native run matches the reference
probabilities. Shared `model_file` admission retains exact bytes, and `onnx::Runtime` is now the single
native owner for both diarization and semantic turns. Cancellable foreground admission, per-frame
preprocessing deadlines and native RunOptions termination respect the policy ticket's remaining
budget. Model/native session construction cannot promise immediate abort. No app caller, download,
Settings switch or capture is added. See [Semantic turn model](reference/semantic-turn-model.md) for
BSD-2-Clause provenance, exact pins, qualification commands and open speech/latency/platform gates.

V6.3.1 adds `store::money` (migration 28): integer micro-USD holds and once-only final usage
settlement, bound to the existing request-ledger identity. Immediate transactions reserve the hold
and starting trace together, so concurrent operations cannot spend the same allowance. Missing/zero
quotes, zero limits, unavailable ledgers and arithmetic overflow refuse admission. Open holds never
expire, including after a crash; unknown final cost remains null and debits the full reservation.
Only a stopped transport may settle. Settled debits expire 24 hours after completion; future timestamps
remain charged after a clock rollback. History erasure preserves holds/debits and a late settlement
does not recreate erased traces. Provider overruns are charged in full rather than clipped.
Request success/failure is traced independently of whether final billing data is available.
The app explicitly rejects `voice-realtime` through connector settings and the count-only gate.
The later runtime sections add admission, transport and recovery on this persistence foundation.
No automatic refund or recovery grants a fresh budget.

V6.3.2 adds the offline `chat::realtime` owner: reviewed integer rates, conservative per-response
ceilings, explicit finite session caps and effective-configuration confirmation, one attempt bound
to its response ID, and once-only usage receipts. Unknown usage, stale/oversized events and changed
configuration cannot reopen an owner or refund its hold. Cached text/audio are priced separately;
known overruns remain visible in full. The synthetic session cassette is offline; no provider call,
microphone or permission authority is added. See [Realtime accounting](reference/realtime-accounting.md)
for source rates, the ceiling proof, qualification expiry and runtime acceptance still required.

V6.3.3 adds a fixed-origin TLS WebSocket provider, bounded streaming PCM channels and an app-owned
monetary gate. Settings 41 keeps independent default-off consent, limit/model and ledger binding.
One revocable voice lease checks consent/lock/cancel throughout setup, IO and publication; shutdown
permanently closes admission. Reservations precede keyring access/connect/audio; receipt settlement
continues after revocation. Unclaimed handoff guards refund only proven-unsent work; unresolved
holds block further reservations within that owner. V6.3.4 adds a Linux data-directory kernel lock
retained by the transport and every detached worker. Only a new exclusive owner reconciles orphaned
holds, atomically keeping their full unknown cost for 24 hours from reconciliation. Restart cannot
refund or slide an already settled debit, and trace erasure is preserved. Other platforms fail closed.
No native capture/playback entry is added. Duplex socket and process/crash tests are offline;
live-provider/native acceptance and user-facing continuation remain open before activation.
The network inventory lists the concrete transport and its tests.

V6.3.5 changes the app's native service to own a complete bounded turn independently of remote
admission. Persistent transport input stays chunked; success releases the original and failure
stops transport before local Whisper recovery. A complete reviewable text result or original audio
with a typed local failure returns to the native caller, without provider substitution, automatic
Chat send or history writes. Cancel/revocation suppresses stale results; shared settings changes
invalidate epochs under the Settings lock and decoder workers retain the native lease after timeout.
The local wait is bounded but cold native loading remains cooperative. Capture/playback and UI
continuation remain V6.4-V6.7 work. See [Realtime accounting](reference/realtime-accounting.md#native-turn-retention-and-local-text).

V6.4.1 adds independent `audio::playback` output and pure `voice::turn::heard` accounting. A bounded
CPAL worker resamples 24 kHz PCM16, distinguishes queued/submitted/estimated consumed frames and
separates producer completion from device drain. Stop/revocation freezes the cursor and clears the
queue; native teardown retains exclusive ownership. The pure splitter estimates whole heard words
and computes interrupted model context plus a one-shot unheard-text hint. It writes no history and
sends nothing. V6.4.2 adds the native service owner: generation-bound complete turns/signals, gated
provider/item/transcript identities, CPAL consumption/drain and exact acknowledged server truncation.
It emits matching in-memory heard-history records and admits a quoted repair hint once in the next
authorized response. Terminal playback can still drain or be interrupted; history without a server
acknowledgement is explicitly unsynchronized and cannot resume the owner. Detached output retains
the native session lease. No microphone, renderer entry or persistent Chat history write is added;
barge-in stays disabled at app entry pending V6.5. See
[Native playback and context repair](reference/realtime-accounting.md#native-playback-and-context-repair)
for limits, clock assumptions, isolated ALSA null checks and the remaining physical/live acceptance.

V6.5.1 adds read-only `audio::echo` inventory for explicitly configured PipeWire WebRTC module pairs.
Opaque snapshots bind the module/source/sink group and server/node identities; rechecking them does
not grant a route lease. A private native experiment qualifies only synthetic echo/tone preservation.
V6.5.2 adds a separate consent-bound native duplex owner with exact target acknowledgements and
`DONT_MOVE` streams. It watches the AEC module's internal connections and physical endpoints,
revokes changed routes and retains native exclusivity through teardown. Headphones require an
explicit choice; unqualified speaker routes keep interruption disabled. Content spans, sink latency
and a separate drain acknowledgement preserve consumed-output accounting. Route invalidation also
clears retained interruption evidence. Bounded capture returns only to the native caller; it is not
automatically uploaded or saved. The default CPAL path remains non-interruptible. No production
module loader, renderer entry or visible capture activation is added. Backend V6.5 is delivered;
V6.6 tools and V6.7 capture/UI are next. See [Echo cancellation](reference/echo-cancellation.md)
for the private native routing check and remaining physical/live acceptance limits.

V6.6.1 routes finalized local transcripts through V8's command dispatcher before realtime upload.
The state owner checks Settings/epochs and cancellable once-only admission, reuses redacted cached
reads, and keeps context separate from independent dictation. Consumed/refused/clarified commands
retire their input generation without Generate or model fallback. V6.6.3 distinguishes completed,
submitted and failed effect receipts; mixed read/action batches wait too. Successful project saves
can complete, while launch handoffs remain Requested. Shared capture consumption blocks recovery
and relabelled-clone replay after commit, settlement or interruption. Queued native work rechecks
its waiter/authority/deadline and keeps the exclusive lease through exit. Linux requires a bounded same-capture
transcript; missing text fails before provider construction and only offers reviewable recovery.
No renderer/capture assembly, model tool or consultation is enabled. See
[Local command admission](reference/realtime-accounting.md#local-commands-before-realtime-upload).

V6.6.2 supplies the existing five cached flock reads to realtime through frozen confirmed specs.
Serial function responses have bounded arguments, exact streamed/final identity checks and replay
sets. Settlement precedes each continuation reservation and read; inert success/error envelopes
are bounded after JSON escaping. Tool receipts release original input without completing playback;
final speech retains its own identity/history path. Consumed calls cannot become automatic recovery
or replay. No action/permission tool or visible activation is added. See
[Read-only realtime tools](reference/realtime-accounting.md#read-only-tools-in-realtime).

## Local dictation

`dictation::command` provides the V8.1 grammar without a runtime caller. Unicode case/accent/
punctuation folding produces lexical keys; anchored politeness/filler stripping and leading verb
forms belong only to command clauses. English, Portuguese and Spanish tables return descriptions
with unresolved target names, never core intents. Explicit sequence connectors produce a batch only
if every clause parses; card-answer phrases refuse the whole utterance. Parsing is bounded to 4 KiB
and eight commands and uses no IO, microphone, model, clipboard or history.

V8.2 adds `command::names`, an owned snapshot of project/agent identities and saved replacements.
Eligible aliases target complete known names; resolution ranks aliases, exact names, single whole
words and bounded Unicode Levenshtein matches. Single-word fuzzy queries also compare each name's
words, so `apy` keeps both `api` and `api-gateway`. A closest-distance tie stays ambiguous; distinct
identities never collapse by label. An injected clock limits a clarification to its original options
for 90 seconds, without renewal on unsuccessful replies. A missing or renamed option invalidates
the question; a successful reply consumes it. No resolution executes an action.

V8.3's `app::voice_commands::Commands` accepts a transcript only from the runtime state owner and
prepares a batch against a clone of that state. Only `Focus`, `FocusNext`, `FocusPrevious`, `Jump`,
`OpenFolder` and `SetProjectPref` pass the exhaustive `Intent::voice_command_allowed` boundary;
card answers, release/skip/review and merges are excluded. Unresolved, refused or unavailable batches
discard the clone and all effects. A successful batch commits it and returns effects for the existing
executor. Qualified session identities disambiguate duplicate labels; preferences group by exact
folder. Pending batches retain earlier target identities and revalidate them when a question resolves.
Repeat and undo keep 90 seconds of injected-time context; preference undo restores the previous value
only while the session's folder and expected value still match. Focus undo checks the current focus
and previous live session. Lock/cancel clears the context and question.

Reads reuse `flock_tools` over the current local view and cached usage, including an exact-key
`session_detail_exact` that shares the existing redacted detail formatter. Pending cards, session
status/steps, usage and GitHub rows become complete templates in all four languages. Outside titles
stay inert text; these reads have no model, network fetch, clipboard write or spoken answer.

V8.4 pins dictation/command mode into each worker ticket before capture. The separate `talk-command`
portal binding always enters that worker, including with Zeca off or Chat focused. Command capture
uses hold-only gestures and the shared local transcription path, bypassing text transformations,
focus/pane observation and clipboard/insertion. The worker reads saved aliases off the core loop and
queues a transcript with its ticket. The runtime admits it under the ticket's short cancellation lock,
calls the synchronous cached-data dispatcher and runs its effects through the existing executor.
There is no await or IO inside that admission. A replaced/cancelled ticket cannot execute queued work.

The existing single-owner dictation line/caption carries command capture and results. Absolute result
expiry prevents surface handoffs or unsuccessful replies from renewing a clarification's original
90 seconds; the heard-text caption still expires within two seconds. Cards and constrained layouts
retain priority. History records commands with a distinct outcome under the same retention/erase
ordering, without dictation usage/timing aggregates. UI replay is absent and backend replay rejects
command rows. Copy again retains only ordinary dictation.

V8.5 adds the Settings-owned command switch (on by default) and a separate, default-off unknown-speech
Chat fallback. Either choice cancels command tickets and context without interrupting ordinary dictation.
The Nest's command page filters retained history and shares the phrase guide with Settings. Only an
unrecognized, non-refused utterance without an active clarification can request a Chat handoff. It
reserves the existing idle Chat, rejects unsent text/files/references, and rechecks settings, lock,
ticket and selected folder/provider after asynchronous preflight. New conversations use that selected
folder, without rebinding an established conversation. No turn queues behind Chat. The accepted message uses the current
conversation's ordinary send, tools, gate, budget, traces and approvals; its reply streams in Chat.
It neither inherits the composer draft nor changes approval authority. Once admitted, Chat owns Stop.

V3.1 introduces `vults-dictation::cleanup` as a pure prompt/reply contract.
It wraps escaped transcript/application/spelling data in real message turns, with questions and
imperatives demonstrated as text. Typed local edits are replaced by opaque, collision-free marks;
their payload stays outside the model prompt and is restored literally only after sequence validation.
Bounded input/output, controls, explicit question punctuation and word/character plausibility checks
reject unusable replies while retaining the complete local original. These checks do not prove
semantic faithfulness. V3.2 connects a separate, default-off `dictation_cleanup` switch (Settings
schema 37) to one tool-free call through the selected Chat & voice API provider, including literal
loopback Ollama/LM Studio. A key-down snapshot cannot inherit later consent; switch, provider/model
or key changes invalidate in-flight filtering. The dedicated worker keeps local transcription and
final insertion/card/target checks intact. Failure or the 4-second deadline keeps the complete local
text; translation has an 8-second deadline and Prompt has 12 seconds.

The gate admits at most 2,000 attempts per rolling 24 hours, local calls included. Migration 25 adds
a content-free admission ledger and starting trace committed together before HTTP. Reservations survive
restart, failed/interrupted requests and Clear history. Settings binds consent to the ledger identity;
a missing, unreadable or recreated ledger denies requests instead of granting a fresh allowance.
Completion updates only surviving traces with provider/model/local/timing/outcome and known token
counts, never transcript/audio/clipboard/key or invented usage. The HTTP client disables proxies,
redirects and retries. V3.3 resolves a frozen application/style/output-language choice in the key-down
callback. Terminals/editors default to Verbatim, mail to Polished, chat apps to Natural and recognized
flock sessions to Prompt. An explicit app override precedes category defaults; sessions retain Prompt.
Other identified apps use the global default (Natural). Missing focus or unreadable preferences keeps
local text without a model. Exact reviewed desktop aliases share overrides; window titles/URLs never
select a style. Settings schema 38 adds a global style, optional output language and default-off
session English. Session English takes priority over the global output language only in recognized
sessions; Verbatim never translates or calls a model.

Migration 26 adds `dictation_styles(app, style)`, preserved by Clear history, and nullable retained
`output_language`, independent of the heard language. The worker records applied style/output language
only on successful filtering; failures retain the local processing style. Style/language edits apply
to the next recording without mutating a frozen choice. X11 reads foreground identity in the key
callback; Plasma Wayland reuses the checked KWin observer with a separate script lifetime, warmed
when clean-up is enabled, without a keyboard portal grant. Observer loss/ambiguity stays local;
asynchronous UI context cannot relabel a captured choice. V3.4 adds the Settings-owned clean-up switch, provider disclosure,
style cards, output-language/session-English choices and collapsed app overrides. UI reads use local
settings/provider metadata only; no model probe or model-list fetch runs on opening Styles. Pending
writes, newer events and stale reads retain their ordering; failure keeps a reload path and an already
enabled feature can still be disabled when ancillary reads fail. The Dictation tab presents one Styles editor through the privileged
`dictationstyles` Settings destination, alongside its Voice, Keyboard shortcuts, Command settings,
Vocabulary, Paste, Privacy and Diagnostics pages. Work-page links return to Nest's transcript
webview; configuration grants no powers to Nest. Voice owns the shared transcription controls,
with a link from Chat & voice, and History names only known applied styles/output
languages. Shared style copy/cards use the production theme and all four UI languages. This path
adds no approval or execution authority. The process keeps a validated Dictation configuration
section separately from Nest's work tab. Only Settings can remember that section; an explicit
work-page route clears it, and returning through the Dictation header/keyboard uses the native
owner to restore the correct webview. No shared-origin storage grants configuration authority.

Independent dictation has its own cancellable worker in `app`; it does not pass microphone
frames or transcripts through the agent event loop. The desktop talk shortcut goes directly to that
worker on both press and release, regardless of the focused app or Zeca. It cannot open Chat or
start Chat voice. Only the Chat microphone button fills its draft directly. Configuration pages
keep their separate explicit, bounded microphone diagnostic.

```text
desktop talk shortcut -> app::dictation worker (generation)
                                                  |
                              audio::Capture (16 kHz mono, <= 10 min, memory only)
                                  |                               |
                          voice::Preview                     finish + 600 ms tail
                          (supported GPU)                         |
                                  |                    Silero + whisper + text filter
                                  |                               |
                                  |                    optional installed-Base retry
                                  |                               |
                                  |                    spoken cancel -> discard
                                  |                               |
                                  |                    local words + context-bound code
                                  |                               |
                                  |                    guarded delivery or clipboard
                                  |                               |
                                  +--> ordered feedback <-- confirmed outcome
                              passive dictation capsule           |
                                                         dictation history + totals
                                                          (store, retention choice)
```

On Linux `app::dictation_overlay` maps a fixed 480 x 160 passive layer surface at the bottom center,
clear of desktop panels. Its empty input region and disabled keyboard keep the destination's
focus. It reuses the production status owner/theme and local events, with no microphone or network
owner. A read-only availability check suppresses duplicate inline indicators; failed construction
keeps the island/Nest fallback. Cards, lock, paused presence, expiry and optional captions keep
their existing authority. The island remains the single start-sound owner.

Every turn captures its microphone and processing choices. Cancellation invalidates late
capture, preview, decode and delivery work; replay uses already delivered text and never repeats
conversion or insertion. Audio, previous clipboard text and window titles stay in memory.
Retained heard/written text and recognized application/session metadata enter the store only
after successful delivery. Transcript erasure preserves measured daily/application totals.

Migration 23 adds nullable millisecond stage timings to retained delivered dictations. The worker
measures a monotonic finish-request-to-delivery total (including the microphone tail and queue),
STT with model loading/retry, the local text pipeline and confirmed delivery. Clean-up remains
unmeasured until implemented. `store::dictations` summarizes each known stage by nearest-rank
p50/p95 over the newest 50 retained deliveries, without loading text. The history command exposes
rows and the summary; cancellation, failed/empty results and replay create no timing samples.
Timings follow transcript retention/erasure; independent daily/app usage totals are unchanged.

`app::doctor::dictation` exposes Settings-only diagnostics using the existing shortcut/keyboard owners,
audio input format, local GPU/VAD probes and a timing-only store read. Final decode reports loop cuts
without counting preview/Chat passes; in-memory counters also track Base retries and Copied outcomes.
The separate six-second microphone test measures initial two-second RMS noise in dBFS and nine bands,
without transcription or disk/network/clipboard access. Test ids reject obsolete polls/stops, capture
shutdown serializes restarts, and an unrenewed two-second Settings heartbeat cancels the test. Native
window/webview changes, screen lock and voice starts cancel it too. Unknown probes and timing failures
are not successful zero readings. The renderer retains the owner across Settings repaints, polls only
while visible and updates bands without rebuilding the focused Stop action.

Migration 24 adds `dictation_app_days` to the existing usage transaction, including unknown apps.
Statistics read daily totals and application attribution in one SQLite snapshot, independent of
retained transcripts. Legacy lifetime app totals remain exact; bounded periods label incomplete
daily attribution. The Nest's Activity > Dictation owns figures, period filters, daily bars and
top apps; the Dictation area links there. Activity > Week includes dictated words and weeks with
dictation-only usage, even when agent history is off. Presentation-only period state uses the
same local storage convention as Nest panels and survives recreating the window.

Insertion starts off. Native X11 and Plasma Wayland capture separate focus capabilities; the
latter also needs a human-approved keyboard-only portal grant, with restore tokens in the OS
keyring. Clipboard leases preserve newer copies. Exact tmux delivery additionally asks the
runtime for the session's current readiness: pending permissions/questions and deferred cards
block delivery. Text never becomes an Enter key, shell command or permission decision.

While capture is alive, its optional `platform::music::Listening` guard lowers or pauses playing
MPRIS owners and restores unchanged players on capture end/drop. That local control reads no
track metadata and is separate from `media`'s Now playing watcher. Chat and future meeting
capture do not enter this guard. Ordinary transcription stays on this computer; later clean-up
and meeting features retain their separate switches and network-gate contracts.

V2.1 adds `store::words`: dictionary words, ordered replacement rules and snippets in migration
22. Canonical Unicode case keys make duplicate checks atomic across database connections.
Replacement updates preserve their id/order; duplicate words and snippet triggers are rejected.
The Nest-only `dictation_words` / `dictation_words_change` commands read a coherent snapshot or
apply a typed edit on a blocking worker, with bound SQL and sanitized database errors. Stored
snippet/replacement bodies remain literal. History clearing leaves these user preferences intact;
individual deletion compacts removed rows. V2.2 snapshots these rules per recording and runs
the pure `dictation::text::Rules` pipeline after raw spoken cancellation: opaque snippet marks,
ordered literal replacements, eligible dictionary spelling, context-bound spoken code and literal
snippet restoration. Unicode case/word matching keeps controls as barriers. Annotated segments
retain the rule and matched text for the sentence preview/highlights; History keeps the original and
final text with a precise processing style. The pipeline runs outside the control lock and
rechecks the generation before recording its result. Snapshot read failure or a 256 KiB input/
intermediate-output limit fails before delivery. Replay and Chat bypass it;
V2.5 provides the Nest Dictionary/Snippets work pages and a read-only `dictation_try` command.
The command checks the Nest caller, snapshots saved rules on a blocking worker and runs the same
`Rules::apply` pipeline, respecting saved spoken cancellation and optional terminal/code context.
Its bounded result includes inert segments and rule annotations; it captures no audio, calls no
model, writes no clipboard/history and grants no insertion capability. Stable form nodes retain
drafts, caret and scroll across reads/relabels; serialized writes disable duplicate actions,
require explicit deletion confirmation and reload saved data. Failed reads keep a stale view
with retry, and sentence changes discard stale asynchronous preview responses. Settings owns
the two opt-in vocabulary controls; work pages only link there.

V2.3 adds opt-in dictionary context in Settings schema 34 (default off; control in Dictation > Vocabulary).
Each recording captures the flag and dictionary once. Preview/final pass only dictionary words
to `voice`; a Base retry rechecks its actual model. The model tokenizer caps the combined coding
seed and whole entries at 224 tokens; blank/control-bearing/duplicate words are skipped. Tokens
belong to one decode, never the cached model, and Chat keeps its original prompt path. A strict
model/language gate currently allows only Turbo/fixed English: the offline 20-sentence set
per language plus silence/noise passed only for that pair. Base/Small and Portuguese remain
blocked even where sentence WER improved. Fixture provenance/results live in
`crates/voice/tests/fixtures/README.md`; synthetic WER is separate from human desktop acceptance.

V2.4 adds an independent `dictation_project_vocabulary` preference (schema 35, default off,
control in Dictation > Vocabulary). The existing off-loop repository reader refreshes its opt-in memory
cache on session start and ordinary repository refresh. Each Git working tree keeps its own
entry (128 maximum), so branches and members do not bleed between worktrees of one roost.
The extractor reads Cargo/npm names and dependencies, literal or trailing-`/*` workspace members,
shallow `src` module names, top-level file names and branch. Reads are bounded: 64 KiB manifests,
64 member directories, 512 entries per directory and 256 unique names of at most 128 bytes.
Manifest final symlinks, linked workspace/source directories, parent/absolute member paths,
malformed files and hidden/control-bearing names are skipped. Source files are never read and
scripts are never run. Repository metadata remains untrusted context, with no permission authority.

Key-down's best-effort foreground reader selects cached names using the pinned/focused session's
folder, or an unambiguous terminal/editor title. Unknown or late context falls back to no project
names. A generation-bound `OnceLock` freezes the result before the first actual decode; preview
and final share it, while cancellation/restart rejects stale observations. Personal dictionary
words precede project names under V2.3's combined token cap and model/language gate. The pure
final-text rules, Chat, insertion capabilities and network gate do not change.

The renderer owns History and its temporary feedback, not native insertion or settings
mutation. Settings-only commands check the caller's webview label before saving. The
[dictation guide](guide/dictation.md#supported-destinations) lists actual supported destinations;
[ADR 0032](adr/0032-dictation-into-other-apps.md) records the capability and permission boundaries.

## Meaning in core, look in the surface

The view says what things mean, so every surface agrees ([ADR 0021](adr/0021-four-sizes-one-nest.md)).
Each session carries its `attention`, an ordered level (`quiet` < `info` < `done` < `failed` <
`needs-you`), and the view carries the most of them; `card` marks the session whose card is first
in line and still waits on it. The island only chooses the look: which sound a level makes, and
how long a state must hold (1.5 s, 3 s for `done`) before it is news.

The runtime broadcasts the view, when it changed, to every window at once (`publish_view`), and
keeps the last one for a page that loads later (`current_view`). Events meant for the island alone
(pointer, shortcuts, chat) are sent to its label. A plain `listen` hears every target, so a new
surface subscribes to those through window-scoped helpers in `bridge.ts`
(`getCurrentWebviewWindow().listen`) to stay out of them.

The Nest's window holds two webviews, the Nest's and Settings', one shown at a time under the same
header (ADR 0021). Commands check the label of the webview that called: the calls only Settings
may make are refused from the Nest's, which shows agents' text.

`ended` lists the cards that left the line most recently, each with its `outcome`: answered here
(`allowed`, `denied`, `answered`), `released` (sent to the terminal from here), `terminal` (the
agent moved on), `expired` or `rule` (an Always on an identical card). The island reads it to say what became of the card it
showed. An outcome is information only: it is recorded where a card leaves the line and never
answers one.

`front` is the session in front, by one rule: the session whose card waits, else the one the user
put in front (`focus`, set by `Intent::Focus` from a click on its row, forgotten when the session
leaves), else the first. Work alone never moves it. Sessions come in the order they arrived, the
order every surface draws them in, and the one `Intent::FocusNext` and `FocusPrevious` (global
shortcuts) walk, wrapping. The chat works in the folder of the session in front.

## Why the hook waits for an acknowledgement

A permission request keeps its connection open. The server only waits for a human once the app's
loop has taken the request and `core` has queued its card (`Effect::AckPermission`). If the app is
stuck or not listening, the agent gets its answer (silence, so it asks in the terminal) within
800 ms instead of two minutes. The island opens on a queued card and stays open until it is
answered; no presence mode may hide it once acknowledged ([ADR 0021](adr/0021-four-sizes-one-nest.md)).

## Why only `core` produces a decision

`core::reduce` turns a `PermissionRequested` event into pending state, and only a human's click
turns pending state into a `RespondPermission` effect: `Intent::Decide` (Allow, Deny) or
`Intent::DecideAlways`, which also saves a rule. A saved rule answers only a request that matches
it exactly: the same agent, folder, tool and target ([ADR 0004](adr/0004-a-human-answers-permissions.md)). A question
(`QuestionAsked`) waits the same way, and only an `Intent::Answer` that fits its questions turns it
into an `AnswerQuestion` effect. A test feeds
every other input in every order and checks none of them answers.

M7's Linux `ask_user` text-question interface is separate from this hook reply path. A private
same-user socket connects the stdio relay to the app's runtime. Kernel peer PID, executable identity,
process start times and retained ancestry bind it to a verified live client; self-supplied headers
cannot establish ownership. App-owned clients use the existing chat server. External clients use
`--questions-mcp`, whose dispatch exposes only question tools, and need the separate default-off
Dictation > Agents consent. The consent generation and transport receive time reject old queued
requests after a new grant. Settings/consent guards stay held through runtime transitions, including
reply handoff, so revocation cannot race between an access read and delivery. The runtime owns `core::questions::Slot`, its fresh
connection nonce, absolute deadline, surface lease and once-only draft confirmation. It defers delivery
until the content-free audit is durable, while continuing to process revocation messages. The relay
acknowledges after flushing the MCP response; failure/lost acknowledgement never reuses consumed text.
Permission cards take priority. Only the owning island/Nest webview can edit or confirm; lock,
navigation, departure, expiry and disconnect invalidate pending work and late completions.
The question-draft contract in [ADR 0028](adr/0028-spoken-replies-through-a-realtime-model.md#human-confirmed-question-drafts)
is implemented by a physical-key-only local recording bound to the live question token and recording
id. `vults_dictate` aliases the same typed `ask_user` exchange. Capture uses the existing microphone
and cached local transcriber, with admission/lifetime guards and no provider, download, clipboard or
history pipeline. Results append to the current editor; a matching apply acknowledgement precedes
separate human Send. Cancellation and every dictation transition advance the view revision, so late
snapshots cannot restore cancelled words. Native capture/decode never blocks the runtime loop. Ordinary worker admission and Chat recorder
registration share the routing fence; revocable microphone grants cannot revive after a question
closes. A matching renderer Status must acknowledge displaying a question before capture can select it.
No question text becomes a permission intent, rule or policy. Only Settings can preview/apply the
question-only MCP configuration for Claude Code/Codex. A cached plan binds the agent, executable,
install/removal choice, exact diff and fingerprint. `agent-config` supplies strict reads, dated backup
and atomic writes; stale/foreign entries are refused. The managed entry can follow an executable
path update, while other servers, hooks and trust remain intact. UI read generations, consent-event
versions and departure epochs prevent stale configuration/status/preview replies from returning.

## The island on Wayland

The island is a layer-shell surface mapped once, at a fixed size, and never resized or hidden: KWin
stops showing a layer surface resized from the webview. The UI draws the island inside it and reports
its rectangle, which becomes the only part that takes the mouse (the input region); everything else
falls through to the windows below. WebKit pauses `requestAnimationFrame` while it thinks the page is
hidden, so the UI measures the DOM synchronously and animates with timers.

The platform code is per window: each surface has a `LayerSpec` (namespace, fixed size, the edges
it hangs from, margin, keyboard) and its own input region, kept by window label. A page's `layout`
and keyboard requests act on the window that sent them, and only if it is a surface
(`SURFACES` in `app/src/lib.rs`). A new surface gets a spec, its label in the `windows` of `app/capabilities/default.json` and, for
its own page, a Vite entry.

## Trying the island without an agent

`cargo run -p vults-hook --example replay` sends a recorded session through the real hook into
the running app: reading, searching, the web, an edit, a permission (it waits for your answer, as an
agent would), a command and the end. Pass your own JSONL file (one hook JSON per line) and
`--delay-ms` or `--agent codex` to change it. The application itself is the UI validation surface.

## Decisions

Why the app is built this way (Rust only, the safety rules, one core for every surface, Zeca
optional, rules before models) is in the [decision records](adr/README.md).

## Documentation

`docs/` is the source for the docs on the website
([rogeriojunior31.github.io/en/docs/vults](https://rogeriojunior31.github.io/en/docs/vults/)),
published on each release tag; `docs/site.json` holds the name and summary the website shows. Pages start with a
`# H1`, use relative links, and keep images in `docs/assets/`. `docs/adr/` holds the decision records. `docs/dev/` is internal and not
published; `docs/pt-br/` holds the Brazilian Portuguese translation (see CONTRIBUTING).
