# Activity

The Nest's **Activity → Week** shows what your agents did: a week's recap and the last year as a
grid; **Settings → Activity** shows the same page. It
is counted on this computer, from a local history that Vults keeps as your agents work.

## What a turn is

A turn is what one prompt set going, until the agent stops. For each one Vults keeps when it ran,
the agent, the project's folder name and counts (steps, commands, lines changed, your answers).
Never a prompt, a reply, a command, a file name or a path, and nothing leaves this computer
([Safety](../safety.md)).

## The week

The card shows one week, Monday to Sunday, by your local days: this week first, and **‹ ›** to go
through the weeks the history keeps (12).

- **Time with your agents**: turns running side by side count once.
- **Turns** and their steps, **lines** added and removed and the files they touched, **commands**,
  **answers** (allowed, denied, questions answered), and **turns failed**.
- Time per day as bars, the agent and project with the most turns, the busiest day and the longest
  turn.

## Sharing the week

**Save as image…** under the week draws it as a picture (1080 × 1350), with Zeca on his wire.
**Hide project names** leaves the project out of it. The desktop's *Save as* dialog asks where (on
Windows it goes to your Pictures folder), and nothing is uploaded.

## On Monday morning

From 08:00 on Monday, the island tells last week in one line (*Last week: 41 turns, 6 h 20 min
with your agents, most on site.*), once, if your agents had a turn that week. It shows as the line
after a screen lock does in each preset ([The island](island.md#while-you-were-away)), never in
*Paused*. **Open Activity** shows the whole week in Settings; it and **×** both mark it read.

## The grid

The last year, a column per week and a row per day, Monday at the top. A day's shade follows its
time with your agents, in four steps by the quartiles of your active days. Hover a day for its
turns and time.

The **GitHub** tab shows your GitHub contribution calendar instead, with GitHub's own levels and its
weeks from Sunday, as on your profile. It needs the GitHub connector on (**Settings → Connectors**)
and is asked through `gh` once when the tab opens, then kept an hour in memory, never on disk.

## The history

- **Keep a history** is on from the start. Off, nothing new is kept and what is there stays.
- **Clear history…** asks once, then removes the turns and days, the audit log of your answers,
  the local trace of requests (Zeca's turns and every request made on your behalf), Zeca's saved
  conversations, his journal, the day summaries and their recall data, the memory proposals
  waiting, and the sleep's runs ([Safety](../safety.md)). What Zeca remembers (his memory files,
  **Settings → Chat → Memory**) stays.
- The history lives in one local database, `vults.sqlite` (each turn for 12 weeks, each day's
  totals for a year), in `~/.local/share/vults/` (`%LOCALAPPDATA%\Vults\` on Windows). See
  [Settings and files](../reference/settings.md).
