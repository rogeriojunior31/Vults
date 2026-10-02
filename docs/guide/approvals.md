# Approving from the island

When Claude Code or Codex asks permission for a tool call, the island opens on a card that shows
exactly what **Allow** authorizes:

- the agent's own words for it, when it gives them (*Run the test suite, then the linter*);
- the whole command, the file or the URL, not just the tool's name. A long command shows in full
  (up to four lines, then it scrolls);
- for an edit, the lines it adds and removes (**+12 −3**). Patches from Codex show their files and
  counts too.

The card takes the whole width of the island until you answer. Then it says what happened for a
moment (**Allowed** in green, **Denied** in red) before the next thing shows.

## Answering

- **Allow** and **Deny** answer the agent directly; it carries on at once.
- **Ctrl+Alt+Y** and **Ctrl+Alt+N** do the same from anywhere, without leaving the window you are in.
  They are global shortcuts through the desktop portal: KDE asks you once to accept them, and you can
  change the keys in **System Settings → Shortcuts**. The buttons show the keys actually bound. A
  shortcut only answers a card that is on screen (an agent's, or one in the chat); with nothing
  waiting, it does nothing.
- **Always allow** allows the request and every identical one from then on: the same agent, the same
  tool and the exact same command, file or URL, in the same project folder. `cargo test` does not
  cover `cargo test && rm -rf build`, nor the same command in another project. Those requests are
  answered at once, without a card, and the step says *always allowed* on the island. An identical
  request already waiting is answered too. **Settings → Approvals** lists every rule, and **Remove**
  takes one away.
- Only you answer: with a click or a shortcut now, or with **Always allow** earlier. No timer or
  default ever approves anything.

## More than one at a time

Requests wait in line, each for its own agent. The card shows the first one with **1 of 3**; the
sessions waiting behind it say *Needs you* in the flock list, with an amber badge. Answer one and the
next comes up.

A subagent working in parallel does not take a card away: only the agent that asked moving on does.

## When nobody answers

A request waits as long as its agent does (a little under two minutes). During the last 30 seconds
the card counts down, *Goes back to the terminal in 0:25*, and then the terminal asks as if Vultures AI
were not there. If you answer in the terminal instead, the card says so and goes away.

## Auto mode and questions

- In auto mode, the classifier clears most permission requests in a blink: the island only shows a
  card, plays a sound or opens for a request still waiting after a moment, so it does not blink on
  every tool call. The same goes for "finished": only a session that stays done is news.
- When the agent asks you a question rather than a permission, it stays in the terminal: the island
  shows the question and takes you there.

If the app is closed or not responding, the hook answers nothing within milliseconds and every agent
asks in its terminal as usual.
