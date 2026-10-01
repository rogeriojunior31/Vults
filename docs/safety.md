# Safety

What Vultures AI promises:

- **It never blocks your agent.** If the app is closed, slow or broken, the hook exits within
  milliseconds and prints nothing, and your agent asks in its terminal as usual.
- **Only a click approves a tool call.** No timer, rule or default ever answers a permission request.
- **Only your user account can talk to it.** The socket lives in your private runtime folder with mode
  `0600`, and both ends check the other process runs as you (on Windows, by SID).
- **It never edits an agent's config behind your back.** Every change gets a dated backup and a diff you
  approve; hooks from other tools are kept; Codex's `trusted_hash` is never written (you trust our hooks yourself, in Codex's `/hooks`).
- **The chat uses the CLIs you logged into** (`claude`, `codex`); Vultures AI never reads their
  credentials. In the chat, every command and every edit waits for your Allow; a card nobody
  answers is a no. Dropped files are copied into an inbox and deleted after a week.
- **Secrets stay in your OS keyring**, and there is no telemetry.
