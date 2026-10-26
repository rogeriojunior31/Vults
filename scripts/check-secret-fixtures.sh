#!/usr/bin/env bash
# Historical exceptions must never hide a new credential-shaped value at the same path.
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$(pwd)
scanner=${VULTS_GITLEAKS_BIN:-gitleaks}
if ! command -v "$scanner" >/dev/null; then
  echo "check-secret-fixtures: install Gitleaks or set VULTS_GITLEAKS_BIN" >&2
  exit 1
fi
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/crates/core/src" "$scratch/ui/src/surfaces/settings"
printf 'const TOKEN: &str = "%s%s";\n' 'ghp_' '0123456789abcdefghijklmnopqrstuvwxyz' > "$scratch/crates/core/src/redact.rs"
printf 'const api_key = "%s";\n' '0123456789abcdefghijklmnopqrstuvwxyz' > "$scratch/ui/src/surfaces/settings/main.ts"
git -C "$scratch" init -q
git -C "$scratch" add .
git -C "$scratch" -c user.name=Canary -c user.email=canary@example.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm "Synthetic scanner canaries"
status=0
"$scanner" detect --no-banner --redact --source "$scratch" --config "$repo/.gitleaks.toml" \
  --gitleaks-ignore-path "$repo/.gitleaksignore" --report-format json --report-path "$scratch/report.json" \
  > "$scratch/scan.log" 2>&1 || status=$?
if [ "$status" -ne 1 ] || ! grep -q '"RuleID": "github-pat"' "$scratch/report.json" \
  || ! grep -q '"RuleID": "generic-api-key"' "$scratch/report.json"; then
  echo "check-secret-fixtures: new same-path canaries were not detected" >&2
  cat "$scratch/scan.log" >&2
  exit 1
fi
echo "check-secret-fixtures: new same-path canaries remain detectable"
