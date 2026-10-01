# The island

The island hangs from the top edge of your screen. On KDE Plasma, Hyprland, Sway and other compositors
with layer-shell it sits above every window like a panel; on GNOME it is a regular always-on-top window.

## The flock

Each agent session is a black vulture on the wire. **Zeca**, the big one, stands for the session in
front: the one that needs you, the one you clicked, or the most recent one. Every other session is a
**vult**. A small band at the base of each neck shows the agent: orange for Claude Code, teal for
Codex.

The birds do what their sessions do: Zeca lowers his head to read, pecks the wire while editing, tugs
at it while a command runs, takes off and circles when the agent goes to the web, and spreads his
wings when a permission waits for you. New sessions fly in and land; finished ones fly off. A bird
left idle for 90 seconds fluffs up and dozes. See [Animations](../ANIMATIONS.md) for every clip and the
behavior it comes from.

## Folded and open

Folded, the island is just the wire and one line: the session in front and its current step. It
opens when:

- you hover it (it waits 200 ms, and folds back 600 ms after you leave);
- there is news: a session finished, failed or asks something, or a connector alert arrives (it
  opens for about five seconds);
- a permission card is waiting, or the chat is open.

Open, it shows the **Flock** and **Chat** tabs, a sound toggle and the settings button, a name under
each bird, and the card of the session in front:

| Session | Card |
|---|---|
| Working | The step ticker: the step just done, and the current one |
| Needs permission | The exact command, file or URL, with **Deny** and **Allow** |
| Has a question | A reminder to answer in the terminal, and **Open terminal** |
| Finished | The last step, **OK** and **Open terminal** |
| Failed | The error, and **Open terminal** |

## Getting to a session

Click a bird or its name to put that session in front. Click a session's line or **Open terminal** to
bring its terminal forward: the multiplexer pane first (herdr, tmux, kitty, wezterm), then, on KDE,
the terminal window itself.

## Sounds

Short 8-bit blips when a session needs you, finishes or fails, and for connector news. The speaker
button on the island, or **Settings → General → Sounds**, turns them off.
