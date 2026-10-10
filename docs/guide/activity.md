# Activity

**Settings → Activity** shows what your agents did: a week's recap and the last year as a grid. It
is counted on this computer, from a local history that Vults keeps as your agents work.

## What a turn is

A turn is what one prompt set going: it starts with the prompt (or the first step or card of an
agent that sends no prompt event) and ends when the agent stops, fails, or its session ends. A
session that dies without a word, or a turn with no event for two hours, ends at its last event.

For each finished turn Vults keeps when it ended and on which day, how long it ran, the agent, the
project's folder name, and counts: steps, commands, files and lines its edits changed, the
permissions you allowed or denied and the questions you answered (a rule you made with **Always**
counts as your Allow), and whether it failed. Never a prompt, a reply, a command, a file name or a
path, and nothing leaves this computer ([Safety](../safety.md)).

## The week

The card shows one week, Monday to Sunday, by your local days: this week first, and **‹ ›** to go
through the weeks the history keeps (12).

- **Time with your agents**: turns running side by side count once, so two sessions working the
  same hour make one hour.
- **Turns** and their steps, **lines** added and removed and the files they touched, **commands**,
  **answers** (allowed, denied, questions answered), and **turns failed**.
- Time per day, as bars, and the agent and project with the most turns, the busiest day and the
  longest turn.

## The grid

The last year, a column per week and a row per day, Monday at the top. A day's shade follows its
time with your agents: none, then four steps by the quartiles of your active days, as GitHub's
contribution graph does. Hover a day for its turns and time.

## The history

- **Keep a history** is on from the start. Off, nothing new is kept and what is there stays.
- **Clear history…** asks once, then removes every kept turn and day.
- The files are `history.jsonl` (one line per turn, 12 weeks) and `days.json` (each day's totals,
  a year), in `~/.local/share/vults/` (`%LOCALAPPDATA%\Vults\` on Windows). See
  [Settings and files](../reference/settings.md).
