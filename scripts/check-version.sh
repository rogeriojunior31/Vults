#!/usr/bin/env bash
# Fails unless Cargo.toml, package.json and app/tauri.conf.json carry one version and, given a
# tag, the tag is that version with a leading "v": a release must never ship packages labelled
# with another version.
#
#   scripts/check-version.sh           the three files agree
#   scripts/check-version.sh v0.1.0    they agree and equal the tag
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${1-}"

# [workspace.package] only: every crate inherits it (version.workspace = true).
cargo=$(awk '/^\[/ { s = ($0 == "[workspace.package]") }
  s && /^version[ \t]*=/ { gsub(/^[^"]*"|".*$/, ""); print; exit }' Cargo.toml)
npm=$(node -p 'require("./package.json").version')
tauri=$(node -p 'require("./app/tauri.conf.json").version')

echo "Cargo.toml:          ${cargo:-<none>}"
echo "package.json:        ${npm:-<none>}"
echo "app/tauri.conf.json: ${tauri:-<none>}"

if [ -z "$cargo" ] || [ "$cargo" != "$npm" ] || [ "$cargo" != "$tauri" ]; then
  echo "check-version: the three versions differ; set them all to the release's version" >&2
  exit 1
fi
if [ -n "$tag" ] && [ "$tag" != "v$cargo" ]; then
  echo "check-version: tag $tag does not match version $cargo (expected v$cargo)" >&2
  exit 1
fi
echo "check-version: ok${tag:+ ($tag)}"
