#!/usr/bin/env bash
# Seeds the fuzz corpus (fuzz/corpus/<target>/) from the agents' recorded hook payloads: real
# shapes give libFuzzer a head start. Run before `cargo +nightly fuzz run`.
set -euo pipefail
cd "$(dirname "$0")/.."

out=fuzz/corpus
version=$(sed -n 's/^pub const VERSION: u32 = \([0-9]*\);/\1/p' crates/protocol/src/lib.rs)
[ -n "$version" ] || { echo "fuzz-corpus: no protocol VERSION found" >&2; exit 1; }
mkdir -p "$out"/{decode_event,reply,agent_event,hook_event,redact}

byte() { printf "\\$(printf '%03o' "$1")"; }

n=0
for f in crates/agents/tests/fixtures/*.jsonl; do
  stem=$(basename "$f" .jsonl)
  # agent_event: one whole session per agent pick (0..7, the high bit for "waits"), a name byte
  # before each line.
  for pick in 0 1 2 3 4 5 6 7 128 129 130 131 132 133 134 135; do
    { byte "$pick"; while IFS= read -r line; do [ -n "$line" ] && { byte 0; printf '%s\n' "$line"; }; done <"$f"; } \
      >"$out/agent_event/$stem-$pick"
  done
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    n=$((n + 1))
    # Antigravity's lines are [event, payload]: the hook's stdin is the payload.
    payload=$line
    case $line in \[*) payload=$(printf '%s' "$line" | sed -E 's/^\["[^"]*", *(.*)\]$/\1/') ;; esac
    for pick in 0 1 2 3 4 5 128 129; do
      { byte "$pick"; byte 2; printf '%s' "$payload"; } >"$out/hook_event/$n-$pick"
    done
    printf '{"kind":"event","v":%s,"id":"seed-%s","agent":"claude","event":"PreToolUse","wants_reply":true,"payload":%s}' \
      "$version" "$n" "$payload" >"$out/decode_event/$n"
    printf '%s' "$payload" >"$out/redact/$n"
  done <"$f"
done
for r in '{"kind":"waiting","v":V,"id":"x"}' '{"kind":"decision","v":V,"id":"x","decision":"allow"}' \
  '{"kind":"decision","v":V,"id":"x","decision":"deny"}'; do
  printf '%s' "${r//V/$version}" >"$out/reply/$(printf '%s' "$r" | cksum | cut -d' ' -f1)"
done
for cmd in 'API_TOKEN=x; rm -rf ~' 'Authorization: Bearer [a-fake-token]' \
  'git push --force origin main' 'mysql -u root -phunter2 db'; do
  printf '%s' "$cmd" >"$out/redact/cmd-$(printf '%s' "$cmd" | cksum | cut -d' ' -f1)"
done
echo "fuzz-corpus: $n payloads, protocol v$version"
