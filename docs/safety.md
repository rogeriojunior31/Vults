# Safety

What Vultures AI promises:

- **It never blocks your agent.** If the app is closed, slow or broken, the hook exits within
  milliseconds and prints nothing, and your agent asks in its terminal as usual.
- **Only a click approves a tool call.** No timer or default ever answers a permission request; the
  only exception is a rule you created yourself with **Always**, for that exact command or file.
- **Only your user account can talk to it.** The socket lives in your private runtime folder with mode
  `0600`, and both ends check the other process runs as you (on Windows, by SID).
- **It never edits an agent's config behind your back.** Every change gets a dated backup and a diff you
  approve; hooks from other tools are kept; Codex's `trusted_hash` is never written (you trust our hooks yourself, in Codex's `/hooks`).
- **The chat uses the CLIs you logged into** (`claude`, `codex`); Vultures AI never reads their
  credentials. In the chat, every command and every edit waits for your Allow; a card nobody
  answers is a no. Dropped files are copied into an inbox and deleted after a week.
- **API keys stay in your OS keyring.** If you give the chat a provider's API key, it is written
  only to the keyring (Secret Service on Linux), never to a file or a log, never sent back to a
  window, and only ever sent to that provider. Local models (Ollama, LM Studio) are reached on
  `127.0.0.1` only. The API chat has no tools: it can't run commands or edit files.
- **Secrets stay in your OS keyring**, and there is no telemetry.
