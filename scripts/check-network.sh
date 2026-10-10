#!/usr/bin/env bash
# Fails when code reaches the network outside scripts/network-allowlist.txt (ADR 0019,
# docs/safety.md, "Every request Vults makes"). Three checks:
#  - crates: a workspace crate that depends on an HTTP or socket client crate (reqwest, hyper,
#    ureq, isahc, attohttpc, surf, curl, tungstenite) must be listed as `crate:<name>`. This holds
#    whatever the code calls it (an alias, a re-export, a shared `client()`), so every file that
#    sends through it is listed too, by the second check's pattern or by hand;
#  - files: a Rust file that names such a client, a socket, `/dev/tcp`, or spawns by name a program
#    that can reach the network (gh, git, curl, wget, codex, claude, a shell, npx, ssh…);
#  - the webview: the CSP's connect-src allows only the app itself.
# A spawn through a variable (`Command::new(bin)`) is not seen: the review is the guard there.
# Each allowlist line is a path (or crate:<name>) and why; a new line is a decision for the
# review. A line that no longer matches fails too, so the list stays the truth.
# `--self-test` checks that an unlisted reqwest::get fails.
set -euo pipefail
cd "$(dirname "$0")/.."

CLIENTS='reqwest|hyper|ureq|isahc|attohttpc|surf|curl|tungstenite|tokio-tungstenite'
PATTERN='\b(reqwest|ureq|hyper|isahc|attohttpc|tungstenite)\b|TcpStream|UdpSocket|/dev/tcp|Command::new\s*\(\s*"(gh|git|curl|wget|codex|claude|sh|bash|zsh|npx|node|python3?|ssh|scp|rsync|nc)(\.exe|\.cmd)?"'

rust_files() {
  if git -C "$1" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git -C "$1" ls-files '*.rs'
  else
    (cd "$1" && find . -name '*.rs' -not -path './target/*' | sed 's|^\./||')
  fi
}

check_files() {
  local root=$1 list=$2 failed=0 allowed found
  allowed=$(grep -vE '^\s*(#|$)' "$list" | awk '{print $1}' | grep -v '^crate:' | sort -u || true)
  found=$(cd "$root" && rust_files . | xargs -r grep -lE "$PATTERN" 2>/dev/null | sort -u || true)
  for f in $found; do
    if ! grep -qxF "$f" <<<"$allowed"; then
      echo "check-network: $f reaches the network and is not in scripts/network-allowlist.txt:" >&2
      (cd "$root" && grep -nE "$PATTERN" "$f" | sed 's/^/  /') >&2
      failed=1
    fi
  done
  for f in $allowed; do
    if ! grep -qxF "$f" <<<"$found"; then
      echo "check-network: $f is in scripts/network-allowlist.txt but no longer matches: remove its line" >&2
      failed=1
    fi
  done
  return $failed
}

check_crates() {
  local list=$1 failed=0 allowed found
  allowed=$(grep -E '^crate:' "$list" | awk '{print $1}' | sed 's/^crate://' | sort -u || true)
  found=$(cargo metadata --format-version 1 --no-deps --offline | jq -r --arg re "^($CLIENTS)$" '
    .packages[] | .name as $c | .dependencies[]
    | select(.kind != "dev" and (.name | test($re))) | $c' | sort -u)
  for c in $found; do
    if ! grep -qxF "$c" <<<"$allowed"; then
      echo "check-network: crate $c depends on an HTTP or socket client and is not listed as crate:$c" >&2
      failed=1
    fi
  done
  for c in $allowed; do
    if ! grep -qxF "$c" <<<"$found"; then
      echo "check-network: crate:$c is listed but no longer depends on a client: remove its line" >&2
      failed=1
    fi
  done
  return $failed
}

check_csp() {
  local csp
  csp=$(jq -r '.app.security.csp' app/tauri.conf.json)
  if ! grep -qE "connect-src 'self' ipc: http://ipc\.localhost(;|$)" <<<"$csp"; then
    echo "check-network: the webview's connect-src must stay 'self' ipc: http://ipc.localhost (app/tauri.conf.json)" >&2
    return 1
  fi
}

if [[ ${1:-} == --self-test ]]; then
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  mkdir -p "$tmp/crates/x/src"
  echo 'async fn f() { let _ = reqwest :: get("https://example.com").await; }' >"$tmp/crates/x/src/lib.rs"
  : >"$tmp/list"
  if check_files "$tmp" "$tmp/list" 2>/dev/null; then
    echo "check-network: self-test failed: an unlisted reqwest::get passed" >&2
    exit 1
  fi
  echo 'fn f() { std::process::Command::new( "gh" ); }' >"$tmp/crates/x/src/lib.rs"
  if check_files "$tmp" "$tmp/list" 2>/dev/null; then
    echo "check-network: self-test failed: an unlisted gh passed" >&2
    exit 1
  fi
  echo "crates/x/src/lib.rs  a listed file passes" >"$tmp/list"
  check_files "$tmp" "$tmp/list" || { echo "check-network: self-test failed: a listed file failed" >&2; exit 1; }
  echo "check-network: self-test ok"
  exit 0
fi

failed=0
check_crates scripts/network-allowlist.txt || failed=1
check_files . scripts/network-allowlist.txt || failed=1
check_csp || failed=1
[[ $failed == 0 ]] || exit 1
echo "check-network: ok"
