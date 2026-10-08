#!/usr/bin/env bash
# Runs the visual tests inside the pinned Playwright image, so this machine and CI render the
# same pixels: the baselines depend on the fonts and the browser build, not only on the code.
#
#   scripts/visual.sh          compare (npm run test:visual)
#   scripts/visual.sh -u       accept a new look on purpose
set -euo pipefail
cd "$(dirname "$0")/.."

# Already in the image (CI's container job, or the call below): run directly.
if [[ -d /ms-playwright ]]; then
  exec npx playwright test -c tests/visual/playwright.config.ts "$@"
fi

# The image must match the installed @playwright/test, or its browsers are not the ones it expects.
version=$(node -p 'require("@playwright/test/package.json").version')
image="mcr.microsoft.com/playwright:v${version}-noble"

engine=$(command -v podman || command -v docker || true)
if [[ -z "$engine" ]]; then
  echo "visual.sh: needs podman or docker to run $image" >&2
  exit 1
fi

# Same uid inside, so new baselines are owned by the user (rootless podman maps root to them).
user=()
[[ "$(basename "$engine")" == docker ]] && user=(--user "$(id -u):$(id -g)")

# node_modules comes from the host (Linux x64, glibc, like the image).
exec "$engine" run --rm --init --ipc=host "${user[@]}" \
  -e CI -e VISUAL_PORT -e HOME=/tmp \
  -v "$PWD:/work" -w /work \
  "$image" \
  npx playwright test -c tests/visual/playwright.config.ts "$@"
