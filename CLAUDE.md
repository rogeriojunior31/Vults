# CLAUDE.md — Vultures AI

Desktop app: Zeca, an 8-bit vulture, and his flock (the vults) perch at the top of the screen and show
what Claude Code and Codex sessions are doing, with approvals, chat, dropped files and connectors
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
crates/
├── brand/       # the name in one place; generates ui/src/brand.ts (a test checks it is fresh)
├── protocol/    # versioned hook <-> app wire format, limits, endpoint names (no tokio)
├── peer/        # same-user checks for the socket / pipe (SO_PEERCRED, SIDs)
├── hook/        # vultures-ai-hook: the relay every agent runs (std + serde_json only: it starts on every agent event)
├── ipc/         # async server: limits, ack-then-decide, Incoming / ReplyHandle (no Tauri)
└── core/        # pure domain: reduce(State, Input, now) -> Vec<Effect>; no IO, no async
ui/              # Vite + TS renderer (from M2)
docs/            # user docs, published to rogeriojunior31.github.io/docs/vultures-ai/ on each release
```

## Rules that never bend

1. **Never block an agent.** The hook exits 0 with empty stdout on any failure.
2. A permission is only answered from a human's click (`core::Intent::Decide`).
3. Never write an agent's config without a dated backup, a diff the user saw, and a click.
   Preserve third-party hooks. Never write Codex's `trusted_hash`.
4. Secrets only in the OS keyring. No telemetry.

## Before every commit

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-brand.sh && scripts/check-english.sh
```
