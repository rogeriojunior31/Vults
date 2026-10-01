#!/usr/bin/env bash
# Fails when a tracked file names the project this one derives from or its character.
# Only NOTICE may (attribution required by the MIT license). The pattern is split
# so this script does not match itself.
set -euo pipefail
cd "$(dirname "$0")/.."

hits=$(git grep -nIiE 'cou''cou|mo''chi' -- . ':(exclude)NOTICE' || true)
if [ -z "$hits" ]; then
  echo "check-brand: ok"
  exit 0
fi
echo "$hits"
echo
echo "check-brand: forbidden names found (only NOTICE may mention them)"
exit 1
