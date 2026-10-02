# More agents and more API keys: research (2026-10-02)

What to add next, from three sources: the reference app (`REF/`, upstream main `a6ee893` and its
`n22-gemini` branch), GitHub trending / Trendshift, and each agent's official hook docs. Star
counts are from the GitHub API on this date. **(unverified)** marks what no official page confirmed.

## Where we stand

| | Us | REF (Linux build) | REF (macOS build) |
|---|---|---|---|
| Agents | Claude Code, Codex, generic `--agent <name>` (branch `feat/generic-agent`, not merged) | Claude Code | Claude, Codex, Cursor (Claude in Cursor's terminal), Gemini CLI, Antigravity |
| Chat | `claude` / `codex` CLIs (subscription) + Anthropic key | Anthropic key | Anthropic, Google, OpenAI keys, live model picker |
| Approvals | Claude + Codex, our own Always rules | Claude | Claude, Codex (Gemini/Antigravity observe only) |

Most of what REF has beyond us lives in its Swift app; the Rust Gemini/Antigravity installers are on
a Windows-only branch. We copy mechanics, not code.

## The finding that shapes the design

Almost every agent CLI has converged on a Claude-Code-shaped hook contract: JSON on stdin, a
`PreToolUse`-like event with a matcher, exit 2 blocks, and stdout carrying
`permissionDecision: allow | deny | ask`. So one relay with a **per-agent field map** (event names,
tool fields, session id, output shape, timeout unit) covers most of the list. Agents without
command hooks (OpenCode, Pi, Amp) need a tiny JS/TS plugin that writes to our wire.

## Agents, ranked by popularity x ease

| # | Agent | Stars | Config | Approve from the island? | Effort |
|---|---|---|---|---|---|
| 1 | **Gemini CLI** | 107k | `~/.gemini/settings.json` `hooks`, timeouts in **ms**; `BeforeTool` / `AfterTool` / `BeforeAgent` / `AfterAgent` / `SessionStart` / `SessionEnd`; session id from `GEMINI_SESSION_ID` | `BeforeTool` returns `decision: allow \| deny`; whether `allow` skips its own prompt **(unverified)** | S |
| 2 | **Qwen Code** | 28k | `~/.qwen/settings.json`, Claude's event set (incl. `PermissionRequest`) | yes, `hookSpecificOutput.permissionDecision` | XS (Claude installer, another path) |
| 3 | **GitHub Copilot CLI** | 11k | `~/.copilot/hooks/*.json`, `{"version":1,"hooks":{...}}`, camelCase events (`preToolUse`, `permissionRequest`, ...) | yes, `permissionRequest` -> `behavior` allow/deny | S |
| 4 | **OpenCode** (+ Kilo fork) | 211k / 27k | JS plugin in `~/.config/opencode/plugins/` (or the `opencode serve` SSE API) | yes, `permission.ask` hook | M (ship a JS shim) |
| 5 | **Crush** | 28k | `crush.json` `hooks.PreToolUse` only | yes, `decision: allow`; exit 2 denies | XS (tool starts only) |
| 6 | **Kimi Code CLI** | 8k | `~/.kimi-code/config.toml` `[[hooks]]` | via PreToolUse | S (TOML) |
| 7 | **Factory Droid** | n/a | `~/.factory/hooks.json`, a Claude clone | yes | XS |
| 8 | **Antigravity** (`agy`) | n/a | `~/.gemini/config/hooks.json`; `toolCall.args` in PascalCase, `conversationId`, `workspacePaths` | docs say yes (`allow`/`deny`/`ask`); REF and others treat it as observe only **(unverified)** | S; REF's two builds disagree on the file shape, check the docs |
| 9 | **Cursor CLI** | n/a | `~/.cursor/hooks.json` | yes | S, but a forum report (2026-08) says `cursor-agent` never runs hooks on Linux: test first |
| 10 | Pi, Amp | 112k / n/a | TS extensions | yes, the extension can block | M |
| 11 | Grok CLI, Auggie, Cline, Windsurf | | hook files | deny only | S each, observe only |
| - | Aider | 49k | none (only a notifications command) | no | skip |

Also cheap: label a Claude/Codex session "Cursor" or "VS Code" from the terminal env the hook already
captures (`TERM_PROGRAM`, `VSCODE_*`, `CURSOR_*`); REF does this on macOS with a bundle id.

Codex detail worth copying now: REF sets `statusMessage` on Codex's `PermissionRequest` hook, so the
terminal says the answer is waiting on the island.

## API keys for the chat

Two wire clients cover nearly everything:

1. **Anthropic Messages** (what `crates/chat/src/api.rs` already is): Anthropic, and by base URL
   any Anthropic-compatible endpoint (DeepSeek, Kimi, Z.ai, Ollama, LM Studio).
2. **OpenAI Chat Completions**: OpenAI, Google Gemini (`/v1beta/openai/`), OpenRouter, Groq,
   Mistral, xAI, DeepSeek, Kimi, Z.ai, Cerebras, Together, Fireworks, Ollama, LM Studio.

| Preset | Base URL | Protocol | Key |
|---|---|---|---|
| Anthropic | `https://api.anthropic.com/v1` | anthropic | `x-api-key` |
| OpenAI | `https://api.openai.com/v1` | openai | Bearer |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai` | openai | Bearer |
| OpenRouter | `https://openrouter.ai/api/v1` | openai | Bearer |
| Groq | `https://api.groq.com/openai/v1` | openai | Bearer |
| DeepSeek | `https://api.deepseek.com` | openai | Bearer |
| Mistral | `https://api.mistral.ai/v1` | openai | Bearer |
| xAI | `https://api.x.ai/v1` | openai | Bearer |
| Ollama (local) | `http://localhost:11434/v1` | openai | none |
| LM Studio (local) | `http://localhost:1234/v1` | openai | none |
| Custom | user's URL | openai or anthropic | optional |

- A preset is data: `{id, label, base_url, protocol, key_account, needs_key}`. A new provider is one
  row, no new code.
- One keyring account per preset (`openai-api-key`, `google-api-key`, ...), as REF names them.
- Models come from `GET {base}/models` (every preset has it), filtered of embeddings, audio, image and
  realtime ids, with the chosen model stored per preset in settings (not the keyring). Hardcoded ids
  go stale within weeks (DeepSeek retired `deepseek-chat` in July).
- Local presets talk to `localhost` only: no key, no network beyond the machine.
- The API chat keeps having no tools, whatever the provider.

## Proposed order

1. Merge `feat/generic-agent`.
2. Codex `statusMessage` (small, visible).
3. Provider presets + OpenAI-compatible client + model picker (the API-key work).
4. Gemini CLI, then Qwen Code and Copilot CLI (Claude-shaped, approvals work).
5. OpenCode plugin shim.
6. Crush, Kimi, Droid; Antigravity after checking its file shape; Cursor CLI after a Linux test.

## Ideas from similar apps

Almost all are macOS only; none is a Linux layer-shell island. Cheap ones to borrow: jump to a
tmux / zellij pane, a git branch chip per session, quiet hours for sounds, a plan-usage meter
(ccusage-style), and filesystem watching as a fallback for agents without hooks.

Sources: geminicli.com/docs/hooks/reference, docs.github.com (Copilot CLI hooks reference),
QwenLM/qwen-code docs/users/features/hooks.md, opencode.ai/docs/plugins, cursor.com/docs/hooks,
charmbracelet/crush docs/hooks, kimi.com/code/docs (hooks), docs.factory.com/reference/hooks-reference,
antigravity.google/docs/hooks, developers.openai.com/codex/hooks, ai.google.dev/gemini-api/docs/openai,
openrouter.ai/docs, console.groq.com/docs, api-docs.deepseek.com, docs.x.ai, docs.mistral.ai,
docs.ollama.com/api/openai-compatibility, lmstudio.ai/docs/developer/openai-compat, github.com/trending,
trendshift.io.
