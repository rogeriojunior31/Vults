#!/usr/bin/env bash
# Fails unless Cargo.toml, package.json, app/tauri.conf.json and the AUR release PKGBUILD carry
# one version and, given a tag, the tag is that version with a leading "v": a release must never
# ship packages labelled with another version.
#
#   scripts/check-version.sh           the four files agree
#   scripts/check-version.sh v0.1.0    they agree, equal the tag and CHANGELOG.md has its section
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${1-}"

# [workspace.package] only: every crate inherits it (version.workspace = true).
cargo=$(awk '/^\[/ { s = ($0 == "[workspace.package]") }
  s && /^version[ \t]*=/ { gsub(/^[^"]*"|".*$/, ""); print; exit }' Cargo.toml)
npm=$(node -p 'require("./package.json").version')
tauri=$(node -p 'require("./app/tauri.conf.json").version')
# The AUR release package builds the tag v$pkgver: a stale one would package the last release.
aur=$(sed -n 's/^pkgver=//p' packaging/aur/vults/PKGBUILD)

echo "Cargo.toml:          ${cargo:-<none>}"
echo "package.json:        ${npm:-<none>}"
echo "app/tauri.conf.json: ${tauri:-<none>}"
echo "aur/vults/PKGBUILD:  ${aur:-<none>}"

if [ -z "$cargo" ] || [ "$cargo" != "$npm" ] || [ "$cargo" != "$tauri" ] || [ "$cargo" != "$aur" ]; then
  echo "check-version: the versions differ; set them all to the release's version" >&2
  exit 1
fi
if [ -n "$tag" ] && [ "$tag" != "v$cargo" ]; then
  echo "check-version: tag $tag does not match version $cargo (expected v$cargo)" >&2
  exit 1
fi
# The release notes are this section (release.yml): a tag without one would ship empty notes.
if [ -n "$tag" ] && ! grep -q "^## $cargo " CHANGELOG.md; then
  echo "check-version: CHANGELOG.md has no \"## $cargo (date)\" section; move Unreleased there" >&2
  exit 1
fi
echo "check-version: ok${tag:+ ($tag)}"
