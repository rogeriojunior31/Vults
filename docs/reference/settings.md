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
| `~/.local/share/vults/history.jsonl`, `days.json` | The local history of agent turns (counts only), for Settings → Activity |
| `$XDG_RUNTIME_DIR/vults.sock` | The socket the hook talks to (mode `0600`) |
| `~/.config/autostart/` | The entry **Start with the desktop** adds |
| System keyring, service `io.github.rogeriojunior31.vults`, account `<provider>-api-key` (`anthropic-api-key`, `openai-api-key`, …) | The chat's API keys, one per provider you gave one (never in a file) |
| `~/.local/state/vults/logs/` | The log: one file a day, the last five kept. It records what happened (event names, decisions, errors), never commands, paths or chat text |

`$XDG_CONFIG_HOME` and `$XDG_DATA_HOME` replace `~/.config` and `~/.local/share` when they are set.
**Settings → About** shows the real paths.

## settings.json

```json
{
  "version": 13,
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
| `version` | `13` | Schema version, so later releases can migrate the file. If it is newer than the app's (you went back to an older release), the app (from 0.1.1 on) uses the keys it knows, keeps the file as it was in `settings.json.v<version>-<time>` and says so in the log; the next change you make saves only the keys it knows, as its own version. Until then each start keeps another copy. If no copy can be kept, changes are not saved. To go back to the newer release's settings, restore that copy. An older file is read with defaults for the keys it lacks, and is saved as the app's version |
| `connectors` | `{}` | Connector id → switched on |
| `sounds` | `true` | 8-bit sounds |
| `volume` | `50` | How loud the sounds play, in percent (0 to 100); `50` is how loud 0.1.0 played them |
| `voice_model` | absent | The chat's voice model (`base`, `small`, `turbo`), downloaded into `~/.local/share/vults/voice/` (with the speech detector, `ggml-silero-v6.2.0.bin`); absent keeps voice off |
| `voice_language` | absent | What the user speaks for the voice: a code (`pt`, `en`), `auto` to detect it each time, absent to follow the system's language |
| `now_playing` | `false` | Show the song your media players are playing on the island (MPRIS on Linux), with play, pause and skip |
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
| `dnd_until` | absent | Do not disturb until then, in seconds since the Unix epoch: no sounds and no notifications at rest, no reminders; a card still opens the island with its sound (and in *Panel* its notification). Set from **Settings → General**; a time already past is off (from version 9) |
| `history` | `true` | Keep a local history of agent turns for **Settings → Activity**: `history.jsonl` (each finished turn's counts, 12 weeks) and `days.json` (each day's totals, a year) in the data folder. Off, nothing new is kept and what is there stays; **Clear history** removes both files (from version 13) |
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
| Gemini CLI | `~/.gemini/settings.json` | One hook entry per event, running `vults-hook --agent gemini` (timeouts in milliseconds) |
| Antigravity | `~/.gemini/config/hooks.json` | One top-level hook named `vults`, with one handler per event running `vults-hook --agent antigravity <Event> \|\| exit 0` (timeouts in seconds). The other named hooks in the file are never touched, **Remove hooks…** deletes only the `vults` key, with anything inside it, and turning it off with `"enabled": false` in Antigravity is kept. A `vults` hook of yours that does not run ours is never overwritten: installing is refused |

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
