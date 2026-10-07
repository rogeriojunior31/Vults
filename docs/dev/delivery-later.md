# Delivery: what comes after 0.2

A proposal for pipelines, distribution and updates, kept here so it is not lost. Most of it waits:
until 0.2, Linux (KDE first) is the only target ([ADR 0015](../adr/0015-everything-by-0.2-in-small-releases.md)),
and distribution comes after the app and its docs (CLAUDE.md, *Priorities*).

## Done now

| Piece | Where |
|---|---|
| CI on every PR: fmt, Clippy, tests, UI build, brand / English / layer checks; Linux and Windows | `.github/workflows/ci.yml` |
| Security: `cargo deny` (advisories, licenses, sources), `npm audit`, gitleaks; on dependency PRs, on main and weekly | `.github/workflows/security.yml`, `deny.toml` |
| Release on a `v*` tag: .deb, .rpm, the Windows installer, SHA256SUMS, a draft release | `.github/workflows/release.yml` |
| Issue forms (bug, task with acceptance criteria and its layer) and a PR checklist | `.github/ISSUE_TEMPLATE/`, `.github/PULL_REQUEST_TEMPLATE.md` |

## Later, and what each one needs first

| Piece | Waits for | Note |
|---|---|---|
| Visual tests in CI | Baselines rendered in a pinned container | Today they depend on this machine's fonts, so a CI run would fail on every shade. One Playwright image for both local runs and CI fixes that. |
| macOS in the build matrix | The macOS work after 0.2 | A compile check is cheap; a .dmg needs signing and notarization (an Apple developer account). |
| Signed Windows installer | A code-signing certificate | Without it SmartScreen warns on every download. |
| AppImage | A faster build | Dropped in #130: the slowest job by far. |
| Flatpak, AUR release package | Wayland and KDE behaviour settled | The sandbox must still reach the socket, the agents' configs and layer-shell. `vults-git` exists in `packaging/aur/`. |
| Tauri updater, with stable / beta / nightly channels | A new ADR | It is the app's first call home: it must be opt-in or at least visible, and agree with [ADR 0006](../adr/0006-keyring-no-telemetry.md). Each channel gets its own endpoint, so stable never gets a nightly. The signing key lives only in the release job's secrets. |

The CI keeps the fast checks on every PR; the full package builds run on tags (and on PRs that
touch packaging). A green matrix is not a test on the real systems: each release is still tried
on KDE before it is published.
