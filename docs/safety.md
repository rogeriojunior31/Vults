# Safety

What Vults promises:

- **It never blocks your agent.** If the app is closed, slow or broken, the hook exits within
  milliseconds and prints nothing, and your agent asks in its terminal as usual.
- **Only a click approves a tool call.** No timer or default ever answers a permission request; the
  only exception is a rule you created yourself with **Always**, for that exact command or file.
  A question from Claude Code is answered only with the choice or the words you gave on its card.
- **Only your user account can talk to it.** The socket lives in your private runtime folder with mode
  `0600`, and both ends check the other process runs as you (on Windows, by SID).
- **It never edits an agent's config behind your back.** Every change gets a dated backup and a diff you
  approve; hooks from other tools are kept, and a Claude Code status line of your own is saved beside the hook, keeps running, and comes back as it was when you remove ours; Codex's `trusted_hash` is never written (you trust our hooks yourself, in Codex's `/hooks`).
- **Plan usage comes from the CLIs only.** Claude Code's status line input is cut down in the hook
  to its `rate_limits` before anything leaves it; Codex is asked with a read-only call, only while
  **Settings → General → Codex usage** is on.
- **The chat uses the CLIs you logged into** (`claude`, `codex`); Vults never reads their
  credentials. In the chat, every command and every edit waits for your Allow; a card nobody
  answers is a no. Dropped files are copied into an inbox and deleted after a week.
- **API keys stay in your OS keyring.** If you give the chat a provider's API key, it is written
  only to the keyring (Secret Service on Linux), never to a file or a log, never sent back to a
  window, and only ever sent to that provider. Local models (Ollama, LM Studio) are reached on
  `127.0.0.1` only. The API chat has no tools: it can't run commands or edit files.
- **The microphone only listens while you hold a recording.** Voice is off until you download a
  model; then the mic records into memory only while its button is red (a minute at most), and
  whisper.cpp transcribes it on this computer. The audio is never saved or sent anywhere.
- **Now playing is off until you turn it on.** Then the app reads your media players over the
  session bus (MPRIS on Linux) to show the song; it is never stored or sent anywhere.
- **The edits it shows stay in memory.** For the island's diff, the hook forwards a finished edit's
  changed lines (400 at most); the app keeps them for the session's last few steps only, and never
  writes them to a file or a log.
- **What the app asks on its own goes through one gate.** A request made without a click at that
  moment (a connector's check, such as GitHub's, and the Codex usage read) asks the app's network
  gate first: only a feature you switched on, at most 3,000 checks in any 24 hours for a connector
  and 400 for the usage read, each one kept as a line in `vults.sqlite` (when, which feature,
  whether it answered; 30 days). A connector over that waits and says so in Settings → Connectors.
- **Connectors keep only their last answer.** GitHub's last check (titles, links, check and review
  states of your open pull requests and recent repositories) is saved in the app's data folder,
  `connectors/github.json`, so a restart does not replay old news. Your contribution calendar, for
  Settings → Activity, is asked only when its tab opens and kept an hour in memory. Nothing else
  is stored, and it is never sent anywhere.
- **The activity history is counts, on this computer.** For Settings → Activity, each finished
  turn of an agent is kept in the data folder (`vults.sqlite`: each turn for 12 weeks, each day's
  totals for a year): when it ended, how long it ran, the agent, the project's folder name, and
  counts (steps, commands, files and lines changed, your answers). Never a prompt, a reply, a
  command, a file name or a path. It is never sent anywhere; turn it off or clear it in Settings →
  Activity.
- **Zeca's turns are traced, on this computer.** Each chat turn and each tool it asked to run is a
  span in `vults.sqlite`, kept 30 days: the engine and model, when it started and ended, the tokens
  in and out, the prompt cache, the cost when the engine reports it, and a tool's name and target
  with secrets redacted. Never a prompt or a reply. Nothing of it leaves the computer; **Clear
  history** removes it too.
- **Every answer is on record, on this computer.** An audit log in the same `vults.sqlite` keeps,
  for 90 days, each answer to a card, the flock's and Zeca's chat's: who gave it (you, one of your
  *Always* rules, or the app when a card's time ran out or *Paused* sent it to the terminal), what
  it did (allow, deny, always allow, answer, send to the terminal), the agent, the project's folder
  name, the tool and the exact command or path it was about, with tokens and `NAME=secret` values
  redacted first (never what follows them). An answer given in the agent's own terminal is not
  Vults', and is not in it. Each agent config the installer writes is kept too. A line is never changed, and none leaves before 90 days, except through **Clear history** in
  Settings → Activity, which removes it with the history. It is always on, and never sent anywhere.
- **Secrets stay in your OS keyring**, and there is no telemetry.

## Every request Vults makes

Nothing else reaches the network. CI checks it: `scripts/check-network.sh` fails on a crate that
depends on an HTTP client, or a file that names one or starts a network command, outside
`scripts/network-allowlist.txt`, which names each one and why; and on a webview allowed to connect
anywhere but the app.

| Request | Who starts it | What turns it on | Where |
|---|---|---|---|
| A chat turn through an API (Anthropic, OpenAI, Gemini, OpenRouter, Groq, DeepSeek, Mistral, xAI; Ollama and LM Studio on `127.0.0.1`) | You, with each message | The provider and key you chose; sent only to that provider | `crates/chat/src/providers.rs` |
| The list of a provider's models | You, opening **Settings → Chat** with a key saved (or a local Ollama or LM Studio), or with **Try again** | The provider and key you chose | `crates/chat/src/providers.rs` |
| A chat turn through `claude` or `codex` | You, with each message | The engine you chose; the CLI talks to its own service on your login | `crates/chat/src/claude.rs`, `codex.rs`, `codex_server.rs` |
| The Codex usage read (`account/rateLimits/read`, through `codex app-server`) | The app, every 5 minutes, never while the screen is locked | **Settings → General → Codex usage**, through the network gate (400 a day) | `app/src/usage.rs` |
| A connector's check (GitHub, through `gh`) | The app, on its own schedule | Each connector's switch in **Settings → Connectors**, through the network gate (3,000 a day) | `crates/connectors/src/github.rs` |
| Your GitHub contribution calendar (through `gh`) | You, opening **Settings → Activity**'s GitHub tab | The GitHub connector on | `crates/connectors/src/github.rs` |
| A voice model's download | You, with a click in **Settings → Chat** | Nothing until you click; the file is checked against its SHA-256 | `crates/voice/src/models.rs` |
| Claude Code's plan usage | Claude Code, which hands it to our status line | No request of ours | the hook |
| The update check | Not built yet (ADR 0016) | It will be off until you turn it on, through the gate | — |
