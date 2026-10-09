# Releasing

A release is a tag. Everything after the tag is automatic except one click.

1. **On a branch, before the tag:**
   - set the version in `Cargo.toml` (`[workspace.package]`), `package.json`, `app/tauri.conf.json`
     and `packaging/aur/vults/PKGBUILD` (`pkgver`, `pkgrel=1`); `scripts/check-version.sh` checks
     they agree;
   - in `CHANGELOG.md`, rename **Unreleased** to `## X.Y.Z (YYYY-MM-DD)` and open a new empty
     **Unreleased** above it: that section becomes the release notes;
   - `node tests/visual/docs-shots.mjs` if the island changed, so the README and the docs show it;
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

The website reads, at the tag: every page in `docs/` but `docs/dev/`, `docs/site.json` (name and
summary in each language), `docs/assets/logo.png` and the island screenshots in `docs/assets/`.
Renaming or removing one of those screenshots needs the same change on the website
(`layouts/partials/vults-showcase.html`).
