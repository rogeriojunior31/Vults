# Activity

Week compares its figures with the preceding calendar week, shows recorded turns by agent and
labels the year by month and weekday. A missing or zero comparison baseline is stated explicitly.
An unreadable history is an error with retry, never an empty dashboard. **History settings**
opens Settings; history retention and clearing stay there. The recap image has its own
**Hide project names** export option.

The Nest's **Activity → Week** shows what your agents did: a week's recap and the last year as a
grid. **Settings → Activity** keeps the history and daily budget controls and an
**Open Activity** button; the recap and calendar have one home in the Nest. Activity is counted on this computer,
from a local history that Vults keeps as your agents work.

The Nest groups Activity into seven views, with the same navigation and summary style:

- **Week**: totals, time per day, the yearly grid and a recap to save. Select a day to read its
  exact duration; **How activity is counted** explains the counting rules.
- **Dictation**: measured words, estimated time saved, speed, daily bars and top applications.
  Week also shows dictated words for its selected calendar week, independent of agent history.
- **By project**: time per repository, worktrees, sessions, pull requests and project memory.
- **History**: finished turns grouped by project, with the result, agent, counts and duration.
- **Cost**: today's budget, plan windows, the last two weeks of cost and tokens, and cost by project.
- **Traces**: Zeca's chat turns and their tool calls, with status, time and available token counts.
- **Audit**: who answered, the result and the complete command or target, grouped by project.

History and Audit show the latest 200 records. Their summaries describe the records loaded in
that view, and searching filters the list without changing those totals. Project headings show
the number of matching records. Navigation, search and resizing do not modify the stored history.

The island's compact **Week** preview uses the same summary style: the week and its dates at the
top, four totals and time by project (up to four projects), or a small daily chart when no project
is named. The arrows switch between the previous week and this week; **Open in the Nest** opens
the full Activity view. Hours in the top summary are rounded; hover that figure for the full
duration. An unread week shows a loading or error message instead of zero totals.

## Dictation statistics

Open **Activity → Dictation**, or **Dictation → Statistics**. Choose **Last 7 days**, **Last 30 days**
or **All time**. Local usage totals stay available after transcript erasure, expiry or **Do not keep**;
reusing a dictation does not count again. Empty periods show unknown figures, with explicit
loading, retry and stale-data feedback when a read fails.

Time saved is `max(0, words / 45 - audio seconds / 60)` minutes. Speed uses measured audio duration,
with zero below one second. Daily bars use local calendar dates; all-time figures include every
recorded day while the chart shows at most the latest 90 calendar days. Select a bar to read it.
The top eight applications are ordered by words, with an explicit unknown application when needed.

Migration 24 starts independent daily application totals. Older lifetime application totals are
preserved; a weekly/monthly breakdown missing those old measurements is labeled partial.
No transcript, session or window title is added to these usage aggregates. Retained stage timings
still follow transcript retention and appear in
[Settings > Dictation > Diagnostics](dictation.md#diagnostics). Usage totals and memory-only session
counters have separate lifetimes. **Dictation settings** opens that configuration page. Room/Full keep the selected statistics period.

![Dictation statistics with synthetic usage in the native app](../assets/dictation-statistics.png)

Native validation example with synthetic totals; these figures are not the reader's usage.

## Cost and a daily budget

**Activity → Cost** shows today's recorded spend, plan windows and costs by day and project.
Subscription costs are shown at API prices; they are not an extra charge to your plan.
Missing measurements stay unknown. Existing spend history remains readable.

**Budget settings** opens **Settings → Activity**, the only place to edit the daily budget.
Enter US dollars, from $0.01 to $10,000, or leave it empty for no budget. Saving disables the
controls until the write finishes; a failed write keeps your draft for another attempt.
At 80% Vults notifies you; once spent, it notifies you again and starts no new session until
tomorrow, including `/continue`. It never stops a running session or blocks an agent you started.
Zeca's Chat, summaries and sleep have their own limits and are not counted in this budget.

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
*Paused*. **Open Activity** shows the whole week in the Nest; it and **×** both mark it read.

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
  conversations, his journal, the day summaries and their recall data, the memory proposals and
  the sleep's runs, saved dictation transcripts and the snapshot of the flock kept for Zeca's MCP
  server, in one go, leaving one line in the audit log: that you cleared it, and when.
  What stays: what Zeca remembers (his memory files, **Settings → Chat & voice → Memory**), the
  policies you accepted, your paired phones, recorded agent costs, measured dictation totals and
  the user-maintained dictionary, replacements and snippets
  ([Safety](../safety.md)). Tables from retired features stay untouched; their old cost rows
  remain readable in **Cost**.
- The history lives in one local database, `vults.sqlite` (each turn for 12 weeks, each day's
  totals for a year), in `~/.local/share/vults/` (`%LOCALAPPDATA%\Vults\` on Windows). See
  [Settings and files](../reference/settings.md).
