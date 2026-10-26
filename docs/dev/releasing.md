# Releasing

A release is a tag. Everything after the tag is automatic except one click.

The next target is **0.1.9**, recorded by REL-1 in
[ADR 0020](../adr/0020-0.2.0-at-wave-4-then-0.2.x.md). It includes the current-main snapshot and
accepted stabilization fixes. R3, the required native acceptance and green CI including Security
must be recorded for the release candidate before REL-2 changes versions or creates a tag.
Pending preview/paid-provider gates are not completed by choosing this version; existing preview
status and opt-ins remain. Keep package versions and Unreleased unchanged until that release step.


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
