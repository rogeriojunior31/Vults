## What and why

<!-- One paragraph. Link the issue: "Closes #…". -->

## Layer

<!-- Core, Connect, Experience, docs, CI. See docs/architecture.md, "Layers". -->

## Checks

- [ ] `npm run build`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- [ ] `scripts/check-brand.sh && scripts/check-english.sh && scripts/check-layers.sh && scripts/check-readme-pt-br.sh`
- [ ] UI or sprite change: validated in the running application, with a screenshot below
- [ ] A prompt, the policy or the redactor changed: `cargo test -p vults-evals --features evals` passes, a moved case is a new case or an accepted snapshot (`VULTS_EVALS_BLESS=1`), and the diff shows it
- [ ] Docs changed in this PR when the feature did
- [ ] Their translations too (`docs/pt-br/`, `README.pt-br.md`), with the new `source:` mark
- [ ] None of the rules that never bend changes, or an ADR in `docs/adr/` does it
