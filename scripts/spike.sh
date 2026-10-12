#!/usr/bin/env bash
# Records a real agent's hooks without an account (docs/dev/plan-every-agent.md, F2): a scratch
# home with our hooks installed as Settings writes them, a fake model scripting its tool calls, a
# fake app answering its cards, and the agent's own CLI, found on the PATH.
#
#   scripts/spike.sh <agent> [allow|deny|none] [script.json]
#
# <agent> is claude, codex, copilot, droid, kimi, opencode or qwen. The script is a JSON list of steps
# (crates/spike/src/model.rs); the default runs a command, edits a file, writes one and ends.
# Out, in $SPIKE_DIR/<agent>-<answer> (default ~/.cache/vults-spike): events.jsonl (what the hook
# sent), replay.txt (what the island makes of it), fixture.jsonl (for crates/agents/tests/fixtures),
# agent.out and requests.jsonl. Nothing outside that folder is touched. A TUI run (qwen, or claude
# with SPIKE_TTY=1) lasts SPIKE_SECONDS; SPIKE_KEYS is typed SPIKE_KEYS_AT seconds in (`\r` for
# Enter), for a question asked in the agent's own terminal.
set -euo pipefail
cd "$(dirname "$0")/.."

agent=${1:?usage: scripts/spike.sh <claude|codex|copilot|droid|kimi|opencode|qwen> [allow|deny|none] [script.json]}
answer=${2:-allow}
port=${SPIKE_PORT:-8790}
root=${SPIKE_DIR:-$HOME/.cache/vults-spike}
# Codex refuses a home under /tmp, and a socket path must stay short: two places.
work_root="$root/$agent-$answer"
run=$(mktemp -d /tmp/vults-spike.XXXXXX)
rm -rf "$work_root"
mkdir -p "$work_root/home" "$work_root/work"
W=$work_root
script=${3:-$W/script.json}
if [[ -z ${3:-} ]]; then
  echo '[{"run":"echo hi > out.txt"},{"read":"notes.txt"},{"edit":["notes.txt","old line","new line"]},{"write":["new.txt","fresh\nsecond"]},{"text":"done"}]' >"$script"
fi

cargo build -q --release -p vults-hook -p vults-spike
spike=target/release/vults-spike
hook=$PWD/target/release/vults-hook

"$spike" app "$run" "$W/events.jsonl" "$answer" & app=$!
"$spike" model "$port" "$script" "$W/work" "$W/requests.jsonl" & model=$!
trap 'kill $app $model 2>/dev/null; rm -rf "$run"' EXIT
"$spike" install "$agent" "$W/home" "$hook" >/dev/null

cd "$W/work"
echo "old line" >notes.txt
git init -q .
export HOME=$W/home XDG_CONFIG_HOME=$W/home/.config XDG_RUNTIME_DIR=$run
spike=$OLDPWD/$spike
base=http://127.0.0.1:$port
prompt="do the steps"

case $agent in
  claude)
    # Onboarding and the trust dialog accepted, as a user would have them.
    printf '{"hasCompletedOnboarding":true,"customApiKeyResponses":{"approved":["sk-spike"]},"projects":{"%s":{"hasTrustDialogAccepted":true}}}' "$W/work" >"$HOME/.claude.json"
    # -p mode asks through the hook too; a question needs the TUI (SPIKE_TTY=1).
    if [[ -n ${SPIKE_TTY:-} ]]; then
      ANTHROPIC_BASE_URL=$base ANTHROPIC_API_KEY=sk-spike "$spike" tty "${SPIKE_SECONDS:-60}" -- claude --model claude-sonnet-4-5 "$prompt" >"$W/agent.out" 2>&1 || true
    else
      ANTHROPIC_BASE_URL=$base ANTHROPIC_API_KEY=sk-spike timeout 120 claude -p "$prompt" --model claude-sonnet-4-5 </dev/null >"$W/agent.out" 2>&1 || true
    fi
    ;;
  codex)
    mkdir -p "$HOME/.codex"
    cat >"$HOME/.codex/config.toml" <<TOML
model = "gpt-5.5"
model_provider = "spike"

[model_providers.spike]
name = "spike"
base_url = "$base/v1"
wire_api = "responses"
env_key = "SPIKE_KEY"

[projects."$W/work"]
trust_level = "trusted"
TOML
    "$spike" codex-trust codex "$HOME" "$W/work"
    # `codex exec` never asks (approval: never): steps only. Its cards need its TUI.
    SPIKE_KEY=x timeout 120 codex exec --skip-git-repo-check "$prompt" </dev/null >"$W/agent.out" 2>&1 || true
    ;;
  copilot)
    # Its own provider (BYOK) and offline mode: no GitHub sign-in. `-p` asks through the hook too.
    COPILOT_OFFLINE=true COPILOT_PROVIDER_BASE_URL=$base/v1 COPILOT_PROVIDER_API_KEY=x COPILOT_MODEL=gpt-4.1 \
      timeout 120 copilot -p "$prompt" </dev/null >"$W/agent.out" 2>&1 || true
    ;;
  droid)
    # Its own models (BYOK) need no Factory account in `exec`; its TUI does. `exec` never asks:
    # a step it may not take ends the run.
    jq --arg base "$base" '.customModels = [{model: "claude-sonnet-4-5", displayName: "Spike", baseUrl: $base,
        apiKey: "x", provider: "anthropic", maxOutputTokens: 4096}]' "$HOME/.factory/settings.json" >"$HOME/.factory/s.json" &&
      mv "$HOME/.factory/s.json" "$HOME/.factory/settings.json"
    timeout 120 droid exec --auto "${SPIKE_AUTO:-high}" -m custom:claude-sonnet-4-5 "$prompt" </dev/null >"$W/agent.out" 2>&1 || true
    ;;
  kimi)
    # Its own provider in its config, no Moonshot account. `-p` runs every tool unasked; its
    # PermissionRequest hook is only told, so there is nothing to answer.
    cfg=$HOME/.kimi-code/config.toml
    printf 'default_model = "spike"\ntelemetry = false\n\n[providers.spike]\ntype = "anthropic"\napi_key = "x"\nbase_url = "%s"\n\n[models.spike]\nprovider = "spike"\nmodel = "claude-sonnet-4-5"\nmax_context_size = 200000\n\n' "$base" | cat - "$cfg" >"$cfg.new" && mv "$cfg.new" "$cfg"
    timeout 120 kimi -p "$prompt" </dev/null >"$W/agent.out" 2>&1 || true
    ;;
  opencode)
    mkdir -p "$XDG_CONFIG_HOME/opencode"
    cat >"$XDG_CONFIG_HOME/opencode/opencode.json" <<JSON
{ "provider": { "spike": { "npm": "@ai-sdk/openai-compatible", "name": "Spike",
    "options": { "baseURL": "$base/v1", "apiKey": "x" }, "models": { "fake-model": { "name": "Fake" } } } },
  "model": "spike/fake-model", "permission": { "bash": "ask", "edit": "ask" } }
JSON
    # `opencode run` rejects every permission at once: its server keeps them for the plugin.
    opencode serve --port 4799 --hostname 127.0.0.1 >"$W/agent.out" 2>&1 & serve=$!
    curl() { command curl --noproxy '*' -m 120 -s "$@"; }
    for _ in $(seq 60); do curl -m 3 -f http://127.0.0.1:4799/session >/dev/null && break; sleep 0.5; done
    id=$(curl -X POST http://127.0.0.1:4799/session -H 'content-type: application/json' -d '{}' | sed -E 's/.*"id":"([^"]+)".*/\1/')
    curl -X POST "http://127.0.0.1:4799/session/$id/message" -H 'content-type: application/json' \
      -d "{\"parts\":[{\"type\":\"text\",\"text\":\"$prompt\"}]}" >>"$W/agent.out" || true
    sleep 3
    kill $serve 2>/dev/null || true
    ;;
  qwen)
    mkdir -p "$HOME/.qwen"
    # Its default mode, which asks; `-p` hides the shell and edit tools then, so its TUI runs.
    jq '.security.auth.selectedType = "openai" | .tools.approvalMode = "default" | .general.disableAutoUpdate = true' \
      "$HOME/.qwen/settings.json" >"$HOME/.qwen/s.json" && mv "$HOME/.qwen/s.json" "$HOME/.qwen/settings.json"
    OPENAI_BASE_URL=$base/v1 OPENAI_API_KEY=x OPENAI_MODEL=fake-model \
      "$spike" tty "${SPIKE_SECONDS:-60}" --screen "$W/screen.log" ${SPIKE_KEYS:+--keys "$SPIKE_KEYS" --keys-at "${SPIKE_KEYS_AT:-20}"} \
      -- qwen -i "$prompt" >"$W/agent.out" 2>&1 || true
    ;;
  *) echo "spike.sh: no recipe for $agent" >&2; exit 1 ;;
esac

cd "$OLDPWD"
"$spike" replay "$W/events.jsonl" | tee "$W/replay.txt"
"$spike" fixture "$W/events.jsonl" "$W/work" >"$W/fixture.jsonl"
echo "spike.sh: $(wc -l <"$W/events.jsonl") events; $W/fixture.jsonl"
