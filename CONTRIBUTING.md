# Contributing to Vults

Thanks for helping. Bug reports, agent support, connectors, sprites and docs fixes are all welcome.

## Before you start

- **Bugs:** open an issue; the template asks for your version, distribution and desktop.
- **Features and new agents:** open an issue first, so we agree on the shape before you write
  code. What comes next is in the README, *Where it is going*.
- **Security issues:** do not open a public issue; see [SECURITY.md](SECURITY.md).

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Setup

```sh
git clone https://github.com/rogeriojunior31/Vults
cd Vults
npm install
npm run tauri dev
```

The system libraries are listed in [Getting started](docs/getting-started.md). The lab
(`/lab/` in the dev server) shows every island state and clip without a real agent.

## Where things go

Read [docs/architecture.md](docs/architecture.md) first. In short: the domain is a pure
`reduce` in `crates/core`, every agent's specifics live in `crates/agents`, and a crate only
depends on its own layer or a lower one (Core, Connect, Experience). A new connector follows
[Adding a connector](docs/contributing/connectors.md).

## Rules

- **English** everywhere: code, comments, UI text, docs, commit messages. `scripts/check-english.sh`
  runs in CI.
- **Comments say why**, briefly: an invariant, a gotcha, a security reason.
- **User-facing sentences are whole strings** in the i18n catalog, never built from fragments.
- **The rules that never bend** (the hook never blocks an agent, only a human's click answers a
  permission, no config write without a backup, a diff and a click, secrets only in the keyring)
  change only through a new decision record in [docs/adr/](docs/adr/README.md).

## Documentation is part of the change

A change a user would notice updates its page in `docs/` and adds a line to
[CHANGELOG.md](CHANGELOG.md) under **Unreleased**, in the same PR.

The docs and the README also exist in Brazilian Portuguese (`docs/pt-br/`, `README.pt-br.md`).
Changing an English page means updating its translation as well, and setting the
`<!-- source: … -->` mark under its title to `sha256sum <original> | cut -c1-12`; CI fails when a
translation is missing or behind its original. If you do not write Portuguese, say so in the PR
and the maintainer will translate it. The docs are published at
[rogeriojunior31.github.io/en/docs/vults](https://rogeriojunior31.github.io/en/docs/vults/) on
each release: every page starts with a `# H1` and links are relative.

## Before opening a PR

```sh
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-brand.sh && scripts/check-english.sh && scripts/check-layers.sh && scripts/check-readme-pt-br.sh
npm run test:visual        # after UI or sprite changes
```

CI runs the same steps.

A change to the protocol, the hook, an agent's parser or the redactor can also be fuzzed (nightly
and `cargo install cargo-fuzz` needed; CI does it weekly, and on PRs that touch them). Targets:
`hook_event`, `agent_event`, `decode_event`, `reply`, `redact`.

```sh
scripts/fuzz-corpus.sh
cd fuzz && cargo +nightly fuzz run hook_event corpus/hook_event -- -max_total_time=60
```

## License

By contributing you agree that your contributions are licensed under the [MIT License](LICENSE).
