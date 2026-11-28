# Releasing

A release is a tag. Everything after the tag is automatic except one click.

## Rules

These are process, not architecture.

- Releases are small and frequent. A patch number does not mean "fixes only", and may pass 9. A
  version is chosen at its release gate from what is validated on `main`; a phase never reserves
  a version number.
- Every tag follows the human **R3** smoke in the real app, the required native acceptance and
  green CI including Security, all recorded against the candidate's source revision. Choosing a
  version or passing automated tests does not complete a desktop acceptance gate.
- Experimental features keep their opt-in and preview status until their own gates pass.
- Linux first through 0.2.x, with Android as the one exception; Windows and macOS betas
  may land late in 0.2.x. 1.0 means "done for Linux".
- The next target and its open gates are in the [roadmap](ROADMAP.md).

## Steps

For R3 before a tag, `npm run bundle:linux` builds local production candidates without publishing.
Keep their actual package version and source revision explicit; changing the release target alone
still leaves package metadata at the current development version. Archive inspection must find the
app, bundled hook, LICENSE, NOTICE, desktop entry and icons, with the notices matching the checkout.
Store package checksums and preflight results with the candidate. Headless entry-point checks and
archive extraction do not count as package installation or native desktop acceptance. The
release workflow's Ubuntu 22.04 builds remain the Linux distribution artifacts; local packages
only establish preflight on their build host.

1. **On a branch, before the tag:**
   - set the version in `Cargo.toml` (`[workspace.package]`), `package.json`, `app/tauri.conf.json`
     and `packaging/aur/vults/PKGBUILD` (`pkgver`, `pkgrel=1`); `scripts/check-version.sh` checks
     they agree;
   - in `CHANGELOG.md`, rename **Unreleased** to `## X.Y.Z (YYYY-MM-DD)` and open a new empty
     **Unreleased** above it: that section becomes the release notes;
   - the Portuguese docs and README are current (CI already fails when one is behind);
   - merge it.
2. **Tag `main`:** `git tag vX.Y.Z && git push origin vX.Y.Z`.
   - `release.yml` refuses a tag that is not the files' version or has no CHANGELOG section, builds
     the `.deb`, `.rpm` and the Windows installer, and opens a **draft** release with the notes and
     `SHA256SUMS`.
   - `docs.yml` validates `docs/` and tells the website: the docs at
     <https://rogeriojunior31.github.io/en/docs/vults/> move to this tag in about two minutes
     (needs the `SITE_DISPATCH_TOKEN` secret; without it, the site's daily build picks it up).
3. **Review the draft** (install a package, read the notes) and click **Publish**.
4. **AUR:** copy `packaging/aur/vults/PKGBUILD` into the `ssh://aur@aur.archlinux.org/vults.git`
   clone, `makepkg --printsrcinfo > .SRCINFO`, build it once with `makepkg -si`, commit both and
   push. `vults-git` only needs a push when its PKGBUILD changes.

A release goes out whenever a step or two are merged and the R3 smoke passes (the island, a card,
the chat, voice, GitHub); patch numbers may pass 9.

The website reads, at the tag: every page in `docs/` but `docs/dev/`, `docs/site.json` (name and
summary in each language), `docs/assets/logo.png` and the island screenshots in `docs/assets/`.
Renaming or removing one of those screenshots needs the same change on the website
(`layouts/partials/vults-showcase.html`).
