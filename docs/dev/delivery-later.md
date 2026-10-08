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
| Dependabot: weekly grouped updates for Actions, Cargo and npm (Tauri's crates together) | `.github/dependabot.yml` |
| Visual tests on PRs that touch the UI, in the pinned Playwright image (the same one local runs use) | `.github/workflows/visual.yml`, `scripts/visual.sh` |

## Later, and what each one needs first

| Piece | Waits for | Note |
|---|---|---|
| macOS in the build matrix | The macOS work after 0.2 | A compile check is cheap; a .dmg needs signing and notarization (an Apple developer account). |
| Signed Windows installer | A code-signing certificate | Without it SmartScreen warns on every download. |
| AppImage | A faster build | Dropped in #130: the slowest job by far. |
| Flatpak, AUR release package | Wayland and KDE behaviour settled | The sandbox must still reach the socket, the agents' configs and layer-shell. `vults-git` exists in `packaging/aur/`. |
| Tauri updater, with stable / beta / nightly channels | [ADR 0016](../adr/0016-updates-asked-for-and-signed.md), proposed | Off until the user turns it on, an install only after a click, one manifest per channel compiled into the build, signed packages, and none in packages a distribution updates. |

The CI keeps the fast checks on every PR; the full package builds run on tags (and on PRs that
touch packaging). A green matrix is not a test on the real systems: each release is still tried
on KDE before it is published.
