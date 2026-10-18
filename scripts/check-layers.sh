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
    protocol | peer | ipc | core | hook | agents | agent-config | store) echo 1 ;;
    connectors | chat | audio | voice | dictation | media | zeca | link | relay | evals | spike) echo 2 ;;
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

# Zeca's brain never reaches the core, not even through another crate: nothing in it may build an
# Intent (ADR 0018). `cargo tree` must succeed for the check to mean anything.
zeca_tree=$(cargo tree -p vults-zeca -e normal,build --prefix none 2>/dev/null) || {
  echo "check-layers: cargo tree failed for vults-zeca"
  exit 1
}
if grep -q '^vults-core ' <<< "$zeca_tree"; then
  echo "zeca depends on vults-core (directly or not): Zeca must not reach the core's Intent"
  failed=1
fi

if [ "$failed" = 1 ]; then
  echo
  echo "check-layers: a lower layer may not depend on a higher one"
  exit 1
fi
echo "check-layers: ok"
