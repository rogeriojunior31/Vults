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
| `audio` | Microphone capture on its own thread (cpal), named device/default fallback with monitors skipped, bounded 16 kHz mono in memory (rubato), scalar level and nine speech bands (rustfft); no transcript or audio written to disk | Tauri, core, transcription, the network |
| `voice` | Chat push-to-talk over `audio`, a one-minute recording cap, Silero end-of-speech and whisper.cpp/live preview on this computer, model downloads checked by SHA-256 | Tauri, core |
| `dictation` | Pure key gestures with injected monotonic time (hold, double tap, Space lock, cancellation and a 600 ms microphone-tail deadline), spoken-cancel matching and terminal-context spoken-code punctuation | IO, async, Tauri, core, the network |
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

## Local dictation

Independent dictation has its own cancellable worker in `app`; it does not pass microphone
frames or transcripts through the agent event loop. The shortcut router pins each key release
to its press owner. A focused Vults Chat with Zeca on uses its existing draft voice path;
external apps and Zeca-off surfaces use independent dictation. Settings cannot start dictation; its explicit microphone diagnostic has a separate bounded capture.

```text
desktop talk shortcut -> app::shortcut_route -> app::dictation worker (generation)
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
                                      island / Nest               |
                                                         dictation history + totals
                                                          (store, retention choice)
```

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

V2.3 adds opt-in dictionary context in Settings schema 34 (default off; control in Settings > Dictation > Your words).
Each recording captures the flag and dictionary once. Preview/final pass only dictionary words
to `voice`; a Base retry rechecks its actual model. The model tokenizer caps the combined coding
seed and whole entries at 224 tokens; blank/control-bearing/duplicate words are skipped. Tokens
belong to one decode, never the cached model, and Chat keeps its original prompt path. A strict
model/language gate currently allows only Turbo/fixed English: the offline 20-sentence set
per language plus silence/noise passed only for that pair. Base/Small and Portuguese remain
blocked even where sentence WER improved. Fixture provenance/results live in
`crates/voice/tests/fixtures/README.md`; synthetic WER is separate from human desktop acceptance.

V2.4 adds an independent `dictation_project_vocabulary` preference (schema 35, default off,
control in Settings > Dictation > Your words). The existing off-loop repository reader refreshes its opt-in memory
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
