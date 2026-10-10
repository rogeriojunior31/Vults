#!/usr/bin/env bash
# Fails when Rust code reaches the network outside scripts/network-allowlist.txt (ADR 0019,
# docs/safety.md, "Every request Vults makes"): an HTTP client (reqwest, ureq, hyper), a raw
# TcpStream::connect, or a network CLI spawned by name (gh, curl, wget, codex, claude). Each
# allowlist line is a file and why it may; a new line is a decision for the review. A line whose
# file no longer matches fails too, so the list stays the truth.
# `--self-test` checks that a new reqwest::get outside the list fails.
set -euo pipefail
cd "$(dirname "$0")/.."

PATTERN='reqwest::|\bureq\b|hyper::|TcpStream::connect|Command::new\("(gh|curl|wget|codex|claude)"'

check() {
  local root=$1 list=$2 failed=0
  local allowed found
  allowed=$(grep -vE '^\s*(#|$)' "$list" | awk '{print $1}' | sort -u)
  found=$(cd "$root" && grep -rlE --include='*.rs' --exclude-dir=target "$PATTERN" crates app 2>/dev/null | sort -u || true)
  for f in $found; do
    if ! grep -qxF "$f" <<<"$allowed"; then
      echo "check-network: $f reaches the network and is not in scripts/network-allowlist.txt:" >&2
      (cd "$root" && grep -nE "$PATTERN" "$f" | sed 's/^/  /') >&2
      failed=1
    fi
  done
  for f in $allowed; do
    if ! grep -qxF "$f" <<<"$found"; then
      echo "check-network: $f is in scripts/network-allowlist.txt but no longer reaches the network: remove its line" >&2
      failed=1
    fi
  done
  return $failed
}

if [[ ${1:-} == --self-test ]]; then
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  mkdir -p "$tmp/crates/x/src" "$tmp/app"
  echo 'async fn f() { let _ = reqwest::get("https://example.com").await; }' >"$tmp/crates/x/src/lib.rs"
  : >"$tmp/list"
  if check "$tmp" "$tmp/list" 2>/dev/null; then
    echo "check-network: self-test failed: an unlisted reqwest::get passed" >&2
    exit 1
  fi
  echo "crates/x/src/lib.rs  a listed file passes" >"$tmp/list"
  check "$tmp" "$tmp/list" || { echo "check-network: self-test failed: a listed file failed" >&2; exit 1; }
  echo "check-network: self-test ok"
  exit 0
fi

check . scripts/network-allowlist.txt
echo "check-network: ok"
