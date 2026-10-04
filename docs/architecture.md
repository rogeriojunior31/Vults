# Architecture

Everything flows one way, through one loop:

```
agent ──hook JSON──▶ vultures-ai-hook ──protocol line──▶ ipc ──┐
GitHub (gh) ─────────────▶ connectors runtime ─────────────────┤
island clicks (Allow, open, jump…) ────────────────────────────┤
                                                                ▼
                                        core::reduce(state, input, now) ──▶ effects
                                                                │            (answer the hook,
                                                    State::view ▼             open a URL, jump)
                                                         island renders
```

| Crate | Role | Must not use |
|---|---|---|
| `brand` | The app's name, slug and bundle id; generates `ui/src/brand.ts` | anything |
| `protocol` | Versioned hook ↔ app messages, limits, socket and pipe names | tokio, Tauri |
| `peer` | Same-user checks on both ends of the connection | tokio, Tauri |
| `hook` | Reads the agent's hook JSON, forwards it, prints the agent's decision format | tokio, HTTP |
| `ipc` | Async server: connection limits, ack-then-decide, routing to the app | Tauri |
| `core` | Pure domain: sessions, approvals, alerts, the view; clock injected | IO, async, Tauri |
| `agents` | Per agent: event names, tool → activity, install entries, Codex trust | Tauri |
| `agent-config` | Safe edits of agent configs: strict read, diff, fingerprint, backup, atomic write | Tauri |
| `chat` | Chat through the `claude` and `codex` CLIs, with permission requests, or the Messages API with the user's key | Tauri |
| `secrets` | The OS keyring (Secret Service, Credential Manager), keyed by the bundle id | Tauri, files |
| `connectors` | The `Connector` trait, the polling runtime, GitHub | Tauri, core |
| `voice` | Push-to-talk: the microphone into memory (cpal), whisper.cpp on this computer, model downloads checked by SHA-256 | Tauri, core |
| `media` | What is playing (MPRIS over the session bus, by its signals) and play/pause/skip | Tauri, core |
| `platform` | Linux island placement (layer-shell, input region) and jump-to-terminal | Tauri, core |
| `app` | The Tauri shell: the runtime loop, effects, commands, tray, settings | — |

The UI (`ui/`) is TypeScript with no framework. `src/bridge.ts` is the only file that talks to Tauri;
`src/island/` renders the island from the view, `src/character/` draws the birds from sprite data.

## Why the hook waits for an acknowledgement

A permission request keeps its connection open. The server only waits for a human once the app's
loop has taken the request and `core` has queued its card (`Effect::AckPermission`). If the app is
stuck or not listening, the agent gets its answer (silence, so it asks in the terminal) within
800 ms instead of two minutes. The island opens on a queued card and stays open until it is
answered; no presence mode may hide it ([ADR 0009](adr/0009-presence-never-hides-a-card.md)).

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

## Trying the island without an agent

`cargo run -p vultures-ai-hook --example replay` sends a recorded session through the real hook into
the running app: reading, searching, the web, an edit, a permission (it waits for your answer, as an
agent would), a command and the end. Pass your own JSONL file (one hook JSON per line) and
`--delay-ms` or `--agent codex` to change it. `ui/lab/` (`npm run dev`, then `/lab/`) shows every clip
and the island with made-up states, without the app at all.

## Decisions

Why the app is built this way (Rust only, the safety rules, one core for every surface, Zeca
optional, rules before models) is in the [decision records](adr/README.md).

## Documentation

`docs/` is the source for the docs on the website, published on each release tag. Pages start with a
`# H1`, use relative links, and keep images in `docs/assets/`. `docs/adr/` holds the decision records. `docs/dev/` is internal and not
published; `docs/pt-br/` will hold the translation.
