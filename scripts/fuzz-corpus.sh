#!/usr/bin/env bash
# Seeds the fuzz corpus (fuzz/corpus/<target>/) from the agents' recorded hook payloads, one file
# per line: real shapes give libFuzzer a head start. Run before `cargo +nightly fuzz run`.
set -euo pipefail
cd "$(dirname "$0")/.."

out=fuzz/corpus
mkdir -p "$out/decode_event" "$out/reply" "$out/agent_event"
n=0
for f in crates/agents/tests/fixtures/*.jsonl; do
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    n=$((n + 1))
    # agent_event: a picker byte per agent (0..5, the high bit for "waits"), a name byte, the payload.
    for pick in 0 1 2 3 4 5 133; do
      { printf "\\$(printf '%03o' "$pick")\\000"; printf '%s' "$line"; } >"$out/agent_event/$n-$pick"
    done
    # decode_event: the payload wrapped as the hook sends it.
    printf '{"kind":"event","v":5,"id":"seed-%s","agent":"claude","event":"PreToolUse","wants_reply":true,"payload":%s}' "$n" "$line" \
      >"$out/decode_event/$n"
  done <"$f"
done
printf '%s' '{"kind":"waiting","v":5,"id":"x"}' >"$out/reply/waiting"
printf '%s' '{"kind":"decision","v":5,"id":"x","decision":"allow"}' >"$out/reply/allow"
printf '%s' '{"kind":"decision","v":5,"id":"x","decision":"deny"}' >"$out/reply/deny"
echo "fuzz-corpus: $n payloads"
