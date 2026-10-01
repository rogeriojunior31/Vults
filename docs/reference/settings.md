# Settings and files

## Where things live (Linux)

| Path | What |
|---|---|
| `~/.config/vultures-ai/settings.json` | The app's settings |
| `~/.local/share/vultures-ai/bin/vultures-ai-hook` | The hook relay your agents run |
| `~/.local/share/vultures-ai/inbox/` | Copies of dropped files, deleted after a week |
| `~/.local/share/vultures-ai/chat/` | The empty folder chats use when no session is in front |
| `~/.local/share/vultures-ai/connectors/` | What each connector last saw |
| `$XDG_RUNTIME_DIR/vultures-ai.sock` | The socket the hook talks to (mode `0600`) |
| `~/.config/autostart/` | The entry **Start with the desktop** adds |

## settings.json

```json
{
  "version": 1,
  "connectors": { "github": true },
  "sounds": true
}
```

| Key | Default | Meaning |
|---|---|---|
| `version` | `1` | Schema version, so later releases can migrate the file |
| `connectors` | `{}` | Connector id → switched on |
| `sounds` | `true` | 8-bit sounds |

## Agent configs Vultures AI edits

Only when you click **Write the file**, after a dated backup and a diff you reviewed:

| Agent | File | What is added |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | One hook entry per event, running `vultures-ai-hook --agent claude` |
| Codex | `~/.codex/hooks.json` | One hook entry per event, running `vultures-ai-hook --agent codex` |

Entries from other tools are kept, and **Remove hooks…** takes out only ours.

## Environment

| Variable | Effect |
|---|---|
| `VULTURES_AI_NO_LAYER_SHELL` | Use a plain always-on-top window even where layer-shell exists |
| `VULTURES_AI_GPU` | Keep WebKit's GPU compositing (off by default: the island renders cheaper in software) |
