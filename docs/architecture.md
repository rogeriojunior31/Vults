# Architecture

Everything flows one way, through one loop:

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
| **Connect** | `connectors`, `chat`, `voice`, `media` | What reaches past the agents: GitHub, the chat CLIs and APIs, the microphone, what is playing |
| **Core** | `core`, `protocol`, `peer`, `ipc`, `hook`, `agents`, `agent-config` | Sessions, events, approvals and decisions: what the agents are doing and what the human said |
| base | `brand`, `secrets` | The name, the keyring |

Only `app` wires the layers together: Connect never calls into Experience, and Core knows neither.
That keeps a new surface cheap (it draws `State::view` and sends intents, [ADR 0008](adr/0008-one-core-many-surfaces.md))
and keeps Zeca optional ([ADR 0010](adr/0010-zeca-is-optional.md)): he uses Chat, Voice and
Connect, and none of them knows about him.

| Crate | Role | Must not use |
|---|---|---|
| `brand` | The app's name, slug and bundle id; generates `ui/src/brand.ts` | anything |
| `protocol` | Versioned hook ↔ app messages, limits, socket and pipe names | tokio, Tauri |
| `peer` | Same-user checks on both ends of the connection | tokio, Tauri |
| `hook` | Reads the agent's hook JSON, forwards it, prints the agent's decision format | tokio, HTTP |
| `ipc` | Async server: connection limits, ack-then-decide, routing to the app | Tauri |
| `core` | Pure domain: sessions, approvals, alerts, the view, whose TypeScript types it generates into `ui/src/view.gen.ts` (a test checks it is fresh); clock injected | IO, async, Tauri |
| `agents` | Per agent: event names, tool → activity, install entries, Codex trust | Tauri |
| `agent-config` | Safe edits of agent configs: strict read, diff, fingerprint, backup, atomic write | Tauri |
| `chat` | Chat through the `claude` and `codex` CLIs, with permission requests, or an API: Anthropic or an OpenAI-compatible cloud with the user's key, or a local Ollama / LM Studio | Tauri |
| `secrets` | The OS keyring (Secret Service, Credential Manager), keyed by the bundle id | Tauri, files |
| `connectors` | Vults Connect: the `Connector` trait, the polling runtime, GitHub | Tauri, core |
| `voice` | Push-to-talk: the microphone into memory (cpal), whisper.cpp on this computer, model downloads checked by SHA-256 | Tauri, core |
| `media` | What is playing (MPRIS over the session bus, by its signals) and play/pause/skip | Tauri, core |
| `platform` | Linux surface placement (layer-shell, an input region per window), the tray item (a StatusNotifierItem), global shortcuts (the desktop portal) and jump-to-terminal | Tauri, core |
| `app` | The Tauri shell: the runtime loop, effects, commands, tray, settings | — |

The UI (`ui/`) is TypeScript with no framework. `src/bridge.ts` is the only file that talks to Tauri;
`src/island/` renders the island from the view, `src/character/` draws the birds from sprite data,
and `src/surfaces/` holds every other surface, one folder each (`widget/`, `settings/`) with its
`main.ts` entry and its stylesheet. Code more than one surface uses sits at the root of `src/`.

## Meaning in core, look in the surface

The view says what things mean, so every surface agrees ([ADR 0008](adr/0008-one-core-many-surfaces.md)).
Each session carries its `attention`, an ordered level (`quiet` < `info` < `done` < `failed` <
`needs-you`), and the view carries the most of them; `card` marks the session whose card is first
in line and still waits on it. The island only chooses the look: which sound a level makes, and
how long a state must hold (1.5 s, 3 s for `done`) before it is news.

The runtime broadcasts the view, when it changed, to every window at once (`publish_view`), and
keeps the last one for a page that loads later (`current_view`). Events meant for the island alone
(pointer, shortcuts, chat) are sent to its label. A plain `listen` hears every target, so a new
surface subscribes to those through window-scoped helpers in `bridge.ts`
(`getCurrentWebviewWindow().listen`) to stay out of them.

`ended` lists the cards that left the line most recently, each with its `outcome`: answered here
(`allowed`, `denied`, `answered`), `released` (sent to the terminal from here), `terminal` (the
agent moved on), `expired` or `rule` (an Always on an identical card). The island reads it to say what became of the card it
showed. An outcome is information only: it is recorded where a card leaves the line and never
answers one.

`front` is the session in front, by one rule: the session whose card waits, else the one the user
put in front (`focus`, set by `Intent::Focus` from a click on its row, forgotten when the session
leaves), else the first at work, else the first. Sessions come in the order they arrived, the
order every surface draws them in, and the one `Intent::FocusNext` and `FocusPrevious` (global
shortcuts) walk, wrapping. The chat works in the folder of the session in front.

## Why the hook waits for an acknowledgement

A permission request keeps its connection open. The server only waits for a human once the app's
loop has taken the request and `core` has queued its card (`Effect::AckPermission`). If the app is
stuck or not listening, the agent gets its answer (silence, so it asks in the terminal) within
800 ms instead of two minutes. The island opens on a queued card and stays open until it is
answered; no presence mode may hide it once acknowledged ([ADR 0009](adr/0009-presence-never-hides-a-card.md)).

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
`--delay-ms` or `--agent codex` to change it. `ui/lab/` (`npm run dev`, then `/lab/`) shows every clip
and the island with made-up states, without the app at all.

## Decisions

Why the app is built this way (Rust only, the safety rules, one core for every surface, Zeca
optional, rules before models) is in the [decision records](adr/README.md).

## Documentation

`docs/` is the source for the docs on the website
([rogeriojunior31.github.io/en/docs/vults](https://rogeriojunior31.github.io/en/docs/vults/)),
published on each release tag; `docs/site.json` holds the name and summary the website shows. Pages start with a
`# H1`, use relative links, and keep images in `docs/assets/`. `docs/adr/` holds the decision records. `docs/dev/` is internal and not
published; `docs/pt-br/` will hold the translation. `node tests/visual/docs-shots.mjs` retakes the
README's and the guides' island images from the lab; run it before each release.
