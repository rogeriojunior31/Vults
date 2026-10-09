#!/usr/bin/env bash
# Fails when README.pt-br.md was not reviewed against the current README.md. Its first line keeps
# "<!-- source: <mark> -->", the mark of the English README it translates
# (sha256sum README.md | cut -c1-12), like every page in docs/pt-br/ (checked by docs.yml).
set -euo pipefail
cd "$(dirname "$0")/.."

current=$(sha256sum README.md | cut -c1-12)
mark=$(grep -oE '<!--[[:space:]]*source:[[:space:]]*[0-9a-f]{12}' README.pt-br.md | grep -oE '[0-9a-f]{12}$' | head -1 || true)
if [ "$mark" = "$current" ]; then
  echo "check-readme-pt-br: ok"
  exit 0
fi
echo "check-readme-pt-br: README.md changed after README.pt-br.md (mark ${mark:-none}); update the translation and set <!-- source: $current -->" >&2
exit 1
