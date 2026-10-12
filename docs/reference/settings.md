# Settings and files

## Where things live (Linux)

| Path | What |
|---|---|
| `~/.config/vults/settings.json` | The app's settings. A field it can't read (a wrong type) falls back to its default and the rest is kept; the file as it was is copied to `settings.json.bad-<time>` first. A file from a newer release is read as far as this one understands it, and copied to `settings.json.v<version>-<time>` first (see `version` below) |
| `~/.local/share/vults/bin/vults-hook` | The hook relay your agents run |
| `~/.local/share/vults/bin/statusline-previous.json` | Your own Claude Code `statusLine`, saved when the hooks went in; the hook runs it, and removing the hooks puts it back |
| `~/.local/share/vults/inbox/` | Copies of dropped files, deleted after a week |
| `~/.local/share/vults/chat/` | The empty folder chats use when no session is in front |
| `~/.local/share/vults/connectors/` | What each connector last saw |
| `~/.local/share/vults/vults.sqlite` | The local database: the history of agent turns (counts only), for Settings → Activity, the audit log of answers to cards (90 days), the policies you accepted (kept until you revoke them, and after), the phones you paired (their public keys and the last counter each signed with; revoked ones stay, so a revoked phone never comes back), Zeca's traces (30 days), his conversations (the newest 200), the journal and its day summaries (90 days), recall vectors, memory proposals (decided ones kept 90 days), sleep runs, your tasks and what the agents cost by day and roost (see [Safety](../safety.md)). **Clear history** empties all of it except the consents, the phones and the tasks. Earlier versions' `history.jsonl` and `days.json` move into it on first start |
| `~/.local/share/vults/memory/` | Zeca's memory: a git repository of small Markdown files, each change a commit (mode of the folder as your umask makes it). **Clear history** leaves it; delete the folder to erase his facts ([Zeca's memory](../guide/memory.md)) |
| `~/.local/share/vults/voice/` | The voice models you downloaded and the speech detector |
| `~/.local/share/vults/backups/` | Dated backups of the files Vults edits inside a project, such as a `CLAUDE.md` it made read `AGENTS.md` (never kept in the repository) |
| `~/.local/share/vults/zeca-mcp.json` | The MCP config the chat's Claude Code gets with `--mcp-config`: this app run as Zeca's server (`vults --zeca-mcp`); mode `0600` |
| `~/.local/share/vults/zeca-flock.json` | The island's sessions (redacted) for that server's `flock_list`, rewritten when they change and removed when the app quits; mode `0600` |
| `$XDG_RUNTIME_DIR/vults.sock` | The socket the hook talks to (mode `0600`) |
| `~/.config/autostart/` | The entry **Start with the desktop** adds |
| System keyring, service `io.github.rogeriojunior31.vults`, account `<provider>-api-key` (`anthropic-api-key`, `openai-api-key`, …) | The chat's API keys, one per provider you gave one (never in a file) |
| `~/.local/state/vults/logs/` | The log: one file a day, the last five kept. It records what happened (event names, decisions, errors), never commands, paths or chat text |

`$XDG_CONFIG_HOME` and `$XDG_DATA_HOME` replace `~/.config` and `~/.local/share` when they are set.
**Settings → About** shows the real paths.

## settings.json

```json
{
  "version": 18,
  "connectors": { "github": true },
  "sounds": true,
  "volume": 50,
  "fold_after": 15,
  "api_provider": "openrouter",
  "api_models": { "openrouter": "anthropic/claude-opus-5.5" }
}
```

| Key | Default | Meaning |
|---|---|---|
| `version` | `22` | Schema version, so later releases can migrate the file. If it is newer than the app's (you went back to an older release), the app (from 0.1.1 on) uses the keys it knows, keeps the file as it was in `settings.json.v<version>-<time>` and says so in the log; the next change you make saves only the keys it knows, as its own version. Until then each start keeps another copy. If no copy can be kept, changes are not saved. To go back to the newer release's settings, restore that copy. An older file is read with defaults for the keys it lacks, and is saved as the app's version |
| `connectors` | `{}` | Connector id → switched on |
| `sounds` | `true` | 8-bit sounds |
| `volume` | `50` | How loud the sounds play, in percent (0 to 100); `50` is how loud 0.1.0 played them |
| `voice_model` | absent | The chat's voice model (`base`, `small`, `turbo`), downloaded into `~/.local/share/vults/voice/` (with the speech detector, `ggml-silero-v6.2.0.bin`); absent keeps voice off |
| `voice_language` | absent | What the user speaks for the voice: a code (`pt`, `en`), `auto` to detect it each time, absent to follow the system's language |
| `now_playing` | `false` | Show the song your media players are playing on the island (MPRIS on Linux), with play, pause and skip |
| `codex_usage` | `false` | Read Codex's plan usage for the island every 5 minutes (and half a minute after a window resets), through the codex you logged into (each read reaches OpenAI, through the app's network gate). Off on a new install; a file from before version 16 gets it on when a `codex` is on the `PATH`, so an upgrade keeps the meters it showed (from version 16) |
| `web` | `false` | Zeca's web tools for the API chat: a search and a page read the model may ask for, each a request through the network gate. Off, it is never offered them |
| `web_engine` | `"brave"` | Where searches go: `brave`, `tavily` (each with its key in the keyring) or `searxng` |
| `searxng_url` | absent | Your own SearXNG's address (`https://…`), when that is the engine |
| `phone` | `false` | Answer cards from a paired phone: each card goes to it and its answers are read back, through the network gate. Set from **Settings → Phone** (from version 17) |
| `phone_transport` | `"relay"` | How the phone reaches this computer: `relay` (through `phone_relay`) or `direct` (this computer listens on `phone_listen`) |
| `phone_relay` | absent | The relay's address (`https://…`; `http://` only on a network you trust) |
| `phone_listen` | absent | In `direct`, this computer's address on your LAN or tailnet and a port (`100.64.0.2:8787`) |
| `chat_picks` | absent | The chat's model and effort per folder (`""` for its own empty folder), then per engine: `{ "/home/me/site": { "claude": { "model": "sonnet", "effort": "high" } } }`. `model` is a name the CLI takes (`opus`, `gpt-6.1-sol`), `effort` one of `low`, `medium`, `high`, `xhigh`, `max`; either absent is the CLI's default. Set from the chat's picker; a new conversation in that folder starts with it |
| `native_chat` | `true` | The chat's `claude` and `codex` read your own setup: your `CLAUDE.md`, skills, MCP servers, hooks and allow rules; a project's settings only in a folder you already trusted in Claude Code. Cards still ask whatever your rules don't allow. Off, Claude reads none of it and Codex leaves out your hooks (it always reads its own `config.toml`). Applies to the next conversation |
| `chat_access` | `"ask"` | Who answers Zeca's cards ([ADR 0026](../adr/0026-the-chats-access-is-the-users-choice.md)): `ask` (a card each time), `policies` (your accepted policies answer what they cover) or `full` (everything runs with no card, but destructive commands and what a policy denies or asks first; cards come back once a conversation reads from outside). Changed in **Settings → Chat & voice**; a value it can't read asks. Holds at once |
| `memory` | `false` | Zeca's memory and journal; off, nothing of it reaches a model ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `journal_summary` | `false` | With `memory` on, **A summary of each day** of the journal, by Claude Code's smallest model ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `local_embeddings` | `false` | With `memory` on, **Recall by meaning**, through an Ollama on `127.0.0.1` ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `memory_suggest` | `false` | With `memory` on, **Suggest what to remember** from the conversation you left ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `sleep` | `false` | With `memory` on, **Let Zeca sleep on it**: his nightly sleep also asks a model for merges and a debrief ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `journal_summary_since` | absent | The day `journal_summary` was turned on (`YYYY-MM-DD`): the journal kept before it never goes to the model (from version 18) |
| `fold_after` | `15` | Seconds the open island waits, once the pointer leaves, before folding (5 to 120) |
| `open_on_hover` | `false` | Resting the pointer on the pill opens the island, without a click; opened that way it folds as soon as the pointer leaves, unless you clicked in it. Not by the panel. Set from **Settings → General** (from version 11) |
| `monitor` | absent | The screen the island sits on, as maker and model (`"Samsung Electric Company LS27AG32x"`); absent lets the desktop choose. Two identical screens share a name, and the first one wins |
| `rules` | `[]` | Always-allow rules: `{ "agent", "cwd", "tool", "target" }`, each matched exactly |
| `zeca_species` | `"atratus"` | Zeca's species, by id (`atratus` is the black vulture; the ids are in `ui/src/character/flock/species.ts`); an unknown one draws the black vulture |
| `zeca_look` | `"auto"` | What Zeca wears: `auto` (the calendar's look: `witch-hat` Oct 1 to Nov 1, `santa-hat` Dec 1 to 26, `party-hat` Dec 31 to Jan 2, `bunny-ears` Good Friday to Easter Monday), `none`, or one of those or an all-year look (`sunglasses`, `west-coast`, `fitted-cap`, `mountain-hat`, `headband`, `dreads`, `front-knot`, `durag`, `crown`, `bucket-hat`, `clock-chain`, `headphones`, `shutter-shades`, `chrome-chain`, `eye-patch`). An unknown one is `auto` |
| `flock` | `"brazil"` | Where the other sessions' birds are drawn from: `brazil` (Brazil's vultures), `americas` (with both condors) or `world` (every vulture). Each project draws its species from it, the same at every start; changing it draws every project again. The king vulture comes by role either way |
| `presence` | `"island"` | The presence preset: `island` (the flock at the top of the screen), `panel` (Zeca in the panel's tray; the island opens by the panel when you click him or a card needs you), `quiet` (nothing at rest; a card still opens the island with its sound) or `paused` (cards go to the agents' terminals at once, connectors stop, no notifications). Version 4 files hold `island` or `panel` and read as is. An unknown value is `island` |
| `zeca` | `true` | Zeca, the companion: off, no chat, microphone or talk shortcut, and the tray has no *Chat…*; the flock, cards, notifications and connectors work as ever |
| `widget` | absent | The corner widget's corner: `top-left`, `top-right`, `bottom-left` or `bottom-right`; absent (the default) for no widget. An unknown value is no widget |
| `projects` | absent | Choices per project folder, set from a session's quick actions or **Settings → Projects**: `{ "/home/me/site": { "pin": true }, "/home/me/x": { "mute": true, "hide": true } }`. `mute`: no sounds or notifications from its sessions at rest (a card keeps both); `pin`: its sessions first; `hide`: its sessions off the island, the tray and the widget, except while one has a card waiting; `species`: its sessions' bird, a species id of the renderer (`"vultur"`), any but the king vulture's (`"papa"`), which is ignored like an unknown one (from version 12). Only the choices that are on are written, and a project with none is dropped (from version 8) |
| `delegate` | absent | Which agent a project's delegated tasks start on, by its roost (`{ "github:me/site": "codex" }`, `claude` or `codex`), set with `/delegate default` in the chat; once the project has 10 finished tasks of a kind, their scores pick instead (from version 19) |
| `budget_daily_cents` | absent | The daily budget, in US cents (`500` is $5.00), set in the Nest's **Activity → Cost**: at 80% a notification, and once spent another, and Vults starts no new session that day. It never stops or blocks an agent. Absent, no budget (from version 20) |
| `dnd_until` | absent | Do not disturb until then, in seconds since the Unix epoch: no sounds and no notifications at rest, no reminders; a card still opens the island with its sound (and in *Panel* its notification). Set from **Settings → General**; a time already past is off (from version 9) |
| `history` | `true` | Keep a local history of agent turns for **Settings → Activity**: each finished turn's counts (12 weeks) and each day's totals (a year), in `vults.sqlite` in the data folder. Off, nothing new is kept and what is there stays; **Clear history** removes all of it (from version 13), and the audit log with it |
| `recap_shown_week` | absent | The Monday of the last week whose recap card you read on the island, so it comes once (from version 14) |
| `language` | absent | The app's language: `en`, `pt-BR`, `es` or `zh` (Simplified Chinese), set in **Settings → General**. Absent follows the system (`LC_ALL`, `LC_MESSAGES` or `LANG`; the user's locale on Windows), and a language Vults does not speak is English (from version 15) |
| `notifications` | `true` | Desktop notifications, only in *Panel* (at the top of the screen the island shows it all): a session finished, failed or gone quiet, and a card waiting, at once. Their only action opens the island |
| `visitors` | `true` | Now and then, while sessions are open, a vulture from outside the flock crosses the sky once, never landing |
| `api_provider` | `"anthropic"` | The API chat's provider: `anthropic`, `openai`, `google`, `openrouter`, `groq`, `deepseek`, `mistral`, `xai`, `ollama`, `lmstudio` |
| `api_models` | `{}` | Provider → the model chosen for it (keys are never here) |

## Agent configs Vults edits

Only when you click **Write the file**, after a dated backup and a diff you reviewed:

| Agent | File | What is added |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | One hook entry per event, running `vults-hook --agent claude`, and a `statusLine` running `vults-hook --agent claude --statusline` (if you have one of your own, only its `command` changes, and the hook keeps running yours) |
| Codex | `~/.codex/hooks.json` | One hook entry per event, running `vults-hook --agent codex` |
| Antigravity | `~/.gemini/config/hooks.json` | One top-level hook named `vults`, with one handler per event running `vults-hook --agent antigravity <Event> \|\| exit 0` (timeouts in seconds). The other named hooks in the file are never touched, **Remove hooks…** deletes only the `vults` key, with anything inside it, and turning it off with `"enabled": false` in Antigravity is kept. A `vults` hook of yours that does not run ours is never overwritten: installing is refused |

Vults no longer supports Gemini CLI. While hooks an earlier Vults installed in
`~/.gemini/settings.json` are still there, they send nothing, and **Settings → Agents** shows a
Gemini CLI card whose only button is **Remove hooks…** (the same diff, dated backup and click).

Each agent's own folder variable moves its file, read the way the agent reads it:
`CLAUDE_CONFIG_DIR` (Claude Code, in place of `~/.claude`), `CODEX_HOME` (Codex, in place of
`~/.codex`, its `config.toml` too), `QWEN_HOME` (Qwen Code, in place of `~/.qwen`), `COPILOT_HOME` (Copilot CLI, in place of
`~/.copilot`), `FACTORY_HOME_OVERRIDE` (Factory Droid, in place of the home: its `.factory` goes
inside), `KIMI_CODE_HOME` (Kimi Code, in place of `~/.kimi-code`) and
`GEMINI_CLI_HOME` (Gemini CLI keeps its `.gemini` inside it; read only to find the old hooks to
remove). Started from the desktop, the
app does not see what your shell's startup files export, so it asks your login shell once, when it
starts, and keeps only these four; a shell that takes more than two seconds is given up on, and
the usual folders stand. The Claude Code and Codex the app starts itself (the chat, the Codex usage
read) get them too, so they use the same folders as in your terminal. Each card in
**Settings → Agents** shows the file it edits. OpenCode's
`OPENCODE_CONFIG_DIR` is not followed yet.

A write never leaves half a file: the new text goes to a file of its own beside it, is synced to
disk, and only then takes the old one's place. A file with other names (a hard link, as some
dotfiles managers and backup tools make) is written in place, so every name sees the change. A
file saved with a byte-order mark or Windows line ends (CRLF) keeps them.

Entries from other tools are kept, and **Remove hooks…** takes out only ours. If you put another
tool's hook in the same group as ours, that group stays: an update changes only our hook in it,
and Remove takes out only our hook. In Codex, a hook or group that came after one of ours we take
out moves up one place, so Codex may ask you to trust it again in `/hooks`. An update changes our
entries where they are: the file keeps its key order, so the diff shows only what changed. Hooks
that run another copy of `vults-hook` (from another data folder) say so in **Settings →
Agents**, and **Update hooks…** points them to this app's (if your own status line is saved beside
that other hook, the card offers no update and says so: remove the hooks first, see below).

A config holds a single `statusLine`, and Claude Code reports the plan's usage only to it. If you
have one of your own, installing saves its whole object in `statusline-previous.json` beside the
hook and changes only its `command` to ours (your `padding` and other fields stay); the diff shows
both files. Ours then hands the usage to the app, runs your command with the same input (through
`sh -c`, up to 10 seconds) and prints exactly what it prints (up to 64 KiB), colors included. If
your command is gone, fails to start or takes longer, the status line is blank that time, whatever
it started in the background is stopped too, and Claude Code carries on. **Remove hooks…** puts your object back as it
was; a `statusLine` you changed since is no longer ours and is left alone. Hooks installed by
another data folder keep your line beside their hook: remove them before installing from this one.

## Environment

| Variable | Effect |
|---|---|
| `VULTS_NO_LAYER_SHELL` | Use a plain always-on-top window even where layer-shell exists |
| `VULTS_LOG` | Log filter, e.g. `debug` (default `info`) |
| `VULTS_LEAN` | Render the island in software (WebKit without compositing): about 44 MB less, but WebKitGTK 2.54 and newer leave parts of the island unpainted. Off by default |
