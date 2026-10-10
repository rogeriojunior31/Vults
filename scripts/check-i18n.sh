#!/usr/bin/env bash
# Fails when a sentence the UI translates is missing from a catalog (scripts/check-i18n.mjs).
set -euo pipefail
cd "$(dirname "$0")/.."
exec node scripts/check-i18n.mjs
