# Approving from the island

When Claude Code or Codex asks permission for a tool call, the island opens on a card that shows
exactly what **Allow** authorizes: the command, the file or the URL, not just the tool's name.

- **Allow** and **Deny** answer the agent directly; it carries on at once.
- **Ctrl+Alt+Y** and **Ctrl+Alt+N** do the same from anywhere, without leaving the window you are in.
  They are global shortcuts through the desktop portal: KDE asks you once to accept them, and you can
  change the keys in **System Settings → Shortcuts**. The buttons show the keys actually bound. A
  shortcut only answers a card that is on screen (an agent's, or one in the chat); with nothing
  waiting, it does nothing.
- In auto mode, the classifier clears most permission requests in a blink: the island only shows a
  card, plays a sound or opens for a request still waiting after a moment, so it does not blink on
  every tool call. The same goes for "finished": only a session that stays done is news.
- Only a click answers. No timer, rule or default ever approves anything.
- One card at a time. A second request while one is open goes back to its terminal, as if Vultures AI
  were not running, so nothing waits on a card you cannot see.
- If you answer in the terminal instead, or the agent moves on, the card goes away by itself.
- A card nobody answers expires with the agent's own wait (about two minutes) and the terminal asks.
- When the agent asks you a question rather than a permission, it stays in the terminal: the island
  only tells you there is a question and takes you there.

If the app is closed or not responding, the hook answers nothing within milliseconds and every agent
asks in its terminal as usual.
