# CLAUDE.md — Vults

Desktop app: Zeca, an 8-bit vulture, and his flock (the vults) perch at the top of the screen and show
what Claude Code, Codex and Gemini CLI sessions are doing, with approvals, chat, dropped files and connectors
(GitHub first).

## Language

**English is the project language**: code, identifiers, comments, UI text, errors, tests, docs and
commit messages. `scripts/check-english.sh` (CI) rejects Portuguese accents; a line that must keep one
carries `check-english:allow`. User-facing sentences go through the i18n catalog as whole strings,
never assembled from fragments: a pt-BR translation comes after 1.0. The only Portuguese folder is
`docs/pt-br/`.

Comments are short and say *why* (invariant, gotcha, security), not what the code already says.

## Brand

The project this one derives from and its character must never be named in code, comments, assets
or docs. Only `NOTICE` mentions them (MIT attribution). `scripts/check-brand.sh` (CI) enforces it.
Anything copied from older prototypes is cleaned before it is committed.

## Stack

- Rust (edition 2024) for every backend crate and tool; TypeScript only to render the UI (Tauri 2).
- No Swift, no Go.

## Architecture

```
vults/
├── app/                  # Tauri shell: one runtime loop owns State and executes Effects; installer commands; tray
├── ui/                   # Vite + TS renderer, one HTML page per surface; lab/ (/lab/, dev only)
│   └── src/
│       ├── island/       # the island (index.html): Zeca, the flock, cards, dock, chat
│       ├── character/    # sprite data and drawing: Zeca, the flock's species, looks
│       ├── surfaces/     # every other desktop surface, one folder each (widget/, settings/)
│       └── bridge.ts     # the only Tauri caller; shared code (dom, sound, view.gen.ts…) sits beside it
├── crates/
│   ├── core/             # pure domain: reduce(State, Input, now) -> Vec<Effect>, State::view(); no IO, no async
│   ├── protocol/         # versioned hook <-> app wire format, limits, endpoint names (no tokio)
│   ├── hook/             # vults-hook: the relay every agent runs (std + serde_json only: it starts on every agent event)
│   ├── ipc/              # async server: limits, ack-then-decide, Incoming / ReplyHandle (no Tauri)
│   ├── peer/             # same-user checks for the socket / pipe (SO_PEERCRED, SIDs)
│   ├── agents/           # per agent: event names, tool -> Activity, install entries (Claude, Codex, Gemini CLI)
│   ├── agent-config/     # safe edits of agent configs: strict read, diff, fingerprint, dated backup, atomic write
│   ├── connectors/       # Vults Connect: Connector trait + polling runtime (snapshot diffs) + GitHub via gh
│   ├── platform/         # Linux surface placement (layer-shell + input region) and jump-to-terminal; no Tauri
│   ├── chat/             # chat through the claude / codex CLIs (permissions asked through an Approver), or the API with a key
│   ├── voice/            # push-to-talk for the chat: mic into memory (cpal), whisper.cpp transcription, checked model downloads
│   ├── media/            # what is playing (MPRIS over D-Bus) and its controls; off until the user turns it on
│   ├── secrets/          # the OS keyring, the only place a secret is ever written
│   └── brand/            # the name in one place; generates ui/src/brand.ts (a test checks it is fresh)
├── design/               # sprite sources (design/mascots/zeca/zeca.py generates the sprite JSON)
├── docs/                 # user docs (guide/, reference/), published on each release; docs change in the same PR as the feature
│   ├── adr/              # decision records
│   └── dev/              # internal plans (road-to-0.2.md), not published
├── tests/                # visual/: Playwright screenshots of the lab
├── packaging/            # AUR PKGBUILD, .desktop entry
├── scripts/              # CI checks (brand, English), perf
└── .github/              # CI, release and docs workflows
```

Three layers, and a crate only depends on its own or a lower one (`scripts/check-layers.sh`, CI):
**Core** (agents, sessions, decisions), **Connect** (connectors, chat, voice, media) and
**Experience** (app, platform, ui). Zeca lives in Experience: he uses Chat, Voice and Connect,
and none of them knows about him. See `docs/architecture.md`, *Layers*.

## Run

```
npm install
npm run tauri dev      # builds the release hook first, then the UI and the app
```

Linux needs `webkit2gtk-4.1`, `gtk3`, `gtk-layer-shell`, `libayatana-appindicator`, `openssl`, `alsa-lib`, `gst-plugins-good`,
`vulkan-icd-loader`; building also needs `cmake`, `vulkan-headers` and `shaderc` (whisper.cpp on the GPU,
for the chat's voice).

Island gotchas (Linux, KWin): the layer surface is mapped once at a fixed size and never resized
or hidden; only the island's rectangle takes the mouse (input region). Measure the DOM
synchronously: `requestAnimationFrame` is paused while WebKit thinks the page is hidden.

## Priorities

Linux first and only, until everything is refined; Windows and macOS come at the end. Order of focus:
it works, the UI, the docs; distribution after.

## Rules that never bend

Each rule has its reason in `docs/adr/` (0003 to 0006). Changing one starts with a new record that
supersedes the old one ([0001](docs/adr/0001-record-decisions.md)).

1. **Never block an agent.** The hook exits 0 with empty stdout on any failure.
2. A permission is only answered from a human's click (`core::Intent::Decide`).
3. Never write an agent's config without a dated backup, a diff the user saw, and a click.
   Preserve third-party hooks. Never write Codex's `trusted_hash`.
4. Secrets only in the OS keyring. No telemetry.

## Before every commit

```
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-brand.sh && scripts/check-english.sh && scripts/check-layers.sh
npm run test:visual        # after UI or sprite changes; `-- -u` accepts a new look on purpose
```

Visual tests (`tests/visual/`, Playwright) screenshot the lab's island states and every clip with
time frozen (`?still=1&t=1500`). The baselines depend on this machine's fonts, so they run locally,
not in CI. They are strict (color threshold 0.02): on a dark UI most changes are subtle shades.
