#!/usr/bin/env bash
# Fails when a crate depends on a higher layer (docs/architecture.md, "Layers"):
# base < Core < Connect < Experience. Normal and build dependencies count, per target too
# (`[target.'cfg(...)'.dependencies]`); dev-dependencies do not: tests may reach further.
# Reads `cargo metadata` rather than the manifests, so no table form is missed.
set -euo pipefail
cd "$(dirname "$0")/.."

layer() {
  case "$1" in
    brand | secrets) echo 0 ;;
    # store: planned (plan-zeca S1).
    protocol | peer | ipc | core | hook | agents | agent-config | store) echo 1 ;;
    # zeca, link, relay: planned (plan-zeca S9, L1, L2).
    connectors | chat | voice | media | zeca | link | relay) echo 2 ;;
    platform | app) echo 3 ;;
    *) echo "check-layers: crate '$1' has no layer: add it here and in docs/architecture.md" >&2; exit 1 ;;
  esac
}

# One "crate dep" line per dependency on another workspace crate (a path dependency); names lose
# "vults-". Each crate also gets a line to itself, so a crate nothing touches still needs a layer.
edges=$(cargo metadata --format-version 1 --no-deps --offline | jq -r '
  .packages[]
  | (.name | sub("^vults-"; "")) as $crate
  | "\($crate) \($crate)",
    (.dependencies[]
     | select(.kind != "dev" and .path != null)
     | "\($crate) \(.name | sub("^vults-"; ""))")' | sort -u)

failed=0
while read -r crate dep; do
  [ -n "$crate" ] || continue
  own=$(layer "$crate")
  theirs=$(layer "$dep")
  if [ "$theirs" -gt "$own" ]; then
    echo "$crate (layer $own) depends on $dep (layer $theirs)"
    failed=1
  fi
done <<< "$edges"

if [ "$failed" = 1 ]; then
  echo
  echo "check-layers: a lower layer may not depend on a higher one"
  exit 1
fi
echo "check-layers: ok"
