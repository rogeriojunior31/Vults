## What and why

<!-- One paragraph. Link the issue: "Closes #…". -->

## Layer

<!-- Core, Connect, Experience, docs, CI. See docs/architecture.md, "Layers". -->

## Checks

- [ ] `npm run build`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- [ ] `scripts/check-brand.sh && scripts/check-english.sh && scripts/check-layers.sh`
- [ ] UI or sprite change: `npm run test:visual` (a new look accepted on purpose with `-- -u`), screenshot below
- [ ] Docs changed in this PR when the feature did
- [ ] None of the rules that never bend changes, or an ADR in `docs/adr/` does it
