#!/usr/bin/env bash
# Fails when the release hook's cold start, with the app closed, goes above a ceiling. Every
# agent runs the hook on every event (rule 1: never block an agent), so its start time is
# the cost of having Vults installed. Needs hyperfine and jq.
#
# Measured 2026-10-10: about 1 ms on a desktop and on the GitHub runner (see the CI log). The
# ceiling leaves room for a noisy runner and still catches a new runtime or a heavy crate.
set -euo pipefail
cd "$(dirname "$0")/.."

CEILING_MS=10
hook=${1:-target/release/vults-hook}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# An empty runtime folder: the app is closed, the hook finds no socket and gives up at once.
mkdir -m 700 "$work/run"
printf '%s' '{"hook_event_name":"PreToolUse","session_id":"s","cwd":"/tmp","tool_name":"Bash","tool_input":{"command":"ls"}}' >"$work/event.json"

XDG_RUNTIME_DIR="$work/run" hyperfine --shell=none --warmup 5 --runs 100 \
  --input "$work/event.json" --output null --export-json "$work/result.json" \
  "$hook --agent claude PreToolUse"

mean_ms=$(jq '.results[0].mean * 100000 | round / 100' "$work/result.json")
if awk -v m="$mean_ms" -v c="$CEILING_MS" 'BEGIN { exit !(m > c) }'; then
  echo "check-hook-start: mean ${mean_ms} ms is above the ${CEILING_MS} ms ceiling"
  exit 1
fi
echo "check-hook-start: ok (mean ${mean_ms} ms, ceiling ${CEILING_MS} ms)"
