# Settings and files

**Dictation > Diagnostics** provides an explicit six-second, memory-only microphone test,
current audio/desktop/model status, counters since app start and retained stage timings. It adds no
persistent setting and requests no desktop grant. See [Diagnostics](../guide/dictation.md#diagnostics).

## Where things live (Linux)

| Path | What |
|---|---|
| `~/.config/vults/settings.json` | The app's settings. A field it can't read (a wrong type) falls back to its default and the rest is kept; the file as it was is copied to `settings.json.bad-<time>` first. A file from a newer release is read as far as this one understands it, and copied to `settings.json.v<version>-<time>` first (see `version` below) |
| `~/.local/share/vults/bin/vults-hook` | The hook relay your agents run |
| `~/.local/share/vults/bin/statusline-previous.json` | Your own Claude Code `statusLine`, saved when the hooks went in; the hook runs it, and removing the hooks puts it back |
| `~/.local/share/vults/inbox/` | Copies of dropped files, deleted after a week |
| `~/.local/share/vults/chat/` | The empty folder chats use when no session is in front |
| `~/.local/share/vults/connectors/` | What each connector last saw |
| `~/.local/share/vults/vults.sqlite` | The local database: the history of agent turns (counts only), for Settings → Activity, the audit log of answers to cards (90 days), the policies you accepted (kept until you revoke them, and after), the phones you paired (their public keys and the last counter each signed with; revoked ones stay, so a revoked phone never comes back), Zeca's traces (30 days), his conversations (the newest 200), the journal and its day summaries (90 days), recall vectors, memory proposals (decided ones kept 90 days), sleep runs and what the agents cost by day and roost (see [Safety](../safety.md)). **Clear history** empties most of it ([what it removes and what stays](../guide/activity.md#the-history)). Earlier versions' `history.jsonl` and `days.json` move into it on first start |
| `~/.local/share/vults/memory/` | Zeca's memory: a git repository of small Markdown files, each change a commit (mode of the folder as your umask makes it). **Clear history** leaves it; delete the folder to erase his facts ([Zeca's memory](../guide/memory.md)) |
| `~/.local/share/vults/voice/` | The voice models you downloaded and the speech detector |
| `~/.local/share/vults/meeting-audio/` | Private temporary audio for explicit one-minute recording/transcription checks; retained through local transcription, removed on completion/cancellation and recovered at startup after interruption. No audio archive |
| `~/.local/share/vults/backups/` | Dated backups of the files Vults edits inside a project, such as a `CLAUDE.md` it made read `AGENTS.md` (never kept in the repository) |
| `~/.local/share/vults/zeca-mcp.json` | The MCP config the chat's Claude Code gets with `--mcp-config`: this app run as Zeca's server (`vults --zeca-mcp`); mode `0600` |
| `~/.local/share/vults/zeca-flock.json` | The island's sessions (redacted) for that server's `flock_list`, rewritten when they change and removed when the app quits; mode `0600` |
| `$XDG_RUNTIME_DIR/vults.sock` | The socket the hook talks to (mode `0600`) |
| `~/.config/autostart/` | The entry **Start with the desktop** adds |
| System keyring, service `io.github.rogeriojunior31.vults`, account `<provider>-api-key` (`anthropic-api-key`, `openai-api-key`, …) | The chat's API keys, one per provider you gave one (never in a file) |
| System keyring, service `io.github.rogeriojunior31.vults`, account `dictation-keyboard-restore-token` | The dictation RemoteDesktop keyboard restore token, when persistence is available; consumed once, rotated after a grant and removed when insertion is disabled. Never in a file or exposed to a webview |
| `~/.local/state/vults/logs/` | The log: one file a day, the last five kept. It records what happened (event names, decisions, errors), never commands, paths or chat text |

Dictation's user-maintained dictionary, replacement rules and snippets share `vults.sqlite`
(schema 22). They are preferences, separate from recorded transcripts: retention, transcript
erasure and **Clear history** preserve them. Individual deletion removes the chosen entry.
Each independent recording snapshots these entries for local final-text processing. Optional
model vocabulary uses dictionary words only, under the measured model/language gate below.
Edit entries in **Dictation > Dictionary / Snippets**, and preview saved rules with **Try a sentence**. See
[Your words processing](../guide/dictation.md#your-words-processing) for order,
literal snippet behavior and size limits.

Migration 25 adds a content-free clean-up admission ledger (attempt id, feature and time). Clear history
preserves it so erasing transcripts/traces cannot renew the rolling request quota. The settings binding
to its identity prevents a missing or recreated database from granting fresh consent or quota. Expired admissions
are removed at the next admitted attempt; future timestamps remain charged after clock rollback.

Migration 26 adds per-app `dictation_styles` preferences, preserved by retention and Clear history,
and nullable retained `output_language`. Only a successful clean-up sets output language; failure
keeps the local processing style and the spoken language unchanged.

`$XDG_CONFIG_HOME` and `$XDG_DATA_HOME` replace `~/.config` and `~/.local/share` when they are set.
**Settings → About** shows the app's version and the real paths.

The global dictation keys are shown in **Settings > Dictation > Keyboard shortcuts**.
**Ctrl+Alt+V** always starts independent dictation; it never opens Chat. The Chat microphone
button fills its draft separately.
**Voice commands > Command key** shows the desktop's actual binding (suggested **Ctrl+Alt+Z**); hold and release
to control Vults without typing into another app. It uses the shared local model and microphone,
with hold-only gestures and a separate command path; text transformations and insertion do not apply.
**Change in KDE…** opens the installed KDE shortcut editor; launch failures offer retry.
On other desktops, change global keys in the desktop's own settings. Opening this editor
does not write Vults preferences or agent configurations.

## Dictation settings

**Meeting settings > Meeting speaker models (experimental)** owns the optional, reviewed model/runtime
installation. It shows progress and cancellation and does not activate analysis. Files live under
`~/.local/share/vults/voice/meetings/`; no persistent enable preference is added. See
[Meeting model artifacts](meeting-models.md) for sources, licenses and request limits.

The **Dictation** tab owns its configuration pages, alongside History, Voice commands,
Dictionary, Snippets and Meetings. **Voice** owns the shared local model and spoken language; **Chat & voice**
links there. Configuration uses the privileged Settings webview, without showing transcripts or
changing command authority. The global Settings sidebar no longer duplicates these pages. During the app run, the Dictation tab
restores its last configuration section; Voice commands links directly to Command settings.
Microphone, gesture, processing, vocabulary and music changes apply to the next recording; an active turn
keeps those captured choices. Caption/sound preferences remain live, retention takes effect
immediately, and turning insertion off immediately revokes pending insertion capabilities.

| Group | Available controls |
| --- | --- |
| Keyboard shortcuts | Current desktop bindings, KDE shortcut editor, hold/double tap, X11 Space/Esc and spoken cancellation |
| Command settings | Enabled by default, actual command binding, names/aliases, local unknown feedback or explicit Chat fallback, searchable phrases, a read-only setup snapshot and a non-executing phrase check |
| Voice | Shared model/language editor, microphone and refresh, repetition removal, Base retry, music, start cue and captions |
| Vocabulary | Opt-in dictionary context and project vocabulary; both off by default, limited to qualified Turbo/fixed English |
| Paste | Spoken-code punctuation, opt-in editor/tmux insertion, actual keyboard permission and explicit retry |
| Styles | Separate clean-up consent, provider/data/quota disclosure, style/output language, session English and per-app overrides |
| Privacy | History retention and confirmed erasure |
| Diagnostics | Explicit microphone test, current system status, session counters and retained stage timings |

Edit dictionary words, replacement rules and literal snippets in the Nest's **Dictation** work
pages; **Try a sentence** previews saved local text rules without a model, clipboard or history.
The local text rules work with every transcription language; only Whisper's optional vocabulary
context has the model/language restriction. Diagnostics reads current state; only its explicit
**Test microphone** action records, and that test stops if you change the microphone. It adds no
saved preference. Additional delivery modes arrive in later phases. See the [dictation guide](../guide/dictation.md)
for supported destinations, clipboard behavior and privacy limits.

## settings.json

```json
{
  "version": 39,
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
| `version` | `41` | Schema version, so later releases can migrate the file. If it is newer than the app's (you went back to an older release), the app (from 0.1.1 on) uses the keys it knows, keeps the file as it was in `settings.json.v<version>-<time>` and says so in the log; the next change you make saves only the keys it knows, as its own version. Until then each start keeps another copy. If no copy can be kept, changes are not saved. To go back to the newer release's settings, restore that copy. An older file is read with defaults for the keys it lacks, and is saved as the app's version |
| `connectors` | `{}` | Connector id → switched on |
| `sounds` | `true` | 8-bit sounds |
| `volume` | `50` | How loud the sounds play, in percent (0 to 100); `50` is how loud 0.1.0 played them |
| `voice_model` | absent | The shared Chat/dictation voice model (`base`, `small`, `turbo`), downloaded into `~/.local/share/vults/voice/` (with the speech detector, `ggml-silero-v6.2.0.bin`); absent keeps voice off. Each Chat recording keeps up to one minute of audio; independent dictation keeps up to ten minutes |
| `voice_language` | absent | What the user speaks for the voice: a code (`pt`, `en`), `auto` to detect it each time, absent to follow the system's language |
| `dictation_insert` | `false` | Off until enabled in **Settings > Dictation > Paste**. Paste into supported plain editors on native X11 or Plasma Wayland, with focus revalidation and a conditional plain-text clipboard lease (up to 64 KiB); no Enter. Wayland requests a keyboard-only portal grant and stores a restore token only in the OS keyring; refusal/revocation keeps copying until an explicit Settings retry. Exact tmux flock panes in XTerm/UXTerm or Kitty can receive one control-free line up to 8 KiB after live session/card checks, without touching the clipboard; other terminal targets stay copy-only. Images and rich formats are not restored; clipboard history exclusion depends on the desktop. See [Dictation](../guide/dictation.md#opt-in-x11-insertion) (from version 24) |
| `dictation_caption` | `true` | Show clipped dictation lines or the final confirmed transcript in a two-row, click-through caption. Cards and limited space take priority; completion captions expire after two seconds across surface changes. Set in **Settings > Dictation > Voice** (from version 25) |
| `dictation_cleanup` | `false` | Separate opt-in for one tool-free text clean-up request through the selected Chat & voice API provider after local dictation. Fallback after four seconds (eight for translation, twelve for Prompt) preserves local text; disabling or provider/model/key changes cancel filtering. Gate quota: 2,000 attempts per rolling 24 hours, including local calls, failures and interruptions. Captured at key-down; no CLI path. Set in **Dictation > Styles** (from version 37) |
| `dictation_cleanup_budget` | absent | Internal local ledger identity saved on first explicit cleanup enablement; retained while off and across Clear history. Missing/mismatched/unreadable budget state denies cleanup requests; never sent to the provider (from version 37) |
| `meeting_summary` | `false` | Independent consent for the meeting-summary backend through the selected API chat provider/model; never inherited from dictation cleanup. Configured in **Settings > Dictation > Meeting settings**; the Summary/To-dos tabs stay in Nest. An explicit Nest request requires a finished transcript and durably reserves one of 20 attempts per rolling 24 hours (local calls and failures included), with a total 120-second deadline. Disabling or provider/model/key changes cancel work; failures preserve local text (from version 39) |
| `meeting_summary_budget` | absent | Local ledger identity bound on first explicit summary enablement; retained while off and across Clear history. Missing/mismatched identity or an unreadable ledger denies calls; replacing the database never silently renews consent (from version 39) |
| `voice_realtime` | `{on:false, ledger:null, limit_microusd:1000000, model:"gpt-realtime-2.1-mini"}` | Backend-only independent voice consent (version 41). Settings webview alone can save it; no renderer/capture control yet. Zero disables remote voice. Ledger identity survives disabling/history erasure and must match before any monetary admission; Linux exclusive owners reconcile abandoned holds at full unknown cost; active owners still deny entry. Local recovery returns a draft/original audio without automatic sending. See [Realtime accounting](realtime-accounting.md) |
| `dictation_style` | `"natural"` | Global fallback for identified apps without a category/override: `"verbatim"`, `"natural"`, `"polished"`, `"prompt"`. Per-app rows live in `dictation_styles`; sessions keep Prompt. Missing focus stays local; styles never enable clean-up (from version 38; controls in **Dictation > Styles**) |
| `dictation_write_in` | absent | Optional output language: `"en"`, `"pt-BR"`, `"es"`, `"zh"`; absent keeps the spoken language. Verbatim makes no request and does not translate. Captured at key-down, with applied output language retained separately from heard language (from version 38) |
| `dictation_sessions_english` | `false` | Override output language with English only for recognized flock sessions, using Prompt. Other apps keep Write in; ambiguous session context never opts into English. Requires the separate cleanup switch (from version 38) |
| `dictation_commands` | `true` | Enable the separate hold-only command key. Disabling cancels command capture/handoffs and context, preserving ordinary dictation and Copy again (from version 36) |
| `dictation_command_media` | `false` | Independent local playback-command consent. Requires voice commands and media controls on; controls only the selected player, without changing project notifications. Disabling cancels pending command work. Set in **Settings > Dictation > Command settings** |
| `dictation_command_apps` | `false` | Independent consent to read visible installed desktop application names and open a validated registered launcher by voice. Bare Open resolves apps and current projects together; an explicit app/project namespace disambiguates. No executable, URL or file arguments come from speech. Set in **Settings > Dictation > Command settings**; changing it cancels pending commands |
| `dictation_command_output` | `false` | Independent opt-in for local default-output mute/volume commands through pactl JSON and a Unix audio server. Pins/rechecks output identity, preserves channel balance, bounds volume to 0..100%, confirms state and rejects stale/cancelled requests. Does not unmute on volume changes or control a microphone/player. Set in **Settings > Dictation > Command settings**; changing it cancels pending commands |
| `dictation_command_chat` | `false` | Explicit fallback for unknown speech to the current idle Chat, with Zeca on and no unsent draft/files/references. Uses its model, gate, budget, traces and approval rules; refused/ambiguous/stale commands stay local. Settings-only changes invalidate old tickets (from version 36) |
| `dictation_sound` | `true` | Short cue for a new recording, through the shared sound switch, volume and Do Not Disturb. Surface changes and settings never replay it. Set in **Settings > Dictation > Voice** (from version 26) |
| `dictation_music` | `"lower"` | **Settings > Dictation > Voice > Music while you speak**: `"lower"` caps already playing MPRIS players at 25% without raising quieter players; `"pause"` pauses them; `"leave"` opens no media-control connection. Restore each untouched player's own volume/playback after capture, cancellation or failure. User changes and crashed/restarted players are left alone. Local controls only, no track metadata/network, separate from Now playing; Chat and future meetings bypass it. Captured at the next recording (from version 33) |
| `dictation_cancel_by_voice` | `true` | **Settings > Dictation > Keyboard shortcuts > Cancel by voice**: discard an independent final transcript that consists of or ends with a whole cancel phrase in English, Portuguese or Spanish. Runs after recording ends, before code conversion/delivery, on X11 and Wayland. Copies/inserts/submits nothing and stores no history or totals; keeps the previous success. Replay and Chat bypass it. Captured at the next recording (from version 32) |
| `dictation_spoken_code` | `true` | **Settings > Dictation > Paste > Terminals & sessions > Spoken code**: convert English/Portuguese/Spanish spoken punctuation between identifier chunks only for recognized terminal/session targets. Preserves case, existing punctuation and control bytes; never adds Enter. Prose apps and unknown targets keep their text. Captured for the next recording; History keeps heard and written text, and replay is not reformatted (from version 31) |
| `dictation_cut_repeats` | `true` | **Settings > Dictation > Voice > Remove repeated loops**: collapse long repetition loops in both preview and final independent dictation. Off preserves repetitions; known sound annotations and standalone silence phrases are still removed. Chat keeps its existing filter. Captured at the start of each recording (from version 30) |
| `dictation_retry_base` | `true` | **Settings > Dictation > Voice > Retry empty dictation with Base**: allow one visible retry only for empty text when speech was detected and a different Base model is installed. Off skips that extra speech check and retry; errors are not retried. Never downloads a model. Captured at the start of each independent recording (from version 30) |
| `dictation_vocabulary` | `false` | Optional local dictionary context for independent dictation, capped at 224 tokens including the coding seed and whole entries. Currently qualified only for Turbo with fixed English (chosen or resolved from the system), on synthetic WER/silence fixtures; Base, Small, Portuguese, other languages and automatic detection keep their existing prompts. Preview/final share the recording snapshot; Base retry checks its actual model and stays unprimed. Never includes replacement/snippet bodies or changes Chat. Set in **Settings > Dictation > Vocabulary > Help the model with these words** (from version 34) |
| `dictation_project_vocabulary` | `false` | Independently opt in to local project names for dictation, under the same Turbo/fixed-English gate and shared 224-token cap. Read known Git working trees on session start/repository refresh: Cargo/npm package and dependency names, shallow workspace members, module/file names and branch; no source bodies or commands. Prefer the focused session's working folder; otherwise require an unambiguous known project in a terminal/editor title. Personal dictionary words have priority. Cache in memory, keep worktrees separate, freeze the first decode's context for preview/final; missing/late/ambiguous context adds nothing. Set in **Settings > Dictation > Vocabulary > Project vocabulary** (from version 35) |
| `dictation_microphone` | absent | The input name chosen in **Settings > Dictation > Voice** for independent dictation. Absent follows the system default. A missing saved input falls back to the default or another input without changing the preference; a listed input that cannot open reports an error. Chat keeps its own default. Listing/refreshing does not record; changes apply to the next capture (from version 29) |
| `dictation_double_tap` | `true` | **Settings > Dictation > Keyboard shortcuts > Recording mode**: allow double tap for hands-free capture. Off, short taps are discarded immediately; holding for at least 300 ms still works. Independent of Space-lock. Changes apply to the next recording (from version 28) |
| `dictation_space_lock` | `true` | Space keeps a held recording running hands-free on X11. Off, Space follows ordinary typing cancellation and does not reserve the Open-island shortcut. Unavailable on Wayland; requires a recognized X11 key listener. Changes apply to the next recording (from version 28) |
| `dictation_escape_cancel` | `true` | Esc cancels capture, its tail or transcription on X11. Off, Esc leaves the turn running. The separately bound Cancel voice shortcut always works. Unavailable on Wayland. Changes apply to the next recording (from version 28) |
| `dictation_keep` | `"30"` | Dictation transcript retention: `"0"` (not kept), `"7"`, `"30"`, `"365"`, or `"all"`. Set in **Settings > Dictation > Privacy**. Shortening retention erases old text immediately; history open and recording prune it too. Daily/app totals survive erasure and Not kept. Audio and previous clipboard contents are never stored (from version 27) |
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
| `chat_access` | `"ask"` | Who answers Zeca's cards ([ADR 0004](../adr/0004-a-human-answers-permissions.md)): `ask` (a card each time), `policies` (your accepted policies answer what they cover) or `full` (everything runs with no card, but destructive commands and what a policy denies or asks first; cards come back once a conversation reads from outside). Changed in **Settings → Chat & voice**; a value it can't read asks. Holds at once |
| `memory` | `false` | Zeca's memory and journal; off, nothing of it reaches a model ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `journal_summary` | `false` | With `memory` on, **A summary of each day** of the journal, by Claude Code's smallest model ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `local_embeddings` | `false` | With `memory` on, **Recall by meaning**, through an Ollama on `127.0.0.1` ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `memory_suggest` | `false` | With `memory` on, **Suggest what to remember** from the conversation you left ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `sleep` | `false` | With `memory` on, **Let Zeca sleep on it**: his nightly sleep also asks a model for merges and a debrief ([Zeca's memory](../guide/memory.md)) (from version 18) |
| `journal_summary_since` | absent | The day `journal_summary` was turned on (`YYYY-MM-DD`): the journal kept before it never goes to the model (from version 18) |
| `fold_after` | `15` | Seconds the open island waits, once the pointer leaves, before folding (5 to 120) |
| `open_on_hover` | `false` | Resting the pointer on the pill opens the island, without a click; opened that way it folds as soon as the pointer leaves, unless you clicked in it. Not by the panel. Set from **Settings → General** (from version 11) |
| `monitor` | absent | The screen the island sits on, as maker and model (`"Samsung Electric Company LS27AG32x"`); absent lets the desktop choose. Two identical screens share a name, and the first one wins |
| `nest_size` | absent | The Nest's window as you last left it, `[width, height]` in logical pixels; absent, it opens at its default size (from version 21) |
| `rules` | `[]` | Always-allow rules: `{ "agent", "cwd", "tool", "target" }`, each matched exactly |
| `zeca_species` | `"atratus"` | Zeca's species, by id (`atratus` is the black vulture; the ids are in `ui/src/character/flock/species.ts`); an unknown one draws the black vulture |
| `zeca_look` | `"auto"` | What Zeca wears: `auto` (the calendar's look: `witch-hat` Oct 1 to Nov 1, `santa-hat` Dec 1 to 26, `party-hat` Dec 31 to Jan 2, `bunny-ears` Good Friday to Easter Monday), `none`, or one of those or an all-year look (`sunglasses`, `west-coast`, `fitted-cap`, `mountain-hat`, `headband`, `dreads`, `front-knot`, `durag`, `crown`, `bucket-hat`, `clock-chain`, `headphones`, `shutter-shades`, `chrome-chain`, `eye-patch`). An unknown one is `auto` |
| `flock` | `"brazil"` | Where the other sessions' birds are drawn from: `brazil` (Brazil's vultures), `americas` (with both condors) or `world` (every vulture). Each project draws its species from it, the same at every start; changing it draws every project again. The king vulture comes by role either way |
| `presence` | `"island"` | The presence preset: `island` (the flock at the top of the screen), `panel` (Zeca in the panel's tray; the island opens by the panel when you click him or a card needs you), `quiet` (nothing at rest; a card still opens the island with its sound) or `paused` (cards go to the agents' terminals at once, connectors stop, no notifications). Version 4 files hold `island` or `panel` and read as is. An unknown value is `island` |
| `zeca` | `true` | Zeca, the companion: off, no Chat or Chat microphone, and the tray has no *Chat…*; independent local dictation, the flock, cards, notifications and connectors keep working |
| `widget` | absent | The corner widget's corner: `top-left`, `top-right`, `bottom-left` or `bottom-right`; absent (the default) for no widget. An unknown value is no widget |
| `projects` | absent | Choices per project folder, set from a session's quick actions or **Settings → Projects**: `{ "/home/me/site": { "pin": true }, "/home/me/x": { "mute": true, "hide": true } }`. `mute`: no sounds or notifications from its sessions at rest (a card keeps both); `pin`: its sessions first; `hide`: its sessions off the island, the tray and the widget, except while one has a card waiting; `species`: its sessions' bird, a species id of the renderer (`"vultur"`), any but the king vulture's (`"papa"`), which is ignored like an unknown one (from version 12). Only the choices that are on are written, and a project with none is dropped (from version 8) |
| `budget_daily_cents` | absent | The daily budget, in US cents (`500` is $5.00), set in **Settings → Activity**: at 80% a notification, and once spent another, and Vults starts no new session that day. It never stops or blocks an agent. Absent, no budget (from version 20) |
| `budget_told` | absent | The budget's step already told today (`2026-10-11:1` for 80%, `:2` once spent), so a restart does not tell it again (from version 20) |
| `dnd_until` | absent | Do not disturb until then, in seconds since the Unix epoch: no sounds and no notifications at rest, no reminders; a card still opens the island with its sound (and in *Panel* its notification). Set from **Settings → General**; a time already past is off (from version 9) |
| `history` | `true` | Keep a local history of agent turns for **Settings → Activity**: each finished turn's counts (12 weeks) and each day's totals (a year), in `vults.sqlite` in the data folder. Off, nothing new is kept and what is there stays; **Clear history** removes all of it (from version 13), and the audit log with it |
| `recap_shown_week` | absent | The Monday of the last week whose recap card you read on the island, so it comes once (from version 14) |
| `theme` | `"dark"` | **Settings → General → Theme**, shared by every window: `dark`, the neutral `light`, or SP Night's `sp-night-paulista` (Noite Paulista), `sp-night-garoa` (Garoa) and `sp-night-jaragua` (Pico do Jaraguá). An unknown value uses Dark, and `black`, its name before version 45, reads as `dark` (from version 23) | <!-- check-english:allow -->
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
| OpenCode | `~/.config/opencode/plugins/vults.js` (under `$XDG_CONFIG_HOME` when set) | A plugin of ours, the whole file, that relays OpenCode's steps, permissions and questions through `vults-hook --agent opencode`. A file there without its *Generated by Vults* first line is yours and is never written |
| Qwen Code | `~/.qwen/settings.json` | One hook entry per event, running `vults-hook --agent qwen` |
| Copilot CLI | `~/.copilot/hooks/vults.json` | A file of ours, one hook per event running `vults-hook --agent copilot`. Copilot reads every file in that folder; the others are never touched |
| Cursor | `~/.cursor/hooks.json` | One hook per event, running `vults-hook --agent cursor <event> \|\| exit 0`; none of the hooks that run before a step, whose output Cursor reads as a decision |
| Factory Droid | `~/.factory/settings.json` | One hook entry per event, running `vults-hook --agent droid` |
| Kimi Code | `~/.kimi-code/config.toml` | One `[[hooks]]` table per event, running `vults-hook --agent kimi`; your comments and layout stay |

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
starts, and keeps only these seven; a shell that takes more than two seconds is given up on, and
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
