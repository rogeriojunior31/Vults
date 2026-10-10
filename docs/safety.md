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
  to its `rate_limits` before anything leaves it; Codex is asked with a read-only call.
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
- **Connectors keep only their last answer.** GitHub's last check (titles, links, check and review
  states of your open pull requests and recent repositories) is saved in the app's data folder,
  `connectors/github.json`, so a restart does not replay old news. Your contribution calendar, for
  Settings → Activity, is asked only when its tab opens and kept an hour in memory. Nothing else
  is stored, and it is never sent anywhere.
- **The activity history is counts, on this computer.** For Settings → Activity, each finished
  turn of an agent is kept in the data folder (`history.jsonl`, 12 weeks; `days.json`, each day's
  totals for a year): when it ended, how long it ran, the agent, the project's folder name, and
  counts (steps, commands, files and lines changed, your answers). Never a prompt, a reply, a
  command, a file name or a path. It is never sent anywhere; turn it off or clear it in Settings →
  Activity.
- **Secrets stay in your OS keyring**, and there is no telemetry.
