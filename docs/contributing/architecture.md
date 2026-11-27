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
That keeps a new surface cheap (it draws `State::view` and sends intents)
and keeps Zeca optional: he uses Chat, Voice and
Connect, and none of them knows about him. His body is in Experience; his brain is the `zeca`
crate in Connect, which reads the flock but never answers a permission
.

| Crate | Role | Must not use |
|---|---|---|
| `brand` | The app's name, slug and bundle id; generates `ui/src/brand.ts` | anything |
| `protocol` | Versioned hook ↔ app messages, limits, socket and pipe names | tokio, Tauri |
| `peer` | Same-user checks on both ends of the connection | tokio, Tauri |
| `hook` | Reads the agent's hook JSON, forwards it, prints the agent's decision format | tokio, HTTP |
| `ipc` | Async server: connection limits, ack-then-decide, routing to the app | Tauri |
| `core` | Pure domain: sessions, approvals (each card an offer in the ledger: bound to its exact request, answered once, only in time), action classes, the audit lines, alerts, the view, whose TypeScript types it generates into `ui/src/view.gen.ts` (a test checks it is fresh); clock injected | IO, async, Tauri |
| `agents` | The registry of every agent (id, name, wire, setup, how it runs, what it can do; `ui/src/agents.gen.ts` is its twin) and, per agent (Claude Code, Codex, Antigravity, OpenCode, Hermes Agent, Qwen Code, Copilot CLI, Cursor, Factory Droid, Kimi Code, any other): event names, tool → activity, install entries, Codex trust; the `AgentDriver` trait for an agent the app runs itself (no runtime) | Tauri, tokio |
| `agent-config` | Safe edits of agent configs (JSON, TOML with its comments kept, whole plugin files): strict read, diff, fingerprint, backup, atomic write | Tauri |
| `store` | The local database: one `vults.sqlite` (bundled SQLite), numbered migrations, the history of turns, the append-only audit log, the consents (never deleted), traces, Zeca's conversations (with what each read from outside and its frozen context), the paired phones and the highest counter each signed with, Zeca's journal and daily summaries with their full-text and vector recall, the memory proposals waiting for a yes, his nights, what the agents cost by day and roost, and text-only dictation history with independent daily/app totals (migration 21), recorded under the chosen retention, plus user-maintained dictionary/replacements/snippets (migration 22) independent of history retention | Tauri, async, the network |
| `chat` | Chat through the `claude` and `codex` CLIs, with permission requests, or an API: Anthropic or an OpenAI-compatible cloud with the user's key, or a local Ollama / LM Studio | Tauri |
| `zeca` | Zeca's brain: a read-only view of the flock, his memory as a git repository of small Markdown files of facts, each with a hard limit (each write a commit) | `core` (so nothing in it can build an `Intent`), Tauri |
| `link` | A paired phone's link: pairing by QR, envelopes sealed with X25519 and XChaCha20-Poly1305 (a relay sees only ciphertext), answers signed by the phone's Keystore keys and checked before `core`'s ledger redeems them | IO, the network, Tauri |
| `relay` | The phone's relay: an axum server that stores and forwards sealed envelopes by box, capped and dropped after 10 minutes, and the client both ends use, each request through the app's network gate; it depends only on `link` | Tauri, `core` |
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

## Voice and dictation

Every voice feature shares one local path: `audio` captures into memory, `voice` transcribes with
whisper.cpp, and the pure `dictation` crate turns text into text or into a typed command. `platform`
owns everything that touches the desktop (shortcuts, the focused window, the clipboard, insertion,
media, system output, app launching); `app` orchestrates and owns the network gate. User-facing
behavior is in [Dictation](../guide/dictation.md), [Voice](../guide/voice.md) and
[Meetings](../guide/meetings.md).

**A dictation** starts at key-down with a ticket that freezes its mode (text or command), the
captured target and a snapshot of the settings and Your words rules. After local transcription the
transcript goes through, in order: spoken cancellation, Your words (`dictation::text`), spoken code,
the optional clean-up request (`dictation::cleanup`, through the gate), then delivery. Delivery
rechecks the target right before acting (window identity and focus epoch on X11 or through a
read-only KWin script, pane and process for tmux) and asks `core` for the session's current state:
a session waiting on a card never receives text. Anything that cannot be proved falls back to the
clipboard. History stores text only, under its retention.

**A command** uses the same capture and transcription, then `dictation::command`: a bounded grammar
(4 KiB, eight commands) in English, Portuguese and Spanish, and a name resolver over a snapshot of
projects, agents and aliases. `app::voice_commands` plans the batch against a clone of the state;
only the intents in `Intent::voice_command_allowed` can come out of it, which excludes every card
answer, release, review and merge. Native actions (media, system output, app launch, terminal
focus) are preflighted together, revalidated one by one, and report only confirmed steps: no retry,
no rollback.

**A voice conversation** keeps local capture and local Whisper, and adds three owners. `voice::turn`
is a pure turn machine (end of turn, interruption, heard-text accounting). `chat::realtime` is the
provider and its pricing. `store::money` reserves a micro-USD hold before any key or socket is
touched and settles it once; a hold never refunds itself. Final local transcripts pass through the
command dispatcher before anything is uploaded, and realtime only gets read-only cached flock tools.

**A meeting** writes two private temporary WAVs (`meeting-audio/`, 0700 and 0600), transcribes them
after Stop in blocks that yield to interactive dictation, and deletes them on every outcome and at
start-up. Speaker analysis and summaries are separate opt-ins.

Invariants every change keeps, each with a test:

- No voice path reaches `Intent::Decide` or `DecideAlways`.
- Audio stays in memory except a meeting's temporary files; no wake word, no always-on microphone
 .
- Every model call on audio or text goes through the gate, with its own switch, budget and trace.
- A late or replaced ticket never executes or delivers anything.
- Spoken text never becomes shell or tmux command text, and Enter is never sent.

The engineering records behind these owners (rates and ceilings, the echo-cancellation routes, the
semantic turn model, meeting model artifacts and speaker evaluation) are in `docs/dev/internals/`.

## Meaning in core, look in the surface

The view says what things mean, so every surface agrees.
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
header. Commands check the label of the webview that called: the calls only Settings
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
answered; no presence mode may hide it once acknowledged.

## Why only `core` produces a decision

`core::reduce` turns a `PermissionRequested` event into pending state, and only a human's click
turns pending state into a `RespondPermission` effect: `Intent::Decide` (Allow, Deny) or
`Intent::DecideAlways`, which also saves a rule. A saved rule answers only a request that matches
it exactly: the same agent, folder, tool and target. A question
(`QuestionAsked`) waits the same way, and only an `Intent::Answer` that fits its questions turns it
into an `AnswerQuestion` effect. A test feeds
every other input in every order and checks none of them answers.

Zeca's Linux `ask_user` text-question interface is separate from this hook reply path. A private
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
The question-draft rule (a dictated draft is sent only by a click)
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
optional, rules before models) is in the decision records.

## Documentation

`docs/` is the source for the docs on the website
([rogeriojunior31.github.io/en/docs/vults](https://rogeriojunior31.github.io/en/docs/vults/)),
published on each release tag; `docs/site.json` holds the name and summary the website shows. Pages start with a
`# H1`, use relative links, and keep images in `docs/assets/`. `docs/dev/` is internal and not
published; `docs/pt-br/` holds the Brazilian Portuguese translation (see CONTRIBUTING).
