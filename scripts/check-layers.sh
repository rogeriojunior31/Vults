#!/usr/bin/env bash
# Fails when a crate depends on a higher layer (docs/architecture.md, "Layers"):
# base < Core < Connect < Experience. Only [dependencies] count; tests may reach further.
set -euo pipefail
cd "$(dirname "$0")/.."

layer() {
  case "$1" in
    brand | secrets) echo 0 ;;
    protocol | peer | ipc | core | hook | agents | agent-config) echo 1 ;;
    connectors | chat | voice | media) echo 2 ;;
    platform | app) echo 3 ;;
    *) echo "check-layers: crate '$1' has no layer: add it here and in docs/architecture.md" >&2; exit 1 ;;
  esac
}

failed=0
for manifest in crates/*/Cargo.toml app/Cargo.toml; do
  crate=$(basename "$(dirname "$manifest")")
  own=$(layer "$crate")
  deps=$(awk '/^\[/{on = ($0 == "[dependencies]")} on && /^vults-/{sub(/[ .=].*/, ""); sub(/^vults-/, ""); print}' "$manifest")
  for dep in $deps; do
    if [ "$(layer "$dep")" -gt "$own" ]; then
      echo "$crate (layer $own) depends on $dep (layer $(layer "$dep"))"
      failed=1
    fi
  done
done

if [ "$failed" = 1 ]; then
  echo
  echo "check-layers: a lower layer may not depend on a higher one"
  exit 1
fi
echo "check-layers: ok"
