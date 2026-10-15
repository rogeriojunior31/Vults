# CLAUDE.md — Vults

Desktop app: Zeca, an 8-bit vulture, and his flock (the vults) perch at the top of the screen and show
what Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI, Cursor, Factory Droid and Kimi Code sessions are doing, with approvals, chat, dropped
files and connectors (GitHub first).

## Language

**English is the project language**: code, identifiers, comments, UI text, errors, tests, docs and
commit messages. `scripts/check-english.sh` (CI) rejects Portuguese accents; a line that must keep one
carries `check-english:allow`. The app speaks English, Brazilian Portuguese, Spanish and Simplified
Chinese: a sentence the user reads is written in English as a whole, never assembled from fragments,
and translated into the other three in the same PR. The core's sentences live in
`crates/core/src/i18n.rs` (one match arm per language); the UI's go through `t("English", {vars})`,
or `tk()` for a label kept in a table, with their translations in `ui/src/i18n/{pt-BR,es,zh}.ts`.
`scripts/check-i18n.sh` (CI) fails on a sentence missing from a catalog.

The docs and the README are in English and Brazilian Portuguese: `docs/pt-br/<path>` translates
`docs/<path>`, `README.pt-br.md` translates `README.md`. A PR that changes an English page updates its
translation too, and sets the `<!-- source: <mark> -->` under its title to
`sha256sum <original> | cut -c1-12` only after reviewing it. CI fails on a missing or stale one
(`docs.yml` for `docs/`, `scripts/check-readme-pt-br.sh` for the README). In a translation, every
heading below the title keeps the English anchor (`<a id="english-slug"></a>` above it; none when the
heading reads the same in both languages, such as a product name: the id would be doubled), and links
to images or files outside `docs/` gain one `../`. Only those two places hold Portuguese.

Comments are short and say *why* (invariant, gotcha, security), not what the code already says.

## Brand

The project this one derives from and its character must never be named in code, comments, assets
or docs. Only `NOTICE` mentions them (MIT attribution). `scripts/check-brand.sh` (CI) enforces it.
Anything copied from older prototypes is cleaned before it is committed.

## Stack

- Rust (edition 2024) for every backend crate and tool; TypeScript only to render the UI (Tauri 2).
- Kotlin only for the Android shell, over the Rust core (ADR 0004).
- No Swift, no Go.

## Architecture

```
vults/
├── app/                  # Tauri shell: one runtime loop owns State and executes Effects; installer commands; tray
├── ui/                   # Vite + TS renderer, one HTML page per surface
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
│   ├── agents/           # per agent: event names, tool -> Activity, install entries (Claude, Codex, Antigravity, OpenCode, Qwen, Copilot, Cursor, Droid, Kimi); the AgentDriver trait
│   ├── agent-config/     # safe edits of agent configs: strict read, diff, fingerprint, dated backup, atomic write
│   ├── store/            # the local database: one vults.sqlite, numbered migrations, the history of turns, the audit log, consents, traces
│   ├── connectors/       # Vults Connect: Connector trait + polling runtime (snapshot diffs) + GitHub via gh
│   ├── platform/         # Linux surface placement (layer-shell + input region), tray, global shortcuts, jump-to-terminal; no Tauri
│   ├── zeca/             # Zeca's brain (ADR 0018): a read-only view of the flock, his memory in git; no vults-core
│   ├── link/             # the paired phone (ADR 0004): pairing by QR, sealed envelopes, signed answers for core's ledger; no IO
│   ├── relay/            # the phone's relay: store-and-forward of sealed envelopes (axum, Docker), and its client through the gate
│   ├── evals/            # evals with cassettes (dev only): policy, redaction, prompts, injection; --features evals
│   ├── spike/            # the spike kit (dev only): fake model, fake app, a terminal for TUIs; scripts/spike.sh records an agent
│   ├── chat/             # chat through the claude / codex CLIs (permissions asked through an Approver), or the API with a key
│   ├── voice/            # push-to-talk for the chat: mic into memory (cpal), whisper.cpp transcription, checked model downloads
│   ├── media/            # what is playing (MPRIS over D-Bus) and its controls; off until the user turns it on
│   ├── secrets/          # the OS keyring, the only place a secret is ever written
│   └── brand/            # the name in one place; generates ui/src/brand.ts (a test checks it is fresh)
├── design/               # sprite sources (design/mascots/zeca/zeca.py generates the sprite JSON)
├── docs/                 # user docs (guide/, reference/), published on each release; docs change in the same PR as the feature
│   ├── adr/              # decision records
│   └── dev/              # internal plans (plan-zeca.md is the roadmap), not published
├── tests/                # automated checks
├── packaging/            # AUR PKGBUILDs (vults, vults-git), .desktop entry
├── scripts/              # CI checks (brand, English, layers, network, version), perf
└── .github/              # CI, security, release and docs workflows
```

Three layers, and a crate only depends on its own or a lower one (`scripts/check-layers.sh`, CI):
**Core** (agents, sessions, decisions), **Connect** (connectors, chat, voice, media) and
**Experience** (app, platform, ui). Zeca's body lives in Experience; his brain, when it lands, is a
`zeca` crate in Connect (ADR 0018). He uses Chat, Voice and Connect, none of them knows about him,
and he never answers a permission. See `docs/architecture.md`, *Layers*.

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

Linux first and only, until everything is refined; Windows and macOS come at the end. The one
exception is the Android companion that answers cards (ADR 0004). Order of focus:
it works, the UI, the docs; distribution after.

## UI and prototypes

Read [DESIGN.md](DESIGN.md) before changing UI or creating a prototype. Reuse the
production theme, components, sprites and interaction owners; preserve accessible
actions and state across layouts. The implementation order is in
[Plan Zeca](docs/dev/plan-zeca.md#ui-delivery-sequence), detailed in
[UI implementation sequence](docs/dev/plan-ui.md): repair existing views and
deliver larger UI changes with their features. Validate UI changes in the running
application.

## Rules that never bend

Each rule has its reason in `docs/adr/`: 0003 for rule 1; 0004 for rule 2 (clicks, rules, policies,
destructive calls, the phone and the chat's access); 0005 for rule 3; 0019 for rule 4 (the keyring,
no telemetry, the update check and requests on the user's behalf). Changing one starts with an edit to
its living record, with a dated line in its *History* ([0001](docs/adr/0001-record-decisions.md)).

1. **Never block an agent.** The hook exits 0 with empty stdout on any failure.
2. A permission is only answered by a human: a click (`core::Intent::Decide`, `DecideAlways`), an
   answer from a paired phone signed by its key (biometrics for an approval), an exact *Always*
   rule they made, or a policy they wrote, saw as a diff and accepted with a click (destructive
   actions always ask, rules included, a deny beats an allow, every answer is audited). In Zeca's
   chat only, also what they chose in Settings for its cards: their policies, or full access
   (destructive actions still ask, a policy's deny or ask still holds, content from outside brings
   cards back, every answer is audited).
3. Never write an agent's config without a dated backup, a diff the user saw, and a click.
   Preserve third-party hooks. Never write Codex's `trusted_hash`.
4. Secrets only in the OS keyring. No telemetry. The app makes a request on its own only when the
   user turned that feature on: the update check (not built yet; at most daily) and each request
   on the user's behalf (connectors, web tools the model calls, Zeca's sleep, routines, the phone's
   relay and push), with its own switch, budget and local trace, through one network gate in `app`
   (`app/src/gate.rs`).

## Before every commit

```
npm run build
node scripts/check-island-geometry.mjs  # dock anchors and initial input region, no browser server
node scripts/check-record-search.mjs  # local records search, no browser server
node scripts/check-nest-close.mjs  # saved close, failure and retry, no desktop server
node scripts/check-chat-models.mjs  # model read failure and explicit retry, no browser server
node scripts/check-flock-place.mjs  # retained flock navigation, no browser server
node scripts/check-project-search.mjs  # retained project search, no browser server
node scripts/check-task-actions.mjs  # task disclosures and confirmations, no browser server
node scripts/check-surface-continuity.mjs  # Nest/Quick handoff, drafts, owners and card visibility
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p vults-evals --features evals   # policy, redaction, prompts, injection (crates/evals)
scripts/check-brand.sh && scripts/check-english.sh && scripts/check-i18n.sh && scripts/check-layers.sh && scripts/check-network.sh && scripts/check-readme-pt-br.sh
cargo deny check           # after a dependency change (security.yml runs it on those PRs and weekly)
```

Validate renderer changes in the running application. The repository has no separate browser Lab or
Playwright screenshot suite.
