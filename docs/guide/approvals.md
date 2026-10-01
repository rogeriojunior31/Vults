# Approving from the island

When Claude Code or Codex asks permission for a tool call, the island opens on a card that shows
exactly what **Allow** authorizes: the command, the file or the URL, not just the tool's name.

- **Allow** and **Deny** answer the agent directly; it carries on at once.
- Only a click answers. No timer, rule or default ever approves anything.
- One card at a time. A second request while one is open goes back to its terminal, as if Vultures AI
  were not running, so nothing waits on a card you cannot see.
- If you answer in the terminal instead, or the agent moves on, the card goes away by itself.
- A card nobody answers expires with the agent's own wait (about two minutes) and the terminal asks.
- When the agent asks you a question rather than a permission, it stays in the terminal: the island
  only tells you there is a question and takes you there.

If the app is closed or not responding, the hook answers nothing within milliseconds and every agent
asks in its terminal as usual.
